// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p112-perf-delta` — P1.12 row 4 perf-delta gate (honest framing).
//!
//! What this binary measures:
//!
//! 1. **Re-export overhead.** The `axiom-l0-zip-verified` umbrella
//!    `pub use`s `axiom_zip_ref::archive::parse_archive` — identical
//!    monomorphisation. We verify the umbrella indirection is
//!    statistically indistinguishable from a direct call (|Δ| ≤ 2σ).
//!    A regression that changed the umbrella from a `pub use` to a
//!    boxing wrapper would surface here.
//!
//! 2. **Absolute per-byte cost.** The verified path's per-byte
//!    parse cost on Bench-10K. Spec gate is **≤ 50 ns/byte** —
//!    a 100 kB APK parses in ≤ 5 ms, a 1 MB APK in ≤ 50 ms. This
//!    is the production-meaningful budget; the original "verified
//!    vs hand-written ≤ 15 %" gate from the README §10 is degenerate
//!    by construction (both arms call the same function via
//!    `pub use`) so we replace it with this absolute budget. ADR-0030
//!    §"Perf-delta calibration" records this reframe.

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
const REEXPORT_SIGMA_BAND: f64 = 2.0;
const NS_PER_BYTE_GATE_DEFAULT: f64 = 50.0;
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

#[inline(never)]
fn arm_direct(samples: &[Vec<u8>]) -> usize {
    let mut acc: usize = 0;
    for bytes in samples {
        if let Ok(a) = axiom_zip_ref::archive::parse_archive(bytes) {
            acc = acc.wrapping_add(a.lfhs.len()).wrapping_add(a.cdrs.len());
        }
    }
    acc
}

#[inline(never)]
fn arm_umbrella(samples: &[Vec<u8>]) -> usize {
    let mut acc: usize = 0;
    for bytes in samples {
        if let Ok(a) = axiom_l0_zip_verified::consistency::parse_archive(bytes) {
            acc = acc.wrapping_add(a.lfhs.len()).wrapping_add(a.cdrs.len());
        }
    }
    acc
}

fn time_arm(label: &str, iters: u64, samples: &[Vec<u8>], f: impl Fn(&[Vec<u8>]) -> usize) -> f64 {
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
    let iters: u64 = parse_arg("--iters", ITERS_PER_RUN_DEFAULT);
    let ns_per_byte_gate: f64 = parse_arg("--ns-per-byte-gate", NS_PER_BYTE_GATE_DEFAULT);
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
    let total_bytes: u64 = samples.iter().map(|s| s.len() as u64).sum();
    let avg_bytes = total_bytes as f64 / samples.len() as f64;

    println!(
        "p112-perf-delta: {RUNS} runs × {iters} iters × {} samples (avg {:.0} B); ns/byte gate ≤ {:.1}",
        samples.len(),
        avg_bytes,
        ns_per_byte_gate
    );

    let mut deltas = Vec::with_capacity(RUNS);
    let mut umbrella_ns_per_archive: Vec<f64> = Vec::with_capacity(RUNS);
    for r in 1..=RUNS {
        println!("--- run {r}/{RUNS} ---");
        let a = time_arm("A direct (zip-ref)", iters, &samples, arm_direct);
        let b = time_arm("B umbrella (l0-verified)", iters, &samples, arm_umbrella);
        let d = (b - a) / a * 100.0;
        println!("  Δ(B vs A): {d:+.2} %");
        deltas.push(d);
        umbrella_ns_per_archive.push(b);
    }
    let (m, s) = mean_stddev(&deltas);
    let in_band = m.abs() <= REEXPORT_SIGMA_BAND * s;
    let (m_b, _) = mean_stddev(&umbrella_ns_per_archive);
    let ns_per_byte = m_b / avg_bytes;

    println!();
    println!("=== summary ===");
    println!("  re-export Δ           : {m:+.2} %  (σ {s:.2} %, n={RUNS})");
    println!(
        "  re-export within ±{REEXPORT_SIGMA_BAND}σ : {}",
        if in_band { "PASS" } else { "FAIL" }
    );
    println!(
        "  verified ns/archive   : {:>7.1}  (avg over {RUNS} runs)",
        m_b
    );
    println!(
        "  verified ns/byte      : {:>7.2}  (gate ≤ {:.1})  {}",
        ns_per_byte,
        ns_per_byte_gate,
        if ns_per_byte <= ns_per_byte_gate {
            "PASS"
        } else {
            "FAIL"
        }
    );

    let pass = in_band && ns_per_byte <= ns_per_byte_gate;
    if !pass {
        eprintln!(
            "::error::p112-perf-delta failed: in_band={in_band}, ns/byte={ns_per_byte:.2} (gate {ns_per_byte_gate:.1})"
        );
        std::process::exit(1);
    }
}
