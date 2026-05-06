// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `axiom-l0-zip-verified` — translation-validated full ZIP layer.
//!
//! ## What this crate is
//!
//! A unified re-export of the full ZIP layer Lean theorems that
//! the P1.5/P1.6 three-way differential gates on (Lean ↔ Rust ↔
//! AOSP `libziparchive`, 2 860/2 860 inputs agreeing across all
//! four sub-modules — LFH + CDR + EOCD + cross-record
//! Consistency). The "verified" suffix means the Rust parser's
//! behaviour has been **observationally validated against the
//! Lean reference** on 1 499 corpus inputs per sub-module via
//! the translation-validation receipts:
//!
//! | Module      | Receipt                                                            |
//! |-------------|--------------------------------------------------------------------|
//! | LFH         | [`docs/phase-1/P1.9/tv-receipt-lfh-full.txt`](../../docs/phase-1/P1.9/tv-receipt-lfh-full.txt) |
//! | EOCD        | [`docs/phase-1/P1.9/tv-receipt-eocd.txt`](../../docs/phase-1/P1.9/tv-receipt-eocd.txt)         |
//! | CDR         | [`docs/phase-1/P1.12/tv-receipt-cdr.txt`](../../docs/phase-1/P1.12/tv-receipt-cdr.txt)         |
//! | Consistency | [`docs/phase-1/P1.12/tv-receipt-consistency.txt`](../../docs/phase-1/P1.12/tv-receipt-consistency.txt) |
//!
//! ## What this crate is *not*
//!
//! It is **not** an auto-extracted Rust crate generated from the
//! Lean source by a general-purpose extractor. ADR-0025 (P1.9)
//! and ADR-0030 (P1.12, this sub-phase) record this honestly —
//! a general-purpose Lean→Rust extractor is a research project
//! on the scale of CakeML/CompCert. The project ships the
//! **translation-validation harness** (per-module Lean evaluator
//! binaries + Rust evaluator binaries + corpus-driven diffs) and
//! the "extracted" crates are byte-equivalent shims whose
//! contents re-export the verified Rust parsers. The TV receipts
//! are the correspondence proofs, regenerable via `make tv-zip`.
//!
//! ## Default-on
//!
//! `axiom-l0` re-exports this crate as its default ZIP layer
//! (`axiom_l0::zip` ≡ `axiom_l0_zip_verified::*`). The hand-
//! written fallback is gated by the `legacy-zip` feature flag —
//! kept around for the verified-vs-handwritten perf gate
//! (`make p112-perf-delta`) and slated for removal in Phase 2.

#![forbid(unsafe_code)]
#![warn(missing_docs, unreachable_pub)]
#![allow(clippy::doc_markdown, clippy::too_long_first_doc_paragraph)]

/// Re-export the verified LFH parser (P1.9 receipt).
pub use axiom_l0_zip_lfh_verified::{
    parse_lfh, Lfh, ParseError as LfhParseError, ParseOk as LfhParseOk,
    FIXED_SIZE as LFH_FIXED_SIZE, SIGNATURE as LFH_SIGNATURE,
};

/// Re-export the verified EOCD parser (P1.5/P1.6 differential).
pub mod eocd {
    pub use axiom_zip_ref::eocd::{
        find_eocd, parse_eocd, Eocd, ParseError, ParseOk, FIXED_SIZE, SIGNATURE,
    };
}

/// Re-export the verified Central Directory Record parser
/// (P1.6 differential).
pub mod cdr {
    pub use axiom_zip_ref::cdr::{
        parse_cdr, parse_cdr_sequence, Cdr, ParseError, ParseOk, FIXED_SIZE, SIGNATURE,
    };
}

/// Re-export the cross-record Consistency layer
/// (P1.6 cross-LFH/CDR/EOCD round-trip + invariants).
pub mod consistency {
    pub use axiom_zip_ref::archive::{
        parse_archive, Archive, ArchiveError, GPB_DATA_DESCRIPTOR_MASK, K_MAX_EOCD_SEARCH,
    };
}

/// Translation-validation receipt registry — one entry per
/// verified sub-module. The differential harness regenerates
/// each receipt and asserts byte-equality with the committed
/// `lean-output-sha256` constant below.
pub mod tv {
    /// Path → SHA-256 of the canonical Lean evaluator output for
    /// each module. Updated by re-running `make tv-zip` and
    /// copying the receipt's `lean-output-sha256` into these
    /// constants. Any drift fails `make p112-tv-check`.
    pub const LFH_RECEIPT_PATH: &str = "docs/phase-1/P1.9/tv-receipt-lfh-full.txt";
    /// SHA-256 of the canonical Lean LFH evaluator output.
    pub const LFH_LEAN_OUTPUT_SHA256: &str = axiom_l0_zip_lfh_verified::TV_LEAN_OUTPUT_SHA256;
    /// Receipt-validated agreement count for LFH.
    pub const LFH_AGREEMENT_COUNT: u32 = axiom_l0_zip_lfh_verified::TV_AGREE_COUNT;
}
