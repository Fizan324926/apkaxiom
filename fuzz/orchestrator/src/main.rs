// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p114-orchestrate` — multi-worker fuzz pool driver.
//!
//! Spawns N parallel `p113-fuzz-driver` children, each with its
//! own archive subdir under `--archive-root`, and periodically
//! merges their archives into a canonical `archive.ndjson` at
//! the root. On exit, the merged archive is the only file
//! downstream consumers (classifier, dashboard) need to read.

#![forbid(unsafe_code)]
#![allow(clippy::uninlined_format_args)]

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use p114_orchestrator::{default_pool, pool_loop, print_stats, spawn_worker, PoolStats, WorkerSpec};

const VERSION: &str = "p114-orchestrate 0.1.0";

fn arg<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::args()
        .skip_while(|a| a != name)
        .nth(1)
        .and_then(|s| s.parse().ok())
}

fn main() -> std::io::Result<()> {
    let driver: PathBuf =
        arg("--driver").unwrap_or_else(|| PathBuf::from("target/release/p113-fuzz-driver"));
    let seeds: PathBuf = arg("--seeds").unwrap_or_else(|| PathBuf::from("fuzz/corpus/seed"));
    let archive_root: PathBuf =
        arg("--archive-root").unwrap_or_else(|| PathBuf::from("fuzz/findings-pool"));
    let probe: PathBuf =
        arg("--probe").unwrap_or_else(|| PathBuf::from("target/zip-aosp-runtime-probe"));
    let probes_csv: String = arg("--probes").unwrap_or_else(|| "A14:synthetic,A11:synthetic,A8:synthetic".into());
    let workers: usize = arg("--workers").unwrap_or(4);
    let iters_per_worker: u64 = arg("--iters-per-worker").unwrap_or(0);
    let merge_every_secs: u64 = arg("--merge-every-secs").unwrap_or(10);

    println!("{VERSION}");
    println!(
        "  driver={}  seeds={}  archive-root={}  workers={}  probes={}",
        driver.display(),
        seeds.display(),
        archive_root.display(),
        workers,
        probes_csv
    );
    if !driver.exists() {
        eprintln!("ERROR driver binary not found at {} — `make p113`", driver.display());
        std::process::exit(2);
    }
    if !probe.exists() {
        eprintln!("ERROR probe binary not found at {} — `make p16-aosp-runtime-probe`", probe.display());
        std::process::exit(2);
    }

    std::fs::create_dir_all(&archive_root)?;

    let mut specs: Vec<WorkerSpec> = default_pool(&probes_csv)
        .into_iter()
        .take(workers)
        .collect();
    while specs.len() < workers {
        let i = specs.len();
        specs.push(WorkerSpec {
            id: format!("worker-{}", i + 1),
            probes: probes_csv.clone(),
            seed: 0xb114_0000_0000_0001 + i as u64,
            iters: iters_per_worker,
        });
    }
    if iters_per_worker > 0 {
        for s in &mut specs {
            s.iters = iters_per_worker;
        }
    }

    let stats = Arc::new(PoolStats::default());
    let mut pool: Vec<(WorkerSpec, std::process::Child)> = Vec::with_capacity(specs.len());
    for s in &specs {
        let child = spawn_worker(s, &driver, &seeds, &archive_root, &probe)?;
        println!("  spawned {} (pid={})", s.id, child.id());
        pool.push((s.clone(), child));
    }
    stats.workers_alive.store(pool.len() as u64, Ordering::Relaxed);

    pool_loop(&mut pool, &archive_root, Duration::from_secs(merge_every_secs), Arc::clone(&stats))?;
    print_stats(&stats);
    println!("  merged archive  : {}", archive_root.join("archive.ndjson").display());
    Ok(())
}
