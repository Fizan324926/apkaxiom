// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p112-latency` — P1.12 row 4 p99 latency gate.
//!
//! Measures per-archive verified-ZIP-layer parse latency over the
//! Bench-10K corpus and reports min / mean / p50 / p95 / p99 /
//! max in nanoseconds.
//!
//! Gate: **p99 ≤ 80 ms** (HARD — P1.12 §4 row 4).
//!
//! Calibration note: 80 ms is the project-wide L0 budget for
//! whole-APK parse latency (LFH + CDR + EOCD + cross-record
//! consistency). The Bench-10K samples are small synthetic ZIPs
//! that exercise the same code paths; observed p99 is in the
//! single-microsecond range, leaving four orders of magnitude
//! of headroom for the larger APK-sized inputs the L1 layer
//! feeds in.

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

fn main() {
    let corpus_dir: String = parse_arg("--corpus", "corpus/zip/bench-10k".to_string());
    let iters: u32 = parse_arg("--iters", ITERS_PER_SAMPLE_DEFAULT);
    let gate_ns: u128 = parse_arg("--gate-ns", GATE_P99_NANOS_DEFAULT);

    let dir = PathBuf::from(&corpus_dir);
    let samples = match load_corpus(&dir) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ERROR loading corpus {corpus_dir}: {e}");
            std::process::exit(2);
        }
    };
    if samples.is_empty() {
        eprintln!("ERROR: corpus {corpus_dir} is empty");
        std::process::exit(2);
    }

    println!(
        "p112-latency: {} samples × {} iters; gate p99 ≤ {} ns ({:.1} ms)",
        samples.len(),
        iters,
        gate_ns,
        gate_ns as f64 / 1_000_000.0
    );

    // Warm up the I-cache and branch predictor.
    for s in samples.iter().take(100) {
        let _ = std::hint::black_box(parse_archive(std::hint::black_box(s)));
    }

    let mut latencies: Vec<u128> = Vec::with_capacity(samples.len() * iters as usize);
    for s in &samples {
        for _ in 0..iters {
            let t0 = Instant::now();
            let res = parse_archive(s);
            let dt = t0.elapsed().as_nanos();
            std::hint::black_box(&res);
            latencies.push(dt);
        }
    }
    latencies.sort_unstable();

    let n = latencies.len() as u128;
    let sum: u128 = latencies.iter().sum();
    let mean = sum / n;
    let min = latencies[0];
    let max = latencies[latencies.len() - 1];
    let p50 = percentile(&latencies, 0.50);
    let p95 = percentile(&latencies, 0.95);
    let p99 = percentile(&latencies, 0.99);
    let p999 = percentile(&latencies, 0.999);

    println!();
    println!("=== latency distribution (ns) ===");
    println!("  samples: {}", n);
    println!("  min   : {:>10}", min);
    println!("  mean  : {:>10}", mean);
    println!("  p50   : {:>10}", p50);
    println!("  p95   : {:>10}", p95);
    println!("  p99   : {:>10}", p99);
    println!("  p99.9 : {:>10}", p999);
    println!("  max   : {:>10}", max);

    let pass = p99 <= gate_ns;
    println!();
    println!(
        "  verdict: p99 = {} ns ({:.3} ms)  vs gate {} ns ({:.1} ms)  {}",
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
