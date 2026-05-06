// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p112-perf-delta` — P1.12 row 4 perf-delta gate.
//!
//! Measures full-archive parse throughput on the Bench-10K corpus
//! via two routes:
//!
//! - **arm A — hand-Rust direct:** `axiom_zip_ref::archive::parse_archive`,
//!   the production parser the P1.5/P1.6 three-way differential
//!   gates on (Lean ↔ Rust ↔ libziparchive, 2860/2860 inputs).
//! - **arm B — verified umbrella:**
//!   `axiom_l0_zip_verified::consistency::parse_archive`, the
//!   re-export from the umbrella crate that backs the
//!   default-on `axiom-l0::zip` ZIP layer.
//!
//! Per ADR-0030 (P1.12 §D-1), arm B is a `pub use` of arm A —
//! same function, same monomorphisation. The measured delta is
//! therefore expected to be zero modulo microbench noise; we
//! still measure and assert the gate so a regression that
//! changed the re-export shape (e.g. into a thin wrapper that
//! boxes the input) would fire the alarm.
//!
//! Spec gate is **≤ 15 %** (P1.12 HARD). This binary defaults to
//! a stricter threshold of **≤ 5 %** plus a ±2σ band because the
//! re-export semantics make any nonzero mean delta surprising.

#![forbid(unsafe_code)]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_lossless,
    clippy::uninlined_format_args
)]

use std::{
    path::{Path, PathBuf},
    time::Instant,
};

const RUNS: usize = 20;
const GATE_PCT_DEFAULT: f64 = 15.0;
const STRICT_PCT_DEFAULT: f64 = 5.0;
const ITERS_PER_RUN_DEFAULT: u64 = 3;

fn load_corpus(dir: &Path) -> std::io::Result<Vec<Vec<u8>>> {
    let mut samples: Vec<Vec<u8>> = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("bin") {
            continue;
        }
        samples.push(std::fs::read(&path)?);
    }
    samples.sort_by_key(Vec::len);
    Ok(samples)
}

fn arm_hand_rust(samples: &[Vec<u8>]) -> usize {
    let mut acc: usize = 0;
    for bytes in samples {
        if let Ok(a) = axiom_zip_ref::archive::parse_archive(bytes) {
            acc = acc.wrapping_add(a.lfhs.len()).wrapping_add(a.cdrs.len());
        }
    }
    acc
}

fn arm_verified(samples: &[Vec<u8>]) -> usize {
    let mut acc: usize = 0;
    for bytes in samples {
        if let Ok(a) = axiom_l0_zip_verified::consistency::parse_archive(bytes) {
            acc = acc.wrapping_add(a.lfhs.len()).wrapping_add(a.cdrs.len());
        }
    }
    acc
}

fn time_arm(label: &str, iters: u64, samples: &[Vec<u8>], f: impl Fn(&[Vec<u8>]) -> usize) -> f64 {
    // Warm.
    std::hint::black_box(f(std::hint::black_box(samples)));
    let start = Instant::now();
    for _ in 0..iters {
        std::hint::black_box(f(std::hint::black_box(samples)));
    }
    let ns = start.elapsed().as_nanos() as f64 / (iters as f64 * samples.len() as f64);
    println!("  arm {label:<22}: {ns:>7.1} ns/archive");
    ns
}

fn mean_stddev(samples: &[f64]) -> (f64, f64) {
    let n = samples.len() as f64;
    let mean = samples.iter().sum::<f64>() / n;
    let var = samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
    (mean, var.sqrt())
}

fn parse_arg<T: std::str::FromStr>(name: &str, default: T) -> T {
    std::env::args()
        .skip_while(|a| a != name)
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn main() {
    let gate_pct: f64 = parse_arg("--gate", GATE_PCT_DEFAULT);
    let strict_pct: f64 = parse_arg("--strict", STRICT_PCT_DEFAULT);
    let iters: u64 = parse_arg("--iters", ITERS_PER_RUN_DEFAULT);
    let corpus_dir: String = parse_arg("--corpus", "corpus/zip/bench-10k".to_string());

    let dir = PathBuf::from(&corpus_dir);
    let samples = match load_corpus(&dir) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ERROR loading corpus {corpus_dir}: {e}");
            std::process::exit(2);
        }
    };
    if samples.is_empty() {
        eprintln!("ERROR: corpus {corpus_dir} is empty — run `make p112-bench-10k` first");
        std::process::exit(2);
    }
    println!(
        "p112-perf-delta: {RUNS} runs × {iters} iters × {} samples; gate ≤ {gate_pct:.1} % (strict {strict_pct:.1} %)",
        samples.len()
    );

    let mut deltas = Vec::with_capacity(RUNS);
    for r in 1..=RUNS {
        println!("--- run {r}/{RUNS} ---");
        let a = time_arm("A hand-rust", iters, &samples, arm_hand_rust);
        let b = time_arm("B verified", iters, &samples, arm_verified);
        let d = (b - a) / a * 100.0;
        println!("  Δ(B vs A): {d:+.2} %");
        deltas.push(d);
    }
    let (m, s) = mean_stddev(&deltas);
    let in_strict_band = m.abs() <= 2.0 * s;
    let pass_hard = m <= gate_pct;
    let pass_strict = m <= strict_pct || in_strict_band;
    println!();
    println!("summary: mean Δ = {m:+.2} % (σ {s:.2} %, n={RUNS})",);
    println!(
        "  hard gate (≤ {gate_pct:.1} %):     {}",
        if pass_hard { "PASS" } else { "FAIL" }
    );
    println!(
        "  strict gate (≤ {strict_pct:.1} % or |Δ|≤2σ): {}",
        if pass_strict { "PASS" } else { "FAIL" }
    );
    if !pass_hard {
        eprintln!("::error::p112-perf-delta mean {m:.2}% exceeds HARD gate {gate_pct:.1}%");
        std::process::exit(1);
    }
}
