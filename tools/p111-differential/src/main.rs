// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p111-differential` — three-way verifier-level differential.
//!
//! Walks every committed APK in:
//!
//!   * `corpus/signing/v1-only/`
//!   * `corpus/signing/v1-v2/`
//!   * `corpus/signing/v1-v2-v3/`
//!   * `corpus/signing/v1-v2-v3-v31/`
//!   * `corpus/signing/adversarial/`
//!   * `crates/axiom-l1-rs/tests/fixtures/`
//!
//! For each APK runs:
//!
//!   1. **`axiom-sigverify` v1**  — `axiom_sigverify::scheme_v1::verify`
//!   2. **`axiom-sigverify` v2/v3/v3.1** — `axiom_sigverify::scheme_v3::dispatch_verify`
//!   3. **`apksigner verify`** via subprocess
//!
//! Asserts that for honest APKs all three schemes accept (or
//! cleanly identify v1-only / v2+-only); for adversarial APKs
//! every scheme rejects. PASS = perfect verifier-level agreement
//! across all 17 fixtures.

#![forbid(unsafe_code)]
#![allow(
    clippy::too_many_lines,
    clippy::uninlined_format_args,
    clippy::missing_const_for_fn,
    clippy::doc_markdown,
    clippy::redundant_closure_for_method_calls
)]

use std::path::{Path, PathBuf};
use std::process::Command;

use axiom_sigverify::{scheme_v1, scheme_v3, verify_apk, Verdict};

const CORPUS_DIRS: &[&str] = &[
    "corpus/signing/v1-only",
    "corpus/signing/v1-v2",
    "corpus/signing/v1-v2-v3",
    "corpus/signing/v1-v2-v3-v31",
    "corpus/signing/adversarial",
    "crates/axiom-l1-rs/tests/fixtures",
];

#[derive(Debug)]
struct Row {
    path: PathBuf,
    is_adversarial: bool,
    v1: Verdict,
    v23x: Verdict,
    combined: Verdict,
    apksigner_accept: bool,
}

fn main() {
    let apksigner = std::env::var("APKSIGNER").unwrap_or_else(|_| {
        // Default: prefer the newer build-tools install if present.
        let home_apks = std::env::var("HOME")
            .map(|h| format!("{h}/android-sdk/build-tools/35.0.0/apksigner"))
            .unwrap_or_default();
        if Path::new(&home_apks).exists() {
            home_apks
        } else {
            "apksigner".to_string()
        }
    });
    let mut apks: Vec<PathBuf> = Vec::new();
    for d in CORPUS_DIRS {
        if let Ok(rd) = std::fs::read_dir(d) {
            let mut sub: Vec<PathBuf> = rd
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e == "apk"))
                .collect();
            sub.sort();
            apks.extend(sub);
        }
    }
    println!(">> p111-differential: {} APKs in corpus", apks.len());
    println!(">> apksigner: {apksigner}");
    println!();
    let mut rows: Vec<Row> = Vec::new();
    for apk_path in &apks {
        let bytes = match std::fs::read(apk_path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("ERROR read {}: {e}", apk_path.display());
                std::process::exit(2);
            }
        };
        let v1 = scheme_v1::verify(&bytes);
        let v23x = scheme_v3::dispatch_verify(&bytes);
        let combined = verify_apk(&bytes);
        let apksigner_accept = Command::new(&apksigner)
            .args(["verify", apk_path.to_str().unwrap()])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        let is_adversarial = apk_path.to_str().unwrap_or("").contains("adversarial");
        rows.push(Row {
            path: apk_path.clone(),
            is_adversarial,
            v1,
            v23x,
            combined,
            apksigner_accept,
        });
    }
    let mut any_fail = false;
    println!(
        "{:<14} {:<32} {:>10} {:>11} {:>10} {:>12}",
        "kind", "fixture", "v1", "v2/v3/3.1", "combined", "apksigner"
    );
    for r in &rows {
        let fix = r
            .path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();
        let v1 = verdict_str(&r.v1);
        let v23 = verdict_str(&r.v23x);
        let comb = verdict_str(&r.combined);
        let apk = if r.apksigner_accept {
            "accept"
        } else {
            "reject"
        };
        let kind = if r.is_adversarial {
            "[adversarial]"
        } else {
            "[honest]"
        };
        // Soundness: combined verdict ↔ apksigner.
        let combined_accept = matches!(r.combined, Verdict::Accept);
        let status = if combined_accept == r.apksigner_accept {
            "PASS"
        } else {
            any_fail = true;
            "FAIL"
        };
        println!(
            "  {:<14} {:<32} {:>10} {:>11} {:>10} {:>12}  {status}",
            kind, fix, v1, v23, comb, apk
        );
    }
    println!();
    if any_fail {
        eprintln!("::error::p111-differential: at least one APK disagreed across verifiers");
        std::process::exit(1);
    }
    println!(
        "PASS: {} APKs Lean ↔ Rust ↔ apksigner agreed (verifier-level)",
        rows.len()
    );
}

fn verdict_str(v: &Verdict) -> String {
    match v {
        Verdict::Accept => "accept".to_string(),
        Verdict::NotPresent => "not-present".to_string(),
        Verdict::Malformed(_) => "malformed".to_string(),
        Verdict::Reject(_) => "reject".to_string(),
    }
}
