// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p112-commit-chain` — P1.12 row 4 "Bench-1K commit-chain
//! reproducibility 100 %" gate.
//!
//! Validates the determinism contract on the verified ZIP layer:
//! parsing the same input twice produces a bit-identical canonical
//! serialisation of the parse result, and hence a bit-identical
//! BLAKE3 commit. The gate runs the verified parser over the first
//! 1 000 archives of the Bench-10K corpus twice in a row and
//! asserts that:
//!
//!   1. Each input file's BLAKE3 is identical between runs (sanity:
//!      the corpus on disk hasn't changed mid-run).
//!   2. Each archive's commit-chain root (BLAKE3 over the canonical
//!      serialisation of the verified parse output) is identical
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

use std::{
    fmt::Write,
    path::{Path, PathBuf},
};

use axiom_blake3_hacl::{Blake3, Hash, Hasher};
use axiom_l0_zip_verified::consistency::{parse_archive, Archive};

const BENCH_SIZE_DEFAULT: usize = 1000;

fn parse_arg<T: std::str::FromStr>(name: &str, default: T) -> T {
    std::env::args()
        .skip_while(|a| a != name)
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

/// Canonicalise a parsed `Archive` into a deterministic byte
/// sequence: leaf-by-leaf, in source order, fixed field encoding.
/// Any drift between the verified parser's output structure
/// (field set / order) would be a loud diff in the resulting hash.
fn canonicalise(a: &Archive) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    // EOCD canonical fields.
    out.extend_from_slice(b"eocd:");
    out.extend_from_slice(&a.eocd.disk_number.to_le_bytes());
    out.extend_from_slice(&a.eocd.cd_start_disk.to_le_bytes());
    out.extend_from_slice(&a.eocd.entries_on_this_disk.to_le_bytes());
    out.extend_from_slice(&a.eocd.total_entries.to_le_bytes());
    out.extend_from_slice(&a.eocd.cd_size.to_le_bytes());
    out.extend_from_slice(&a.eocd.cd_offset.to_le_bytes());
    out.extend_from_slice(&(a.eocd.comment.len() as u32).to_le_bytes());
    out.extend_from_slice(&a.eocd.comment);
    // LFHs in source order.
    out.extend_from_slice(b"|lfhs:");
    out.extend_from_slice(&(a.lfhs.len() as u32).to_le_bytes());
    for lfh in &a.lfhs {
        out.extend_from_slice(&lfh.version_needed.to_le_bytes());
        out.extend_from_slice(&lfh.general_flags.to_le_bytes());
        out.extend_from_slice(&lfh.compression_method.to_le_bytes());
        out.extend_from_slice(&lfh.last_mod_time.to_le_bytes());
        out.extend_from_slice(&lfh.last_mod_date.to_le_bytes());
        out.extend_from_slice(&lfh.crc32.to_le_bytes());
        out.extend_from_slice(&lfh.compressed_size.to_le_bytes());
        out.extend_from_slice(&lfh.uncompressed_size.to_le_bytes());
        out.extend_from_slice(&(lfh.file_name.len() as u32).to_le_bytes());
        out.extend_from_slice(&lfh.file_name);
        out.extend_from_slice(&(lfh.extra_field.len() as u32).to_le_bytes());
        out.extend_from_slice(&lfh.extra_field);
    }
    // CDRs in source order.
    out.extend_from_slice(b"|cdrs:");
    out.extend_from_slice(&(a.cdrs.len() as u32).to_le_bytes());
    for c in &a.cdrs {
        out.extend_from_slice(&c.version_made_by.to_le_bytes());
        out.extend_from_slice(&c.version_needed.to_le_bytes());
        out.extend_from_slice(&c.general_flags.to_le_bytes());
        out.extend_from_slice(&c.compression_method.to_le_bytes());
        out.extend_from_slice(&c.last_mod_time.to_le_bytes());
        out.extend_from_slice(&c.last_mod_date.to_le_bytes());
        out.extend_from_slice(&c.crc32.to_le_bytes());
        out.extend_from_slice(&c.compressed_size.to_le_bytes());
        out.extend_from_slice(&c.uncompressed_size.to_le_bytes());
        out.extend_from_slice(&c.disk_number_start.to_le_bytes());
        out.extend_from_slice(&c.internal_file_attributes.to_le_bytes());
        out.extend_from_slice(&c.external_file_attributes.to_le_bytes());
        out.extend_from_slice(&c.lfh_offset.to_le_bytes());
        out.extend_from_slice(&(c.file_name.len() as u32).to_le_bytes());
        out.extend_from_slice(&c.file_name);
        out.extend_from_slice(&(c.extra_field.len() as u32).to_le_bytes());
        out.extend_from_slice(&c.extra_field);
        out.extend_from_slice(&(c.file_comment.len() as u32).to_le_bytes());
        out.extend_from_slice(&c.file_comment);
    }
    out
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
    let mut s = String::with_capacity(h.len() * 2);
    for b in h {
        let _ = write!(s, "{b:02x}");
    }
    s
}

struct RunOutcome {
    input_hashes: Vec<Hash>,
    output_hashes: Vec<Hash>,
    aggregate_root: Hash,
}

fn run(corpus: &Path, count: usize) -> std::io::Result<RunOutcome> {
    let mut input_hashes = Vec::with_capacity(count);
    let mut output_hashes = Vec::with_capacity(count);
    for i in 0..count {
        let path = corpus.join(format!("{i:05}.bin"));
        let bytes = std::fs::read(&path)?;
        input_hashes.push(b3_hash(&bytes));
        let parsed = parse_archive(&bytes).map_err(|e| {
            std::io::Error::other(format!("verified parse failed at sample {i}: {e:?}"))
        })?;
        let canon = canonicalise(&parsed);
        output_hashes.push(b3_hash(&canon));
    }
    let aggregate_root = merkle_root(&output_hashes);
    Ok(RunOutcome {
        input_hashes,
        output_hashes,
        aggregate_root,
    })
}

fn main() {
    let corpus_dir: String = parse_arg("--corpus", "corpus/zip/bench-10k".to_string());
    let count: usize = parse_arg("--count", BENCH_SIZE_DEFAULT);
    let corpus = PathBuf::from(&corpus_dir);

    println!(
        "p112-commit-chain: {} archives × 2 runs from {}",
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
        "  output-hash agreement : {}/{}  ({})",
        count - output_diffs,
        count,
        if output_match { "PASS" } else { "FAIL" }
    );
    println!("  aggregate root run 1  : {}", hex(&r1.aggregate_root));
    println!("  aggregate root run 2  : {}", hex(&r2.aggregate_root));
    println!(
        "  aggregate root match  : {}",
        if agg_match { "PASS" } else { "FAIL" }
    );

    let pass = input_match && output_match && agg_match;
    if !pass {
        eprintln!("::error::p112-commit-chain reproducibility FAILED");
        std::process::exit(1);
    }
    println!();
    println!(
        "p112-commit-chain: 100 % reproducibility on {} archives ✓",
        count
    );
}
