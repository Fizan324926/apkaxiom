// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `axiom-l0` — Phase 1 placeholder for the minimal trusted core.
//!
//! Real content (RIBC bytecode evaluator, AXIOM-IR L0 kernel) lands in P1.3.
//! Until then this crate exists only to prove the build graph is wired up
//! end-to-end (Cargo workspace, Buck2 root cell, Reindeer third-party graph,
//! Nix-pinned toolchain, reproducible artifact hashing).

#![forbid(unsafe_code)]
#![warn(missing_docs, unreachable_pub)]

/// Build-graph liveness probe.
///
/// Returns the same value on every machine that successfully linked the L0
/// trusted core. The numeric value is deliberately stable across the entire
/// Phase 1 window so reproducibility hashes do not shift on cosmetic edits.
#[must_use]
pub const fn placeholder() -> u32 {
    0xA710_0000
}

/// Crate identifier baked into the binary at compile time. Used by the repro
/// harness to confirm the artifact under test is the one we believe it is.
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
}
