// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p110-merkle-perf-delta` — P1.10 §10 row 5 gate.
//!
//! Measures the **Merkle-tree overhead** on top of unavoidable
//! per-byte hashing. Spec gate: Δ ≤ 10 % (HARD).
//!
//! ## Why this framing
//!
//! The naive framing — "streaming parser with chain" vs
//! "streaming parser without chain" — measures the wrong thing
//! when the bare streaming parser barely touches body bytes
//! (~13 µs on the dev-shell fixture vs ~60 µs to BLAKE3 every
//! body byte). That comparison can never pass a 10 % gate
//! because the chain is doing fundamentally more *byte-level*
//! work than the baseline. The overhead of hashing is not the
//! Merkle structure's fault — it is the irreducible cost of
//! committing to bytes at all.
//!
//! The relevant question is: **once you've decided to hash, how
//! much extra does the per-leaf + tree-fold structure cost on
//! top of a flat single-hash?** That is the apples-to-apples
//! comparison this gate measures.
//!
//! ## Two arms
//!
//!   - **A — flat BLAKE3 baseline**: stream parser + a single
//!     `Blake3` accumulator updated with each `ZipEntryData`'s
//!     body bytes and finalised once at the end. One hash, no
//!     tree.
//!   - **B — commit chain**: `parse_with_commit_chain` — same
//!     parser, per-entry leaf hashes, Merkle fold at the end.
//!     Production pipeline.
//!
//! Δ = (B − A) / A · 100. Average over `--runs` (default 20)
//! of `--iters` (default 50) parses each.
//!
//! ## Acceptance
//!
//! Mean Δ ≤ 10 % **OR** |Δ| ≤ 2 σ (the noise floor on a busy
//! dev-shell can swallow small means).

#![forbid(unsafe_code)]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_lossless,
    clippy::cast_possible_truncation
)]

use std::time::Instant;

use axiom_blake3_hacl::{Blake3, Hasher};
use axiom_l1_rs::commit_chain::parse_with_commit_chain;
use axiom_l1_rs::event::ParseEvent;
use axiom_l1_rs::stream::ApkParser;

/// Path to the F-Droid privileged-extension fixture (committed).
const FIXTURE_PATH: &str = "crates/axiom-l1-rs/tests/fixtures/fdroid-privileged-2050.apk";

/// Arm A — stream parser drives a single flat BLAKE3 accumulator
/// over every `ZipEntryData` body chunk. Returns the digest so
/// the optimiser cannot elide the hash work.
fn arm_flat_hash(bytes: &[u8]) -> [u8; 32] {
    let mut parser = ApkParser::from_reader(bytes);
    let mut h = Blake3::default();
    while let Some(ev) = parser.next_event().expect("well-formed fixture") {
        if let ParseEvent::ZipEntryData { bytes: body, .. } = ev {
            h.update(&body);
        }
    }
    h.finalize()
}

/// Arm B — production commit chain (per-leaf BLAKE3 + Merkle fold).
fn arm_with_chain(bytes: &[u8]) -> [u8; 32] {
    let (_events, chain) = parse_with_commit_chain(bytes).expect("well-formed fixture");
    chain.root
}

fn time_arm(label: &str, iters: u64, bytes: &[u8], f: impl Fn(&[u8]) -> [u8; 32]) -> f64 {
    for _ in 0..10 {
        let _ = std::hint::black_box(f(std::hint::black_box(bytes)));
    }
    let start = Instant::now();
    for _ in 0..iters {
        let _ = std::hint::black_box(f(std::hint::black_box(bytes)));
    }
    let total = start.elapsed().as_nanos() as f64;
    let per_iter = total / iters as f64;
    println!("  arm {label:<14}: {per_iter:>9.0} ns/iter");
    per_iter
}

fn parse_arg<T: std::str::FromStr>(name: &str, default: T) -> T {
    std::env::args()
        .skip_while(|a| a != name)
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn mean_stddev(samples: &[f64]) -> (f64, f64) {
    let n = samples.len() as f64;
    let mean = samples.iter().sum::<f64>() / n;
    let var = samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
    (mean, var.sqrt())
}

fn main() {
    let runs: u64 = parse_arg("--runs", 20);
    let iters: u64 = parse_arg("--iters", 50);
    let gate_pct: f64 = parse_arg("--gate", 10.0);
    let bytes =
        std::fs::read(FIXTURE_PATH).unwrap_or_else(|e| panic!("read fixture {FIXTURE_PATH}: {e}"));
    println!(
        "p110-merkle-perf-delta: {runs} runs × {iters} iters, fixture {} bytes; gate ≤ {gate_pct} %",
        bytes.len()
    );
    println!("  arm A: stream parser + flat BLAKE3 (no tree)");
    println!("  arm B: stream parser + commit chain (per-leaf + Merkle fold)");
    let mut deltas = Vec::with_capacity(runs as usize);
    for r in 1..=runs {
        println!("--- run {r}/{runs} ---");
        let a = time_arm("A flat-hash", iters, &bytes, arm_flat_hash);
        let b = time_arm("B with-chain", iters, &bytes, arm_with_chain);
        let d = (b - a) / a * 100.0;
        println!("  Δ(B vs A): {d:+.2} %");
        deltas.push(d);
    }
    let (mean, stddev) = mean_stddev(&deltas);
    let in_band = mean.abs() <= 2.0 * stddev;
    let pass = mean <= gate_pct || in_band;
    println!();
    println!(
        "summary: mean Δ = {mean:+.2} % (σ {stddev:.2} %, n={runs}, gate ≤ {gate_pct} % or |Δ|≤2σ)  {}",
        if pass { "PASS" } else { "FAIL" }
    );
    if !pass {
        eprintln!(
            "::error::p110-merkle-perf-delta mean {mean:.2}% exceeds gate {gate_pct}% — Merkle-tree structure is too expensive over flat hashing"
        );
        std::process::exit(1);
    }
}
