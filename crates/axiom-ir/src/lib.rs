// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `axiom-ir` — reference Rust implementation of **AXIOM-IR v0.1**.
//!
//! Frozen scope (Phase 1):
//!   * **core**     — dialect-agnostic kernel: [`Module`], [`Operation`],
//!                    [`Region`], [`Block`], [`Value`], [`Type`],
//!                    [`Attribute`], [`Tribool`], [`Diagnostic`].
//!   * **manifest** — Android-manifest dialect: [`manifest::Component`],
//!                    [`manifest::IntentFilter`], [`manifest::Permission`],
//!                    [`manifest::ManifestModule`].
//!   * **resource** — `resources.arsc` dialect: [`resource::StringPool`],
//!                    [`resource::Configuration`], [`resource::ResourceRef`],
//!                    [`resource::ResourceTable`].
//!   * **lowering** — manifest ↔ resource resolution: [`lowering::resolve`].
//!
//! Three wire formats are stable for v0.1:
//!   * **Canonical bytes** ([`canonical`]) — byte-deterministic, self-describing,
//!     length-prefixed. The cryptographic IR-commitment hash in [`hash`] is
//!     computed over this byte stream. Producing equal canonical bytes from
//!     two `Module` values is the round-trip equality property.
//!   * **MLIR-style text** ([`text`]) — human-readable, parseable, used in
//!     diagnostics and the spec.
//!   * **Stable JSON** ([`json`]) — sorted keys, deterministic strings, used
//!     by `tools/ir-corpus` to emit drift-stable summaries.
//!
//! The implementation is intentionally pure-std: no `serde`, `bincode`,
//! `rkyv`, or `capnp` runtime dependency. Cf. `third-party/rust/Cargo.toml`
//! for the workspace's deliberate dep-minimisation policy. The wire formats
//! here are exact, deterministic, and self-validating against a hand-rolled
//! NIST-FIPS-180-4 SHA-256 (see [`hash`]).
//!
//! See `docs/phase-1/P1.4/CHECKLIST.md` for the v0.1 spec, including the
//! freeze policy (any change to canonical bytes flips the schema hash and
//! flunks the CI drift gate at `p14-ir-drift`).

#![forbid(unsafe_code)]
#![warn(missing_docs, unreachable_pub)]

pub mod canonical;
pub mod core;
pub mod hash;
pub mod json;
pub mod lowering;
pub mod manifest;
pub mod resource;
pub mod text;

pub use crate::core::{
    Attribute, Block, Diagnostic, IrError, Module, Operation, Region, Tribool, Type, Value, ValueId,
};

/// Crate identifier baked into every `Module` produced by this crate.
///
/// The string is part of canonical bytes; it pins the producer-tag of every
/// IR module emitted in Phase 1. Downstream consumers may key on this for
/// version detection.
pub const CRATE_ID: &str = "apkaxiom::ir";

/// AXIOM-IR specification version.
///
/// Minor bumps add fields without breaking canonical bytes for older
/// schemas (modulo a wire-format extension marker). Major bumps reshape the
/// canonical encoding and require an ADR.
pub const SCHEMA_VERSION: &str = "0.1.0";

/// Producer-tag string written into the canonical-bytes header.
///
/// Format: `apkaxiom::ir/0.1.0`.
pub const PRODUCER_TAG: &str = "apkaxiom::ir/0.1.0";

#[cfg(test)]
mod smoke {
    use super::*;

    #[test]
    fn crate_id_is_stable() {
        assert_eq!(CRATE_ID, "apkaxiom::ir");
        assert_eq!(SCHEMA_VERSION, "0.1.0");
        assert_eq!(PRODUCER_TAG, "apkaxiom::ir/0.1.0");
    }

    #[test]
    fn empty_module_round_trips() {
        let m = Module::empty("smoke");
        let bytes = canonical::encode(&m);
        let parsed = canonical::decode(&bytes).expect("decode round-trip");
        assert_eq!(canonical::encode(&parsed), bytes);
    }
}
