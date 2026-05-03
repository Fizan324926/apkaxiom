// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `axiom-ir` — Phase 1 placeholder for the canonical AXIOM-IR.
//!
//! The real IR (with SSA, effects, dependent types in the kernel slice) lands
//! incrementally across P1.5, P1.6, P3.x and is frozen to v1.0 in P6.10.
//! In Phase 1 this crate exists to:
//!  1. Demonstrate a *third-party* dependency vended through Reindeer
//!     (`thiserror` → `//third-party/rust:thiserror`).
//!  2. Demonstrate an *intra-workspace* dependency on `axiom-l0`.

#![forbid(unsafe_code)]
#![warn(missing_docs, unreachable_pub)]

use thiserror::Error;

/// Errors surfaced by the (future) AXIOM-IR pipeline.
///
/// Phase 1 ships only the placeholder variant; downstream phases extend
/// this enum.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum IrError {
    /// Phase-1 placeholder error returned by [`probe`].
    #[error("axiom-ir placeholder error (real diagnostics arrive in P1.5)")]
    Placeholder,
}

/// Build-graph liveness probe.
///
/// Threads through L0 *and* a third-party crate so a successful build proves
/// Reindeer and Buck2 third-party graphs match the Cargo graph.
///
/// # Errors
/// Always returns [`IrError::Placeholder`] in Phase 1 — the success path
/// arrives once the IR is real.
pub const fn probe() -> Result<u32, IrError> {
    let _ = axiom_l0::placeholder();
    Err(IrError::Placeholder)
}

/// Crate identifier baked into the binary.
pub const CRATE_ID: &str = "apkaxiom::ir";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_returns_placeholder() {
        assert!(matches!(probe(), Err(IrError::Placeholder)));
    }

    #[test]
    fn error_renders() {
        let s = IrError::Placeholder.to_string();
        assert!(s.contains("placeholder"));
    }
}
