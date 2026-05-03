// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `axiom-l1-rs` — Phase 1 placeholder for the L1 untrusted shell (Rust side).
//!
//! Real content (DEX/native lifters, IR translators) lands in P1.7+.
//! Existence here exercises an *intra-workspace* Buck2 dependency edge.

#![forbid(unsafe_code)]
#![warn(missing_docs, unreachable_pub)]

/// Build-graph liveness probe.
///
/// Returns a value derived from L0's probe. If L0's constant changes, this
/// changes — by design, so a single L0 edit flushes downstream
/// reproducibility hashes.
#[must_use]
pub const fn placeholder() -> u32 {
    axiom_l0::placeholder() ^ 0x0000_00A1
}

/// Crate identifier baked into the binary.
pub const CRATE_ID: &str = "apkaxiom::l1-rs";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_xors_l0() {
        assert_eq!(placeholder(), 0xA710_0000 ^ 0x0000_00A1);
    }
}
