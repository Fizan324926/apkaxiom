// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! In-house property-based fuzz harness on `parse_lfh` / `parse_eocd`.
//!
//! Uses the same in-tree LCG as the corpus generator (no `proptest`
//! dep, no `rand` dep) — the seed determinism is the same property
//! the differential harness depends on.
//!
//! Two invariants asserted across 10K random inputs each:
//!
//!   1. **No panics.** The parser must never panic on any input,
//!      including all-zeros, all-ones, garbage of arbitrary length,
//!      or pathological filename / extra-field length declarations.
//!   2. **Closed verdict space.** Every result is either `Ok` or one
//!      of the four declared `ParseError` variants. The `tag()` byte
//!      always lands in [1, 4].

use crate::{eocd, fuzz_helpers::Lcg, lfh};

const FUZZ_ITERATIONS: usize = 10_000;
const MAX_INPUT_LEN: usize = 1024;

#[test]
fn lfh_fuzz_no_panics_closed_verdicts() {
    let mut rng = Lcg::new(0xa9c1_d4b1_f7e2_3d51);
    let mut input = vec![0u8; MAX_INPUT_LEN];
    let mut ok_count = 0usize;
    let mut err_counts = [0usize; 5]; // index 0 unused; indices 1-4 = tags
    for _ in 0..FUZZ_ITERATIONS {
        let len = rng.next_in_range(0, MAX_INPUT_LEN as u32 + 1) as usize;
        let slice = &mut input[..len];
        rng.fill(slice);
        match lfh::parse_lfh(slice) {
            Ok(_) => ok_count += 1,
            Err(e) => {
                let t = e.tag();
                assert!((1..=4).contains(&t), "LFH tag out of range: {t}");
                err_counts[t as usize] += 1;
            }
        }
    }
    // We deliberately don't assert exact counts (the LCG outcome is
    // sensitive to the iteration count) — the invariant is "no
    // panics, closed verdict space".
    assert_eq!(
        ok_count + err_counts[1] + err_counts[2] + err_counts[3] + err_counts[4],
        FUZZ_ITERATIONS,
        "every iteration must produce exactly one verdict"
    );
}

#[test]
fn eocd_fuzz_no_panics_closed_verdicts() {
    let mut rng = Lcg::new(0xa9c1_d4b1_f7e2_3d51);
    let mut input = vec![0u8; MAX_INPUT_LEN];
    let mut ok_count = 0usize;
    let mut err_counts = [0usize; 5];
    for _ in 0..FUZZ_ITERATIONS {
        let len = rng.next_in_range(0, MAX_INPUT_LEN as u32 + 1) as usize;
        let slice = &mut input[..len];
        rng.fill(slice);
        match eocd::parse_eocd(slice) {
            Ok(_) => ok_count += 1,
            Err(e) => {
                let t = e.tag();
                assert!((1..=4).contains(&t), "EOCD tag out of range: {t}");
                err_counts[t as usize] += 1;
            }
        }
    }
    assert_eq!(
        ok_count + err_counts[1] + err_counts[2] + err_counts[3] + err_counts[4],
        FUZZ_ITERATIONS,
        "every iteration must produce exactly one verdict"
    );
}

#[test]
fn find_eocd_fuzz_no_panics() {
    // `find_eocd` runs a backward scan; ensure it never panics on
    // pathological inputs.
    let mut rng = Lcg::new(0xa9c1_d4b1_f7e2_3d51);
    let mut input = vec![0u8; MAX_INPUT_LEN];
    for _ in 0..FUZZ_ITERATIONS {
        let len = rng.next_in_range(0, MAX_INPUT_LEN as u32 + 1) as usize;
        let slice = &mut input[..len];
        rng.fill(slice);
        let _ = eocd::find_eocd(slice);
    }
}
