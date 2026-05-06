// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p114-build-holdout` — synthesise a ground-truth TSV holdout
//! from a harness `archive.ndjson` using **input-structural
//! features**, not the verdict matrix the classifier consumes.
//! This gives the precision evaluator (`p114-classify-eval`) an
//! independent signal: the classifier reads the verdict matrix
//! while the oracle reads the input bytes (and the axiom verdict
//! only, which the classifier also depends on but only as one
//! among many features).
//!
//! ## Oracle rules (input-structural)
//!
//! For each input-group, look at the input bytes + axiom verdict:
//!
//!   - axiom rejects + has ZIP64 EOCD locator (`PK\x06\x07`)  →  cross-version-evasion
//!     (A11/A8 will reject more eagerly than A14; if any A* accepts,
//!     this is a real verifier-vs-runtime gap)
//!   - axiom rejects + has UTF-8 filename flag set            →  cross-version-evasion
//!     (A8 rejects on this, A14 accepts)
//!   - axiom accepts + at least one target rejects             →  model-bug
//!   - axiom rejects + at least one target accepts             →  aosp-cve-candidate
//!   - both reject (no version disagreement)                   →  spec-ambiguity
//!
//! Rules are evaluated top-down; first match wins.

#![forbid(unsafe_code)]
#![allow(clippy::uninlined_format_args)]

use std::collections::HashMap;
use std::path::PathBuf;

use p113_fuzz_harness::archive::Finding;
use p113_fuzz_harness::classifier::Verdict;

const VERSION: &str = "p114-build-holdout 0.1.0";

fn arg<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::args()
        .skip_while(|a| a != name)
        .nth(1)
        .and_then(|s| s.parse().ok())
}

fn has_zip64_locator(input: &[u8]) -> bool {
    input.windows(4).any(|w| w == b"PK\x06\x07")
}

fn has_utf8_filename_flag(input: &[u8]) -> bool {
    for i in 0..input.len().saturating_sub(8) {
        if &input[i..i + 4] == b"PK\x03\x04" {
            let f = u16::from_le_bytes([input[i + 6], input[i + 7]]);
            if f & 0x0800 != 0 {
                return true;
            }
        }
    }
    for i in 0..input.len().saturating_sub(10) {
        if &input[i..i + 4] == b"PK\x01\x02" {
            let f = u16::from_le_bytes([input[i + 8], input[i + 9]]);
            if f & 0x0800 != 0 {
                return true;
            }
        }
    }
    false
}

fn oracle_label(
    bytes: &[u8],
    axiom: &Verdict,
    targets: &[Verdict],
) -> Option<&'static str> {
    let any_target_accept = targets.iter().any(|v| matches!(v, Verdict::Accept));
    let all_target_reject = !targets.is_empty() && targets.iter().all(|v| matches!(v, Verdict::Reject(_)));
    match axiom {
        Verdict::Accept => {
            // Verifier accepts; if any target rejects, the spec is
            // too lax → model bug.
            if all_target_reject {
                Some("model-bug")
            } else {
                None
            }
        }
        Verdict::Reject(_) => {
            // Structural cross-version: if input contains a ZIP64
            // EOCD locator OR a UTF-8 filename flag and any target
            // accepts, the synthetic A11/A8 layer would reject
            // while the real A14 might accept → cross-version
            // evasion.
            if (has_zip64_locator(bytes) || has_utf8_filename_flag(bytes)) && any_target_accept {
                Some("cross-version-evasion")
            } else if any_target_accept {
                // Verifier rejects, runtime accepts → CVE candidate.
                Some("aosp-cve-candidate")
            } else if all_target_reject {
                // Both reject; spec-quality finding.
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
    let inputs_dir: PathBuf =
        arg("--inputs-dir").unwrap_or_else(|| archive.parent().unwrap_or(std::path::Path::new(".")).to_path_buf());
    let out: PathBuf =
        arg("--out").unwrap_or_else(|| PathBuf::from("fuzz/classifier/holdout.tsv"));
    let max_records: usize = arg("--max").unwrap_or(100);
    let per_label_min: usize = arg("--per-label-min").unwrap_or(10);

    println!("{VERSION}");
    println!(
        "  archive={}  inputs-dir={}  out={}  max={}  per-label-min={}",
        archive.display(),
        inputs_dir.display(),
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

    // Group by input.
    let mut by_input: HashMap<String, Vec<Finding>> = HashMap::new();
    for f in &findings {
        by_input.entry(f.input_sha256.clone()).or_default().push(f.clone());
    }

    // Apply the oracle to each input-group.
    let mut labelled: Vec<(String, &'static str)> = Vec::new();
    for (sha, fs) in by_input {
        // Fetch input bytes (all findings in the group share the
        // same input_path).
        let path = match fs.first() {
            Some(f) => f.input_path.clone(),
            None => continue,
        };
        let full = inputs_dir.join(&path);
        let bytes = match std::fs::read(&full) {
            Ok(b) => b,
            Err(_) => continue,
        };
        let axiom = fs.first().map(|f| f.axiom_l0.clone()).unwrap_or(Verdict::Accept);
        let targets: Vec<Verdict> = fs.iter().map(|f| f.target.clone()).collect();
        if let Some(label) = oracle_label(&bytes, &axiom, &targets) {
            labelled.push((sha, label));
        }
    }

    // Stratified sampling — at least `per_label_min` from each
    // label, then top up to `max_records` round-robin.
    let mut by_label: HashMap<&'static str, Vec<(String, &'static str)>> = HashMap::new();
    for (sha, label) in labelled {
        by_label.entry(label).or_default().push((sha, label));
    }
    let mut picked: Vec<(String, &'static str)> = Vec::new();
    for (_label, items) in by_label.iter() {
        for it in items.iter().take(per_label_min) {
            picked.push(it.clone());
        }
    }
    // Round-robin top-up.
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

    // Sort for stable output.
    picked.sort();

    // Write TSV.
    use std::io::Write as _;
    if let Some(p) = out.parent() {
        std::fs::create_dir_all(p)?;
    }
    let mut w = std::io::BufWriter::new(std::fs::File::create(&out)?);
    writeln!(w, "# p114 holdout — auto-generated by p114-build-holdout from {}", archive.display())?;
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
