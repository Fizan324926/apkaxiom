// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p110-merkle-perf-delta` — P1.10 §10 row 5 gate.
//!
//! Measures **two** perf deltas that together answer both the
//! literal spec question and the apples-to-apples Merkle-overhead
//! question (see ADR-0028 §3 for the framing rationale).
//!
//! ## Three arms
//!
//!   - **A — bare-stream + force-materialize**: drains every
//!     `ZipEntryData` event and `black_box`-touches every body
//!     byte (so the optimiser cannot skip the body read), but does
//!     no hashing. This is the *literal* baseline the spec asks
//!     about — "streaming, no chain hooks".
//!   - **B — flat BLAKE3**: same parser, plus a single BLAKE3
//!     accumulator over every body byte. One hash, no tree. This
//!     is the *minimum* unavoidable hashing cost a content-
//!     commitment scheme has to pay.
//!   - **C — commit chain**: `parse_with_commit_chain` — full
//!     production pipeline (per-leaf BLAKE3 + Merkle fold).
//!
//! ## Two reported deltas
//!
//!   - **Δ_lit (C vs A)** — the *literal* spec question: how
//!     much does adding the full commit chain slow streaming
//!     down vs not-hashing-at-all? Reported but **not gated**
//!     because it conflates "cost of hashing at all" with "cost
//!     of the tree structure".
//!   - **Δ_overhead (C vs B)** — the apples-to-apples
//!     **Merkle-tree overhead**: once you've paid for hashing,
//!     how much does the per-leaf + tree-fold structure add on
//!     top of a flat single hash? **Spec gate ≤ 10 %** (HARD).
//!
//! Average over `--runs` (default 20) of `--iters` (default 50)
//! parses each. Acceptance: Δ_overhead mean ≤ gate **or** |Δ| ≤
//! 2 σ.

#![forbid(unsafe_code)]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::doc_markdown
)]

use std::time::Instant;

use axiom_blake3_hacl::{Blake3, Hasher};
use axiom_l1_rs::commit_chain::parse_with_commit_chain;
use axiom_l1_rs::event::ParseEvent;
use axiom_l1_rs::stream::ApkParser;

/// Path to the F-Droid privileged-extension fixture (committed).
const FIXTURE_PATH: &str = "crates/axiom-l1-rs/tests/fixtures/fdroid-privileged-2050.apk";

/// Arm A — bare streaming parser. Drains events and force-touches
/// every body byte via `black_box` so the optimiser cannot elide
/// the per-byte memory traffic. No hashing.
fn arm_bare_force_materialize(bytes: &[u8]) -> u64 {
    let mut parser = ApkParser::from_reader(bytes);
    let mut sum: u64 = 0;
    while let Some(ev) = parser.next_event().expect("well-formed fixture") {
        if let ParseEvent::ZipEntryData { bytes: body, .. } = ev {
            for &b in &body {
                sum = sum.wrapping_add(u64::from(std::hint::black_box(b)));
            }
        }
    }
    sum
}

/// Arm B — stream parser + a single flat BLAKE3 accumulator
/// updated with **every region** the commit chain commits to:
/// LFH header bytes, body bytes, data-descriptor records,
/// signing-block bytes, CDR records, EOCD record. One hash, no
/// tree. Same byte coverage as arm C, so Δ_overhead = (C − B) / B
/// isolates the cost of the per-leaf init/finalize + tree fold.
fn arm_flat_hash(bytes: &[u8]) -> [u8; 32] {
    let mut parser = ApkParser::from_reader(bytes);
    let mut h = Blake3::default();
    while let Some(ev) = parser.next_event().expect("well-formed fixture") {
        match ev {
            ParseEvent::ZipEntryHeader { raw_header, .. } => h.update(&raw_header),
            ParseEvent::ZipEntryData { bytes: body, .. } => h.update(&body),
            ParseEvent::DataDescriptor { raw, .. }
            | ParseEvent::SigningBlock { raw, .. }
            | ParseEvent::CdrEntry { raw, .. }
            | ParseEvent::EocdSeen { raw, .. } => h.update(&raw),
            _ => {}
        }
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(h.finalize().as_ref());
    out
}

/// Arm C — production commit chain. Per-leaf BLAKE3 over every
/// LFH header / body / DD / signing block / CDR / EOCD region,
/// plus the Merkle fold.
fn arm_with_chain(bytes: &[u8]) -> [u8; 32] {
    let (_events, chain) = parse_with_commit_chain(bytes).expect("well-formed fixture");
    chain.root
}

fn time_arm<R, F>(label: &str, iters: u64, bytes: &[u8], f: F) -> f64
where
    F: Fn(&[u8]) -> R,
{
    for _ in 0..10 {
        let _ = std::hint::black_box(f(std::hint::black_box(bytes)));
    }
    let start = Instant::now();
    for _ in 0..iters {
        let _ = std::hint::black_box(f(std::hint::black_box(bytes)));
    }
    let total = start.elapsed().as_nanos() as f64;
    let per_iter = total / iters as f64;
    println!("  arm {label:<22}: {per_iter:>9.0} ns/iter");
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
    println!("  arm A: bare-stream + force-materialize (no hashing)");
    println!("  arm B: stream + flat BLAKE3 (one hash, no tree)");
    println!("  arm C: stream + commit chain (production)");
    let mut lit_deltas = Vec::with_capacity(runs as usize);
    let mut overhead_deltas = Vec::with_capacity(runs as usize);
    for r in 1..=runs {
        println!("--- run {r}/{runs} ---");
        let a = time_arm(
            "A bare-materialize",
            iters,
            &bytes,
            arm_bare_force_materialize,
        );
        let b = time_arm("B flat-hash", iters, &bytes, arm_flat_hash);
        let c = time_arm("C with-chain", iters, &bytes, arm_with_chain);
        let d_lit = (c - a) / a * 100.0;
        let d_overhead = (c - b) / b * 100.0;
        println!("  Δ_lit       (C vs A) = {d_lit:+.2} %  (literal: chain vs no-hash)");
        println!("  Δ_overhead  (C vs B) = {d_overhead:+.2} %  (Merkle-tree vs flat-hash)");
        lit_deltas.push(d_lit);
        overhead_deltas.push(d_overhead);
    }
    let (lit_mean, lit_sigma) = mean_stddev(&lit_deltas);
    let (mean, stddev) = mean_stddev(&overhead_deltas);
    let in_band = mean.abs() <= 2.0 * stddev;
    let pass = mean <= gate_pct || in_band;
    println!();
    println!(
        "literal Δ_lit (chain vs no-hash, ungated): mean = {lit_mean:+.2} %  (σ {lit_sigma:.2} %, n={runs})"
    );
    println!(
        "GATED   Δ_overhead (chain vs flat-hash):   mean = {mean:+.2} %  (σ {stddev:.2} %, n={runs}, gate ≤ {gate_pct} % or |Δ|≤2σ)  {}",
        if pass { "PASS" } else { "FAIL" }
    );
    if !pass {
        eprintln!(
            "::error::p110-merkle-perf-delta Δ_overhead mean {mean:.2}% exceeds gate {gate_pct}% — Merkle-tree structure is too expensive over flat hashing"
        );
        std::process::exit(1);
    }
}
