// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `axiom-l0` — APKAXIOM L0 minimal trusted core.
//!
//! Phase-1 scope:
//!
//!   - **`zip` module** (P1.12): translation-validated ZIP layer
//!     (LFH + CDR + EOCD + cross-record consistency). Default route
//!     is `axiom-l0-zip-verified` (the umbrella whose per-module
//!     correspondence with the Lean reference is recorded in
//!     `docs/phase-1/P1.12/tv-receipt-*.txt`). The `legacy-zip`
//!     feature flag re-routes through `axiom-zip-ref` (the hand-
//!     written reference parser the P1.5/P1.6 three-way differential
//!     gates on); kept around for the verified-vs-hand-written perf
//!     gate (`make p112-perf-delta`) and slated for removal in
//!     Phase 2.
//!
//!   - **build-graph liveness probe** (`placeholder()`, `CRATE_ID`):
//!     present from P1.1 onward; pinned constants so reproducibility
//!     hashes do not shift on cosmetic edits.

#![forbid(unsafe_code)]
#![warn(missing_docs, unreachable_pub)]

#[cfg(all(feature = "verified-zip", feature = "legacy-zip"))]
compile_error!(
    "axiom-l0: features `verified-zip` and `legacy-zip` are mutually exclusive — pick one"
);

#[cfg(all(feature = "verified-zip", not(feature = "legacy-zip")))]
pub mod zip {
    //! Translation-validated ZIP layer (default route).
    //!
    //! Re-exports the `axiom-l0-zip-verified` umbrella crate. Per-module
    //! TV receipts:
    //!
    //!   - LFH         : `docs/phase-1/P1.9/tv-receipt-lfh-full.txt`
    //!   - EOCD        : `docs/phase-1/P1.9/tv-receipt-eocd.txt`
    //!   - CDR         : `docs/phase-1/P1.12/tv-receipt-cdr.txt`
    //!   - Consistency : `docs/phase-1/P1.12/tv-receipt-consistency.txt`
    pub use axiom_l0_zip_verified::*;

    /// String tag identifying which ZIP route this build is using.
    /// Used by `axiom_l0::route()` for run-time observability of the
    /// trust boundary at the L0 layer.
    pub const ROUTE: &str = "verified";
}

#[cfg(all(feature = "legacy-zip", not(feature = "verified-zip")))]
pub mod zip {
    //! Legacy hand-written ZIP layer.
    //!
    //! Re-exports `axiom-zip-ref`, the P1.5/P1.6 reference parser the
    //! three-way differential (Lean ↔ Rust ↔ AOSP libziparchive)
    //! gates on. Byte-equivalent to the verified umbrella for every
    //! input the differential corpus covers; the dedicated
    //! `legacy-zip` route exists so the perf-delta tool can compare
    //! the two without rebuilding the world.
    pub use axiom_zip_ref::*;

    /// String tag identifying which ZIP route this build is using.
    pub const ROUTE: &str = "legacy";
}

#[cfg(all(not(feature = "verified-zip"), not(feature = "legacy-zip")))]
compile_error!("axiom-l0: enable exactly one of {`verified-zip` (default), `legacy-zip`}");

/// Run-time observable of which ZIP route this build is using.
/// Used by the operator-facing CLI to surface the trust boundary in
/// `--version`-style output.
#[must_use]
pub const fn route() -> &'static str {
    zip::ROUTE
}

/// Build-graph liveness probe.
///
/// Returns the same value on every machine that successfully linked
/// the L0 trusted core. The numeric value is deliberately stable
/// across the entire Phase 1 window so reproducibility hashes do
/// not shift on cosmetic edits.
#[must_use]
pub const fn placeholder() -> u32 {
    0xA710_0000
}

/// Crate identifier baked into the binary at compile time. Used by
/// the repro harness to confirm the artifact under test is the one
/// we believe it is.
pub const CRATE_ID: &str = "apkaxiom::l0";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_is_stable() {
        assert_eq!(placeholder(), 0xA710_0000);
    }

    #[test]
    fn crate_id_is_stable() {
        assert_eq!(CRATE_ID, "apkaxiom::l0");
    }

    #[cfg(feature = "verified-zip")]
    #[test]
    fn default_route_is_verified() {
        assert_eq!(route(), "verified");
    }

    #[cfg(feature = "legacy-zip")]
    #[test]
    fn legacy_route_is_legacy() {
        assert_eq!(route(), "legacy");
    }

    /// The default verified route must surface `parse_archive` from
    /// the umbrella. Compile-time check via a function pointer cast.
    #[cfg(feature = "verified-zip")]
    #[test]
    fn verified_route_exposes_parse_archive() {
        let f: fn(&[u8]) -> Result<zip::consistency::Archive, zip::consistency::ArchiveError> =
            zip::consistency::parse_archive;
        // Touch the binding so clippy sees it as used.
        std::hint::black_box(f);
    }
}
