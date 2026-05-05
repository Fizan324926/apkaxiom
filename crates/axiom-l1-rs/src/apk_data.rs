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

// SigVariant import dropped — the per-variant APK Signing Block ID
// constants (AOSP V2_BLOCK_ID = 0x7109_871a, V3 = 0xf057_41b3) are
// not yet load-bearing here; P1.10's real verifier will read them
// once it parses the block. Keeping them as live `pub(crate)`
// constants invites drift, so we defer until they are actually
// consumed.

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

/// Bytes that make up the verified APK Signing Block.
///
/// A real APK Signing Block (APPNOTE-extension §APK Signing Block
/// v2/v3) has the on-disk shape:
///
/// ```text
///   u64 size_of_block
///   ID-value pairs (signature scheme records)
///   u64 size_of_block (repeated for backward compatibility)
///   "APK Sig Block 42"   (16-byte magic, ASCII)
/// ```
///
/// P1.8 verifies the trailer magic + variant-marker ID-value pair;
/// the structured certificate-chain breakdown lands in P1.10
/// alongside the actual cryptographic verifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureBlock {
    /// Variant tag (matches `SigVariant::TAG`).
    pub variant_tag: u8,
    /// Raw signing-block bytes (everything between the LFH and
    /// the central directory's tail-pointer to the EOCD).
    pub block_bytes: Vec<u8>,
}

// ---------------------------------------------------------------------
// Per-state runtime payloads
// ---------------------------------------------------------------------

/// Raw bodies captured during streaming for downstream verify /
/// parse to consume. `Apk<Unverified>` is the only state that
/// holds the full triple — once a transition consumes one of
/// them, the field is moved out and dropped from the active state.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CapturedBodies {
    /// Raw bytes of the entry that carried the APK Signing Block.
    /// Captured for any `META-INF/<token>.RSA` / `.DSA` / `.EC`
    /// entry. None until verify_v* runs.
    pub signing_block: Option<Vec<u8>>,
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
