// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p113-fuzz-replay` — read a finding archive, replay each
//! recorded finding, and assert the verdicts match.
//!
//! This is the binding gate behind "every disagreement reproducible
//! byte-for-byte" (README §9 row 3). Run protocol:
//!
//! ```text
//!   p113-fuzz-replay --archive fuzz/findings/archive.ndjson \
//!                    --probe target/zip-aosp-runtime-probe \
//!                    [--id <finding-id>]   # replay one
//!                    [--limit N]           # replay first N findings
//! ```

#![forbid(unsafe_code)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::uninlined_format_args,
    clippy::option_if_let_else,
    clippy::manual_let_else,
    clippy::missing_const_for_fn
)]

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use p113_fuzz_harness::{archive, classifier, differ};

const VERSION: &str = "p113-fuzz-replay 0.1.0";

fn parse_arg<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::args()
        .skip_while(|a| a != name)
        .nth(1)
        .and_then(|s| s.parse().ok())
}

fn main() -> std::io::Result<()> {
    let archive_path: PathBuf =
        parse_arg("--archive").unwrap_or_else(|| PathBuf::from("fuzz/findings/archive.ndjson"));
    let probe: PathBuf =
        parse_arg("--probe").unwrap_or_else(|| PathBuf::from("target/zip-aosp-runtime-probe"));
    let only_id: Option<String> = parse_arg("--id");
    let limit: Option<usize> = parse_arg("--limit");
    let timeout_ms: u64 = parse_arg("--timeout-ms").unwrap_or(2000);

    println!("{VERSION}");
    println!(
        "  archive={}  probe={}",
        archive_path.display(),
        probe.display()
    );

    let findings = archive::read_findings(&archive_path)?;
    if findings.is_empty() {
        eprintln!("WARN: archive empty");
        return Ok(());
    }
    let inputs_root = archive_path.parent().unwrap_or_else(|| Path::new("."));

    let mut total = 0usize;
    let mut replayed = 0usize;
    let mut bit_identical = 0usize;
    let mut diverged = 0usize;
    let mut missing_input = 0usize;

    for f in findings.iter().filter(|f| match &only_id {
        Some(id) => f.finding_id == *id,
        None => true,
    }) {
        if limit.map_or(false, |n| replayed >= n) {
            break;
        }
        total += 1;
        let path = inputs_root.join(&f.input_path);
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!(
                    "WARN missing input for finding {}: {} ({e})",
                    f.finding_id,
                    path.display()
                );
                missing_input += 1;
                continue;
            }
        };
        replayed += 1;
        let (axiom, target, bucket) =
            match differ::run_diff(&bytes, &probe, Duration::from_millis(timeout_ms)) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("WARN replay i/o {}: {e}", f.finding_id);
                    diverged += 1;
                    continue;
                }
            };
        let same = axiom == f.axiom_l0 && target == f.target && bucket.label() == f.bucket.label();
        let _ = classifier::classify(&axiom, &target);
        if same {
            bit_identical += 1;
        } else {
            diverged += 1;
            eprintln!(
                "FAIL replay {}: archive=({:?},{:?},{}) replay=({:?},{:?},{})",
                f.finding_id,
                f.axiom_l0,
                f.target,
                f.bucket.label(),
                axiom,
                target,
                bucket.label()
            );
        }
    }

    println!();
    println!("=== summary ===");
    println!("  archive entries scanned : {}", total);
    println!("  replayed                : {}", replayed);
    println!("  bit-identical           : {}", bit_identical);
    println!("  diverged                : {}", diverged);
    println!("  missing inputs          : {}", missing_input);

    if diverged > 0 {
        eprintln!(
            "::error::p113-fuzz-replay: {diverged} divergence(s) — findings not reproducible"
        );
        std::process::exit(1);
    }
    if missing_input > 0 {
        eprintln!(
            "::error::p113-fuzz-replay: {missing_input} finding(s) reference inputs that no longer exist on disk"
        );
        std::process::exit(1);
    }
    println!("  verdict                 : PASS");
    Ok(())
}
