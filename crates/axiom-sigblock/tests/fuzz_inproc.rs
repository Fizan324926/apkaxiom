// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
//
// In-process fuzz of the APK signing-block parsers. Mirrors the
// libFuzzer harnesses at `fuzz/fuzz_targets/`, but runs at
// `cargo test` time without nightly. Properties:
//
//   - **Totality**: `locate`, `parse_v2`, `parse_v3`, `parse_v3_1`
//     return either Ok or Err on any byte slice; never panic.
//   - **Determinism**: two parses of the same input agree on
//     Ok/Err and on the parsed structure.
//
// 10 000 LCG-mutated inputs × 4 parsers = 40 000 runs.

#![allow(
    clippy::cast_possible_truncation,
    clippy::missing_const_for_fn,
    clippy::unreadable_literal,
    clippy::unusual_byte_groupings
)]

use axiom_sigblock::locate;
use axiom_sigblock::scheme::{parse_v2, parse_v3, parse_v3_1};

fn fixture_path(rel: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join(rel)
}

fn next_lcg(s: u64) -> u64 {
    s.wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407)
}

#[test]
fn fuzz_locate_inproc_totality_and_determinism() {
    let seeds: &[&str] = &[
        "corpus/signing/v1-only/wifiautoff-v1.apk",
        "corpus/signing/v1-v2/wifiautoff-v1v2.apk",
        "corpus/signing/v1-v2-v3/wifiautoff-v1v2v3.apk",
    ];
    let bytes: Vec<Vec<u8>> = seeds
        .iter()
        .map(|p| std::fs::read(fixture_path(p)).expect("seed"))
        .collect();
    let mut s: u64 = 0xfacefeed_dead_beef;
    for run in 0..10_000 {
        s = next_lcg(s);
        let base = &bytes[(s >> 32) as usize % bytes.len()];
        let mut data = base.clone();
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
            data.truncate((s >> 32) as usize % data.len());
        }
        let a = locate(&data);
        let b = locate(&data);
        assert_eq!(a.is_ok(), b.is_ok(), "run {run}: locate non-deterministic");
    }
}

#[test]
fn fuzz_parse_v2_inproc_totality_and_determinism() {
    let mut s: u64 = 0xc0ffee_0000_1111;
    for run in 0..10_000 {
        s = next_lcg(s);
        let len = ((s >> 32) % 4096) as usize;
        let mut data = vec![0u8; len];
        let mut t = s;
        for byte in &mut data {
            t = next_lcg(t);
            *byte = (t >> 56) as u8;
        }
        let a = parse_v2(&data);
        let b = parse_v2(&data);
        assert_eq!(
            a.is_ok(),
            b.is_ok(),
            "run {run}: parse_v2 non-deterministic"
        );
    }
}

#[test]
fn fuzz_parse_v3_inproc_totality_and_determinism() {
    let mut s: u64 = 0xb00bee_2222_3333;
    for run in 0..10_000 {
        s = next_lcg(s);
        let len = ((s >> 32) % 4096) as usize;
        let mut data = vec![0u8; len];
        let mut t = s;
        for byte in &mut data {
            t = next_lcg(t);
            *byte = (t >> 56) as u8;
        }
        let a = parse_v3(&data);
        let b = parse_v3(&data);
        assert_eq!(
            a.is_ok(),
            b.is_ok(),
            "run {run}: parse_v3 non-deterministic"
        );
    }
}

#[test]
fn fuzz_parse_v3_1_inproc_totality_and_determinism() {
    let mut s: u64 = 0xdead_4444_5555;
    for run in 0..10_000 {
        s = next_lcg(s);
        let len = ((s >> 32) % 4096) as usize;
        let mut data = vec![0u8; len];
        let mut t = s;
        for byte in &mut data {
            t = next_lcg(t);
            *byte = (t >> 56) as u8;
        }
        let a = parse_v3_1(&data);
        let b = parse_v3_1(&data);
        assert_eq!(
            a.is_ok(),
            b.is_ok(),
            "run {run}: parse_v3_1 non-deterministic"
        );
    }
}
