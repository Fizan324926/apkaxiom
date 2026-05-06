// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p112-tamper-fuzz` — P1.12 Gap-8 differential fuzz gate.
//!
//! 10 000 random single-bit flips × the first 1 000 Bench-10K
//! archives. For every mutation, asserts the **verified umbrella
//! and the hand-Rust direct route produce the same `ArchiveError`
//! tag** (or both accept). Any divergence is a real bug — either
//! the umbrella has drifted from a `pub use`, or one of the two
//! paths is non-deterministic.
//!
//! Exit non-zero on any divergence. Prints the offending sample
//! id, byte offset, bit position, and the two verdicts so the
//! divergence is reproducible from the printed seed.

#![forbid(unsafe_code)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::uninlined_format_args
)]

use std::path::PathBuf;

use axiom_l0_zip_verified::consistency as verified;
use axiom_zip_ref::archive as direct;

const RUNS_DEFAULT: u64 = 10_000;
const ARCHIVES_DEFAULT: usize = 1_000;
const SEED_DEFAULT: u64 = 0xb112_fa17_c0de_0001;

const fn lcg_next(s: u64) -> u64 {
    s.wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407)
}

fn parse_arg<T: std::str::FromStr>(name: &str, default: T) -> T {
    std::env::args()
        .skip_while(|a| a != name)
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn verdict_tag(a: Result<verified::Archive, verified::ArchiveError>) -> u8 {
    match a {
        Ok(_) => 0,
        Err(e) => e.tag(),
    }
}

fn verdict_tag_direct(a: Result<direct::Archive, direct::ArchiveError>) -> u8 {
    match a {
        Ok(_) => 0,
        Err(e) => e.tag(),
    }
}

fn main() {
    let runs: u64 = parse_arg("--runs", RUNS_DEFAULT);
    let archives: usize = parse_arg("--archives", ARCHIVES_DEFAULT);
    let mut seed: u64 = parse_arg("--seed", SEED_DEFAULT);
    let corpus: String = parse_arg("--corpus", "corpus/zip/bench-10k".to_string());
    let dir = PathBuf::from(&corpus);

    let mut samples: Vec<Vec<u8>> = Vec::with_capacity(archives);
    for i in 0..archives {
        let p = dir.join(format!("{i:05}.bin"));
        let bytes = std::fs::read(&p).unwrap_or_else(|e| {
            eprintln!("ERROR read {}: {}", p.display(), e);
            std::process::exit(2);
        });
        samples.push(bytes);
    }

    println!(
        "p112-tamper-fuzz: {} mutations × {} archives = {} trials, seed={:#018x}",
        runs,
        samples.len(),
        runs * samples.len() as u64,
        SEED_DEFAULT,
    );

    let mut total_trials: u64 = 0;
    let mut both_accept: u64 = 0;
    let mut both_reject_same_tag: u64 = 0;
    let mut divergences: u64 = 0;
    let mut first_divergences: Vec<String> = Vec::new();

    for r in 0..runs {
        for (idx, base) in samples.iter().enumerate() {
            seed = lcg_next(seed);
            let off = (seed >> 32) as usize % base.len();
            seed = lcg_next(seed);
            let bit = ((seed >> 56) & 0x07) as u8;

            let mut mutated = base.clone();
            mutated[off] ^= 1 << bit;

            let v = verdict_tag(verified::parse_archive(&mutated));
            let d = verdict_tag_direct(direct::parse_archive(&mutated));

            total_trials += 1;
            if v == d {
                if v == 0 {
                    both_accept += 1;
                } else {
                    both_reject_same_tag += 1;
                }
            } else {
                divergences += 1;
                if first_divergences.len() < 20 {
                    first_divergences.push(format!(
                        "  run={r} sample={idx} off={off} bit={bit} verified={v} direct={d}"
                    ));
                }
            }
        }
    }

    println!();
    println!("=== summary ===");
    println!("  total trials          : {total_trials}");
    println!("  both accept           : {both_accept}");
    println!("  both reject same tag  : {both_reject_same_tag}");
    println!("  divergences           : {divergences}");
    if divergences > 0 {
        println!();
        println!("first {} divergences:", first_divergences.len());
        for s in &first_divergences {
            println!("{s}");
        }
        eprintln!("::error::p112-tamper-fuzz: {divergences} verified-vs-direct divergences");
        std::process::exit(1);
    }
    println!("  verdict               : PASS — verified ≡ direct on every mutation");
}
