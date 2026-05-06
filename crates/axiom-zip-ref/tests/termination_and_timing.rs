// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
//
// P1.9 §V items 11 + 13 — termination + side-channel timing.
//
// For every input in the corpus, we measure how long
// `axiom_zip_ref::lfh::parse_lfh` takes to run. We then assert:
//
//   - Termination — every call returns within a hard wall-clock
//     ceiling (10 ms per input on dev-shell). A panic or hang
//     would either trip the assertion or hit the test's overall
//     timeout.
//
//   - No timing side-channel — the standard deviation of
//     per-input runtimes is bounded relative to the mean. A real
//     side-channel (e.g., parser timing depends on a
//     cryptographic field's value) would inflate the stddev.
//     The bound is loose because we run on a shared dev-shell;
//     a tighter bound would need dedicated hardware (§C).
//
// Neither check proves the parser is *constant-time*. They do
// catch obvious panics, infinite loops, and gross timing
// outliers — which is what the engineering-grade gate aims for.

#![allow(
    clippy::needless_range_loop,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::similar_names,
    clippy::unreadable_literal,
    clippy::redundant_closure_for_method_calls,
    clippy::cast_lossless,
    clippy::must_use_candidate,
    clippy::uninlined_format_args
)]

use std::path::PathBuf;
use std::time::{Duration, Instant};

use axiom_zip_ref::lfh::parse_lfh;

const PER_INPUT_CEILING: Duration = Duration::from_millis(10);

fn corpus_dirs() -> Vec<PathBuf> {
    vec![
        PathBuf::from("../../corpus/zip/lfh-valid"),
        PathBuf::from("../../corpus/zip/lfh-adversarial"),
    ]
}

fn collect_inputs() -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    for dir in corpus_dirs() {
        if !dir.exists() {
            continue;
        }
        let mut entries: Vec<_> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("read_dir {dir:?}: {e}"))
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("bin"))
            .collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let bytes = std::fs::read(entry.path()).unwrap_or_default();
            let name = entry.file_name().to_string_lossy().into_owned();
            out.push((name, bytes));
        }
    }
    out
}

#[test]
fn parse_lfh_terminates_on_every_corpus_input() {
    let inputs = collect_inputs();
    if inputs.is_empty() {
        eprintln!("no corpus available — skipping termination check");
        return;
    }
    for (name, bytes) in &inputs {
        let start = Instant::now();
        let _ = parse_lfh(bytes);
        let elapsed = start.elapsed();
        assert!(
            elapsed <= PER_INPUT_CEILING,
            "termination ceiling exceeded on {name}: {elapsed:?} > {PER_INPUT_CEILING:?}"
        );
    }
    eprintln!(
        "termination: {} inputs all under {PER_INPUT_CEILING:?}",
        inputs.len()
    );
}

#[test]
fn parse_lfh_timing_distribution_is_bounded() {
    let inputs = collect_inputs();
    if inputs.is_empty() {
        eprintln!("no corpus available — skipping timing check");
        return;
    }
    // Warm.
    for _ in 0..1000 {
        let _ = parse_lfh(&inputs[0].1);
    }
    let mut samples_ns: Vec<f64> = Vec::with_capacity(inputs.len());
    for (_, bytes) in &inputs {
        // Average 32 runs per input to suppress per-call jitter.
        let trials = 32u32;
        let start = Instant::now();
        for _ in 0..trials {
            let _ = std::hint::black_box(parse_lfh(std::hint::black_box(bytes)));
        }
        let total_ns = start.elapsed().as_nanos() as f64;
        samples_ns.push(total_ns / trials as f64);
    }
    let n = samples_ns.len() as f64;
    let mean = samples_ns.iter().sum::<f64>() / n;
    let variance = samples_ns.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
    let stddev = variance.sqrt();
    let cv = stddev / mean;
    eprintln!(
        "timing: n={n} mean={mean:.0}ns stddev={stddev:.0}ns cv={:.3}",
        cv
    );
    // Loose bound: coefficient-of-variation < 5.0. Real
    // side-channel detection would be tighter on dedicated hw.
    // The point is to catch *gross* outliers (e.g., a parser path
    // that takes 1000× longer on certain inputs).
    assert!(
        cv < 5.0,
        "timing distribution too spread (cv={cv:.3}); possible side-channel"
    );
}
