// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p114-fuzz-replay` — cross-version-aware replay tool. Reads
//! a `p114-finding-1.1` archive (with `target_version` +
//! `synthetic` fields) and replays each finding against the
//! matching per-version probe; asserts byte-identical
//! reproducibility (HARD gate).
//!
//! Usage:
//!
//! ```text
//!   p114-fuzz-replay \
//!     --archive fuzz/findings/archive.ndjson \
//!     --probe target/zip-aosp-runtime-probe \
//!     --probes "A14:synthetic,A11:synthetic,A8:synthetic" \
//!     [--limit N]
//! ```
//!
//! Mirrors `p113-fuzz-replay` (single-version, schema 1.0) but
//! looks up the right probe per finding via the `--probes` CSV.
//! Synthetic findings are replayed through the synthetic rule
//! layer documented in `version_probes.rs`. Real-probe findings
//! need the matching per-version binary on `--probes`'s path.

#![forbid(unsafe_code)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::uninlined_format_args,
    clippy::option_if_let_else,
    clippy::manual_let_else
)]

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use p113_fuzz_harness::{
    archive,
    classifier::Verdict,
    differ,
    probe::PersistentProbe,
    version_probes::{parse_probes_csv, AndroidVersion, VersionedProbe},
};

const VERSION: &str = "p114-fuzz-replay 0.1.0";

fn parse_arg<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::args()
        .skip_while(|a| a != name)
        .nth(1)
        .and_then(|s| s.parse().ok())
}

fn parse_version(s: &str) -> Option<AndroidVersion> {
    AndroidVersion::parse(s)
}

fn main() -> std::io::Result<()> {
    let archive_path: PathBuf =
        parse_arg("--archive").unwrap_or_else(|| PathBuf::from("fuzz/findings/archive.ndjson"));
    let probe: PathBuf =
        parse_arg("--probe").unwrap_or_else(|| PathBuf::from("target/zip-aosp-runtime-probe"));
    let probes_csv: String = parse_arg("--probes").unwrap_or_default();
    let limit: Option<usize> = parse_arg("--limit");
    let timeout_ms: u64 = parse_arg("--timeout-ms").unwrap_or(5_000);

    println!("{VERSION}");
    println!(
        "  archive={}  probe={}  probes={}",
        archive_path.display(),
        probe.display(),
        probes_csv
    );

    let findings = archive::read_findings(&archive_path)?;
    if findings.is_empty() {
        eprintln!("WARN: archive empty");
        return Ok(());
    }
    let inputs_root = archive_path.parent().unwrap_or_else(|| Path::new("."));

    let probe_timeout = Duration::from_millis(timeout_ms);

    // Build a per-version probe registry. For each declared
    // version in `--probes`, spawn a `VersionedProbe`. If the
    // archive references a version that's not in the registry,
    // we fall back to the primary A14 probe.
    let mut by_version: HashMap<AndroidVersion, Arc<VersionedProbe>> = HashMap::new();
    for (version, path) in parse_probes_csv(&probes_csv) {
        let p_str = path.to_str().unwrap_or("");
        if p_str == "synthetic" {
            match PersistentProbe::spawn(
                &format!("aosp-libziparchive-base-{}", version.label().to_lowercase()),
                &probe,
            ) {
                Ok(base) => {
                    let vp = VersionedProbe::synthetic_layer(version, base.with_timeout(probe_timeout));
                    by_version.insert(version, Arc::new(vp));
                }
                Err(e) => eprintln!("WARN spawn synthetic for {}: {e}", version.label()),
            }
        } else {
            match VersionedProbe::real(version, &path, probe_timeout) {
                Ok(vp) => {
                    by_version.insert(version, Arc::new(vp));
                }
                Err(e) => eprintln!("WARN spawn real {} from {}: {e}", version.label(), path.display()),
            }
        }
    }
    println!("  registered {} versioned probe(s)", by_version.len());

    let mut total = 0usize;
    let mut replayed = 0usize;
    let mut bit_identical = 0usize;
    let mut diverged = 0usize;
    let mut missing_input = 0usize;

    for f in findings.iter() {
        if limit.map_or(false, |n| replayed >= n) {
            break;
        }
        total += 1;
        let path = inputs_root.join(&f.input_path);
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!(
                    "WARN missing input for finding {} ({}): {e}",
                    f.finding_id,
                    path.display()
                );
                missing_input += 1;
                continue;
            }
        };
        replayed += 1;

        // Run axiom (always the same — version-independent).
        let axiom = differ::run_axiom(&bytes);

        // Look up probe by version.
        let target_version = parse_version(&f.target_version);
        let target_v: Verdict = if let Some(v) = target_version.and_then(|v| by_version.get(&v)) {
            match v.run_one(&bytes) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("WARN probe i/o for {}: {e}", f.finding_id);
                    diverged += 1;
                    continue;
                }
            }
        } else {
            // Unknown version or no probe registered — fall back
            // to the primary probe's one-shot mode.
            match differ::run_diff(&bytes, &probe, probe_timeout) {
                Ok((_, target, _)) => target,
                Err(e) => {
                    eprintln!("WARN one-shot replay for {}: {e}", f.finding_id);
                    diverged += 1;
                    continue;
                }
            }
        };

        let same = axiom == f.axiom_l0 && target_v == f.target;
        if same {
            bit_identical += 1;
        } else {
            diverged += 1;
            eprintln!(
                "FAIL replay {} ({} {}): archive=({:?},{:?}) replay=({:?},{:?})",
                f.finding_id,
                f.target_version,
                if f.synthetic { "synthetic" } else { "real" },
                f.axiom_l0,
                f.target,
                axiom,
                target_v
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
            "::error::p114-fuzz-replay: {diverged} divergence(s) — cross-version findings not reproducible"
        );
        std::process::exit(1);
    }
    if missing_input > 0 {
        eprintln!(
            "::error::p114-fuzz-replay: {missing_input} finding(s) reference inputs that no longer exist on disk"
        );
        std::process::exit(1);
    }
    println!("  verdict                 : PASS");
    Ok(())
}
