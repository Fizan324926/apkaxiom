// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p114-corpus-push` — walk a harness `archive.ndjson`, locate
//! every input file referenced by `input_path`, and PUT each
//! into the configured S3-compatible object store.

#![forbid(unsafe_code)]
#![allow(clippy::uninlined_format_args)]

use std::collections::HashSet;
use std::path::PathBuf;

use p113_fuzz_harness::archive::Finding;

const VERSION: &str = "p114-corpus-push 0.1.0";

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
    let max_objects: usize = arg("--max").unwrap_or(usize::MAX);

    println!("{VERSION}");
    let ep = p114_corpus_archive::Endpoint::from_env()?;
    println!(
        "  endpoint={}  bucket={}  archive={}  inputs-dir={}",
        ep.url,
        ep.bucket,
        archive.display(),
        inputs_dir.display(),
    );

    // Ensure bucket exists. 200 / 409 (already exists) are both OK.
    let code = p114_corpus_archive::create_bucket(&ep)?;
    println!("  bucket-create http={}", code);
    if code != 200 && code != 409 {
        eprintln!("WARN bucket create returned {code} — continuing anyway");
    }

    let raw = std::fs::read_to_string(&archive)?;
    let findings: Vec<Finding> = raw
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(Finding::from_ndjson_line)
        .collect();
    println!("  parsed {} finding records", findings.len());

    let mut seen: HashSet<String> = HashSet::new();
    let mut pushed = 0u64;
    let mut errors = 0u64;
    for f in &findings {
        if !seen.insert(f.input_sha256.clone()) {
            continue;
        }
        if seen.len() > max_objects {
            break;
        }
        let bytes = match std::fs::read(inputs_dir.join(&f.input_path)) {
            Ok(b) => b,
            Err(_) => continue,
        };
        let key = ep.key(&f.input_sha256);
        match p114_corpus_archive::put_object(&ep, &key, &bytes) {
            Ok(code) if (200..300).contains(&code) => pushed += 1,
            Ok(code) => {
                eprintln!("WARN put {} returned {}", key, code);
                errors += 1;
            }
            Err(e) => {
                eprintln!("WARN put {} failed: {}", key, e);
                errors += 1;
            }
        }
        if pushed % 50 == 0 && pushed > 0 {
            println!("  pushed {} objects (errors={errors})", pushed);
        }
    }
    println!();
    println!("=== summary ===");
    println!("  distinct inputs    : {}", seen.len());
    println!("  pushed objects     : {pushed}");
    println!("  errors             : {errors}");
    Ok(())
}
