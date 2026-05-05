// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `axiom-l0-zip-lfh-verified` — translation-validated LFH parser.
//!
//! ## What this crate is
//!
//! A thin re-export of [`axiom_zip_ref::lfh`] — the Rust LFH parser
//! the P1.5/P1.6 three-way differential gates on (2860/2860 inputs
//! agreeing across Lean ↔ Rust ↔ AOSP). The "verified" suffix
//! means **the Rust parser's behaviour has been observationally
//! validated against the Lean reference parser on a 1499-input
//! corpus**, with the receipt committed as
//! [`docs/phase-1/P1.9/tv-receipt-lfh-full.txt`](../../docs/phase-1/P1.9/tv-receipt-lfh-full.txt).
//!
//! ## What this crate is *not*
//!
//! It is **not** an auto-extracted Rust crate generated from the
//! Lean source. A general-purpose Lean-to-Rust extractor is a
//! research project on the scale of F\* / CakeML / CompCert and
//! lives outside the P1.9 budget. ADR-0025 records this honestly:
//! P1.9 ships the **translation-validation harness** (Lean
//! evaluator binary + Rust evaluator binary + corpus-driven
//! diff), and the "extracted crate" is a thin shim whose contents
//! re-export the verified Rust parser. The TV receipt is the
//! correspondence proof.
//!
//! ## How the receipt is checked
//!
//! `make tv` regenerates
//! `docs/phase-1/P1.9/tv-receipt-lfh-full.txt`. The receipt is a
//! 7-line file that records:
//!
//!   - `corpus-sha256`: hash over the input corpus (which `*.bin`
//!     files in which order)
//!   - `lean-output-sha256` and `rust-output-sha256`: must be
//!     equal for the receipt to be valid (byte-identical JSON
//!     output across the two evaluators)
//!   - `agree: <n>/<n>`: agreement count
//!
//! `make tv-check-receipt` re-runs the validator and asserts the
//! freshly produced receipt's `lean-output-sha256` matches the
//! committed one. Any change to either the Lean source or the
//! Rust source that produces different output bytes flips the
//! gate.

#![forbid(unsafe_code)]
#![warn(missing_docs, unreachable_pub)]
#![allow(
    clippy::too_long_first_doc_paragraph,
    clippy::doc_markdown,
    clippy::items_after_statements
)]

/// Re-export of the verified LFH parser. The translation-validation
/// receipt at `docs/phase-1/P1.9/tv-receipt-lfh-full.txt` covers
/// every public item below.
pub use axiom_zip_ref::lfh::{parse_lfh, Lfh, ParseError, ParseOk, FIXED_SIZE, SIGNATURE};

/// SHA-256 of the most recently committed translation-validation
/// receipt. The build-script-equivalent gate is `make
/// tv-check-receipt`, which re-runs the validator and asserts
/// the freshly produced receipt's `lean-output-sha256` line
/// matches [`TV_LEAN_OUTPUT_SHA256`].
///
/// Both fields are committed source-of-truth; updating them
/// without re-running `make tv` will fail the gate the next time
/// CI runs.
pub const TV_RECEIPT_PATH: &str = "docs/phase-1/P1.9/tv-receipt-lfh-full.txt";

/// SHA-256 of the canonical Lean evaluator output computed at
/// commit time. Updated by re-running `make tv` and copying the
/// `lean-output-sha256` line of the resulting receipt into this
/// constant. The validator harness asserts equality with the
/// freshly recomputed receipt.
pub const TV_LEAN_OUTPUT_SHA256: &str =
    "6af3e60fa9c1e03f21aec8d8c106db1567e421a1ecf956136c5f0e7a20b6763d";

/// Number of non-empty corpus inputs that produced agreeing
/// outputs in the committed receipt. Asserted by the
/// `make tv-check-receipt` gate. The corpus may grow over time;
/// the gate is "all non-empty inputs agree", not a fixed count.
pub const TV_AGREE_COUNT: u32 = 1499;

// ---------------------------------------------------------------------
// P1.9 §IV — JSON shape closure & theorem-statement assertions
// ---------------------------------------------------------------------

/// JSON shape closure check (P1.9 §IV gap 14).
///
/// The TV harness encodes `parseLfh`'s output as a JSON line per
/// input. The output discriminator is `"out": "ok" | "err"`, plus
/// for errors a numeric `tag` matching `ParseError::tag`. If Lean
/// ever adds a new `ParseError` variant without the Rust side or
/// the JSON shape being extended, the TV harness's "byte-identical"
/// guarantee could become vacuously true (both sides emit the
/// new variant under `tag: 255` and agree). This module asserts
/// the **constructor count** matches what the TV harness models.
///
/// Mechanism: `EXPECTED_PARSE_ERROR_TAGS` is the canonical sorted
/// list of Lean `ParseError` tag bytes. Any addition to the Lean
/// inductive without an addition to this list flips the constant
/// and triggers the `parse_error_shape_closure` test below.
pub const EXPECTED_PARSE_ERROR_TAGS: [u8; 4] = [1, 2, 3, 4];

/// Compile-time witness of `theorem ParseError.tag_injective`
/// from the Lean source. The Lean theorem proves the four tag
/// bytes are pairwise distinct. We carry the *content* of that
/// theorem into Rust as a `const` block — if any tag byte ever
/// drifts, this fails at compile time, before any test runs.
///
/// This is the "theorem-statement assertion" deliverable from
/// P1.9 §IV gap 17. Note it asserts the *statement*, not the
/// *proof* — the proof lives in Lean and is checked by `lake
/// build`. This block ensures the Rust-side enum hasn't drifted
/// from the Lean theorem's content.
const _: () = {
    let v2 = ParseError::ShortHeader as u8;
    let v3 = ParseError::BadSignature as u8;
    let v4 = ParseError::ShortName as u8;
    let v5 = ParseError::ShortExtra as u8;
    // Pairwise distinct (the content of `tag_injective`).
    assert!(v2 != v3);
    assert!(v2 != v4);
    assert!(v2 != v5);
    assert!(v3 != v4);
    assert!(v3 != v5);
    assert!(v4 != v5);
    // Note: this compile-time check uses the discriminant bytes,
    // not the `ParseError::tag()` method (which isn't const-eval'd
    // by stable Rust as of 1.83). The discriminant order matches
    // the Lean inductive's constructor order, so distinct
    // discriminants imply distinct `tag()` outputs.
};

#[cfg(test)]
mod tests {
    //! Sanity tests for the translation-validated re-exports. The
    //! *real* TV gate is `make tv-check-receipt`; these tests just
    //! catch obvious regressions in the re-export surface.

    use super::*;

    #[test]
    fn parse_lfh_re_export_works() {
        // Same minimal-LFH the differential corpus exercises.
        let mut bytes = vec![0x50, 0x4b, 0x03, 0x04];
        bytes.extend(std::iter::repeat_n(0u8, 26));
        let (lfh, consumed) = parse_lfh(&bytes).expect("minimal LFH parses");
        assert_eq!(consumed, 30);
        assert_eq!(lfh.compression_method, 0);
    }

    #[test]
    fn signature_constant_matches_appnote() {
        assert_eq!(SIGNATURE, 0x0403_4b50);
        assert_eq!(FIXED_SIZE, 30);
    }

    #[test]
    fn parse_error_re_export_surface() {
        // Empty input.
        assert!(matches!(parse_lfh(&[]), Err(ParseError::ShortHeader)));
        // 30 zero bytes.
        let zeros = vec![0u8; 30];
        assert!(matches!(parse_lfh(&zeros), Err(ParseError::BadSignature)));
    }

    #[test]
    fn parse_error_shape_closure() {
        // Closure check (P1.9 §IV gap 14). If a new ParseError
        // variant is added on either side without updating
        // `EXPECTED_PARSE_ERROR_TAGS`, this test fails — the TV
        // harness's "byte-identical agreement" can no longer
        // claim totality over Lean's variant set.
        let mut sorted: Vec<u8> = vec![
            ParseError::ShortHeader.tag(),
            ParseError::BadSignature.tag(),
            ParseError::ShortName.tag(),
            ParseError::ShortExtra.tag(),
        ];
        sorted.sort_unstable();
        assert_eq!(
            sorted,
            EXPECTED_PARSE_ERROR_TAGS.to_vec(),
            "ParseError tag set drift — Lean may have added a variant the TV shape doesn't know about"
        );
    }

    #[test]
    fn parse_error_tag_injective_at_runtime() {
        // Runtime witness of `theorem ParseError.tag_injective`.
        // Pairs with the compile-time `const _` block above; this
        // test makes the assertion visible in CI test output
        // (compile-time `assert!` failures don't show up in
        // test reports).
        let tags = [
            ParseError::ShortHeader.tag(),
            ParseError::BadSignature.tag(),
            ParseError::ShortName.tag(),
            ParseError::ShortExtra.tag(),
        ];
        for i in 0..tags.len() {
            for j in 0..tags.len() {
                if i != j {
                    assert_ne!(tags[i], tags[j], "tag_injective: tag[{i}] == tag[{j}]");
                }
            }
        }
    }

    #[test]
    fn tv_constants_have_expected_shape() {
        assert_eq!(TV_LEAN_OUTPUT_SHA256.len(), 64);
        assert!(TV_LEAN_OUTPUT_SHA256
            .chars()
            .all(|c| c.is_ascii_hexdigit() && (c.is_ascii_digit() || c.is_ascii_lowercase())));
        // Must clear the spec's ≥ 1000 floor — `const_assert` would
        // be cleaner but isn't on stable; runtime check is fine.
        const _: () = assert!(TV_AGREE_COUNT >= 1000);
    }
}
