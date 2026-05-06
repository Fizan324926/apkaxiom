// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p114-classify` — read an `archive.ndjson` produced by the
//! cross-version harness and emit one `classified.ndjson` line
//! per finding-group, with the four-way label, the firing rule
//! id, and the rule's weight. Optionally writes a per-label
//! summary to stdout for dashboard ingestion.

#![forbid(unsafe_code)]
#![allow(clippy::uninlined_format_args)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use p113_fuzz_harness::archive::Finding;
use p114_classifier::{group_by_input, Classifier, Label};

const VERSION: &str = "p114-classify 0.1.0";

fn arg<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::args()
        .skip_while(|a| a != name)
        .nth(1)
        .and_then(|s| s.parse().ok())
}

fn main() -> std::io::Result<()> {
    let archive: PathBuf =
        arg("--archive").unwrap_or_else(|| PathBuf::from("fuzz/findings/archive.ndjson"));
    let out: PathBuf =
        arg("--out").unwrap_or_else(|| PathBuf::from("fuzz/findings/classified.ndjson"));
    let summary_only: bool = std::env::args().any(|a| a == "--summary-only");

    println!("{VERSION}");
    println!("  archive={}  out={}", archive.display(), out.display());

    let raw = std::fs::read_to_string(&archive)?;
    let findings: Vec<Finding> = raw
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(Finding::from_ndjson_line)
        .collect();
    println!("  parsed {} finding records", findings.len());

    let groups = group_by_input(&findings);
    println!("  collapsed into {} input-groups", groups.len());

    let mut writer: Option<std::io::BufWriter<std::fs::File>> = if summary_only {
        None
    } else {
        if let Some(p) = out.parent() {
            std::fs::create_dir_all(p)?;
        }
        Some(std::io::BufWriter::new(std::fs::File::create(&out)?))
    };

    let mut by_label: BTreeMap<&'static str, u64> = BTreeMap::new();
    let mut by_rule: BTreeMap<&'static str, u64> = BTreeMap::new();
    let mut classified = 0u64;
    let mut unlabelled = 0u64;

    for g in &groups {
        match Classifier::classify(g) {
            Some((label, rule_id, weight)) => {
                classified += 1;
                *by_label.entry(label.as_str()).or_default() += 1;
                *by_rule.entry(rule_id).or_default() += 1;
                if let Some(w) = writer.as_mut() {
                    use std::io::Write as _;
                    writeln!(
                        w,
                        "{{\"input_sha256\":\"{}\",\"label\":\"{}\",\"rule\":\"{}\",\"weight\":{},\"versions\":[{}]}}",
                        g.input_sha256,
                        label.as_str(),
                        rule_id,
                        weight,
                        g.findings
                            .iter()
                            .map(|f| format!("\"{}\"", f.target_version))
                            .collect::<Vec<_>>()
                            .join(",")
                    )?;
                }
            }
            None => unlabelled += 1,
        }
    }

    if let Some(mut w) = writer {
        use std::io::Write as _;
        w.flush()?;
    }

    println!();
    println!("=== summary ===");
    println!("  classified groups   : {classified}");
    println!("  unlabelled groups   : {unlabelled} (no rule fired — usually bucket-A)");
    println!();
    for (label, n) in &by_label {
        println!("  {:<30} {n}", label);
    }
    println!();
    println!("=== firing rules ===");
    for (rule, n) in &by_rule {
        println!("  {:<30} {n}", rule);
    }

    // Exit non-zero only if we found AOSP-CVE candidates — useful
    // for CI gates where the dashboard alert should pop on first
    // verifier-vs-runtime gap.
    let cves = by_label
        .get(Label::AospCveCandidate.as_str())
        .copied()
        .unwrap_or(0);
    if cves > 0 {
        eprintln!(
            "p114-classify: {cves} AOSP-CVE candidate(s) — see {}",
            out.display()
        );
    }

    // CI regression gates (audit-2 closure). When a floor is set,
    // missing it is a CI failure. The XV-evasion floor is the
    // canonical "we are still finding cross-version disagreements"
    // signal — a regression here means either the harness regressed
    // or the synthetic-divergence opt-in was unintentionally turned
    // off.
    let min_xv: Option<u64> = arg("--min-xv-gate");
    let min_cve: Option<u64> = arg("--min-cve-gate");
    let xv = by_label
        .get(Label::CrossVersionEvasion.as_str())
        .copied()
        .unwrap_or(0);
    let mut gate_failed = false;
    if let Some(floor) = min_xv {
        if xv < floor {
            eprintln!(
                "::error::p114-classify: cross-version-evasion {xv} below gate {floor}"
            );
            gate_failed = true;
        }
    }
    if let Some(floor) = min_cve {
        if cves < floor {
            eprintln!("::error::p114-classify: aosp-cve-candidate {cves} below gate {floor}");
            gate_failed = true;
        }
    }
    if gate_failed {
        std::process::exit(1);
    }
    Ok(())
}
