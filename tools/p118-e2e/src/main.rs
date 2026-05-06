// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p118-e2e` — Phase 1 end-to-end evaluation harness.
//!
//! Runs the full Phase-1 pipeline (ZIP parse → IR emit → signature verify →
//! BLAKE3 file digest) on every APK in a corpus directory and reports:
//!
//! - Per-APK latency histogram (p50 / p95 / p99)
//! - Peak RSS
//! - Single-core throughput
//! - Verdict summary (accept / reject / error counts)
//!
//! With `--json-out <file>`, emits deterministic NDJSON (sorted by filename,
//! no timing fields) for cross-architecture parity and reproducibility checks.
//!
//! ## Pipeline per APK
//!
//! 1. Read file bytes.
//! 2. BLAKE3 one-shot over all bytes → `file_blake3` (deterministic fingerprint).
//! 3. `Apk::from_reader` → ZIP structure + manifest/resources bytes captured.
//! 4. `emit_manifest` → `reencode_manifest` → SHA-256 of re-encoded bytes → `ir_sha256`.
//! 5. `verify_apk_bytes` → `verdict`.
//! 6. Record (file, verdict, ir_sha256, file_blake3) to NDJSON; elapsed_ms to histogram.

#![forbid(unsafe_code)]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::print_stdout,
    clippy::too_many_lines
)]

use std::io::{self, Write as IoWrite};
use std::path::{Path, PathBuf};
use std::time::Instant;

use axiom_blake3_hacl::{Blake3, Hasher as _};
use axiom_l1_rs::ir::emit as ir_emit;
use axiom_l1_rs::{Apk, Manifest, Unverified};
use axiom_l1_signing_verified::verify_apk_bytes;
use walkdir::WalkDir;

// ── Entry ────────────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let corpus_dir = parse_flag(&args, "--corpus").unwrap_or_else(|| {
        eprintln!(
            "Usage: p118-e2e --corpus <dir> [--json-out <file>] [--bench]"
        );
        std::process::exit(1);
    });
    let json_out = parse_flag(&args, "--json-out");
    let bench_mode = args.iter().any(|a| a == "--bench");

    let apks = collect_apks(Path::new(&corpus_dir));
    if apks.is_empty() {
        eprintln!("No APK files found in {corpus_dir}");
        std::process::exit(1);
    }
    println!("corpus: {} APKs in {corpus_dir}", apks.len());

    let mut latencies_ms: Vec<f64> = Vec::with_capacity(apks.len());
    let mut n_accept = 0usize;
    let mut n_reject = 0usize;
    let mut n_error = 0usize;

    // NDJSON records for parity output (no timing — deterministic across arches).
    let mut records: Vec<String> = Vec::with_capacity(apks.len());

    let wall_t0 = Instant::now();

    for apk_path in &apks {
        let t0 = Instant::now();

        let bytes = match std::fs::read(apk_path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("  SKIP {}: {e}", apk_path.display());
                n_error += 1;
                continue;
            }
        };

        // 1. BLAKE3 file digest.
        let file_blake3 = Blake3::hash_oneshot(&bytes);
        let blake3_hex = hex_32(&file_blake3);

        // 2. Parse ZIP + capture manifest bytes.
        let ir_sha256 = match Apk::<Unverified>::from_reader(std::io::Cursor::new(&bytes)) {
            Ok(apk) => compute_ir_sha256(&apk),
            Err(_) => "parse-err".to_string(),
        };

        // 3. Signature verification.
        let verdict = verify_apk_bytes(&bytes);
        let verdict_str = if verdict.is_accept() {
            n_accept += 1;
            "accept"
        } else {
            n_reject += 1;
            "reject"
        };

        let elapsed_ms = t0.elapsed().as_secs_f64() * 1000.0;
        latencies_ms.push(elapsed_ms);

        // NDJSON record: deterministic fields only.
        let fname = apk_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        records.push(format!(
            r#"{{"file":"{fname}","verdict":"{verdict_str}","ir_sha256":"{ir_sha256}","file_blake3":"{blake3_hex}"}}"#
        ));
    }

    let wall_elapsed = wall_t0.elapsed();

    // ── Write NDJSON ──────────────────────────────────────────────────────────
    if let Some(path) = &json_out {
        // Records are already sorted (APKs collected sorted) — just write.
        let content = records.join("\n") + "\n";
        if let Err(e) = std::fs::write(path, content) {
            eprintln!("ERROR writing {path}: {e}");
            std::process::exit(1);
        }
        println!("json-out: {path}");
    }

    // ── Report ────────────────────────────────────────────────────────────────
    let n_total = n_accept + n_reject + n_error;
    let throughput = n_total as f64 / wall_elapsed.as_secs_f64();

    latencies_ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p50 = percentile(&latencies_ms, 50.0);
    let p95 = percentile(&latencies_ms, 95.0);
    let p99 = percentile(&latencies_ms, 99.0);
    let p_max = latencies_ms.last().copied().unwrap_or(0.0);

    let rss_kb = peak_rss_kb();

    println!(
        "\nverdicts: {n_accept} accept  {n_reject} reject  {n_error} error  ({n_total} total)"
    );
    println!(
        "latency:  p50={p50:.1}ms  p95={p95:.1}ms  p99={p99:.1}ms  max={p_max:.1}ms"
    );
    println!(
        "throughput: {throughput:.0} APKs/sec  ({:.2}s wall)",
        wall_elapsed.as_secs_f64()
    );
    if let Some(kb) = rss_kb {
        let mb = kb / 1024;
        println!("peak RSS: {mb} MB ({kb} kB)");
        if bench_mode {
            if mb <= 150 {
                println!("PASS K3 peak-RSS gate: {mb} MB ≤ 150 MB");
            } else {
                eprintln!("FAIL K3 peak-RSS gate: {mb} MB > 150 MB");
                std::process::exit(1);
            }
        }
    }

    if bench_mode {
        // K2 latency gates (Bench-1K HARD per PHASE_GATES.md §5).
        let mut fail = false;
        if p99 > 300.0 {
            eprintln!("FAIL K2 p99 gate: {p99:.1}ms > 300ms");
            fail = true;
        } else {
            println!("PASS K2 p99 gate: {p99:.1}ms ≤ 300ms");
        }
        if p95 > 150.0 {
            eprintln!("FAIL K2 p95 gate: {p95:.1}ms > 150ms");
            fail = true;
        } else {
            println!("PASS K2 p95 gate: {p95:.1}ms ≤ 150ms");
        }
        if p50 > 50.0 {
            eprintln!("FAIL K2 p50 gate: {p50:.1}ms > 50ms");
            fail = true;
        } else {
            println!("PASS K2 p50 gate: {p50:.1}ms ≤ 50ms");
        }
        if fail {
            std::process::exit(1);
        }
    }

    let _ = io::stdout().flush();
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn compute_ir_sha256(apk: &Apk<Unverified>) -> String {
    let Some(mb) = apk.manifest_bytes() else {
        return "no-manifest".to_string();
    };
    let manifest = Manifest { axml_bytes: mb.to_vec() };
    match ir_emit::emit_manifest(&manifest) {
        Ok(ir) => {
            let reencoded = ir_emit::reencode_manifest(&ir);
            let digest = axiom_crypto_hacl::sha256(&reencoded);
            hex_32(&digest)
        }
        Err(_) => "ir-err".to_string(),
    }
}

fn collect_apks(dir: &Path) -> Vec<PathBuf> {
    let mut apks: Vec<PathBuf> = WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.path()
                    .extension()
                    .is_some_and(|x| x.eq_ignore_ascii_case("apk"))
        })
        .map(|e| e.path().to_owned())
        .collect();
    apks.sort();
    apks
}

fn parse_flag(args: &[String], flag: &str) -> Option<String> {
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == flag {
            return it.next().cloned();
        }
    }
    None
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((p / 100.0) * (sorted.len() as f64 - 1.0)) as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn hex_32(bytes: &[u8; 32]) -> String {
    bytes.iter().fold(String::with_capacity(64), |mut s, b| {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// Read VmPeak from /proc/self/status (Linux only).
fn peak_rss_kb() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let status = std::fs::read_to_string("/proc/self/status").ok()?;
        for line in status.lines() {
            if let Some(rest) = line.strip_prefix("VmPeak:") {
                return rest.split_whitespace().next()?.parse().ok();
            }
        }
    }
    None
}
