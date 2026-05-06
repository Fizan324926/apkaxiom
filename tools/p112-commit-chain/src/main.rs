// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p112-commit-chain` — P1.12 row 4 "Bench-1K commit-chain
//! reproducibility 100 %" gate.
//!
//! Validates the determinism contract on the **production**
//! P1.10 commit-chain pathway: parsing the same input twice via
//! `axiom_l1_rs::commit_chain::parse_with_commit_chain` produces
//! a bit-identical leaf list and Merkle root. The gate runs the
//! streaming parser over the first 1 000 archives of the
//! Bench-10K corpus twice in a row and asserts:
//!
//!   1. Each input file's BLAKE3 is identical between runs (sanity:
//!      the corpus on disk hasn't changed mid-run).
//!   2. Each archive's commit-chain Merkle root is identical
//!      between runs.
//!   3. The aggregate Merkle root (BLAKE3 fold over the per-archive
//!      roots) is identical between runs.
//!
//! Together these guarantee bit-identical commit chains under
//! re-execution — the substrate Phase 4 .axc artifacts will rely on.

#![forbid(unsafe_code)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::uninlined_format_args
)]

use std::path::{Path, PathBuf};

use axiom_blake3_hacl::{hex_encode, Blake3, Hash, Hasher};
use axiom_l1_rs::commit_chain::{parse_with_commit_chain, CommitChain};

const BENCH_SIZE_DEFAULT: usize = 1000;

fn parse_arg<T: std::str::FromStr>(name: &str, default: T) -> T {
    std::env::args()
        .skip_while(|a| a != name)
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn b3_hash(data: &[u8]) -> Hash {
    let mut h = Blake3::default();
    h.update(data);
    h.finalize_borrow()
}

/// `BLAKE3(0x00 || left || right)` — same internal-node combiner
/// shape the P1.10 commit chain uses, so the aggregate Merkle root
/// shape is consistent across the project.
fn combine(left: &Hash, right: &Hash) -> Hash {
    let mut s = Blake3::default();
    s.update(&[0x00]);
    s.update(left);
    s.update(right);
    s.finalize_borrow()
}

fn merkle_root(leaves: &[Hash]) -> Hash {
    if leaves.is_empty() {
        return b3_hash(b"");
    }
    let mut level: Vec<Hash> = leaves.to_vec();
    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len().div_ceil(2));
        let mut i = 0;
        while i < level.len() {
            let l = level[i];
            let r = if i + 1 < level.len() {
                level[i + 1]
            } else {
                level[i]
            };
            next.push(combine(&l, &r));
            i += 2;
        }
        level = next;
    }
    level[0]
}

fn hex(h: &Hash) -> String {
    hex_encode(h)
}

struct RunOutcome {
    input_hashes: Vec<Hash>,
    /// One BLAKE3 root per archive — emitted by the production
    /// `parse_with_commit_chain` pathway, not a custom canonicaliser.
    output_hashes: Vec<Hash>,
    /// Total leaf count summed across the run, surfaces drift early.
    total_leaves: u64,
    aggregate_root: Hash,
}

fn run(corpus: &Path, count: usize) -> std::io::Result<RunOutcome> {
    let mut input_hashes = Vec::with_capacity(count);
    let mut output_hashes = Vec::with_capacity(count);
    let mut total_leaves: u64 = 0;
    for i in 0..count {
        let path = corpus.join(format!("{i:05}.bin"));
        let bytes = std::fs::read(&path)?;
        input_hashes.push(b3_hash(&bytes));
        let cursor = std::io::Cursor::new(bytes.as_slice());
        let (_events, chain): (_, CommitChain) = parse_with_commit_chain(cursor).map_err(|e| {
            std::io::Error::other(format!(
                "P1.10 commit-chain parse failed at sample {i}: {e:?}"
            ))
        })?;
        output_hashes.push(chain.root);
        total_leaves += chain.leaves.len() as u64;
    }
    let aggregate_root = merkle_root(&output_hashes);
    Ok(RunOutcome {
        input_hashes,
        output_hashes,
        total_leaves,
        aggregate_root,
    })
}

fn main() {
    let corpus_dir: String = parse_arg("--corpus", "corpus/zip/bench-10k".to_string());
    let count: usize = parse_arg("--count", BENCH_SIZE_DEFAULT);
    let corpus = PathBuf::from(&corpus_dir);

    println!(
        "p112-commit-chain: {} archives × 2 runs from {} (P1.10 production chain)",
        count, corpus_dir
    );

    let r1 = match run(&corpus, count) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("ERROR run 1: {e}");
            std::process::exit(2);
        }
    };
    let r2 = match run(&corpus, count) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("ERROR run 2: {e}");
            std::process::exit(2);
        }
    };

    let input_match = r1.input_hashes == r2.input_hashes;
    let output_match = r1.output_hashes == r2.output_hashes;
    let agg_match = r1.aggregate_root == r2.aggregate_root;
    let leaves_match = r1.total_leaves == r2.total_leaves;

    let mut input_diffs = 0usize;
    let mut output_diffs = 0usize;
    for i in 0..count {
        if r1.input_hashes[i] != r2.input_hashes[i] {
            input_diffs += 1;
        }
        if r1.output_hashes[i] != r2.output_hashes[i] {
            output_diffs += 1;
        }
    }

    println!();
    println!("=== summary ===");
    println!(
        "  input-hash agreement  : {}/{}  ({})",
        count - input_diffs,
        count,
        if input_match { "PASS" } else { "FAIL" }
    );
    println!(
        "  per-archive root match: {}/{}  ({})",
        count - output_diffs,
        count,
        if output_match { "PASS" } else { "FAIL" }
    );
    println!(
        "  total leaves run 1/2  : {} / {}  ({})",
        r1.total_leaves,
        r2.total_leaves,
        if leaves_match { "PASS" } else { "FAIL" }
    );
    println!("  aggregate root run 1  : {}", hex(&r1.aggregate_root));
    println!("  aggregate root run 2  : {}", hex(&r2.aggregate_root));
    println!(
        "  aggregate root match  : {}",
        if agg_match { "PASS" } else { "FAIL" }
    );

    let pass = input_match && output_match && agg_match && leaves_match;
    if !pass {
        eprintln!("::error::p112-commit-chain reproducibility FAILED");
        std::process::exit(1);
    }
    println!();
    println!(
        "p112-commit-chain: 100 % reproducibility on {} archives via P1.10 production chain ✓",
        count
    );
}
