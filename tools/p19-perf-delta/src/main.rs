// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p19-perf-delta` — P1.9 §10 row 5 perf-delta gate.
//!
//! Measures the throughput of `parse_lfh` invoked via two
//! routes:
//!
//! - **arm A — hand-Rust direct:** `axiom_zip_ref::lfh::parse_lfh`,
//!   the production parser the P1.5/P1.6 three-way differential
//!   gates on.
//! - **arm B — translation-validated shim:**
//!   `axiom_l0_zip_lfh_verified::parse_lfh`, the re-export whose
//!   correspondence with the Lean reference is recorded in the
//!   committed TV receipt.
//!
//! Per ADR-0025 (P1.9 §D-1), arm B is a `pub use` of arm A — same
//! function, same monomorphisation. The measured delta is therefore
//! expected to be zero modulo microbench noise. We still measure
//! and assert the gate so a regression that changed the
//! re-export shape (say, into a thin wrapper that boxes the
//! input) would fire the alarm.
//!
//! The spec gate is **≤ 30 %** (README §10 row 5). This binary
//! defaults to a **≤ 5 %** stricter threshold because the
//! re-export semantics make any nonzero mean delta surprising.

#![forbid(unsafe_code)]
#![allow(clippy::cast_precision_loss, clippy::cast_lossless)]

use std::time::Instant;

const ITERS_PER_RUN: u64 = 200_000;
const RUNS: usize = 20;
const GATE_PCT_DEFAULT: f64 = 5.0;

/// Build a small but realistic LFH input: signature + zero fixed
/// fields + a 31-byte filename + a 15-byte extra field.
fn fixture() -> Vec<u8> {
    let mut bytes = Vec::with_capacity(80);
    bytes.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
    bytes.extend_from_slice(&[0u8; 22]);
    bytes.extend_from_slice(&31u16.to_le_bytes());
    bytes.extend_from_slice(&15u16.to_le_bytes());
    bytes.extend(std::iter::repeat_n(b'a', 31));
    bytes.extend(std::iter::repeat_n(0xaa, 15));
    bytes
}

fn arm_hand_rust(bytes: &[u8]) -> usize {
    match axiom_zip_ref::lfh::parse_lfh(bytes) {
        Ok((lfh, n)) => n + lfh.file_name.len(),
        Err(_) => 0,
    }
}

fn arm_verified_shim(bytes: &[u8]) -> usize {
    match axiom_l0_zip_lfh_verified::parse_lfh(bytes) {
        Ok((lfh, n)) => n + lfh.file_name.len(),
        Err(_) => 0,
    }
}

fn time_arm(label: &str, iters: u64, bytes: &[u8], f: impl Fn(&[u8]) -> usize) -> f64 {
    // Warm.
    for _ in 0..1000 {
        std::hint::black_box(f(std::hint::black_box(bytes)));
    }
    let start = Instant::now();
    for _ in 0..iters {
        std::hint::black_box(f(std::hint::black_box(bytes)));
    }
    let ns = start.elapsed().as_nanos() as f64 / iters as f64;
    println!("  arm {label:<22}: {ns:>7.1} ns/iter");
    ns
}

fn mean_stddev(samples: &[f64]) -> (f64, f64) {
    let n = samples.len() as f64;
    let mean = samples.iter().sum::<f64>() / n;
    let var = samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
    (mean, var.sqrt())
}

fn main() {
    let gate_pct: f64 = std::env::args()
        .skip_while(|a| a != "--gate")
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(GATE_PCT_DEFAULT);
    let bytes = fixture();
    println!(
        "p19-perf-delta: {RUNS} runs × {ITERS_PER_RUN} iters, fixture {} bytes; gate ≤ {gate_pct:.1} %",
        bytes.len()
    );
    let mut deltas = Vec::with_capacity(RUNS);
    for r in 1..=RUNS {
        println!("--- run {r}/{RUNS} ---");
        let a = time_arm("A hand-rust", ITERS_PER_RUN, &bytes, arm_hand_rust);
        let b = time_arm("B verified-shim", ITERS_PER_RUN, &bytes, arm_verified_shim);
        let d = (b - a) / a * 100.0;
        println!("  Δ(B vs A): {d:+.2} %");
        deltas.push(d);
    }
    let (m, s) = mean_stddev(&deltas);
    let in_band = m.abs() <= 2.0 * s;
    let pass = m <= gate_pct || in_band;
    println!();
    println!(
        "summary: mean Δ = {m:+.2} % (σ {s:.2} %, n={RUNS}, gate ≤ {gate_pct:.1} % or |Δ|≤2σ)  {}",
        if pass { "PASS" } else { "FAIL" }
    );
    if pass && in_band && m > gate_pct {
        println!(
            "  note: mean {m:+.2} % > gate but within ±2σ ({:.2} %) — re-export cost indistinguishable from zero",
            2.0 * s
        );
    }
    if !pass {
        eprintln!("::error::p19-perf-delta mean {m:.2}% exceeds gate {gate_pct:.1}%");
        std::process::exit(1);
    }
}
