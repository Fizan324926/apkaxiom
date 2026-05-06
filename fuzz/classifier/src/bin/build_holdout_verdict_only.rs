// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p114-build-holdout-verdict-only` — second, independent
//! ground-truth oracle. Uses **only** the verdict matrix (no
//! input-byte inspection); chosen so that any agreement with the
//! `p114-build-holdout` (input-feature) oracle is non-trivial.
//!
//! ## Oracle rules (verdict-only)
//!
//! For each input-group, look at:
//!   - axiom verdict (accept / reject)
//!   - target verdict per version
//!
//! Then:
//!
//!   1. axiom rejects + at least one target accepts  →  aosp-cve-candidate
//!      (treats verifier-vs-runtime as the primary signal,
//!      ignoring whether OTHER versions also accept — strictly
//!      MORE conservative than the classifier's weight-96 XV
//!      rule, so disagreements with the classifier on borderline
//!      inputs are real signal about the design choice in
//!      `xv.disagreement-real`.)
//!   2. axiom accepts + every target rejects         →  model-bug
//!   3. axiom rejects + every target rejects, same tag  →  spec-ambiguity
//!   4. axiom rejects + every target rejects, mixed tags →  spec-ambiguity
//!   5. otherwise (e.g. axiom-accept, all-target-accept)  →  no label
//!
//! Note rules (1) and (2) intentionally **do not use any
//! input-byte features**. The cross-version-evasion label is
//! therefore *never* emitted by this oracle. Disagreements with
//! the classifier on Cross-version-evasion inputs are
//! deliberately introduced to test the classifier's
//! design-choice (XV > CVE per README §2).

#![forbid(unsafe_code)]
#![allow(clippy::uninlined_format_args)]

use std::collections::HashMap;
use std::path::PathBuf;

use p113_fuzz_harness::archive::Finding;
use p113_fuzz_harness::classifier::Verdict;

const VERSION: &str = "p114-build-holdout-verdict-only 0.1.0";

fn arg<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::args()
        .skip_while(|a| a != name)
        .nth(1)
        .and_then(|s| s.parse().ok())
}

fn oracle_verdict_only(axiom: &Verdict, targets: &[Verdict]) -> Option<&'static str> {
    let any_accept = targets.iter().any(|v| matches!(v, Verdict::Accept));
    let all_reject = !targets.is_empty() && targets.iter().all(|v| matches!(v, Verdict::Reject(_)));
    match axiom {
        Verdict::Accept => {
            if all_reject {
                Some("model-bug")
            } else {
                None
            }
        }
        Verdict::Reject(_) => {
            if any_accept {
                Some("aosp-cve-candidate")
            } else if all_reject {
                Some("spec-ambiguity")
            } else {
                None
            }
        }
    }
}

fn main() -> std::io::Result<()> {
    let archive: PathBuf =
        arg("--archive").unwrap_or_else(|| PathBuf::from("fuzz/findings/archive.ndjson"));
    let out: PathBuf = arg("--out")
        .unwrap_or_else(|| PathBuf::from("fuzz/classifier/holdout-verdict-only.tsv"));
    let max_records: usize = arg("--max").unwrap_or(100);
    let per_label_min: usize = arg("--per-label-min").unwrap_or(10);

    println!("{VERSION}");
    println!(
        "  archive={}  out={}  max={}  per-label-min={}",
        archive.display(),
        out.display(),
        max_records,
        per_label_min
    );

    let raw = std::fs::read_to_string(&archive)?;
    let findings: Vec<Finding> = raw
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(Finding::from_ndjson_line)
        .collect();
    println!("  parsed {} finding records", findings.len());

    let mut by_input: HashMap<String, Vec<Finding>> = HashMap::new();
    for f in &findings {
        by_input.entry(f.input_sha256.clone()).or_default().push(f.clone());
    }

    let mut labelled: Vec<(String, &'static str)> = Vec::new();
    for (sha, fs) in by_input {
        let axiom = fs.first().map(|f| f.axiom_l0.clone()).unwrap_or(Verdict::Accept);
        let targets: Vec<Verdict> = fs.iter().map(|f| f.target.clone()).collect();
        if let Some(label) = oracle_verdict_only(&axiom, &targets) {
            labelled.push((sha, label));
        }
    }

    // Stratified sample.
    let mut by_label: HashMap<&'static str, Vec<(String, &'static str)>> = HashMap::new();
    for it in labelled {
        by_label.entry(it.1).or_default().push(it);
    }
    let mut picked: Vec<(String, &'static str)> = Vec::new();
    for items in by_label.values() {
        for it in items.iter().take(per_label_min) {
            picked.push(it.clone());
        }
    }
    let labels: Vec<_> = by_label.keys().copied().collect();
    let mut cursors: HashMap<&'static str, usize> = labels.iter().map(|l| (*l, per_label_min)).collect();
    while picked.len() < max_records {
        let mut progress = false;
        for label in &labels {
            if picked.len() >= max_records {
                break;
            }
            let cur = cursors.entry(label).or_default();
            if let Some(item) = by_label.get(label).and_then(|v| v.get(*cur)) {
                picked.push(item.clone());
                *cur += 1;
                progress = true;
            }
        }
        if !progress {
            break;
        }
    }

    picked.sort();

    use std::io::Write as _;
    if let Some(p) = out.parent() {
        std::fs::create_dir_all(p)?;
    }
    let mut w = std::io::BufWriter::new(std::fs::File::create(&out)?);
    writeln!(w, "# p114 holdout (verdict-only) — auto-generated by p114-build-holdout-verdict-only from {}", archive.display())?;
    writeln!(w, "# format: <input_sha256>\\t<ground_truth_label>")?;
    let mut counts: HashMap<&'static str, u64> = HashMap::new();
    for (sha, label) in &picked {
        writeln!(w, "{sha}\t{label}")?;
        *counts.entry(*label).or_default() += 1;
    }
    w.flush()?;

    println!();
    println!("=== holdout written ===");
    println!("  records: {}", picked.len());
    for (label, n) in &counts {
        println!("    {:<28} {n}", label);
    }
    Ok(())
}
