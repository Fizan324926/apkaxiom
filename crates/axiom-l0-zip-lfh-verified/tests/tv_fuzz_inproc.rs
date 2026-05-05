// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! P1.9 §IV gap-4 — mutation fuzz of the TV harness's hot path.
//!
//! Generates LCG-seeded mutants from the LFH corpus and asserts:
//!
//!   - the verified-shim parser (`axiom_l0_zip_lfh_verified::parse_lfh`,
//!     i.e. hand-Rust)
//!   - the extracted parser (`axiom_l0_zip_lfh_extracted::parse_lfh`,
//!     auto-generated from Lean by `tools/lean-to-rust`)
//!
//! ...produce **byte-identical** results across 10 000 random
//! mutations. If they ever diverge, either:
//!
//!   * the extractor has a semantic bug, OR
//!   * the hand-Rust parser drifted from the Lean reference (and
//!     hence from the extracted parser).
//!
//! Both are real bugs the fuzzer catches *before* they show up in
//! the deterministic 1499-input corpus we already verify in
//! `make tv-three-way`.
//!
//! Seed `0xa9c1_d4b1_f7e2_3d51` matches the P1.5/P1.6/P1.8 seed
//! convention.

#![allow(
    clippy::needless_pass_by_value,
    clippy::cast_possible_truncation,
    clippy::trivially_copy_pass_by_ref,
    clippy::needless_range_loop,
    clippy::similar_names,
    clippy::unreadable_literal,
    clippy::manual_let_else,
    clippy::redundant_closure_for_method_calls,
    clippy::missing_const_for_fn
)]

use std::path::PathBuf;

use axiom_l0_zip_lfh_verified::parse_lfh as verified_parse_lfh;

const LCG_SEED: u64 = 0xa9c1_d4b1_f7e2_3d51;
const ITERATIONS: usize = 10_000;
const FLIPS_PER_MUTANT: usize = 4;

#[derive(Debug, Clone, Copy)]
struct Lcg(u64);
impl Lcg {
    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }
}

fn corpus_seeds() -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let bases: [&str; 2] = [
        "../../corpus/zip/lfh-valid",
        "../../corpus/zip/lfh-adversarial",
    ];
    for base in bases {
        let dir = PathBuf::from(base);
        if !dir.exists() {
            continue;
        }
        let mut entries: Vec<_> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("read_dir {dir:?}: {e}"))
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("bin"))
            .collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let bytes = std::fs::read(entry.path()).unwrap_or_default();
            if !bytes.is_empty() {
                out.push(bytes);
            }
        }
    }
    out
}

fn mutate(buf: &mut [u8], rng: &mut Lcg) {
    if buf.is_empty() {
        return;
    }
    for _ in 0..FLIPS_PER_MUTANT {
        let idx = (rng.next_u64() as usize) % buf.len();
        let xor = (rng.next_u64() & 0xff) as u8;
        buf[idx] ^= xor;
    }
}

#[test]
fn fuzz_verified_vs_extracted_byte_identical() {
    let seeds = corpus_seeds();
    if seeds.is_empty() {
        eprintln!("no corpus seeds found — skipping");
        return;
    }
    let mut rng = Lcg(LCG_SEED);
    let mut accepted = 0u64;
    let mut diverged = 0u64;

    for i in 0..ITERATIONS {
        let seed = &seeds[(rng.next_u64() as usize) % seeds.len()];
        let mut buf = seed.clone();
        mutate(&mut buf, &mut rng);

        let verified = verified_parse_lfh(&buf);
        let extracted = axiom_l0_zip_lfh_extracted::parse_lfh(&buf);

        // Compare structurally — byte-identical isn't applicable
        // because the parsers return Rust values, not byte streams.
        // We compare the discriminant + every public field.
        let agree = match (&verified, &extracted) {
            (Ok((lv, nv)), Ok((le, ne))) => {
                lv.version_needed == le.version_needed
                    && lv.general_flags == le.general_flags
                    && lv.compression_method == le.compression_method
                    && lv.last_mod_time == le.last_mod_time
                    && lv.last_mod_date == le.last_mod_date
                    && lv.crc32 == le.crc32
                    && lv.compressed_size == le.compressed_size
                    && lv.uncompressed_size == le.uncompressed_size
                    && lv.file_name == le.file_name
                    && lv.extra_field == le.extra_field
                    && nv == ne
            }
            (Err(e_v), Err(e_e)) => e_v.tag() == axiom_l0_zip_lfh_extracted_tag(e_e),
            _ => false,
        };
        if agree {
            accepted += 1;
        } else {
            diverged += 1;
            if diverged <= 3 {
                eprintln!("DIVERGE @ iter {i}: verified={verified:?} extracted={extracted:?}");
            }
        }
    }
    eprintln!("tv-fuzz: iters={ITERATIONS} accepted={accepted} diverged={diverged}");
    assert_eq!(
        diverged, 0,
        "verified ↔ extracted disagreement on fuzz inputs"
    );
}

/// Bridge: the extracted crate has its own `ParseError` (auto-
/// generated) that's structurally identical to the verified one,
/// but they're nominally distinct types. Map extracted's tag to
/// the same byte the verified shim emits.
fn axiom_l0_zip_lfh_extracted_tag(e: &axiom_l0_zip_lfh_extracted::ParseError) -> u8 {
    use axiom_l0_zip_lfh_extracted::ParseError::*;
    match e {
        ShortHeader => 1,
        BadSignature => 2,
        ShortName => 3,
        ShortExtra => 4,
    }
}
