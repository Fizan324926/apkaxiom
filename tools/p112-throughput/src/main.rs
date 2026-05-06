// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p112-throughput` — P1.12 row 4 throughput gate.
//!
//! Measures multi-core verified-ZIP-layer throughput on the four
//! real wifiautoff APK fixtures (corpus/signing/{v1-only,v1-v2,
//! v1-v2-v3,v1-v2-v3-v31}/wifiautoff-*.apk). After P1.12 gap-2
//! closure (relaxed DD-mode `cdr_lfh_fields_agree` accepting
//! either zero LFH fields or LFH-matches-CDR), all four fixtures
//! parse via the verified path.
//!
//! Gate: **≥ 250 APKs/sec/16-core** (HARD — P1.12 §4 row 4).
//!
//! Per-core normalisation: we run on `std::thread::available_parallelism()`
//! cores, measure throughput, then linearly extrapolate to a
//! hypothetical 16-core run. The per-core ZIP-parse work is
//! embarrassingly parallel (parse_archive is allocator-light, no
//! shared state).

#![forbid(unsafe_code)]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::uninlined_format_args,
    clippy::doc_markdown
)]

use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use axiom_l0_zip_verified::consistency::parse_archive;

const APK_FIXTURES: &[&str] = &[
    "corpus/signing/v1-only/wifiautoff-v1.apk",
    "corpus/signing/v1-v2/wifiautoff-v1v2.apk",
    "corpus/signing/v1-v2-v3/wifiautoff-v1v2v3.apk",
    "corpus/signing/v1-v2-v3-v31/wifiautoff-v1v2v3v31.apk",
];

const GATE_APKS_PER_SEC_16C_DEFAULT: f64 = 250.0;
const TARGET_CORES: usize = 16;
const RUN_SECONDS_DEFAULT: u64 = 5;

fn parse_arg<T: std::str::FromStr>(name: &str, default: T) -> T {
    std::env::args()
        .skip_while(|a| a != name)
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn load_fixtures() -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    for p in APK_FIXTURES {
        let bytes = std::fs::read(p).unwrap_or_else(|e| {
            eprintln!("ERROR read {p}: {e}");
            std::process::exit(2);
        });
        if let Err(e) = parse_archive(&bytes) {
            eprintln!("ERROR canonical verified parse {p}: {e:?}");
            std::process::exit(2);
        }
        out.push(bytes);
    }
    out
}

fn worker(samples: &Arc<Vec<Vec<u8>>>, budget: Duration) -> u64 {
    let start = Instant::now();
    let mut count: u64 = 0;
    let n = samples.len();
    while start.elapsed() < budget {
        for i in 0..n {
            let res = parse_archive(&samples[i]);
            std::hint::black_box(&res);
            count = count.wrapping_add(1);
        }
        if start.elapsed() >= budget {
            break;
        }
    }
    count
}

fn main() {
    let cores: usize = parse_arg(
        "--cores",
        thread::available_parallelism()
            .map(std::num::NonZeroUsize::get)
            .unwrap_or(1),
    );
    let seconds: u64 = parse_arg("--seconds", RUN_SECONDS_DEFAULT);
    let gate: f64 = parse_arg("--gate", GATE_APKS_PER_SEC_16C_DEFAULT);

    let samples = Arc::new(load_fixtures());
    println!(
        "p112-throughput: {} real APK fixtures, {cores} cores, {seconds}s budget; gate ≥ {gate:.0} APKs/sec/{TARGET_CORES}-core",
        samples.len()
    );
    let total_bytes: usize = samples.iter().map(Vec::len).sum();
    let avg_bytes = total_bytes / samples.len();
    println!(
        "  fixtures: {} total bytes, avg {} B per APK",
        total_bytes, avg_bytes
    );

    let budget = Duration::from_secs(seconds);
    let t0 = Instant::now();
    let mut handles = Vec::with_capacity(cores);
    for _ in 0..cores {
        let s = Arc::clone(&samples);
        handles.push(thread::spawn(move || worker(&s, budget)));
    }
    let mut total: u64 = 0;
    for h in handles {
        total = total.wrapping_add(h.join().unwrap());
    }
    let elapsed = t0.elapsed().as_secs_f64();
    let apks_per_sec = total as f64 / elapsed;
    let per_core = apks_per_sec / cores as f64;
    let extrapolated_16c = per_core * TARGET_CORES as f64;

    println!();
    println!("=== summary ===");
    println!("  total parses          : {}", total);
    println!("  wall time             : {:.2}s", elapsed);
    println!("  cores used            : {}", cores);
    println!(
        "  throughput            : {:.0} APKs/sec ({} cores)",
        apks_per_sec, cores
    );
    println!("  per-core throughput   : {:.0} APKs/sec/core", per_core);
    println!(
        "  extrapolated 16-core  : {:.0} APKs/sec  (gate ≥ {:.0})",
        extrapolated_16c, gate
    );
    let pass = extrapolated_16c >= gate;
    println!(
        "  verdict               : {}",
        if pass { "PASS" } else { "FAIL" }
    );
    if !pass {
        eprintln!(
            "::error::p112-throughput {:.0} APKs/sec/{}-core below gate {:.0}",
            extrapolated_16c, TARGET_CORES, gate
        );
        std::process::exit(1);
    }
}
