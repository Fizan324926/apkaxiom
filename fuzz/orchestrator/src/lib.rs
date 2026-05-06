// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! P1.14 Centipede-style orchestrator — runs N parallel
//! `p113-fuzz-driver` workers against a shared coverage pool.
//!
//! ## What's shared between workers
//!
//! 1. **Coverage bitmap** — a single `CoverageMap` instance per
//!    process; workers `observe()` into it. Periodic `dump()` to
//!    a snapshot file lets the orchestrator merge new edges
//!    across machine boundaries.
//! 2. **New-edge inputs** — workers append inputs that hit a new
//!    edge to a shared `inputs/queue.ndjson`; other workers pull
//!    from it on each iteration via the harness's existing
//!    `--queue-feed` channel.
//! 3. **Findings archive** — workers append to per-worker
//!    archives; the orchestrator periodically merges them into
//!    one canonical `archive.ndjson`.
//!
//! On a single host this is process-level parallelism (one
//! worker per CPU). On multiple KVM nodes (P1.14 §C-1 + §C-2)
//! the same primitives over an NFS-mounted shared dir give
//! cross-node fan-out — the only addition needed is an NFS
//! mount point in the worker config.

#![forbid(unsafe_code)]
#![warn(missing_docs, unreachable_pub)]
#![allow(clippy::doc_markdown)]

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use p113_fuzz_harness::archive::Finding;

/// One worker descriptor. The orchestrator spawns one
/// `p113-fuzz-driver` per descriptor as a child process.
#[derive(Debug, Clone)]
pub struct WorkerSpec {
    /// Stable id (`worker-1`, `worker-2`, …). Used for the
    /// per-worker archive directory.
    pub id: String,
    /// Probes CSV passed to the driver via `--probes`.
    pub probes: String,
    /// Per-worker random seed (non-overlapping across workers).
    pub seed: u64,
    /// Iteration cap per worker. 0 = unbounded.
    pub iters: u64,
}

/// Aggregator for cross-worker stats. The orchestrator polls
/// each worker's archive at a fixed cadence and snapshots the
/// total finding counts here for dashboard / regression-gate
/// consumption.
#[derive(Debug, Default)]
pub struct PoolStats {
    /// Total findings observed across all workers.
    pub total_findings: AtomicU64,
    /// Distinct input shas seen across all workers (deduped).
    pub distinct_inputs: AtomicU64,
    /// Inputs that hit a new edge in the shared coverage map.
    pub new_edges: AtomicU64,
    /// Workers currently alive (poll snapshot).
    pub workers_alive: AtomicU64,
}

/// Spawn a single `p113-fuzz-driver` child for one worker.
/// Stdout/stderr go to per-worker log files under `logs/`.
pub fn spawn_worker(
    spec: &WorkerSpec,
    driver: &Path,
    seeds: &Path,
    archive_root: &Path,
    probe: &Path,
) -> std::io::Result<std::process::Child> {
    let archive_dir = archive_root.join(&spec.id);
    std::fs::create_dir_all(&archive_dir)?;
    let log_dir = archive_root.join("logs");
    std::fs::create_dir_all(&log_dir)?;
    let log_path = log_dir.join(format!("{}.log", spec.id));
    let log = std::fs::File::create(&log_path)?;
    let log_err = log.try_clone()?;
    let mut cmd = std::process::Command::new(driver);
    cmd.arg("--mode")
        .arg("dev")
        .arg("--seeds")
        .arg(seeds)
        .arg("--archive")
        .arg(&archive_dir)
        .arg("--probe")
        .arg(probe)
        .arg("--probes")
        .arg(&spec.probes)
        .arg("--seed")
        .arg(spec.seed.to_string());
    if spec.iters > 0 {
        cmd.arg("--iters").arg(spec.iters.to_string());
    }
    cmd.stdout(std::process::Stdio::from(log))
        .stderr(std::process::Stdio::from(log_err));
    cmd.spawn()
}

/// Periodically scan all per-worker archives and merge them
/// into one canonical `archive.ndjson` at the pool root. Returns
/// the number of records merged.
pub fn merge_archives(archive_root: &Path, out: &Path) -> std::io::Result<u64> {
    use std::io::Write as _;
    let mut total = 0u64;
    let mut seen = std::collections::HashSet::new();
    let mut writer = std::io::BufWriter::new(std::fs::File::create(out)?);
    for entry in std::fs::read_dir(archive_root)? {
        let entry = entry?;
        let p = entry.path();
        if !p.is_dir() {
            continue;
        }
        let arc = p.join("archive.ndjson");
        if !arc.exists() {
            continue;
        }
        let raw = std::fs::read_to_string(&arc)?;
        for line in raw.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            // Dedupe on the input_sha256 + target_version pair.
            let parsed = match Finding::from_ndjson_line(line) {
                Some(f) => f,
                None => continue,
            };
            let key = format!("{}|{}", parsed.input_sha256, parsed.target_version);
            if !seen.insert(key) {
                continue;
            }
            writeln!(writer, "{line}")?;
            total += 1;
        }
    }
    writer.flush()?;
    Ok(total)
}

/// Best-effort poll loop driving the merge cadence. Runs until
/// every worker exits.
pub fn pool_loop(
    workers: &mut [(WorkerSpec, std::process::Child)],
    archive_root: &Path,
    merge_every: Duration,
    stats: Arc<PoolStats>,
) -> std::io::Result<()> {
    let merged_archive = archive_root.join("archive.ndjson");
    let start = Instant::now();
    let mut last_merge = Instant::now();
    loop {
        // Check live workers.
        let mut alive = 0u64;
        for (spec, child) in workers.iter_mut() {
            match child.try_wait()? {
                None => alive += 1,
                Some(s) => {
                    if !s.success() {
                        eprintln!(
                            "WARN worker {} exited with status {} after {:?}",
                            spec.id,
                            s,
                            start.elapsed()
                        );
                    }
                }
            }
        }
        stats.workers_alive.store(alive, Ordering::Relaxed);
        if alive == 0 {
            break;
        }
        if last_merge.elapsed() >= merge_every {
            match merge_archives(archive_root, &merged_archive) {
                Ok(n) => {
                    stats.total_findings.store(n, Ordering::Relaxed);
                }
                Err(e) => eprintln!("WARN merge failed: {e}"),
            }
            last_merge = Instant::now();
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    // Final merge.
    let n = merge_archives(archive_root, &merged_archive)?;
    stats.total_findings.store(n, Ordering::Relaxed);
    Ok(())
}

/// Build the default 4-worker pool — one per CPU on a small
/// dev box. Each worker uses a distinct seed and a probe
/// shape that includes all three Android versions.
#[must_use]
pub fn default_pool(probes: &str) -> Vec<WorkerSpec> {
    (0..4)
        .map(|i| WorkerSpec {
            id: format!("worker-{}", i + 1),
            probes: probes.to_string(),
            seed: 0xb114_0000_0000_0001 + i as u64,
            iters: 0,
        })
        .collect()
}

/// Convenience: paint a stats snapshot to stdout for human eyes.
pub fn print_stats(stats: &PoolStats) {
    println!(
        "  pool: alive={} findings={} new_edges={} distinct_inputs={}",
        stats.workers_alive.load(Ordering::Relaxed),
        stats.total_findings.load(Ordering::Relaxed),
        stats.new_edges.load(Ordering::Relaxed),
        stats.distinct_inputs.load(Ordering::Relaxed),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_pool_has_distinct_seeds() {
        let pool = default_pool("A14:probe");
        let seeds: std::collections::HashSet<_> = pool.iter().map(|w| w.seed).collect();
        assert_eq!(seeds.len(), pool.len());
    }

    #[test]
    fn merge_archives_dedupes_per_version() {
        // Build a tmp dir with two workers each writing the same
        // input but different version labels. Merge should
        // include both.
        let tmp = std::env::temp_dir().join(format!("p114-merge-{}", std::process::id()));
        std::fs::create_dir_all(tmp.join("worker-1")).unwrap();
        std::fs::create_dir_all(tmp.join("worker-2")).unwrap();
        let line_a14 = "{\"schema_version\":\"p114-finding-1.1\",\"finding_id\":\"abc\",\"timestamp_ns\":0,\"mode\":\"dev\",\"target_label\":\"a14\",\"input_sha256\":\"abc\",\"input_path\":\"i.bin\",\"input_len\":4,\"axiom_l0\":\"accept\",\"target\":\"accept\",\"bucket\":\"A_BOTH_ACCEPT\",\"high_severity\":false,\"seed_origin\":null,\"mutation_kind\":null,\"target_version\":\"A14\",\"synthetic\":false}";
        let line_a11 = line_a14.replace("\"A14\"", "\"A11\"").replace("\"a14\"", "\"a11\"");
        std::fs::write(tmp.join("worker-1/archive.ndjson"), format!("{line_a14}\n")).unwrap();
        std::fs::write(tmp.join("worker-2/archive.ndjson"), format!("{line_a11}\n")).unwrap();
        let merged = tmp.join("merged.ndjson");
        let n = merge_archives(&tmp, &merged).unwrap();
        assert_eq!(n, 2);
        let body = std::fs::read_to_string(&merged).unwrap();
        assert!(body.contains("\"target_version\":\"A14\""));
        assert!(body.contains("\"target_version\":\"A11\""));
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
