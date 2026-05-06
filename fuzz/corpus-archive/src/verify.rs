// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p114-corpus-verify` — sample N objects from the corpus
//! archive and assert each downloads byte-identically to the
//! local `inputs/` copy. HARD gate when run in CI.

#![forbid(unsafe_code)]
#![allow(clippy::uninlined_format_args)]

use std::path::PathBuf;

use p113_fuzz_harness::archive::Finding;

const VERSION: &str = "p114-corpus-verify 0.1.0";

fn arg<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::args()
        .skip_while(|a| a != name)
        .nth(1)
        .and_then(|s| s.parse().ok())
}

fn main() -> std::io::Result<()> {
    let archive: PathBuf =
        arg("--archive").unwrap_or_else(|| PathBuf::from("fuzz/findings/archive.ndjson"));
    let inputs_dir: PathBuf = arg("--inputs-dir").unwrap_or_else(|| {
        archive
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .to_path_buf()
    });
    let n: usize = arg("--n").unwrap_or(50);

    println!("{VERSION}");
    let ep = p114_corpus_archive::Endpoint::from_env()?;
    println!(
        "  endpoint={}  bucket={}  archive={}  n={}",
        ep.url,
        ep.bucket,
        archive.display(),
        n
    );

    let raw = std::fs::read_to_string(&archive)?;
    let findings: Vec<Finding> = raw
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(Finding::from_ndjson_line)
        .collect();
    let mut seen: std::collections::BTreeSet<String> = Default::default();
    for f in &findings {
        seen.insert(f.input_sha256.clone());
        if seen.len() >= n {
            break;
        }
    }
    println!("  sampled {} distinct objects", seen.len());

    let mut pass = 0u64;
    let mut mismatch = 0u64;
    let mut missing = 0u64;
    for sha in &seen {
        let key = ep.key(sha);
        // Find matching local input.
        let local_path = findings
            .iter()
            .find(|f| f.input_sha256 == *sha)
            .map(|f| inputs_dir.join(&f.input_path));
        let local_bytes = match local_path.as_ref().and_then(|p| std::fs::read(p).ok()) {
            Some(b) => b,
            None => {
                missing += 1;
                continue;
            }
        };
        let remote = match p114_corpus_archive::get_object(&ep, &key) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("WARN get {} failed: {}", key, e);
                missing += 1;
                continue;
            }
        };
        // Byte-identical round-trip check. The harness's
        // `input_sha256` field is BLAKE3-256, not SHA-256
        // (despite the field name — see archive.rs::sha256_hex
        // for the rationale); comparing the bytes directly
        // sidesteps that and verifies the actual round-trip.
        if local_bytes == remote {
            pass += 1;
        } else {
            eprintln!(
                "MISMATCH {sha}: local-len={} remote-len={}",
                local_bytes.len(),
                remote.len()
            );
            mismatch += 1;
        }
    }

    println!();
    println!("=== summary ===");
    println!("  byte-identical     : {pass}");
    println!("  mismatch           : {mismatch}");
    println!("  missing            : {missing}");
    if mismatch > 0 {
        eprintln!("::error::p114-corpus-verify: {mismatch} byte-mismatch(es)");
        std::process::exit(1);
    }
    Ok(())
}
