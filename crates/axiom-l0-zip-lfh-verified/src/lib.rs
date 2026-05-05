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
