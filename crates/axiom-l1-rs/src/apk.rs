// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `Apk<S: ApkState>` — type-state-guarded handle on a parsed APK.
//!
//! P1.8's deliverable. Wraps the streaming parser ([`crate::ApkParser`])
//! and lifts pipeline-stage correctness from runtime panics into
//! the type system. The state markers in [`crate::state`] are
//! zero-sized; the wrapper compiles to a pure `ApkInner` under
//! release codegen, and the §F-1 perf-delta gate verifies the
//! ≤ 0.1 % overhead requirement against the P1.7 baseline.
//!
//! ## Usage
//!
//! ```no_run
//! use axiom_l1_rs::{Apk, FullyParsed, SignatureVerified, Unverified, V2};
//! use std::io::Cursor;
//!
//! # fn run() -> Result<(), axiom_l1_rs::ApkError> {
//! let bytes: Vec<u8> = std::fs::read("path/to/app.apk").unwrap();
//! let apk: Apk<Unverified> = Apk::from_reader(Cursor::new(bytes))?;
//!
//! // Structural data (entry table) is available immediately.
//! let _entry_count = apk.entries().len();
//!
//! // Cryptographic verification is a state transition.
//! let apk: Apk<SignatureVerified> = apk.verify_v2()?;
//! let _ = apk.signature_block();
//!
//! // Decoding the manifest commits to a signature variant at the
//! // type level (here: V2).
//! let apk: Apk<FullyParsed<V2>> = apk.parse_v2()?;
//! let _ = apk.manifest();
//! let _ = apk.resources();
//! # Ok(())
//! # }
//! ```
//!
//! ## Compile-fail proofs
//!
//! Each gated method is paired with a `compile_fail` doc-test that
//! the Rust toolchain runs through `cargo test --doc`. They are
//! the sub-phase's primary correctness artefact: 24 misuse
//! patterns rejected by the compiler, listed in §C of the P1.8
//! CHECKLIST.

use core::marker::PhantomData;
use std::io::Read;

use crate::event::ParseEvent;
use crate::state::{ApkState, FullyParsed, SigVariant, SignatureVerified, Unverified, V2, V3, V4};
use crate::stream::{ApkParser, StreamError};

// ---------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------

/// Errors surfaced by [`Apk`] state transitions.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ApkError {
    /// Structural ZIP parse rejected the input.
    #[error("structural-parse: {0}")]
    Structural(#[from] StreamError),

    /// Signature verification failed for the requested variant.
    /// `variant_tag` matches `SigVariant::TAG`.
    #[error("sig-verify failed for variant {variant_tag} ({reason})")]
    SignatureVerify {
        /// Numeric tag of the variant attempted.
        variant_tag: u8,
        /// Human-readable reason.
        reason: &'static str,
    },

    /// Manifest decode rejected the AXML byte stream.
    #[error("manifest-decode: {0}")]
    ManifestDecode(&'static str),

    /// Resources decode rejected the ARSC byte stream.
    #[error("resources-decode: {0}")]
    ResourcesDecode(&'static str),
}

// ---------------------------------------------------------------------
// Inner storage
// ---------------------------------------------------------------------

/// Lightweight per-entry metadata exposed by every state. Mirrors
/// the fields of [`ParseEvent::ZipEntryHeader`] but owns its
/// strings so the entry table outlives the streaming parser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryMeta {
    /// File-name as bytes (APK file-names are conventionally UTF-8
    /// but the spec only constrains them to be byte-strings).
    pub file_name: Vec<u8>,
    /// `0` (stored) or `8` (deflate); other methods are rejected
    /// by `axiom-zip-ref` upstream.
    pub compression_method: u16,
    /// Compressed size in bytes (declared in the LFH).
    pub compressed_size: u32,
    /// Uncompressed size in bytes.
    pub uncompressed_size: u32,
    /// CRC32 of the uncompressed body.
    pub crc32: u32,
    /// LFH general-purpose flags.
    pub general_flags: u16,
}

/// Decoded AndroidManifest.xml view. P1.8 ships a placeholder
/// with the raw AXML byte slice; real AXML decoding (string-pool +
/// resource table) lands in P1.9.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// Raw AXML buffer that was extracted from `AndroidManifest.xml`.
    pub axml_bytes: Vec<u8>,
}

/// Decoded resources.arsc view. Placeholder shape; structured
/// access lands in P1.9.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resources {
    /// Raw ARSC buffer that was extracted from `resources.arsc`.
    pub arsc_bytes: Vec<u8>,
}

/// Bytes that make up the verified APK Signing Block. Placeholder
/// view; the structured certificate-chain breakdown lands in P1.10
/// alongside the actual cryptographic verifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureBlock {
    /// Variant tag (matches `SigVariant::TAG`).
    pub variant_tag: u8,
    /// Raw signing-block bytes.
    pub block_bytes: Vec<u8>,
}

/// Internal storage shared across every `Apk<S>`. Methods exposed
/// publicly are gated by the wrapping state — `inner` itself is
/// crate-private.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct ApkInner {
    entries: Vec<EntryMeta>,
    /// Populated once a `verify_v*()` transition succeeds.
    signature_block: Option<SignatureBlock>,
    /// Populated once a `parse_v*()` transition succeeds.
    manifest: Option<Manifest>,
    /// Populated once a `parse_v*()` transition succeeds.
    resources: Option<Resources>,
}

// ---------------------------------------------------------------------
// Apk<S>
// ---------------------------------------------------------------------

/// Type-state-guarded handle on a parsed APK.
///
/// `S` is one of [`Unverified`], [`SignatureVerified`], or
/// [`FullyParsed`]`<V>` (with `V ∈ {`[`V2`]`, `[`V3`]`, `[`V4`]`}`).
/// State transitions consume `self` and return the next state, so
/// the misuse cases listed in §C of the CHECKLIST are statically
/// rejected by the compiler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Apk<S: ApkState> {
    inner: ApkInner,
    /// Phantom marker — zero-sized at runtime under release codegen
    /// (verified by the §F-1 perf-delta gate).
    _state: PhantomData<S>,
}

// ---------------------------------------------------------------------
// Universal accessors (every state)
// ---------------------------------------------------------------------

impl<S: ApkState> Apk<S> {
    /// Read-only entry table. Available in every state because
    /// the structural ZIP parse runs in the constructor.
    #[must_use]
    pub fn entries(&self) -> &[EntryMeta] {
        &self.inner.entries
    }

    /// Stable name of the current state, equal to the Lean
    /// constructor suffix. Used for diagnostics and by P1.9's
    /// translation-validation pass.
    #[must_use]
    pub const fn state_name(&self) -> &'static str {
        S::NAME
    }
}

// ---------------------------------------------------------------------
// Apk<Unverified> — constructors and verify transitions
// ---------------------------------------------------------------------

impl Apk<Unverified> {
    /// Drain a [`crate::ApkParser`] over `reader`, build the entry
    /// table, and return an `Apk<Unverified>`. The structural
    /// guarantees of [`crate::stream`] (delegated to the verified
    /// `axiom-zip-ref` parser the P1.5/P1.6 three-way differential
    /// gates on) carry through unchanged.
    ///
    /// # Errors
    ///
    /// Any [`StreamError`] surfacing from the underlying parser.
    pub fn from_reader<R: Read>(reader: R) -> Result<Self, ApkError> {
        let mut parser = ApkParser::from_reader(reader);
        let mut entries = Vec::new();
        while let Some(event) = parser.next_event()? {
            if let ParseEvent::ZipEntryHeader {
                file_name,
                compression_method,
                compressed_size,
                uncompressed_size,
                crc32,
                general_flags,
            } = event
            {
                entries.push(EntryMeta {
                    file_name,
                    compression_method,
                    compressed_size,
                    uncompressed_size,
                    crc32,
                    general_flags,
                });
            }
        }
        Ok(Self {
            inner: ApkInner {
                entries,
                ..ApkInner::default()
            },
            _state: PhantomData,
        })
    }

    /// Verify the APK Signing Block v2.
    ///
    /// P1.8 ships a structural placeholder verifier; the real
    /// crypto landing happens in P1.10 (BLAKE3 hooks, certificate
    /// chain). Until P1.10, this transition checks only that the
    /// archive contains an entry whose name marks it as a v2
    /// signing-block carrier.
    ///
    /// # Errors
    ///
    /// [`ApkError::SignatureVerify`] when no v2 carrier is present.
    ///
    /// ## Compile-fail proofs
    ///
    /// ```compile_fail
    /// // C-01 — `manifest()` is not in scope on Unverified.
    /// use axiom_l1_rs::{Apk, Unverified};
    /// fn use_manifest(apk: Apk<Unverified>) {
    ///     let _ = apk.manifest();
    /// }
    /// ```
    ///
    /// ```compile_fail
    /// // C-02 — `resources()` is not in scope on Unverified.
    /// use axiom_l1_rs::{Apk, Unverified};
    /// fn use_resources(apk: Apk<Unverified>) {
    ///     let _ = apk.resources();
    /// }
    /// ```
    ///
    /// ```compile_fail
    /// // C-03 — `signature_block()` is not in scope on Unverified.
    /// use axiom_l1_rs::{Apk, Unverified};
    /// fn use_sigblock(apk: Apk<Unverified>) {
    ///     let _ = apk.signature_block();
    /// }
    /// ```
    pub fn verify_v2(self) -> Result<Apk<SignatureVerified>, ApkError> {
        verify_with_variant::<V2>(self)
    }

    /// Verify the APK Signing Block v3 (key-rotation support).
    ///
    /// # Errors
    /// [`ApkError::SignatureVerify`] when no v3 carrier is present.
    pub fn verify_v3(self) -> Result<Apk<SignatureVerified>, ApkError> {
        verify_with_variant::<V3>(self)
    }

    /// Verify the APK Signing Block v4 (incremental delivery).
    ///
    /// # Errors
    /// [`ApkError::SignatureVerify`] when no v4 carrier is present.
    pub fn verify_v4(self) -> Result<Apk<SignatureVerified>, ApkError> {
        verify_with_variant::<V4>(self)
    }
}

/// Internal helper shared by the three `verify_v*` transitions.
/// Records the variant tag in the inner signature-block view so
/// downstream `parse_v*` transitions can cross-check that the
/// caller's chosen `V` matches the one verified.
fn verify_with_variant<V: SigVariant>(
    apk: Apk<Unverified>,
) -> Result<Apk<SignatureVerified>, ApkError> {
    // Placeholder check: the archive must declare an APK Signing
    // Block carrier. Real verification (digest, cert chain) lands
    // in P1.10.
    let has_signing_block = apk
        .inner
        .entries
        .iter()
        .any(|e| e.file_name.starts_with(b"META-INF/"));
    if !has_signing_block {
        return Err(ApkError::SignatureVerify {
            variant_tag: V::TAG,
            reason: "no META-INF/ signing-block carrier present",
        });
    }
    let block_bytes = vec![]; // P1.10 wires the real bytes.
    let mut inner = apk.inner;
    inner.signature_block = Some(SignatureBlock {
        variant_tag: V::TAG,
        block_bytes,
    });
    Ok(Apk {
        inner,
        _state: PhantomData,
    })
}

// ---------------------------------------------------------------------
// Apk<SignatureVerified> — sig-block accessor + parse transitions
// ---------------------------------------------------------------------

impl Apk<SignatureVerified> {
    /// Verified-but-not-yet-decoded signing-block view.
    ///
    /// ## Compile-fail proofs
    ///
    /// ```compile_fail
    /// // C-04 — verify_v2() consumes the input; can't call it again.
    /// use axiom_l1_rs::Apk;
    /// fn double_verify<R: std::io::Read>(r: R) {
    ///     let apk = Apk::from_reader(r).unwrap().verify_v2().unwrap();
    ///     let _ = apk.verify_v2(); // method does not exist on Apk<SignatureVerified>
    /// }
    /// ```
    ///
    /// ```compile_fail
    /// // C-05 — verify_v3() does not exist on Apk<SignatureVerified>.
    /// use axiom_l1_rs::Apk;
    /// fn extra_v3<R: std::io::Read>(r: R) {
    ///     let apk = Apk::from_reader(r).unwrap().verify_v2().unwrap();
    ///     let _ = apk.verify_v3();
    /// }
    /// ```
    ///
    /// ```compile_fail
    /// // C-06 — verify_v4() does not exist on Apk<SignatureVerified>.
    /// use axiom_l1_rs::Apk;
    /// fn extra_v4<R: std::io::Read>(r: R) {
    ///     let apk = Apk::from_reader(r).unwrap().verify_v3().unwrap();
    ///     let _ = apk.verify_v4();
    /// }
    /// ```
    ///
    /// ```compile_fail
    /// // C-07 — manifest() is not on Apk<SignatureVerified>.
    /// use axiom_l1_rs::Apk;
    /// fn early_manifest<R: std::io::Read>(r: R) {
    ///     let apk = Apk::from_reader(r).unwrap().verify_v2().unwrap();
    ///     let _ = apk.manifest();
    /// }
    /// ```
    ///
    /// ```compile_fail
    /// // C-08 — resources() is not on Apk<SignatureVerified>.
    /// use axiom_l1_rs::Apk;
    /// fn early_resources<R: std::io::Read>(r: R) {
    ///     let apk = Apk::from_reader(r).unwrap().verify_v2().unwrap();
    ///     let _ = apk.resources();
    /// }
    /// ```
    ///
    /// # Panics
    ///
    /// Never under sound use — the only constructor of
    /// `Apk<SignatureVerified>` is the crate-internal
    /// `verify_with_variant` helper, which always populates
    /// `signature_block`. Documented for `clippy::missing_panics_doc`.
    #[must_use]
    pub const fn signature_block(&self) -> &SignatureBlock {
        // SAFETY (logical): the only constructor of `Apk<SignatureVerified>`
        // is `verify_with_variant`, which always populates `signature_block`.
        // The wrapper module enforces this invariant; no `unsafe` is needed.
        self.inner
            .signature_block
            .as_ref()
            .expect("internal invariant: SignatureVerified state populates signature_block")
    }

    /// Decode manifest + resources, committing to signature
    /// variant `V2` at the type level.
    ///
    /// # Errors
    /// [`ApkError::ManifestDecode`] / [`ApkError::ResourcesDecode`].
    pub fn parse_v2(self) -> Result<Apk<FullyParsed<V2>>, ApkError> {
        parse_with_variant::<V2>(self)
    }

    /// Decode manifest + resources, committing to signature
    /// variant `V3` at the type level.
    ///
    /// # Errors
    /// [`ApkError::ManifestDecode`] / [`ApkError::ResourcesDecode`].
    pub fn parse_v3(self) -> Result<Apk<FullyParsed<V3>>, ApkError> {
        parse_with_variant::<V3>(self)
    }

    /// Decode manifest + resources, committing to signature
    /// variant `V4` at the type level.
    ///
    /// # Errors
    /// [`ApkError::ManifestDecode`] / [`ApkError::ResourcesDecode`].
    pub fn parse_v4(self) -> Result<Apk<FullyParsed<V4>>, ApkError> {
        parse_with_variant::<V4>(self)
    }
}

fn parse_with_variant<V: SigVariant>(
    apk: Apk<SignatureVerified>,
) -> Result<Apk<FullyParsed<V>>, ApkError> {
    let mut inner = apk.inner;
    // Cross-check: the caller's chosen `V` must match the variant
    // the upstream `verify_v*` recorded. P1.10 will replace this
    // with a cryptographic re-binding step; today it's a runtime
    // sanity guard that complements the type-level commitment.
    let block_tag = inner
        .signature_block
        .as_ref()
        .expect("internal invariant: SignatureVerified populates signature_block")
        .variant_tag;
    if block_tag != V::TAG {
        return Err(ApkError::SignatureVerify {
            variant_tag: V::TAG,
            reason: "parse_v*() variant disagrees with the verify_v*() that produced this state",
        });
    }
    // Locate manifest / resources entries by canonical name. The
    // bytes themselves are placeholder until P1.9 wires the real
    // AXML / ARSC decoder; today we record presence + zero-length
    // buffers so downstream methods are well-typed.
    let has_manifest = inner
        .entries
        .iter()
        .any(|e| e.file_name == b"AndroidManifest.xml");
    if !has_manifest {
        return Err(ApkError::ManifestDecode(
            "AndroidManifest.xml entry not found in archive",
        ));
    }
    let has_resources = inner
        .entries
        .iter()
        .any(|e| e.file_name == b"resources.arsc");
    if !has_resources {
        return Err(ApkError::ResourcesDecode(
            "resources.arsc entry not found in archive",
        ));
    }
    inner.manifest = Some(Manifest {
        axml_bytes: Vec::new(),
    });
    inner.resources = Some(Resources {
        arsc_bytes: Vec::new(),
    });
    Ok(Apk {
        inner,
        _state: PhantomData,
    })
}

// ---------------------------------------------------------------------
// Apk<FullyParsed<V>>
// ---------------------------------------------------------------------

impl<V: SigVariant> Apk<FullyParsed<V>> {
    /// Decoded `AndroidManifest.xml` view.
    ///
    /// ## Compile-fail proofs
    ///
    /// ```compile_fail
    /// // C-09 — parse_v2() is not on Apk<FullyParsed<V2>>.
    /// use axiom_l1_rs::Apk;
    /// fn double_parse<R: std::io::Read>(r: R) {
    ///     let apk = Apk::from_reader(r).unwrap()
    ///         .verify_v2().unwrap()
    ///         .parse_v2().unwrap();
    ///     let _ = apk.parse_v2();
    /// }
    /// ```
    ///
    /// ```compile_fail
    /// // C-10 — parse_v3() is not on Apk<FullyParsed<V2>>.
    /// use axiom_l1_rs::Apk;
    /// fn cross_parse<R: std::io::Read>(r: R) {
    ///     let apk = Apk::from_reader(r).unwrap()
    ///         .verify_v2().unwrap()
    ///         .parse_v2().unwrap();
    ///     let _ = apk.parse_v3();
    /// }
    /// ```
    ///
    /// ```compile_fail
    /// // C-11 — verify_v2() is not on Apk<FullyParsed<V2>>.
    /// use axiom_l1_rs::Apk;
    /// fn re_verify<R: std::io::Read>(r: R) {
    ///     let apk = Apk::from_reader(r).unwrap()
    ///         .verify_v2().unwrap()
    ///         .parse_v2().unwrap();
    ///     let _ = apk.verify_v2();
    /// }
    /// ```
    ///
    /// ```compile_fail
    /// // C-12 — sig-variant mismatch — V2 from V3 chain.
    /// use axiom_l1_rs::{Apk, FullyParsed, V2};
    /// fn mismatched_variant<R: std::io::Read>(r: R) {
    ///     let apk: Apk<FullyParsed<V2>> = Apk::from_reader(r).unwrap()
    ///         .verify_v3().unwrap()
    ///         .parse_v3().unwrap();
    /// }
    /// ```
    ///
    /// # Panics
    ///
    /// Never under sound use — the crate-internal `parse_with_variant`
    /// helper always populates `manifest` before transitioning to
    /// `FullyParsed<V>`. Documented for `clippy::missing_panics_doc`.
    #[must_use]
    pub const fn manifest(&self) -> &Manifest {
        self.inner
            .manifest
            .as_ref()
            .expect("internal invariant: FullyParsed populates manifest")
    }

    /// Decoded `resources.arsc` view.
    ///
    /// # Panics
    ///
    /// Never under sound use — the crate-internal `parse_with_variant`
    /// helper always populates `resources` before transitioning to
    /// `FullyParsed<V>`.
    #[must_use]
    pub const fn resources(&self) -> &Resources {
        self.inner
            .resources
            .as_ref()
            .expect("internal invariant: FullyParsed populates resources")
    }

    /// Verified signing-block view, carried through from the
    /// upstream [`SignatureVerified`] state.
    ///
    /// # Panics
    ///
    /// Never under sound use — populated by the upstream `verify_v*`
    /// transition.
    #[must_use]
    pub const fn signature_block(&self) -> &SignatureBlock {
        self.inner
            .signature_block
            .as_ref()
            .expect("internal invariant: FullyParsed populates signature_block")
    }

    /// Numeric tag of the verified signing-block variant. Equals
    /// `V::TAG`.
    #[must_use]
    pub const fn signing_variant_tag(&self) -> u8 {
        V::TAG
    }
}

// ---------------------------------------------------------------------
// Module-level compile-fail proofs (catch-all patterns that don't
// hang naturally off a single method)
// ---------------------------------------------------------------------

/// Catch-all compile-fail proofs that don't naturally hang off a
/// single transition method.
///
/// ```compile_fail
/// // C-13 — outsiders cannot mint a new state marker.
/// use axiom_l1_rs::state::ApkState;
/// struct ImAState;
/// impl ApkState for ImAState {
///     const NAME: &'static str = "im-a-state";
/// }
/// ```
///
/// ```compile_fail
/// // C-14 — outsiders cannot mint a new SigVariant.
/// use axiom_l1_rs::state::SigVariant;
/// struct V99;
/// impl SigVariant for V99 {
///     const TAG: u8 = 99;
///     const NAME: &'static str = "v99";
/// }
/// ```
///
/// ```compile_fail
/// // C-15 — Unverified is the only public constructor target.
/// use axiom_l1_rs::{Apk, SignatureVerified};
/// fn skip_verify<R: std::io::Read>(r: R) {
///     let _: Apk<SignatureVerified> = Apk::from_reader(r).unwrap();
/// }
/// ```
///
/// ```compile_fail
/// // C-16 — cannot construct FullyParsed directly from a reader.
/// use axiom_l1_rs::{Apk, FullyParsed, V2};
/// fn skip_pipeline<R: std::io::Read>(r: R) {
///     let _: Apk<FullyParsed<V2>> = Apk::from_reader(r).unwrap();
/// }
/// ```
///
/// ```compile_fail
/// // C-17 — Apk<S> rejects non-state types in S.
/// use axiom_l1_rs::Apk;
/// fn bad_state<R: std::io::Read>(r: R) {
///     let _: Apk<u32> = Apk::from_reader(r).unwrap();
/// }
/// ```
///
/// ```compile_fail
/// // C-18 — FullyParsed<V> rejects non-SigVariant V.
/// use axiom_l1_rs::FullyParsed;
/// type Bad = axiom_l1_rs::Apk<FullyParsed<u32>>;
/// fn rejected(_: Bad) {}
/// ```
///
/// ```compile_fail
/// // C-19 — sealed: outside crates can't impl axiom_l1_rs::state::ApkState.
/// use axiom_l1_rs::state::ApkState;
/// struct Custom;
/// impl ApkState for Custom { const NAME: &'static str = "custom"; }
/// ```
///
/// ```compile_fail
/// // C-20 — verify_v* methods are gone after the verify happens.
/// use axiom_l1_rs::Apk;
/// fn double_verify_v3<R: std::io::Read>(r: R) {
///     let apk = Apk::from_reader(r).unwrap().verify_v3().unwrap();
///     let _ = apk.verify_v2();
/// }
/// ```
///
/// ```compile_fail
/// // C-21 — parse_v3 not callable on Apk<Unverified>.
/// use axiom_l1_rs::Apk;
/// fn skip_to_parse<R: std::io::Read>(r: R) {
///     let apk = Apk::from_reader(r).unwrap();
///     let _ = apk.parse_v3();
/// }
/// ```
///
/// ```compile_fail
/// // C-22 — manifest() not callable on FullyParsed<V3> via FullyParsed<V2>
/// // chain (mismatched type witness).
/// use axiom_l1_rs::{Apk, FullyParsed, V2, V3};
/// fn typeparam_mix<R: std::io::Read>(r: R) {
///     let apk: Apk<FullyParsed<V3>> = Apk::from_reader(r).unwrap()
///         .verify_v2().unwrap()
///         .parse_v2().unwrap();
/// }
/// ```
///
/// ```compile_fail
/// // C-23 — signing_variant_tag is gated on FullyParsed<V>.
/// use axiom_l1_rs::Apk;
/// fn early_tag<R: std::io::Read>(r: R) {
///     let apk = Apk::from_reader(r).unwrap();
///     let _ = apk.signing_variant_tag();
/// }
/// ```
///
/// ```compile_fail
/// // C-24 — `_state` field is private.
/// use axiom_l1_rs::{Apk, Unverified};
/// fn poke_state<R: std::io::Read>(r: R) {
///     let apk = Apk::from_reader(r).unwrap();
///     let _: std::marker::PhantomData<Unverified> = apk._state;
/// }
/// ```
#[allow(dead_code, clippy::missing_const_for_fn)]
const fn _module_compile_fail_anchor() {}

// ---------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::tests as stream_tests;

    /// Reuse `stream::tests::realistic_archive` to build a realistic
    /// 4-entry archive whose entry table covers META-INF (signing
    /// block carrier), AndroidManifest.xml, classes.dex, and
    /// resources.arsc.
    fn realistic_apk_bytes() -> Vec<u8> {
        stream_tests::realistic_archive(&[
            (b"META-INF/CERT.RSA", &[0xab; 32]),
            (b"AndroidManifest.xml", &[0xa5; 100]),
            (b"classes.dex", &[0x5a; 1024]),
            (b"resources.arsc", &[0xc3; 256]),
        ])
    }

    #[test]
    fn from_reader_lands_in_unverified() {
        let bytes = realistic_apk_bytes();
        let apk = Apk::<Unverified>::from_reader(bytes.as_slice()).unwrap();
        assert_eq!(apk.entries().len(), 4);
        assert_eq!(apk.state_name(), "unverified");
    }

    #[test]
    fn full_pipeline_v2() {
        let bytes = realistic_apk_bytes();
        let apk = Apk::<Unverified>::from_reader(bytes.as_slice())
            .unwrap()
            .verify_v2()
            .unwrap()
            .parse_v2()
            .unwrap();
        assert_eq!(apk.signing_variant_tag(), 2);
        assert_eq!(apk.signature_block().variant_tag, 2);
        assert_eq!(apk.state_name(), "fully-parsed");
    }

    #[test]
    fn full_pipeline_v3() {
        let bytes = realistic_apk_bytes();
        let apk = Apk::<Unverified>::from_reader(bytes.as_slice())
            .unwrap()
            .verify_v3()
            .unwrap()
            .parse_v3()
            .unwrap();
        assert_eq!(apk.signing_variant_tag(), 3);
    }

    #[test]
    fn full_pipeline_v4() {
        let bytes = realistic_apk_bytes();
        let apk = Apk::<Unverified>::from_reader(bytes.as_slice())
            .unwrap()
            .verify_v4()
            .unwrap()
            .parse_v4()
            .unwrap();
        assert_eq!(apk.signing_variant_tag(), 4);
    }

    #[test]
    fn variant_mismatch_rejected_at_runtime() {
        // The type system prevents the `Apk<FullyParsed<V2>>` =
        // verify_v3().parse_v2() chain (C-22 covers that), but a
        // legitimate verify_v2().parse_v3() chain is statically
        // *allowed* — both return Apk<FullyParsed<V3>> via the
        // type-witness on parse_v3. We runtime-guard this with the
        // variant_tag cross-check inside `parse_with_variant`.
        let bytes = realistic_apk_bytes();
        let result = Apk::<Unverified>::from_reader(bytes.as_slice())
            .unwrap()
            .verify_v2()
            .unwrap()
            .parse_v3();
        assert!(matches!(
            result,
            Err(ApkError::SignatureVerify { variant_tag: 3, .. })
        ));
    }

    #[test]
    fn missing_signing_block_rejected() {
        // Build an archive without any META-INF/ entry.
        let bytes = stream_tests::realistic_archive(&[
            (b"AndroidManifest.xml", &[0xa5; 100]),
            (b"resources.arsc", &[0xc3; 256]),
        ]);
        let apk = Apk::<Unverified>::from_reader(bytes.as_slice()).unwrap();
        let result = apk.verify_v2();
        assert!(matches!(
            result,
            Err(ApkError::SignatureVerify { variant_tag: 2, .. })
        ));
    }

    #[test]
    fn missing_manifest_rejected() {
        // META-INF present so verify passes; AndroidManifest.xml
        // missing so parse fails.
        let bytes = stream_tests::realistic_archive(&[
            (b"META-INF/CERT.RSA", &[0xab; 32]),
            (b"resources.arsc", &[0xc3; 256]),
        ]);
        let apk = Apk::<Unverified>::from_reader(bytes.as_slice())
            .unwrap()
            .verify_v2()
            .unwrap();
        assert!(matches!(apk.parse_v2(), Err(ApkError::ManifestDecode(_))));
    }

    #[test]
    fn apk_is_zero_overhead_over_apkinner() {
        // Whole point of the phantom design: Apk<S> is the same
        // size as ApkInner under release codegen. The compiler
        // *should* drop PhantomData<S> entirely.
        assert_eq!(
            core::mem::size_of::<Apk<Unverified>>(),
            core::mem::size_of::<ApkInner>()
        );
        assert_eq!(
            core::mem::size_of::<Apk<SignatureVerified>>(),
            core::mem::size_of::<ApkInner>()
        );
        assert_eq!(
            core::mem::size_of::<Apk<FullyParsed<V2>>>(),
            core::mem::size_of::<ApkInner>()
        );
        assert_eq!(
            core::mem::size_of::<Apk<FullyParsed<V3>>>(),
            core::mem::size_of::<ApkInner>()
        );
        assert_eq!(
            core::mem::size_of::<Apk<FullyParsed<V4>>>(),
            core::mem::size_of::<ApkInner>()
        );
    }
}
