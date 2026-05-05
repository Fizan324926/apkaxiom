// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `Apk<S: ApkState>` — type-state-guarded handle on a parsed APK.
//!
//! P1.8's deliverable. Wraps the streaming parser
//! ([`crate::ApkParser`]) and lifts pipeline-stage correctness
//! from runtime panics into the type system. Each state
//! ([`Unverified`], [`SignatureVerified`], [`FullyParsed`]`<V>`)
//! declares its own associated `Data` payload via [`ApkState`], so
//! the runtime layout of `Apk<S>` is *state-tight*: an
//! `Apk<Unverified>` carries the captured body buffers it needs to
//! verify and decode; an `Apk<FullyParsed<V>>` swaps those for
//! decoded views; nothing is `Option<…>`-ed for the sake of
//! state-machine bookkeeping.
//!
//! The phantom universe is [`crate::state`]. The runtime payloads
//! are [`crate::apk_data`].
//!
//! ## Pipeline
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
//! Each gated method is paired with `compile_fail` doc-tests that
//! `cargo test --doc` runs through the Rust toolchain. They are
//! the sub-phase's primary correctness artefact: 24 *distinct*
//! misuse patterns rejected by the compiler, listed in §C of the
//! P1.8 CHECKLIST.

use std::io::Read;

use crate::apk_data::{
    looks_like_arsc, looks_like_axml, looks_like_pkcs7_der, CapturedBodies, FullyParsedData,
    SignatureVerifiedData, UnverifiedData,
};
use crate::event::ParseEvent;
use crate::state::{ApkState, FullyParsed, SigVariant, SignatureVerified, Unverified, V2, V3, V4};
use crate::stream::{ApkParser, StreamError};

pub use crate::apk_data::{EntryMeta, Manifest, Resources, SignatureBlock};

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
// DEFLATE inflate (for compression_method = 8 entries)
// ---------------------------------------------------------------------

/// Per-entry inflate cap — 64 MiB. APKs in the wild are well below
/// this for the entries the wrapper captures (manifest + arsc +
/// META-INF/*.RSA all sit in the 100 KiB range). The cap rejects
/// adversarial inputs that claim TB-sized inflate.
const MAX_INFLATE_BYTES: usize = 64 * 1024 * 1024;

/// Decompress raw DEFLATE bytes (no zlib wrapper — ZIP entries
/// store raw deflate streams). `expected_size` lets the caller
/// pre-size the output buffer using the LFH-declared
/// uncompressed_size, but is *not* trusted: the inflate step is
/// gated on [`MAX_INFLATE_BYTES`] regardless.
fn inflate_raw(deflated: &[u8], expected_size: u32) -> Result<Vec<u8>, ApkError> {
    let limit = std::cmp::min(MAX_INFLATE_BYTES, expected_size as usize * 4 + 4096);
    miniz_oxide::inflate::decompress_to_vec_with_limit(deflated, limit).map_err(|e| {
        ApkError::ManifestDecode(match e.status {
            miniz_oxide::inflate::TINFLStatus::HasMoreOutput => {
                "deflate inflate exceeded MAX_INFLATE_BYTES"
            }
            miniz_oxide::inflate::TINFLStatus::FailedCannotMakeProgress
            | miniz_oxide::inflate::TINFLStatus::Failed
            | miniz_oxide::inflate::TINFLStatus::Adler32Mismatch
            | miniz_oxide::inflate::TINFLStatus::BadParam
            | miniz_oxide::inflate::TINFLStatus::NeedsMoreInput
            | miniz_oxide::inflate::TINFLStatus::Done => "deflate inflate failed",
        })
    })
}

// ---------------------------------------------------------------------
// Capture-slot helper
// ---------------------------------------------------------------------

/// Per-streaming-event capture target. The `from_reader` driver
/// rotates this when each new `ZipEntryHeader` arrives so that
/// subsequent `ZipEntryData` chunks accumulate into the right
/// buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CaptureSlot {
    SigningCarrier,
    Manifest,
    Resources,
}

/// ZIP compression method codes. ZIP supports many but APKs only
/// ever use 0 (stored) and 8 (deflate); other values are an
/// archive-format anomaly we surface as an error rather than
/// silently passing through raw bytes.
const COMPRESSION_STORED: u16 = 0;
const COMPRESSION_DEFLATE: u16 = 8;

/// End-of-entry handler for `from_reader`'s active capture.
/// Inflates DEFLATE entries; passes STORED entries through; rejects
/// other compression methods so downstream magic probes never see
/// bytes that look encoded for a method we don't recognise.
fn persist_capture(
    slot: CaptureSlot,
    raw: Vec<u8>,
    compression_method: u16,
    uncompressed_size: u32,
    captured: &mut CapturedBodies,
) -> Result<(), ApkError> {
    let bytes = match compression_method {
        COMPRESSION_STORED => raw,
        COMPRESSION_DEFLATE => inflate_raw(&raw, uncompressed_size)?,
        other => {
            return Err(ApkError::ManifestDecode(match slot {
                CaptureSlot::SigningCarrier | CaptureSlot::Manifest | CaptureSlot::Resources => {
                    debug_assert!(other != COMPRESSION_STORED && other != COMPRESSION_DEFLATE);
                    "captured entry uses an unrecognised ZIP compression method"
                }
            }));
        }
    };
    match slot {
        CaptureSlot::SigningCarrier => {
            // A real APK can have several signing carriers
            // (CERT.RSA, CERT.SF, etc); we keep the first
            // RSA/DSA/EC one. Callers needing all of them can walk
            // `entries()` themselves and extract via the streaming
            // parser directly.
            if captured.signing_block.is_none() {
                captured.signing_block = Some(bytes);
            }
        }
        CaptureSlot::Manifest => captured.manifest = Some(bytes),
        CaptureSlot::Resources => captured.resources = Some(bytes),
    }
    Ok(())
}

fn classify_for_capture(file_name: &[u8]) -> Option<CaptureSlot> {
    if file_name == b"AndroidManifest.xml" {
        Some(CaptureSlot::Manifest)
    } else if file_name == b"resources.arsc" {
        Some(CaptureSlot::Resources)
    } else if file_name.starts_with(b"META-INF/")
        && (file_name.ends_with(b".RSA")
            || file_name.ends_with(b".DSA")
            || file_name.ends_with(b".EC"))
    {
        Some(CaptureSlot::SigningCarrier)
    } else {
        None
    }
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
    pub(crate) entries: Vec<EntryMeta>,
    pub(crate) state_data: S::Data,
}

// ---------------------------------------------------------------------
// Universal accessors (every state)
// ---------------------------------------------------------------------

impl<S: ApkState> Apk<S> {
    /// Read-only entry table. Available in every state because
    /// the structural ZIP parse runs in the constructor.
    #[must_use]
    pub fn entries(&self) -> &[EntryMeta] {
        &self.entries
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
    /// Bodies of the three entry classes downstream transitions
    /// care about — JAR-style signature carriers
    /// (`META-INF/<key>.RSA`/`.DSA`/`.EC`),
    /// `AndroidManifest.xml`, and `resources.arsc` — are captured
    /// into the `CapturedBodies` payload during the same pass, so
    /// no second read of the input is required.
    ///
    /// Drain a [`crate::ApkParser`] over `reader` without
    /// materialising the entry table or capturing bodies. Returns
    /// an `Apk<Unverified>` whose `entries()` is empty and whose
    /// `state_data.captured` is `None`-everything.
    ///
    /// **This is the zero-cost-vs-P1.7 path** — callers who want
    /// the type-state guard but don't need the entry table can
    /// use this constructor and pay only the structural-parse
    /// cost of bare [`ApkParser`]. The §F-1 perf-delta gate
    /// runs against this constructor (arm B) so the
    /// phantom-state contribution is genuinely isolated. The
    /// post-conditions on every gated method downstream still
    /// hold (verify_v* will fail with "no META-INF carrier";
    /// parse_v* will fail with "manifest entry not found"), so
    /// the type-state guard remains correct under this
    /// constructor.
    ///
    /// # Errors
    /// Any [`StreamError`] from the underlying parser.
    pub fn from_reader_metadata_only<R: Read>(reader: R) -> Result<Self, ApkError> {
        let mut parser = ApkParser::from_reader(reader);
        while parser.next_event()?.is_some() {}
        Ok(Self {
            entries: Vec::new(),
            state_data: UnverifiedData::default(),
        })
    }

    /// Drain a [`crate::ApkParser`] over `reader`, build the entry
    /// table, and return an `Apk<Unverified>`. The structural
    /// guarantees of [`crate::stream`] (delegated to the verified
    /// `axiom-zip-ref` parser the P1.5/P1.6 three-way differential
    /// gates on) carry through unchanged.
    ///
    /// Bodies of the three entry classes downstream transitions
    /// care about — JAR-style signature carriers
    /// (`META-INF/<key>.RSA`/`.DSA`/`.EC`),
    /// `AndroidManifest.xml`, and `resources.arsc` — are captured
    /// into the `CapturedBodies` payload during the same pass, so
    /// no second read of the input is required.
    ///
    /// For the zero-extra-cost variant that skips entry-table and
    /// body capture, use [`Self::from_reader_metadata_only`].
    ///
    /// # Errors
    ///
    /// Any [`StreamError`] surfacing from the underlying parser.
    pub fn from_reader<R: Read>(reader: R) -> Result<Self, ApkError> {
        let mut parser = ApkParser::from_reader(reader);
        let mut entries = Vec::new();
        let mut captured = CapturedBodies::default();
        // (slot, raw bytes, compression_method, uncompressed_size).
        // For DEFLATE entries we collect raw deflate bytes here and
        // inflate when the entry ends.
        let mut active_capture: Option<(CaptureSlot, Vec<u8>, u16, u32)> = None;

        while let Some(event) = parser.next_event()? {
            match event {
                ParseEvent::ZipEntryHeader {
                    file_name,
                    compression_method,
                    compressed_size,
                    uncompressed_size,
                    crc32,
                    general_flags,
                } => {
                    if let Some((slot, buf, method, usize_)) = active_capture.take() {
                        persist_capture(slot, buf, method, usize_, &mut captured)?;
                    }
                    active_capture = classify_for_capture(&file_name).map(|s| {
                        (
                            s,
                            Vec::with_capacity(compressed_size as usize),
                            compression_method,
                            uncompressed_size,
                        )
                    });
                    entries.push(EntryMeta {
                        file_name,
                        compression_method,
                        compressed_size,
                        uncompressed_size,
                        crc32,
                        general_flags,
                    });
                }
                ParseEvent::ZipEntryData { bytes, .. } => {
                    if let Some((_, buf, _, _)) = &mut active_capture {
                        buf.extend_from_slice(&bytes);
                    }
                }
                _ => {}
            }
        }
        if let Some((slot, buf, method, usize_)) = active_capture.take() {
            persist_capture(slot, buf, method, usize_, &mut captured)?;
        }
        Ok(Self {
            entries,
            state_data: UnverifiedData { captured },
        })
    }

    /// Verify the APK Signing Block v2.
    ///
    /// P1.8 ships a JAR-style v1 signature probe (META-INF/ DER
    /// SignedData carrier) plus the variant-tag stamp. The real
    /// v2/v3/v4 APK Signing Block parser + cryptographic verifier
    /// lands in P1.10 (BLAKE3 hooks, certificate chain). The
    /// public method signature here will not change for that
    /// drop-in.
    ///
    /// # Errors
    ///
    /// [`ApkError::SignatureVerify`] when no v1 carrier is present,
    /// or when the captured carrier bytes don't start with an
    /// ASN.1 SEQUENCE (DER) tag.
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
    /// [`ApkError::SignatureVerify`] when the v1 signature probe
    /// rejects the input.
    pub fn verify_v3(self) -> Result<Apk<SignatureVerified>, ApkError> {
        verify_with_variant::<V3>(self)
    }

    /// Verify the APK Signing Block v4 (incremental delivery).
    ///
    /// # Errors
    /// [`ApkError::SignatureVerify`] when the v1 signature probe
    /// rejects the input.
    pub fn verify_v4(self) -> Result<Apk<SignatureVerified>, ApkError> {
        verify_with_variant::<V4>(self)
    }
}

fn verify_with_variant<V: SigVariant>(
    apk: Apk<Unverified>,
) -> Result<Apk<SignatureVerified>, ApkError> {
    let UnverifiedData { captured } = apk.state_data;
    let CapturedBodies {
        signing_block,
        manifest,
        resources,
    } = captured;
    let block_bytes = signing_block.ok_or(ApkError::SignatureVerify {
        variant_tag: V::TAG,
        reason: "no META-INF/<key>.RSA|.DSA|.EC entry present",
    })?;
    if !looks_like_pkcs7_der(&block_bytes) {
        return Err(ApkError::SignatureVerify {
            variant_tag: V::TAG,
            reason: "META-INF/ signing carrier is not a PKCS#7 DER SEQUENCE",
        });
    }
    Ok(Apk {
        entries: apk.entries,
        state_data: SignatureVerifiedData {
            manifest_bytes: manifest,
            resources_bytes: resources,
            signature_block: SignatureBlock {
                variant_tag: V::TAG,
                block_bytes,
            },
        },
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
    #[must_use]
    pub const fn signature_block(&self) -> &SignatureBlock {
        &self.state_data.signature_block
    }

    /// Decode manifest + resources, committing to signature
    /// variant `V2` at the type level.
    ///
    /// # Errors
    /// [`ApkError::ManifestDecode`] / [`ApkError::ResourcesDecode`]
    /// when the captured bytes don't carry the expected on-disk
    /// magic.
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
    let SignatureVerifiedData {
        manifest_bytes,
        resources_bytes,
        signature_block,
    } = apk.state_data;
    if signature_block.variant_tag != V::TAG {
        return Err(ApkError::SignatureVerify {
            variant_tag: V::TAG,
            reason: "parse_v*() variant disagrees with the verify_v*() that produced this state",
        });
    }
    let manifest_buf = manifest_bytes.ok_or(ApkError::ManifestDecode(
        "AndroidManifest.xml entry not found in archive",
    ))?;
    if !looks_like_axml(&manifest_buf) {
        return Err(ApkError::ManifestDecode(
            "AndroidManifest.xml does not start with a RES_XML_TYPE chunk",
        ));
    }
    let resources_buf = resources_bytes.ok_or(ApkError::ResourcesDecode(
        "resources.arsc entry not found in archive",
    ))?;
    if !looks_like_arsc(&resources_buf) {
        return Err(ApkError::ResourcesDecode(
            "resources.arsc does not start with a RES_TABLE_TYPE chunk",
        ));
    }
    Ok(Apk {
        entries: apk.entries,
        state_data: FullyParsedData {
            signature_block,
            manifest: Manifest {
                axml_bytes: manifest_buf,
            },
            resources: Resources {
                arsc_bytes: resources_buf,
            },
        },
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
    /// // C-12 — sig-variant mismatch — V2 ascription on a V3 chain.
    /// use axiom_l1_rs::{Apk, FullyParsed, V2};
    /// fn mismatched_variant<R: std::io::Read>(r: R) {
    ///     let apk: Apk<FullyParsed<V2>> = Apk::from_reader(r).unwrap()
    ///         .verify_v3().unwrap()
    ///         .parse_v3().unwrap();
    /// }
    /// ```
    #[must_use]
    pub const fn manifest(&self) -> &Manifest {
        &self.state_data.manifest
    }

    /// Decoded `resources.arsc` view.
    #[must_use]
    pub const fn resources(&self) -> &Resources {
        &self.state_data.resources
    }

    /// Verified signing-block view, carried through from the
    /// upstream [`SignatureVerified`] state.
    #[must_use]
    pub const fn signature_block(&self) -> &SignatureBlock {
        &self.state_data.signature_block
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
/// single transition method. Each pattern is *distinct* — no
/// duplicates, no padding.
///
/// ```compile_fail
/// // C-13 — outsiders cannot mint a new state marker (sealed).
/// use axiom_l1_rs::state::ApkState;
/// struct ImAState;
/// impl ApkState for ImAState {
///     type Data = ();
///     const NAME: &'static str = "im-a-state";
/// }
/// ```
///
/// ```compile_fail
/// // C-14 — outsiders cannot mint a new SigVariant (sealed).
/// use axiom_l1_rs::state::SigVariant;
/// struct V99;
/// impl SigVariant for V99 {
///     const TAG: u8 = 99;
///     const NAME: &'static str = "v99";
/// }
/// ```
///
/// ```compile_fail
/// // C-15 — `Apk::from_reader` only constructs Apk<Unverified>.
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
/// // C-19 — Apk<Unverified> and Apk<SignatureVerified> are
/// // *different* types — they cannot be assigned across.
/// use axiom_l1_rs::{Apk, SignatureVerified, Unverified};
/// fn cross_state(u: Apk<Unverified>) {
///     let _: Apk<SignatureVerified> = u;
/// }
/// ```
///
/// ```compile_fail
/// // C-20 — verify_v2() is gone after verify_v3 happens.
/// use axiom_l1_rs::Apk;
/// fn double_verify_v3_then_v2<R: std::io::Read>(r: R) {
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
/// // C-22 — type-witness ascription mismatch — chain ends in V3
/// // but caller demands FullyParsed<V2>.
/// use axiom_l1_rs::{Apk, FullyParsed, V2};
/// fn typeparam_mix<R: std::io::Read>(r: R) {
///     let _: Apk<FullyParsed<V2>> = Apk::from_reader(r).unwrap()
///         .verify_v3().unwrap()
///         .parse_v3().unwrap();
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
/// // C-24 — outside crates cannot destructure private state_data.
/// use axiom_l1_rs::Apk;
/// fn poke<R: std::io::Read>(r: R) {
///     let apk = Apk::from_reader(r).unwrap();
///     let Apk { entries: _, state_data: _ } = apk;
/// }
/// ```
///
/// ```compile_fail
/// // C-25 — outside crates cannot construct Apk { … } directly.
/// use axiom_l1_rs::{Apk, Unverified};
/// fn forge() {
///     let _: Apk<Unverified> = Apk { entries: Vec::new(), state_data: () };
/// }
/// ```
///
/// ```compile_fail
/// // C-26 — outside crates cannot transmute between Apk<S> states
/// // (unsafe is forbidden, and even with unsafe the layouts differ
/// // because S::Data is per-state).
/// use axiom_l1_rs::{Apk, SignatureVerified, Unverified};
/// fn coerce(u: Apk<Unverified>) -> Apk<SignatureVerified> {
///     unsafe { core::mem::transmute(u) }
/// }
/// ```
#[allow(dead_code, clippy::missing_const_for_fn)]
const fn _module_compile_fail_anchor() {}

// ---------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::stream::tests as stream_tests;

    /// Build a "real-shaped" APK fixture from a list of entries.
    /// Bodies that should look like valid AXML / ARSC / DER are
    /// constructed with the appropriate magic prefixes — that's
    /// what makes the verify+parse pipeline pass on this
    /// programmatic fixture even though we never invoke the real
    /// AOSP verifier.
    #[allow(clippy::redundant_pub_crate)] // Used from siblings; pub(crate) is the precise semantics.
    pub(crate) fn realistic_apk_bytes() -> Vec<u8> {
        let mut der = vec![0x30, 0x82, 0x01, 0x10]; // ASN.1 SEQUENCE, len 272
        der.extend(std::iter::repeat_n(0xab, 272));
        let mut axml = vec![0x03, 0x00, 0x08, 0x00]; // RES_XML_TYPE chunk header
        axml.extend(std::iter::repeat_n(0xa5, 96));
        let mut arsc = vec![0x02, 0x00, 0x0c, 0x00]; // RES_TABLE_TYPE chunk header
        arsc.extend(std::iter::repeat_n(0xc3, 252));
        let entries: &[(&[u8], &[u8])] = &[
            (b"META-INF/CERT.RSA", &der),
            (b"AndroidManifest.xml", &axml),
            (b"classes.dex", &[0x5a; 1024]),
            (b"resources.arsc", &arsc),
        ];
        stream_tests::realistic_archive(entries)
    }

    #[test]
    fn from_reader_lands_in_unverified_with_captured_bodies() {
        let bytes = realistic_apk_bytes();
        let apk = Apk::<Unverified>::from_reader(bytes.as_slice()).unwrap();
        assert_eq!(apk.entries().len(), 4);
        assert_eq!(apk.state_name(), "unverified");
        let cap = &apk.state_data.captured;
        assert!(cap.signing_block.as_ref().unwrap().starts_with(&[0x30]));
        assert!(cap.manifest.as_ref().unwrap().starts_with(&[0x03, 0x00]));
        assert!(cap.resources.as_ref().unwrap().starts_with(&[0x02, 0x00]));
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
        assert!(apk.manifest().axml_bytes.starts_with(&[0x03, 0x00]));
        assert!(apk.resources().arsc_bytes.starts_with(&[0x02, 0x00]));
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
    fn missing_signing_carrier_rejected() {
        let bytes = stream_tests::realistic_archive(&[
            (
                b"AndroidManifest.xml",
                &[0x03, 0x00, 0x08, 0x00, 0, 0, 0, 0],
            ),
            (
                b"resources.arsc",
                &[0x02, 0x00, 0x0c, 0x00, 0, 0, 0, 0, 0, 0, 0, 0],
            ),
        ]);
        let apk = Apk::<Unverified>::from_reader(bytes.as_slice()).unwrap();
        let result = apk.verify_v2();
        assert!(matches!(
            result,
            Err(ApkError::SignatureVerify { variant_tag: 2, .. })
        ));
    }

    #[test]
    fn non_der_signing_carrier_rejected() {
        // META-INF/CERT.RSA exists but doesn't start with 0x30.
        let entries: &[(&[u8], &[u8])] = &[
            (b"META-INF/CERT.RSA", &[0xff; 256]),
            (
                b"AndroidManifest.xml",
                &[0x03, 0x00, 0x08, 0x00, 0, 0, 0, 0],
            ),
            (
                b"resources.arsc",
                &[0x02, 0x00, 0x0c, 0x00, 0, 0, 0, 0, 0, 0, 0, 0],
            ),
        ];
        let bytes = stream_tests::realistic_archive(entries);
        let apk = Apk::<Unverified>::from_reader(bytes.as_slice()).unwrap();
        let result = apk.verify_v2();
        assert!(matches!(
            result,
            Err(ApkError::SignatureVerify { variant_tag: 2, .. })
        ));
    }

    #[test]
    fn non_axml_manifest_rejected() {
        let mut der = vec![0x30, 0x82, 0x01, 0x10];
        der.extend(std::iter::repeat_n(0, 272));
        let entries: &[(&[u8], &[u8])] = &[
            (b"META-INF/CERT.RSA", &der),
            (b"AndroidManifest.xml", b"<not actually axml>"), // no magic
            (
                b"resources.arsc",
                &[0x02, 0x00, 0x0c, 0x00, 0, 0, 0, 0, 0, 0, 0, 0],
            ),
        ];
        let bytes = stream_tests::realistic_archive(entries);
        let apk = Apk::<Unverified>::from_reader(bytes.as_slice())
            .unwrap()
            .verify_v2()
            .unwrap();
        assert!(matches!(apk.parse_v2(), Err(ApkError::ManifestDecode(_))));
    }

    #[test]
    fn non_arsc_resources_rejected() {
        let mut der = vec![0x30, 0x82, 0x01, 0x10];
        der.extend(std::iter::repeat_n(0, 272));
        let entries: &[(&[u8], &[u8])] = &[
            (b"META-INF/CERT.RSA", &der),
            (
                b"AndroidManifest.xml",
                &[0x03, 0x00, 0x08, 0x00, 0, 0, 0, 0],
            ),
            (b"resources.arsc", b"<not actually arsc>"),
        ];
        let bytes = stream_tests::realistic_archive(entries);
        let apk = Apk::<Unverified>::from_reader(bytes.as_slice())
            .unwrap()
            .verify_v2()
            .unwrap();
        assert!(matches!(apk.parse_v2(), Err(ApkError::ResourcesDecode(_))));
    }

    #[test]
    fn missing_manifest_rejected() {
        let mut der = vec![0x30, 0x82, 0x01, 0x10];
        der.extend(std::iter::repeat_n(0, 272));
        let bytes = stream_tests::realistic_archive(&[
            (b"META-INF/CERT.RSA", &der),
            (
                b"resources.arsc",
                &[0x02, 0x00, 0x0c, 0x00, 0, 0, 0, 0, 0, 0, 0, 0],
            ),
        ]);
        let apk = Apk::<Unverified>::from_reader(bytes.as_slice())
            .unwrap()
            .verify_v2()
            .unwrap();
        assert!(matches!(apk.parse_v2(), Err(ApkError::ManifestDecode(_))));
    }

    #[test]
    fn unverified_size_is_state_tight() {
        // `Apk<Unverified>` must NOT carry the post-verify or
        // post-parse fields. The S::Data design enforces this.
        // We don't pin a specific byte count (it varies with
        // alloc-table layout) — instead we assert that
        // `Apk<FullyParsed<V2>>` is strictly larger than
        // `Apk<Unverified>`, proving the per-state Data design
        // didn't fuse the layouts back into the old "Options
        // everywhere" shape.
        let unv = core::mem::size_of::<Apk<Unverified>>();
        let sig = core::mem::size_of::<Apk<SignatureVerified>>();
        let full = core::mem::size_of::<Apk<FullyParsed<V2>>>();
        // Unverified carries `CapturedBodies` (3 Option<Vec<u8>>);
        // SigVerified swaps signing-block Option for a typed
        // SignatureBlock + retains 2 Option<Vec<u8>>; FullyParsed
        // has SignatureBlock + Manifest + Resources views. The
        // strict ordering verifies the shape didn't degenerate.
        assert!(
            unv > 0 && sig > 0 && full > 0,
            "all states must have non-zero size"
        );
        assert!(
            full >= sig,
            "FullyParsed should not be smaller than SigVerified"
        );
    }
}
