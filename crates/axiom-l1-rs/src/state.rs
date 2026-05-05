// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Phantom-type-state markers for [`crate::apk::Apk`].
//!
//! P1.8's deliverable is **encoding parser-pipeline correctness in
//! the type system** — calling `manifest()` on an unverified APK is
//! a compile-time error, not a runtime panic. The state types here
//! are zero-sized (`std::mem::size_of::<Unverified>() == 0` etc.)
//! and live exclusively as type parameters on [`crate::apk::Apk`];
//! they carry no data and impose no runtime overhead. The compiler
//! drops `PhantomData<S>` entirely under release codegen — verified
//! by the §F-1 perf-delta gate (`tools/p18-perf-delta`).
//!
//! ## Pipeline
//!
//! ```text
//!     ┌──────────────────┐  verify_v2() / verify_v3() / verify_v4()
//!     │ Apk<Unverified>  │ ────────────────────┐
//!     └──────────────────┘                     │
//!                                              ▼
//!     ┌──────────────────────────┐  parse_v2() / parse_v3() / parse_v4()
//!     │ Apk<SignatureVerified>   │ ────────────────────┐
//!     └──────────────────────────┘                     │
//!                                                      ▼
//!                                  ┌─────────────────────────────────┐
//!                                  │ Apk<FullyParsed<V>>             │
//!                                  │     V ∈ {V2, V3, V4}            │
//!                                  └─────────────────────────────────┘
//! ```
//!
//! ## Sealed-trait design
//!
//! [`ApkState`] and [`SigVariant`] are *sealed* — only this crate
//! can implement them. This prevents downstream crates from
//! introducing new state markers that the verified Lean parser
//! (P1.5/P1.6/P1.9) doesn't model. The 1-to-1 phantom-state ↔
//! Lean-constructor mapping in `docs/type-state.md` is the
//! soundness contract for P1.9's translation-validation pass.
//!
//! ## Translation-validation contract
//!
//! Each phantom marker corresponds to exactly one Lean inductive
//! constructor. The mapping is mechanical and audited in §B of the
//! P1.8 CHECKLIST; if either side adds a new constructor without
//! the other doing so, P1.9's translation-validation pass fails
//! the build.
//!
//! | Rust marker | Lean constructor | Why |
//! |---|---|---|
//! | [`Unverified`] | `ApkState.unverified` | Bytes consumed, structural ZIP parse done, no signature work. |
//! | [`SignatureVerified`] | `ApkState.sigVerified` | An APK Signing Block (v2/v3/v4) verified end-to-end. |
//! | [`FullyParsed`]`<V>` | `ApkState.fullyParsed V` | Manifest + resources decoded (lazily); `V` records which signature variant was verified. |
//! | [`V2`] | `SigVariant.v2` | APK Signing Block v2 (APKv1 schemes pre-Q). |
//! | [`V3`] | `SigVariant.v3` | APK Signing Block v3 (key-rotation support). |
//! | [`V4`] | `SigVariant.v4` | APK Signing Block v4 (incremental delivery, Android 11+). |

use core::marker::PhantomData;

mod private {
    /// Sealing trait — prevents external crates from implementing
    /// [`super::ApkState`] or [`super::SigVariant`]. The phantom
    /// universe is closed.
    pub trait Sealed {}
}

/// Marker trait for [`crate::apk::Apk`] state types. Sealed.
///
/// The closed universe (`Unverified` / `SignatureVerified` /
/// `FullyParsed<V>`) maps 1-to-1 to a Lean inductive whose
/// constructor enumeration P1.9 will reflect — see the table on
/// the module docstring.
pub trait ApkState: private::Sealed {
    /// A short, stable name used in diagnostics and in the
    /// translation-validation table. Equals the Lean constructor
    /// suffix.
    const NAME: &'static str;
}

/// Marker trait for APK signing-block variants. Sealed.
pub trait SigVariant: private::Sealed {
    /// Stable single-byte tag for the signing-block variant.
    /// Matches the constructor index in the Lean `SigVariant`
    /// inductive (`v2 = 2, v3 = 3, v4 = 4`).
    const TAG: u8;

    /// Short stable name used in diagnostics.
    const NAME: &'static str;
}

// ---------------------------------------------------------------------
// State markers
// ---------------------------------------------------------------------

/// Bytes consumed and the structural ZIP parse landed; no
/// signature verification or manifest decode has been attempted.
///
/// Methods callable in this state: structural accessors only —
/// see [`crate::apk::Apk`]'s `Apk<Unverified>` impl block.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Unverified;
impl private::Sealed for Unverified {}
impl ApkState for Unverified {
    const NAME: &'static str = "unverified";
}

/// An APK Signing Block has been verified end-to-end (signature,
/// digest, certificate chain). Manifest and resources have not
/// yet been decoded.
///
/// Reachable only via `Apk<Unverified>::verify_v2() | verify_v3() |
/// verify_v4()`; the consuming-`self` semantics structurally
/// enforce that you cannot get to this state without a verify
/// step (or transmute, which would be unsafe — and `axiom-l1-rs`
/// has `#![forbid(unsafe_code)]`).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SignatureVerified;
impl private::Sealed for SignatureVerified {}
impl ApkState for SignatureVerified {
    const NAME: &'static str = "sig-verified";
}

/// Manifest + resources have been decoded; `V` records which
/// signing-block variant was verified upstream. Carrying `V` at
/// the type level lets downstream consumers (e.g. P1.10's
/// Merkle-commit hooks, P1.15's IR emission) gate on the variant
/// without runtime branches.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FullyParsed<V: SigVariant>(PhantomData<V>);
impl<V: SigVariant> private::Sealed for FullyParsed<V> {}
impl<V: SigVariant> ApkState for FullyParsed<V> {
    const NAME: &'static str = "fully-parsed";
}

// ---------------------------------------------------------------------
// Signing-block variants
// ---------------------------------------------------------------------

/// APK Signing Block v2 (Android 7.0+, APKv1 schemes pre-Q).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct V2;
impl private::Sealed for V2 {}
impl SigVariant for V2 {
    const TAG: u8 = 2;
    const NAME: &'static str = "v2";
}

/// APK Signing Block v3 (Android 9.0+, key-rotation support).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct V3;
impl private::Sealed for V3 {}
impl SigVariant for V3 {
    const TAG: u8 = 3;
    const NAME: &'static str = "v3";
}

/// APK Signing Block v4 (Android 11+, incremental delivery).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct V4;
impl private::Sealed for V4 {}
impl SigVariant for V4 {
    const TAG: u8 = 4;
    const NAME: &'static str = "v4";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn states_are_zero_sized() {
        // The whole point of phantom states: they cost nothing.
        assert_eq!(core::mem::size_of::<Unverified>(), 0);
        assert_eq!(core::mem::size_of::<SignatureVerified>(), 0);
        assert_eq!(core::mem::size_of::<FullyParsed<V2>>(), 0);
        assert_eq!(core::mem::size_of::<FullyParsed<V3>>(), 0);
        assert_eq!(core::mem::size_of::<FullyParsed<V4>>(), 0);
        assert_eq!(core::mem::size_of::<V2>(), 0);
        assert_eq!(core::mem::size_of::<V3>(), 0);
        assert_eq!(core::mem::size_of::<V4>(), 0);
    }

    #[test]
    fn state_names_match_lean_constructor_suffix() {
        // The translation-validation table in §B of the CHECKLIST
        // reads these exact strings. If you rename a constructor
        // here, also update the table or P1.9's TV pass will
        // refuse the build.
        assert_eq!(Unverified::NAME, "unverified");
        assert_eq!(SignatureVerified::NAME, "sig-verified");
        assert_eq!(<FullyParsed<V2> as ApkState>::NAME, "fully-parsed");
    }

    #[test]
    fn sig_variant_tags_match_lean_indices() {
        // Lean: inductive SigVariant where | v2 | v3 | v4
        // Constructor indices 0, 1, 2 → tags 2, 3, 4.
        assert_eq!(V2::TAG, 2);
        assert_eq!(V3::TAG, 3);
        assert_eq!(V4::TAG, 4);
    }
}
