// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
//
// In-process differential fuzz of `parse_with_commit_chain`
// against the bare streaming parser. Mirrors the P1.10 §B item 10
// libFuzzer harness in `fuzz/fuzz_targets/fuzz_commit_chain.rs`,
// but runs at `cargo test` time without needing nightly /
// cargo-fuzz. Properties asserted on every input:
//
//   1. **No panics.** `parse_with_commit_chain` returns either
//      `Ok((events, chain))` or `Err(_)` — never panics.
//   2. **Determinism.** A successful chain run is byte-identical
//      to a re-run on the same input (Merkle root, leaf list,
//      event count).
//   3. **Acceptance parity.** The chain wrapper's accept-set
//      equals the bare streaming parser's: chain accepts iff
//      bare accepts.
//
// 10 000 random LCG-mutated inputs derived from the four real
// F-Droid APK fixtures + a synthetic "minimal archive" + raw
// pseudo-random bytes. The seed is committed so failures
// reproduce.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::single_match_else,
    clippy::manual_assert,
    clippy::cast_sign_loss
)]

use axiom_l1_rs::commit_chain::parse_with_commit_chain;
use axiom_l1_rs::stream::ApkParser;

const FIXTURES: &[&str] = &[
    "fdroid-privileged-2050.apk",
    "clipboard.apk",
    "tickytacky-mirror.apk",
    "wifiautoff.apk",
];

fn fixture_path(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn fuzz_commit_chain_inproc_10k_mutations() {
    let mut corpus: Vec<Vec<u8>> = FIXTURES
        .iter()
        .map(|n| std::fs::read(fixture_path(n)).expect("read fixture"))
        .collect();
    // Add a fully-pseudo-random 1 KiB seed and an empty seed —
    // the latter exercises the parser's truncated-input path.
    corpus.push(Vec::new());
    let mut s: u64 = 0x000c_0ffe_ebad_dead;
    let mut rand_blob = vec![0u8; 1024];
    for byte in &mut rand_blob {
        s = next_lcg(s);
        *byte = (s >> 32) as u8;
    }
    corpus.push(rand_blob);

    let total_runs = 10_000;
    let mut chain_accepts = 0u64;
    let mut bare_accepts = 0u64;
    let mut both_reject = 0u64;
    for run in 0..total_runs {
        s = next_lcg(s);
        let base = &corpus[(s >> 32) as usize % corpus.len()];
        let mut data = base.clone();
        // Apply 0–8 random byte mutations + maybe truncate.
        let n_muts = ((s >> 56) & 0x07) as usize;
        for _ in 0..n_muts {
            if data.is_empty() {
                break;
            }
            s = next_lcg(s);
            let off = (s >> 32) as usize % data.len();
            s = next_lcg(s);
            let val = (s >> 56) as u8;
            data[off] = val;
        }
        s = next_lcg(s);
        if (s >> 60) & 1 == 0 && !data.is_empty() {
            s = next_lcg(s);
            let new_len = (s >> 32) as usize % data.len();
            data.truncate(new_len);
        }

        // Run bare parser.
        let bare_ok = bare_streaming_accepts(&data);
        if bare_ok {
            bare_accepts += 1;
        }

        // Run chain — must not panic; if it accepts, it must be deterministic.
        match parse_with_commit_chain(data.as_slice()) {
            Ok((events_a, chain_a)) => {
                chain_accepts += 1;
                let (events_b, chain_b) = parse_with_commit_chain(data.as_slice())
                    .expect("re-run rejected what first run accepted");
                assert_eq!(
                    chain_a.root, chain_b.root,
                    "run #{run} merkle root non-deterministic"
                );
                assert_eq!(
                    chain_a.leaves.len(),
                    chain_b.leaves.len(),
                    "run #{run} leaf count non-deterministic"
                );
                for (i, (la, lb)) in chain_a.leaves.iter().zip(&chain_b.leaves).enumerate() {
                    assert_eq!(la.hash, lb.hash, "run #{run} leaf {i} hash drift");
                    assert_eq!(la.tag, lb.tag, "run #{run} leaf {i} tag drift");
                }
                assert_eq!(events_a.len(), events_b.len(), "run #{run} event count");
                assert!(
                    bare_ok,
                    "run #{run}: chain accepted bytes bare rejected — accept set diverged"
                );
            }
            Err(_) => {
                if bare_ok {
                    panic!(
                        "run #{run}: chain rejected input bare parser accepted ({} bytes)",
                        data.len()
                    );
                }
                both_reject += 1;
            }
        }
    }
    eprintln!(
        "fuzz_commit_chain_inproc: runs={total_runs} chain_accepts={chain_accepts} bare_accepts={bare_accepts} both_reject={both_reject}"
    );
    assert_eq!(
        chain_accepts,
        bare_accepts,
        "chain and bare disagreed on {} inputs",
        chain_accepts.abs_diff(bare_accepts)
    );
}

fn bare_streaming_accepts(data: &[u8]) -> bool {
    let mut parser = ApkParser::from_reader(data);
    loop {
        match parser.next_event() {
            Ok(Some(_)) => {}
            Ok(None) => return true,
            Err(_) => return false,
        }
    }
}

const fn next_lcg(s: u64) -> u64 {
    s.wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407)
}
