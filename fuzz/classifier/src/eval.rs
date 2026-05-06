// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p114-classify-eval` — measures classifier precision against
//! a hand-labelled holdout. Reads two files:
//!
//!   1. `--archive <path>` — the harness's `archive.ndjson`.
//!   2. `--holdout <path>` — TSV (`input_sha256\tground_truth_label\n`)
//!      curated by hand. Ground-truth labels are one of:
//!      `aosp-cve-candidate`, `cross-version-evasion`,
//!      `model-bug`, `spec-ambiguity`.
//!
//! Outputs per-label precision, recall, F1, and an overall
//! micro-precision. Exits non-zero if micro-precision falls
//! below `--min-precision` (default 0.80, the P1.14 §10 HARD
//! gate).

#![forbid(unsafe_code)]
#![allow(clippy::uninlined_format_args, clippy::cast_precision_loss)]

use std::collections::HashMap;
use std::path::PathBuf;

use p113_fuzz_harness::archive::Finding;
use p114_classifier::{group_by_input, Classifier};

const VERSION: &str = "p114-classify-eval 0.1.0";

fn arg<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::args()
        .skip_while(|a| a != name)
        .nth(1)
        .and_then(|s| s.parse().ok())
}

fn main() -> std::io::Result<()> {
    let archive: PathBuf =
        arg("--archive").unwrap_or_else(|| PathBuf::from("fuzz/findings/archive.ndjson"));
    let holdout: PathBuf =
        arg("--holdout").unwrap_or_else(|| PathBuf::from("fuzz/classifier/holdout.tsv"));
    let min_precision: f64 = arg("--min-precision").unwrap_or(0.80);

    println!("{VERSION}");
    println!(
        "  archive={}  holdout={}  min-precision={:.2}",
        archive.display(),
        holdout.display(),
        min_precision
    );

    // Build sha → ground-truth map.
    let raw = std::fs::read_to_string(&holdout)?;
    let mut gt: HashMap<String, String> = HashMap::new();
    for (lineno, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.splitn(2, '\t');
        let sha = match parts.next() {
            Some(s) => s.trim().to_string(),
            None => continue,
        };
        let label = match parts.next() {
            Some(l) => l.trim().to_string(),
            None => {
                eprintln!("WARN holdout line {} missing label: {line}", lineno + 1);
                continue;
            }
        };
        gt.insert(sha, label);
    }
    println!("  holdout records   : {}", gt.len());

    // Load harness archive + classify.
    let raw = std::fs::read_to_string(&archive)?;
    let findings: Vec<Finding> = raw
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(Finding::from_ndjson_line)
        .collect();
    let groups = group_by_input(&findings);
    println!("  archive groups    : {}", groups.len());

    // For each holdout sha, fetch the predicted label.
    let mut tp: HashMap<String, u64> = HashMap::new();
    let mut fp: HashMap<String, u64> = HashMap::new();
    let mut fn_: HashMap<String, u64> = HashMap::new();
    let mut total_evaluated: u64 = 0;
    let mut missing: u64 = 0;

    for g in &groups {
        let truth = match gt.get(&g.input_sha256) {
            Some(t) => t.clone(),
            None => continue, // not in holdout
        };
        total_evaluated += 1;
        let pred = match Classifier::classify(g) {
            Some((l, _, _)) => l.as_str().to_string(),
            None => {
                missing += 1;
                continue;
            }
        };
        if pred == truth {
            *tp.entry(pred.clone()).or_default() += 1;
        } else {
            *fp.entry(pred.clone()).or_default() += 1;
            *fn_.entry(truth).or_default() += 1;
        }
    }

    let labels = ["aosp-cve-candidate", "cross-version-evasion", "model-bug", "spec-ambiguity"];
    println!();
    println!("=== per-label metrics ===");
    println!("  {:<28} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}", "label", "tp", "fp", "fn", "prec", "recall", "f1");
    let mut sum_tp: u64 = 0;
    let mut sum_fp: u64 = 0;
    for label in &labels {
        let t = *tp.get(*label).unwrap_or(&0);
        let f = *fp.get(*label).unwrap_or(&0);
        let n = *fn_.get(*label).unwrap_or(&0);
        sum_tp += t;
        sum_fp += f;
        let prec = if t + f == 0 { 0.0 } else { t as f64 / (t + f) as f64 };
        let recall = if t + n == 0 { 0.0 } else { t as f64 / (t + n) as f64 };
        let f1 = if prec + recall == 0.0 { 0.0 } else { 2.0 * prec * recall / (prec + recall) };
        println!(
            "  {:<28} {:>8} {:>8} {:>8} {:>8.3} {:>8.3} {:>8.3}",
            label, t, f, n, prec, recall, f1
        );
    }

    let micro_prec = if sum_tp + sum_fp == 0 {
        0.0
    } else {
        sum_tp as f64 / (sum_tp + sum_fp) as f64
    };
    println!();
    println!("  total evaluated          : {total_evaluated}");
    println!("  missing predictions      : {missing}");
    println!("  micro precision          : {:.4}", micro_prec);
    println!("  gate (>= {:.2})            : {}", min_precision, if micro_prec >= min_precision { "PASS" } else { "FAIL" });

    if micro_prec < min_precision {
        eprintln!(
            "::error::p114-classify-eval: micro precision {:.4} below gate {}",
            micro_prec, min_precision
        );
        std::process::exit(1);
    }
    Ok(())
}
