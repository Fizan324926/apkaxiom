// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Per-state runtime payloads for [`crate::apk::Apk`].
//!
//! Each `ApkState` declares an associated `Data` type; `Apk<S>`
//! stores `S::Data` directly. Compared to the earlier
//! "single-`ApkInner`-with-Options-everywhere" layout, this:
//!
//!   - eliminates the always-`None` `Option<SignatureBlock>`,
//!     `Option<Manifest>`, `Option<Resources>` carried by every
//!     `Apk<Unverified>` (≈ 80 bytes recovered per APK in
//!     `Unverified` state),
//!   - removes the need for `PhantomData<S>` on `Apk` itself —
//!     `S` is already projected into the type via `S::Data`,
//!   - turns the `expect("internal invariant…")` panics on
//!     `signature_block()` / `manifest()` / `resources()` into
//!     direct field accesses (statically infallible — the field
//!     simply doesn't exist on a state that wouldn't have it).

// The per-variant APK Signing Block ID constants (AOSP
// V2_BLOCK_ID = 0x7109_871a, V3 = 0xf057_41b3) are not yet
// load-bearing — P1.10's real verifier will read them once it
// parses the block. Keeping them as live `pub(crate)` constants
// invites drift, so we defer until they are consumed.

use crate::apk::ApkError;

/// Lightweight per-entry metadata exposed by every state.
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

/// Decoded `AndroidManifest.xml` view. P1.8 ships a wrapper around
/// the raw AXML byte slice with the magic confirmed; the structured
/// string-pool / resource-table decoder lands in P1.9 and replaces
/// the byte slice with proper field accessors without changing the
/// public method signatures on [`crate::apk::Apk`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// Raw AXML buffer that was extracted from `AndroidManifest.xml`.
    /// First four bytes confirmed to be the AXML magic
    /// (`0x00080003`) when this struct was built.
    pub axml_bytes: Vec<u8>,
}

/// Decoded `resources.arsc` view. Wraps the raw ARSC bytes with
/// the magic confirmed; structured access lands in P1.9.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resources {
    /// Raw ARSC buffer that was extracted from `resources.arsc`.
    /// First two bytes confirmed to be the ARSC `RES_TABLE_TYPE`
    /// header (`0x0002`) when this struct was built.
    pub arsc_bytes: Vec<u8>,
}

/// JAR-style v1 signature carrier — the inflated bytes of a
/// `META-INF/<key>.RSA` (or `.DSA` / `.EC`) entry. The carrier
/// holds a PKCS#7 SignedData blob that lists the algorithm,
/// digest list, and certificate chain that signed the APK
/// pre-v2.
///
/// **Not the APK Signing Block.** A real APK Signing Block
/// (Android 7.0+, the v2/v3/v4 schemes) lives in a separate
/// on-disk region between the last LFH and the central
/// directory, with a 16-byte trailer magic `"APK Sig Block 42"`
/// and ID-value-pair records keyed by `0x7109_871a` (v2),
/// `0xf057_4322` (v3), or `0x4239_4b41` (v4). P1.8 does not yet
/// parse that block — the wrapper today *probes* the JAR carrier
/// (DER tag) and *stamps* the requested variant tag from the
/// caller. The `Jarv1Carrier::block_bytes` is the v1 SignedData
/// blob; `ApkSigBlock::block_bytes` is the v2/v3/v4 region (left
/// `None` until P1.10 wires the parser).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Jarv1Carrier {
    /// Inflated PKCS#7 SignedData bytes (start with `0x30` ASN.1
    /// SEQUENCE tag, verified at construction).
    pub block_bytes: Vec<u8>,
}

/// APK Signing Block (v2/v3/v4) — the on-disk region between the
/// last LFH and the central directory. P1.8 leaves `block_bytes`
/// as `None` because the parser for this region is owned by P1.10
/// (alongside the cryptographic verifier). The variant tag is
/// already stamped by `verify_v*` so consumers can read it; the
/// raw bytes will follow when P1.10 lands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApkSigBlock {
    /// Raw bytes of the APK Signing Block region. `None` in P1.8;
    /// `Some(_)` in P1.10+.
    pub block_bytes: Option<Vec<u8>>,
}

/// Verified signing material — the union of what a `verify_v*`
/// transition produces. P1.8's placeholder verifier produces only
/// the `jar_v1_carrier` half; P1.10's real verifier will populate
/// `apk_sig_block.block_bytes` and validate certificate chains.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureBlock {
    /// Variant tag (matches `SigVariant::TAG`). Stamped by the
    /// `verify_v*` transition that produced this struct.
    pub variant_tag: u8,
    /// JAR-style v1 SignedData carrier (always present in P1.8 —
    /// the placeholder verifier requires a META-INF/<key>.RSA
    /// entry).
    pub jar_v1_carrier: Jarv1Carrier,
    /// APK Signing Block v2/v3/v4 region. P1.8: always
    /// `block_bytes: None`. P1.10: will be `Some(_)` when the
    /// real verifier lands.
    pub apk_sig_block: ApkSigBlock,
}

impl SignatureBlock {
    /// Compatibility shim — until P1.10 lands the APK Signing
    /// Block parser, downstream consumers (Merkle hooks,
    /// translation validation) read the v1 carrier bytes via the
    /// same accessor that will read the v2 block. Returns the v2
    /// block bytes if present, else falls back to the v1 carrier
    /// bytes.
    #[must_use]
    pub fn block_bytes(&self) -> &[u8] {
        self.apk_sig_block
            .block_bytes
            .as_deref()
            .unwrap_or(&self.jar_v1_carrier.block_bytes)
    }
}

// ---------------------------------------------------------------------
// Per-state runtime payloads
// ---------------------------------------------------------------------

/// Raw bodies captured during streaming for downstream verify /
/// parse to consume.
///
/// A real APK can carry several v1 signing carriers (multi-signed
/// builds, debug + release keys, etc.); we keep all of them in
/// `signing_carriers` rather than truncating to the first. The
/// placeholder `verify_v*` accepts any one of them; P1.10's real
/// verifier will iterate and validate each.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CapturedBodies {
    /// Inflated bytes of every `META-INF/<key>.RSA|.DSA|.EC`
    /// entry encountered. Empty until the constructor finds one.
    pub signing_carriers: Vec<Vec<u8>>,
    /// Raw bytes of `AndroidManifest.xml`.
    pub manifest: Option<Vec<u8>>,
    /// Raw bytes of `resources.arsc`.
    pub resources: Option<Vec<u8>>,
}

/// Runtime payload for `Apk<Unverified>`. The structural
/// `entries: Vec<EntryMeta>` field lives on the outer `Apk<S>`
/// struct, shared across every state.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UnverifiedData {
    /// Raw bodies captured during streaming for downstream
    /// verify / parse to consume.
    pub captured: CapturedBodies,
}

/// Runtime payload for `Apk<SignatureVerified>`. The structural
/// `entries: Vec<EntryMeta>` field lives on the outer `Apk<S>`
/// struct, shared across every state; per-state `Data` only
/// carries the fields that *change* with the state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureVerifiedData {
    /// Raw `AndroidManifest.xml` bytes (still pre-decode).
    pub manifest_bytes: Option<Vec<u8>>,
    /// Raw `resources.arsc` bytes (still pre-decode).
    pub resources_bytes: Option<Vec<u8>>,
    /// Verified signing-block view.
    pub signature_block: SignatureBlock,
}

/// Runtime payload for `Apk<FullyParsed<V>>`. The phantom `V` on
/// the marker side guarantees the static signature-variant witness;
/// the runtime `signature_block.variant_tag` is cross-bound to
/// `V::TAG` by the `parse_v*` transition before this struct is
/// constructed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullyParsedData {
    /// Verified signing-block view.
    pub signature_block: SignatureBlock,
    /// Decoded manifest view (raw AXML bytes with magic confirmed
    /// at construction; P1.9 will replace the byte slice with a
    /// structured representation).
    pub manifest: Manifest,
    /// Decoded resources view (raw ARSC bytes with magic confirmed
    /// at construction).
    pub resources: Resources,
}

// ---------------------------------------------------------------------
// Format probes
// ---------------------------------------------------------------------
//
// P1.8's verify and parse transitions ground in real on-disk magic.
// They are *not* full cryptographic verifiers — that's P1.10's job —
// but they reject inputs that obviously aren't a JAR signature, an
// AXML manifest, or an ARSC table. The format constants below are
// the aapt2 / android.content.res constants from AOSP.

/// First chunk type of a binary AndroidManifest.xml — `RES_XML_TYPE`
/// in `android/util/Resource.h`.
pub(crate) const AXML_CHUNK_TYPE: u16 = 0x0003;
/// First chunk type of a `resources.arsc` table — `RES_TABLE_TYPE`.
pub(crate) const ARSC_CHUNK_TYPE: u16 = 0x0002;

/// PKCS#7 SignedData blocks (`META-INF/<key>.RSA` / `.DSA` / `.EC`)
/// are DER-encoded and start with an ASN.1 SEQUENCE tag (`0x30`).
/// This isn't a substitute for parsing the certificate chain —
/// that's P1.10 — but it rejects obviously-not-DER bytes.
pub(crate) const DER_SEQUENCE_TAG: u8 = 0x30;

/// Quick AXML magic probe — first chunk type is `RES_XML_TYPE` and
/// the buffer is at least one chunk header long (8 bytes).
pub(crate) const fn looks_like_axml(bytes: &[u8]) -> bool {
    bytes.len() >= 8 && u16::from_le_bytes([bytes[0], bytes[1]]) == AXML_CHUNK_TYPE
}

/// Quick ARSC magic probe — first chunk type is `RES_TABLE_TYPE`.
pub(crate) const fn looks_like_arsc(bytes: &[u8]) -> bool {
    bytes.len() >= 8 && u16::from_le_bytes([bytes[0], bytes[1]]) == ARSC_CHUNK_TYPE
}

// ---------------------------------------------------------------------
// Capture pipeline (shared by `apk.rs` sync + `apk_async.rs`)
// ---------------------------------------------------------------------

/// Per-entry inflate cap — 64 MiB. APKs in the wild are well below
/// this for the entries the wrapper captures (manifest + arsc +
/// META-INF/*.RSA all sit in the 100 KiB range).
pub(crate) const MAX_INFLATE_BYTES: usize = 64 * 1024 * 1024;
/// Total inflate budget for one constructor call — 256 MiB.
pub(crate) const MAX_INFLATE_BUDGET: usize = 256 * 1024 * 1024;

const COMPRESSION_STORED: u16 = 0;
const COMPRESSION_DEFLATE: u16 = 8;

/// Per-streaming-event capture target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CaptureSlot {
    SigningCarrier,
    Manifest,
    Resources,
}

/// Classify a file-name into a capture slot, if any.
pub(crate) fn classify_for_capture(file_name: &[u8]) -> Option<CaptureSlot> {
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

/// Decompress raw DEFLATE bytes (no zlib wrapper).
pub(crate) fn inflate_raw(deflated: &[u8], expected_size: u32) -> Result<Vec<u8>, ApkError> {
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

/// End-of-entry handler. Inflates DEFLATE entries; passes STORED
/// through; rejects other methods. Enforces the aggregate inflate
/// budget against `inflate_used`.
pub(crate) fn persist_capture(
    slot: CaptureSlot,
    raw: Vec<u8>,
    compression_method: u16,
    uncompressed_size: u32,
    captured: &mut CapturedBodies,
    inflate_used: &mut usize,
) -> Result<(), ApkError> {
    let bytes = match compression_method {
        COMPRESSION_STORED => raw,
        COMPRESSION_DEFLATE => {
            let projected = inflate_used.saturating_add(uncompressed_size as usize);
            if projected > MAX_INFLATE_BUDGET {
                return Err(ApkError::ManifestDecode(
                    "aggregate inflate would exceed MAX_INFLATE_BUDGET",
                ));
            }
            let out = inflate_raw(&raw, uncompressed_size)?;
            *inflate_used = inflate_used.saturating_add(out.len());
            out
        }
        _ => {
            return Err(ApkError::ManifestDecode(
                "captured entry uses an unrecognised ZIP compression method",
            ));
        }
    };
    match slot {
        CaptureSlot::SigningCarrier => captured.signing_carriers.push(bytes),
        CaptureSlot::Manifest => captured.manifest = Some(bytes),
        CaptureSlot::Resources => captured.resources = Some(bytes),
    }
    Ok(())
}

/// Quick DER probe — the buffer starts with an ASN.1 SEQUENCE tag
/// (`0x30`) followed by a length field that fits within the buffer.
/// Real PKCS#7 SignedData carriers are always tens of KB, so we
/// also reject obviously-too-small buffers.
pub(crate) fn looks_like_pkcs7_der(bytes: &[u8]) -> bool {
    if bytes.len() < 16 || bytes[0] != DER_SEQUENCE_TAG {
        return false;
    }
    // ASN.1 length encoding: short form (length < 128) is a single
    // byte; long form has the high bit set on the first byte and
    // the remaining bits give the number of subsequent length
    // bytes. We accept either if it doesn't overflow the buffer.
    match bytes[1] {
        n if n < 0x80 => bytes.len() >= 2 + n as usize,
        n => {
            let len_bytes = (n & 0x7f) as usize;
            if len_bytes == 0 || len_bytes > 4 || bytes.len() < 2 + len_bytes {
                return false;
            }
            let mut declared = 0usize;
            for &b in &bytes[2..2 + len_bytes] {
                declared = (declared << 8) | b as usize;
            }
            bytes.len() >= 2 + len_bytes + declared
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axml_magic_probe() {
        assert!(looks_like_axml(&[0x03, 0x00, 0x08, 0x00, 0, 0, 0, 0]));
        assert!(!looks_like_axml(&[0x02, 0x00, 0x0c, 0x00, 0, 0, 0, 0]));
        assert!(!looks_like_axml(b"abc"));
    }

    #[test]
    fn arsc_magic_probe() {
        assert!(looks_like_arsc(&[0x02, 0x00, 0x0c, 0x00, 0, 0, 0, 0]));
        assert!(!looks_like_arsc(&[0x03, 0x00, 0x08, 0x00, 0, 0, 0, 0]));
    }

    #[test]
    fn pkcs7_der_probe_accepts_short_form_sequence() {
        // 0x30 0x10 (length 16) + 16 bytes of body
        let mut buf = vec![0x30, 0x10];
        buf.extend(std::iter::repeat_n(0xab, 16));
        assert!(looks_like_pkcs7_der(&buf));
    }

    #[test]
    fn pkcs7_der_probe_accepts_long_form_sequence() {
        // 0x30 0x82 0x10 0x00 + 4096 bytes of body
        let mut buf = vec![0x30, 0x82, 0x10, 0x00];
        buf.extend(std::iter::repeat_n(0xab, 4096));
        assert!(looks_like_pkcs7_der(&buf));
    }

    #[test]
    fn pkcs7_der_probe_rejects_short_buffer() {
        assert!(!looks_like_pkcs7_der(&[0x30, 0x10, 0x00, 0x00]));
    }

    #[test]
    fn pkcs7_der_probe_rejects_wrong_tag() {
        let mut buf = vec![0x31, 0x10];
        buf.extend(std::iter::repeat_n(0, 16));
        assert!(!looks_like_pkcs7_der(&buf));
    }
}
