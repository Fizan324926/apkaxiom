// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
//
// P1.10 §B item 9 (HARD) — cross-implementation BLAKE3 check.
//
// `crates/axiom-blake3-hacl/src/cross_impl.rs` is auto-generated
// from `test-vectors/cross-impl-python-blake3.json`, which is in
// turn produced by the Python `blake3` package wrapping the
// BLAKE3-team reference C library — an independent codebase from
// the Rust `blake3` crate we ship in production. This integration
// test asserts the production Rust path produces byte-identical
// digests to the C reference on:
//
//   - The four real F-Droid APK fixtures committed under
//     `crates/axiom-l1-rs/tests/fixtures/`.
//   - All 35 official `paint_test_input` lengths (0…102400 bytes).
//
// Re-derive locally with:
//
//   pip3 install --break-system-packages blake3
//   python3 scripts/gen-cross-impl-rs.py
//   cargo test -p axiom-blake3-hacl --test cross_impl
//
// Any divergence here means either:
//   - The Rust crate has a regression vs the C reference (bug); or
//   - Reindeer pinned a non-canonical blake3 release (vendor bug); or
//   - Python's package shipped a regression of its own.
// In every case the test fails closed and forces a code-review.

use axiom_blake3_hacl::cross_impl::{FIXTURE_BLAKE3, PAINT_VECTORS_BLAKE3};
use axiom_blake3_hacl::{paint_test_input, Blake3, Hasher};

#[test]
fn rust_blake3_matches_c_reference_on_four_apks() {
    let fixture_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("axiom-l1-rs/tests/fixtures");
    for &(name, expected_len, expected_hash) in FIXTURE_BLAKE3 {
        let path = fixture_dir.join(name);
        let body = std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        assert_eq!(
            body.len(),
            expected_len,
            "{name}: fixture size drift — expected {expected_len}, got {}",
            body.len()
        );
        let got = Blake3::hash_oneshot(&body);
        assert_eq!(
            got, expected_hash,
            "{name}: Rust BLAKE3 disagrees with C-reference BLAKE3"
        );
    }
}

#[test]
fn rust_blake3_matches_c_reference_on_35_paint_vectors() {
    for &(len, expected_hash) in PAINT_VECTORS_BLAKE3 {
        let input = paint_test_input(len);
        let got = Blake3::hash_oneshot(&input);
        assert_eq!(
            got, expected_hash,
            "paint_test_input(len={len}): Rust BLAKE3 disagrees with C-reference BLAKE3"
        );
    }
}
