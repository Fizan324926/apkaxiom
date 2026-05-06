// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p110-hash-throughput` — P1.10 §10 row 2 hash-throughput gate.
//!
//! Measures BLAKE3 single-core throughput on a 256 MiB random
//! buffer (in-memory, no I/O). Spec gate: **≥ 1.5 GB/s**.
//!
//! Reports `mean ± σ` over `--runs` (default 100) full hashings
//! of the buffer, plus min/max/p50/p95 quantiles. Per-run
//! variance is captured so the operator can see whether the
//! 1.5 GB/s gate is being met by a comfortable margin or only at
//! mean - the σ matters for the headroom assessment.
//!
//! Output is also emitted as machine-readable JSON via
//! `--json /path/to/file.json` so CI can diff history across
//! runs.

#![forbid(unsafe_code)]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use std::time::Instant;

use axiom_blake3_hacl::{Blake3, Hasher};

const PAYLOAD_BYTES: usize = 256 * 1024 * 1024;
const GATE_GB_PER_SEC_DEFAULT: f64 = 1.5;
const RUNS_DEFAULT: usize = 100;
const WARMUP_RUNS: usize = 5;

fn parse_arg<T: std::str::FromStr>(name: &str, default: T) -> T {
    std::env::args()
        .skip_while(|a| a != name)
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn parse_string(name: &str) -> Option<String> {
    std::env::args().skip_while(|a| a != name).nth(1)
}

fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() as f64 - 1.0) * q).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn main() {
    let runs: usize = parse_arg("--runs", RUNS_DEFAULT);
    let gate: f64 = parse_arg("--gate", GATE_GB_PER_SEC_DEFAULT);
    let json_out = parse_string("--json");

    // LCG-seeded 256 MiB buffer.
    let mut payload = vec![0u8; PAYLOAD_BYTES];
    let mut s: u64 = 0x1357_9bdf_2468_ace0;
    for chunk in payload.chunks_mut(8) {
        s = s
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let bytes = s.to_le_bytes();
        for (i, b) in chunk.iter_mut().enumerate() {
            *b = bytes[i];
        }
    }

    println!(
        "p110-hash-throughput: hashing 256 MiB × {runs} runs (warmup {WARMUP_RUNS}); gate ≥ {gate:.2} GB/s"
    );

    // Warmup.
    for _ in 0..WARMUP_RUNS {
        let _ = Blake3::hash_oneshot(&payload);
    }

    // Measure.
    let mut samples_gbps = Vec::with_capacity(runs);
    for r in 0..runs {
        let start = Instant::now();
        let h = Blake3::hash_oneshot(&payload);
        let elapsed = start.elapsed();
        std::hint::black_box(h);
        let gbps = (PAYLOAD_BYTES as f64 / 1e9) / elapsed.as_secs_f64();
        samples_gbps.push(gbps);
        if r < 3 || r == runs - 1 || r % (runs / 10).max(1) == 0 {
            println!(
                "  run {:>4}/{runs}: {gbps:>5.2} GB/s ({:.4}s)",
                r + 1,
                elapsed.as_secs_f64()
            );
        }
    }

    // Stats.
    let n = samples_gbps.len() as f64;
    let mean = samples_gbps.iter().sum::<f64>() / n;
    let variance = samples_gbps.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
    let stddev = variance.sqrt();
    let mut sorted = samples_gbps.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).expect("no NaN"));
    let p_min = sorted[0];
    let p50 = quantile(&sorted, 0.50);
    let p95 = quantile(&sorted, 0.95);
    let p_max = sorted[sorted.len() - 1];
    println!();
    println!("summary: mean = {mean:.3} GB/s  σ = {stddev:.3} GB/s  n = {runs}",);
    println!("         min = {p_min:.3}  p50 = {p50:.3}  p95 = {p95:.3}  max = {p_max:.3} GB/s",);
    let pass = mean >= gate;
    println!(
        "         gate ≥ {gate:.2} GB/s  →  {} (mean - 2σ = {:.3} GB/s)",
        if pass { "PASS" } else { "FAIL" },
        2.0_f64.mul_add(-stddev, mean),
    );

    if let Some(path) = json_out {
        let json = format!(
            "{{\"runs\":{runs},\"mean_gbps\":{mean},\"stddev_gbps\":{stddev},\"min_gbps\":{p_min},\"p50_gbps\":{p50},\"p95_gbps\":{p95},\"max_gbps\":{p_max},\"gate_gbps\":{gate},\"pass\":{pass}}}\n"
        );
        if let Err(e) = std::fs::write(&path, json) {
            eprintln!("warning: failed to write JSON to {path}: {e}");
        } else {
            println!("         wrote JSON: {path}");
        }
    }

    if !pass {
        eprintln!("::error::p110-hash-throughput: mean {mean:.3} GB/s < gate {gate:.2} GB/s");
        std::process::exit(1);
    }
}
