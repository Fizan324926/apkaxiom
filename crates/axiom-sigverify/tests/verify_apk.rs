// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
//
// Integration tests for the combined `verify_apk` entry point.
// Covers honest fixtures + every adversarial fixture; mirrors
// `tools/p111-differential` at cargo-test time without needing
// apksigner.

use axiom_sigverify::{verify_apk, Verdict};

fn fixture(rel: &str) -> Vec<u8> {
    let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join(rel);
    std::fs::read(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

#[test]
fn verify_v1_only_real_fdroid_accept() {
    let v = verify_apk(&fixture("crates/axiom-l1-rs/tests/fixtures/wifiautoff.apk"));
    assert!(matches!(v, Verdict::Accept), "v1-only: {v:?}");
}

#[test]
fn verify_v1_only_resigned_accept() {
    let v = verify_apk(&fixture("corpus/signing/v1-only/wifiautoff-v1.apk"));
    assert!(matches!(v, Verdict::Accept), "v1: {v:?}");
}

#[test]
fn verify_v1_v2_accept() {
    let v = verify_apk(&fixture("corpus/signing/v1-v2/wifiautoff-v1v2.apk"));
    assert!(matches!(v, Verdict::Accept), "v1+v2: {v:?}");
}

#[test]
fn verify_v1_v2_v3_accept() {
    let v = verify_apk(&fixture("corpus/signing/v1-v2-v3/wifiautoff-v1v2v3.apk"));
    assert!(matches!(v, Verdict::Accept), "v1+v2+v3: {v:?}");
}

#[test]
fn verify_v1_v2_v3_v3_1_accept() {
    let v = verify_apk(&fixture(
        "corpus/signing/v1-v2-v3-v31/wifiautoff-v1v2v3v31.apk",
    ));
    assert!(matches!(v, Verdict::Accept), "v1+v2+v3+v3.1: {v:?}");
}

#[test]
fn verify_clipboard_fdroid_accept() {
    let v = verify_apk(&fixture("crates/axiom-l1-rs/tests/fixtures/clipboard.apk"));
    assert!(matches!(v, Verdict::Accept), "clipboard: {v:?}");
}

#[test]
fn verify_fdroid_privileged_accept() {
    let v = verify_apk(&fixture(
        "crates/axiom-l1-rs/tests/fixtures/fdroid-privileged-2050.apk",
    ));
    assert!(matches!(v, Verdict::Accept), "fdroid-privileged: {v:?}");
}

#[test]
fn verify_tickytacky_accept() {
    let v = verify_apk(&fixture(
        "crates/axiom-l1-rs/tests/fixtures/tickytacky-mirror.apk",
    ));
    assert!(matches!(v, Verdict::Accept), "tickytacky: {v:?}");
}

#[test]
fn verify_adversarial_bad_magic_rejects() {
    let v = verify_apk(&fixture("corpus/signing/adversarial/bad-magic.apk"));
    assert!(!matches!(v, Verdict::Accept), "bad-magic accepted: {v:?}");
}

#[test]
fn verify_adversarial_janus_rejects() {
    let v = verify_apk(&fixture(
        "corpus/signing/adversarial/janus-dex-prepended.apk",
    ));
    assert!(!matches!(v, Verdict::Accept), "janus accepted: {v:?}");
}

#[test]
fn verify_adversarial_pair_overflow_rejects() {
    let v = verify_apk(&fixture("corpus/signing/adversarial/pair-overflow.apk"));
    assert!(!matches!(v, Verdict::Accept), "pair-overflow: {v:?}");
}

#[test]
fn verify_adversarial_pair_too_short_rejects() {
    let v = verify_apk(&fixture("corpus/signing/adversarial/pair-too-short.apk"));
    assert!(!matches!(v, Verdict::Accept), "pair-too-short: {v:?}");
}

#[test]
fn verify_adversarial_size_mismatch_rejects() {
    let v = verify_apk(&fixture("corpus/signing/adversarial/size-mismatch.apk"));
    assert!(!matches!(v, Verdict::Accept), "size-mismatch: {v:?}");
}

#[test]
fn verify_adversarial_truncated_block_rejects() {
    let v = verify_apk(&fixture("corpus/signing/adversarial/truncated-block.apk"));
    assert!(!matches!(v, Verdict::Accept), "truncated-block: {v:?}");
}

#[test]
fn verify_adversarial_truncated_eocd_rejects() {
    let v = verify_apk(&fixture("corpus/signing/adversarial/truncated-eocd.apk"));
    assert!(!matches!(v, Verdict::Accept), "truncated-eocd: {v:?}");
}

#[test]
fn verify_adversarial_v1_janus_cve_2017_13156_rejects() {
    let v = verify_apk(&fixture(
        "corpus/signing/adversarial/v1-janus-cve-2017-13156.apk",
    ));
    assert!(!matches!(v, Verdict::Accept), "v1-janus: {v:?}");
}

#[test]
fn verify_adversarial_v3_stripped_rejects() {
    let v = verify_apk(&fixture("corpus/signing/adversarial/v3-stripped.apk"));
    assert!(!matches!(v, Verdict::Accept), "v3-stripped accepted: {v:?}");
}
