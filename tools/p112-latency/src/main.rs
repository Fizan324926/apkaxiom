// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p112-latency` — P1.12 row 4 p99 latency gate.
//!
//! Measures per-archive verified-ZIP-layer parse latency on a
//! mixed corpus:
//!
//!   - Bench-10K (10 000 small archives, 98–2 442 B), and
//!   - the four real wifiautoff APK fixtures (11 kB – 21 kB).
//!
//! Reports min / mean / p50 / p95 / p99 / p99.9 / max in
//! nanoseconds, separately for each cohort and aggregated.
//!
//! Gate: **p99 ≤ 80 ms** (HARD — P1.12 §4 row 4) on the merged
//! cohort. Real APKs are the binding distribution; Bench-10K is
//! the volume that drives the percentile shape.

#![forbid(unsafe_code)]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::cast_sign_loss,
    clippy::uninlined_format_args
)]

use std::{path::PathBuf, time::Instant};

use axiom_l0_zip_verified::consistency::parse_archive;

const GATE_P99_NANOS_DEFAULT: u128 = 80_000_000; // 80 ms
const ITERS_PER_SAMPLE_DEFAULT: u32 = 16;
const REAL_APK_FIXTURES: &[&str] = &[
    "corpus/signing/v1-only/wifiautoff-v1.apk",
    "corpus/signing/v1-v2/wifiautoff-v1v2.apk",
    "corpus/signing/v1-v2-v3/wifiautoff-v1v2v3.apk",
    "corpus/signing/v1-v2-v3-v31/wifiautoff-v1v2v3v31.apk",
];

fn parse_arg<T: std::str::FromStr>(name: &str, default: T) -> T {
    std::env::args()
        .skip_while(|a| a != name)
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn load_corpus(dir: &PathBuf) -> std::io::Result<Vec<Vec<u8>>> {
    let mut samples: Vec<Vec<u8>> = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let p = entry.path();
        if p.extension().and_then(|s| s.to_str()) != Some("bin") {
            continue;
        }
        samples.push(std::fs::read(&p)?);
    }
    samples.sort_by_key(Vec::len);
    Ok(samples)
}

fn percentile(sorted: &[u128], q: f64) -> u128 {
    if sorted.is_empty() {
        return 0;
    }
    let pos = (q * (sorted.len() - 1) as f64).round() as usize;
    sorted[pos]
}

fn measure(samples: &[Vec<u8>], iters: u32) -> Vec<u128> {
    // Warm.
    for s in samples.iter().take(samples.len().min(100)) {
        let _ = std::hint::black_box(parse_archive(std::hint::black_box(s)));
    }
    let mut latencies: Vec<u128> = Vec::with_capacity(samples.len() * iters as usize);
    for s in samples {
        for _ in 0..iters {
            let t0 = Instant::now();
            let res = parse_archive(s);
            let dt = t0.elapsed().as_nanos();
            std::hint::black_box(&res);
            latencies.push(dt);
        }
    }
    latencies.sort_unstable();
    latencies
}

fn report(cohort: &str, latencies: &[u128]) {
    let n = latencies.len() as u128;
    let sum: u128 = latencies.iter().sum();
    let mean = sum / n.max(1);
    let min = latencies[0];
    let max = latencies[latencies.len() - 1];
    let p50 = percentile(latencies, 0.50);
    let p95 = percentile(latencies, 0.95);
    let p99 = percentile(latencies, 0.99);
    let p999 = percentile(latencies, 0.999);
    println!("--- {cohort} ---");
    println!("  samples: {}", n);
    println!("  min   : {min:>10}");
    println!("  mean  : {mean:>10}");
    println!("  p50   : {p50:>10}");
    println!("  p95   : {p95:>10}");
    println!("  p99   : {p99:>10}");
    println!("  p99.9 : {p999:>10}");
    println!("  max   : {max:>10}");
}

fn main() {
    let corpus_dir: String = parse_arg("--corpus", "corpus/zip/bench-10k".to_string());
    let iters: u32 = parse_arg("--iters", ITERS_PER_SAMPLE_DEFAULT);
    let gate_ns: u128 = parse_arg("--gate-ns", GATE_P99_NANOS_DEFAULT);

    let dir = PathBuf::from(&corpus_dir);
    let bench = match load_corpus(&dir) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ERROR loading corpus {corpus_dir}: {e}");
            std::process::exit(2);
        }
    };
    let mut real_apks: Vec<Vec<u8>> = Vec::new();
    for p in REAL_APK_FIXTURES {
        match std::fs::read(p) {
            Ok(b) => {
                if let Err(e) = parse_archive(&b) {
                    eprintln!("ERROR canonical verified parse {p}: {e:?}");
                    std::process::exit(2);
                }
                real_apks.push(b);
            }
            Err(e) => {
                eprintln!("WARN read {p}: {e} — skipping cohort");
            }
        }
    }
    if bench.is_empty() && real_apks.is_empty() {
        eprintln!("ERROR: both cohorts empty");
        std::process::exit(2);
    }

    println!(
        "p112-latency: bench-10k={} samples, real-apks={} fixtures, iters={}; gate p99 ≤ {} ns ({:.1} ms)",
        bench.len(),
        real_apks.len(),
        iters,
        gate_ns,
        gate_ns as f64 / 1_000_000.0
    );

    let bench_lat = measure(&bench, iters);
    // Real APKs get more iterations because there are only 4 of them.
    let apk_lat = if real_apks.is_empty() {
        Vec::new()
    } else {
        measure(&real_apks, iters * 64)
    };
    let mut combined: Vec<u128> = bench_lat.clone();
    combined.extend_from_slice(&apk_lat);
    combined.sort_unstable();

    println!();
    if !bench_lat.is_empty() {
        report("Bench-10K", &bench_lat);
    }
    if !apk_lat.is_empty() {
        println!();
        report("Real APKs (wifiautoff v1/v2/v3/v3.1)", &apk_lat);
    }
    println!();
    report("Combined", &combined);

    let p99 = percentile(&combined, 0.99);
    let pass = p99 <= gate_ns;
    println!();
    println!(
        "  verdict: combined p99 = {} ns ({:.3} ms)  vs gate {} ns ({:.1} ms)  {}",
        p99,
        p99 as f64 / 1_000_000.0,
        gate_ns,
        gate_ns as f64 / 1_000_000.0,
        if pass { "PASS" } else { "FAIL" }
    );
    if !pass {
        eprintln!(
            "::error::p112-latency p99 {} ns exceeds gate {} ns",
            p99, gate_ns
        );
        std::process::exit(1);
    }
}
