// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Internal structure of the v2 / v3 / v3.1 APK signing-scheme
//! blocks.
//!
//! Each block is a length-prefixed sequence of signers. A
//! signer carries:
//!
//! ```text
//!   signer (length-prefixed):
//!     signed_data (length-prefixed)
//!     [v3/v3.1 only: min_sdk u32, max_sdk u32]
//!     signatures (length-prefixed sequence of length-prefixed
//!                 signature elements)
//!     public_key (length-prefixed bytes — SPKI DER)
//! ```
//!
//! `signed_data` walks as:
//!
//! ```text
//!   signed_data:
//!     digests (length-prefixed sequence of length-prefixed
//!              digest elements)
//!     certificates (length-prefixed sequence of length-prefixed
//!                   X.509 DER certificates)
//!     [v3/v3.1 only: min_sdk u32, max_sdk u32]
//!     additional_attributes (length-prefixed sequence of
//!                            length-prefixed (id u32, bytes value))
//! ```
//!
//! Digest element: `algorithm_id u32 || length-prefixed bytes digest`.
//! Signature element: `algorithm_id u32 || length-prefixed bytes signature`.
//!
//! Signature-algorithm IDs (per AOSP `tools/apksig`):
//!
//! | ID       | Algorithm                                    |
//! |----------|----------------------------------------------|
//! | `0x0101` | RSA-PSS+SHA-256, 1MB-chunked SHA-256 digest  |
//! | `0x0102` | RSA-PSS+SHA-512, 1MB-chunked SHA-512 digest  |
//! | `0x0103` | RSA-PKCS1-v1.5+SHA-256, chunked SHA-256      |
//! | `0x0104` | RSA-PKCS1-v1.5+SHA-512, chunked SHA-512      |
//! | `0x0201` | ECDSA+SHA-256, chunked SHA-256                |
//! | `0x0202` | ECDSA+SHA-512, chunked SHA-512                |
//! | `0x0301` | DSA+SHA-256, chunked SHA-256                  |
//! | `0x0421` | RSA-PKCS1-v1.5+SHA-256 over Verity tree root |
//! | `0x0423` | ECDSA+SHA-256 over Verity tree root           |
//! | `0x0425` | DSA+SHA-256 over Verity tree root             |

#![allow(
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::similar_names,
    clippy::len_without_is_empty,
    clippy::cast_possible_truncation,
    clippy::cast_lossless
)]

/// Signature-algorithm ID — `algorithm_id` field in v2/v3 entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum SignatureAlgorithmId {
    /// `0x0101` — RSA-PSS+SHA-256, 1 MiB-chunked SHA-256 digest.
    RsaPssSha256 = 0x0101,
    /// `0x0102` — RSA-PSS+SHA-512, 1 MiB-chunked SHA-512 digest.
    RsaPssSha512 = 0x0102,
    /// `0x0103` — RSA-PKCS1-v1.5+SHA-256, chunked SHA-256.
    RsaPkcs1Sha256 = 0x0103,
    /// `0x0104` — RSA-PKCS1-v1.5+SHA-512, chunked SHA-512.
    RsaPkcs1Sha512 = 0x0104,
    /// `0x0201` — ECDSA+SHA-256, chunked SHA-256.
    EcdsaSha256 = 0x0201,
    /// `0x0202` — ECDSA+SHA-512, chunked SHA-512.
    EcdsaSha512 = 0x0202,
    /// `0x0301` — DSA+SHA-256, chunked SHA-256.
    DsaSha256 = 0x0301,
    /// `0x0421` — RSA-PKCS1+SHA-256 over Verity tree root.
    VerityRsaPkcs1Sha256 = 0x0421,
    /// `0x0423` — ECDSA+SHA-256 over Verity tree root.
    VerityEcdsaSha256 = 0x0423,
    /// `0x0425` — DSA+SHA-256 over Verity tree root.
    VerityDsaSha256 = 0x0425,
}

impl SignatureAlgorithmId {
    /// Lift a wire ID to a known algorithm. Returns `None` for
    /// IDs the spec hasn't assigned (the parser surfaces them as
    /// raw u32 in [`UnknownAlgorithmEntry`] so consumers don't
    /// silently drop signatures).
    #[must_use]
    pub const fn from_u32(id: u32) -> Option<Self> {
        match id {
            0x0101 => Some(Self::RsaPssSha256),
            0x0102 => Some(Self::RsaPssSha512),
            0x0103 => Some(Self::RsaPkcs1Sha256),
            0x0104 => Some(Self::RsaPkcs1Sha512),
            0x0201 => Some(Self::EcdsaSha256),
            0x0202 => Some(Self::EcdsaSha512),
            0x0301 => Some(Self::DsaSha256),
            0x0421 => Some(Self::VerityRsaPkcs1Sha256),
            0x0423 => Some(Self::VerityEcdsaSha256),
            0x0425 => Some(Self::VerityDsaSha256),
            _ => None,
        }
    }

    /// Wire ID of this algorithm.
    #[must_use]
    pub const fn to_u32(self) -> u32 {
        self as u32
    }

    /// Underlying digest algorithm.
    #[must_use]
    pub const fn digest_kind(self) -> DigestKind {
        match self {
            Self::RsaPssSha256
            | Self::RsaPkcs1Sha256
            | Self::EcdsaSha256
            | Self::DsaSha256
            | Self::VerityRsaPkcs1Sha256
            | Self::VerityEcdsaSha256
            | Self::VerityDsaSha256 => DigestKind::Sha256,
            Self::RsaPssSha512 | Self::RsaPkcs1Sha512 | Self::EcdsaSha512 => DigestKind::Sha512,
        }
    }

    /// True iff this algorithm uses the Verity tree-root variant
    /// (chunked SHA over Verity hash tree, not the standard 1 MiB
    /// chunked digest).
    #[must_use]
    pub const fn is_verity(self) -> bool {
        matches!(
            self,
            Self::VerityRsaPkcs1Sha256 | Self::VerityEcdsaSha256 | Self::VerityDsaSha256
        )
    }
}

/// Digest kind used by the chunked-digest computation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DigestKind {
    /// 32-byte SHA-256 digest.
    Sha256,
    /// 64-byte SHA-512 digest.
    Sha512,
}

impl DigestKind {
    /// Output length in bytes.
    #[must_use]
    pub const fn len(self) -> usize {
        match self {
            Self::Sha256 => 32,
            Self::Sha512 => 64,
        }
    }
}

/// One signer inside a v2/v3/v3.1 block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signer {
    /// Verbatim `signed_data` bytes — the SHA over THESE bytes is
    /// what each signature algorithm signs.
    pub signed_data: Vec<u8>,
    /// Digests declared by this signer (one per algorithm).
    pub digests: Vec<DigestEntry>,
    /// X.509 certificate chain — first cert is the leaf.
    pub certificates: Vec<Vec<u8>>,
    /// Additional attributes (id u32, value bytes).
    pub additional_attributes: Vec<AttributeEntry>,
    /// Signatures declared by this signer (one per algorithm).
    pub signatures: Vec<SignatureEntry>,
    /// Subject Public Key Info DER — the leaf cert's public key.
    pub public_key: Vec<u8>,
    /// `(min_sdk, max_sdk)` — present only for v3 / v3.1 signers,
    /// `None` for v2.
    pub sdk_range: Option<(u32, u32)>,
}

/// One entry of the digests sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigestEntry {
    /// Wire ID of the digest's signing algorithm.
    pub algorithm_id: u32,
    /// Looked-up algorithm; `None` for unknown IDs.
    pub algorithm: Option<SignatureAlgorithmId>,
    /// Verbatim digest bytes.
    pub digest: Vec<u8>,
}

/// One entry of the signatures sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureEntry {
    /// Wire ID of the signature's algorithm.
    pub algorithm_id: u32,
    /// Looked-up algorithm; `None` for unknown IDs.
    pub algorithm: Option<SignatureAlgorithmId>,
    /// Verbatim signature bytes.
    pub signature: Vec<u8>,
}

/// One additional-attribute entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeEntry {
    /// Attribute ID.
    pub id: u32,
    /// Attribute bytes (interpretation depends on `id`).
    pub value: Vec<u8>,
}

/// Variant tag for a v2/v3/v3.1 block (which scheme parsed it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemeVariant {
    /// v2 — `0x7109871a`. No SDK range fields.
    V2,
    /// v3 — `0xf05368c0`. Signers carry SDK range; signed_data
    /// also carries SDK range (must match signer's range).
    V3,
    /// v3.1 — `0x1b93ad61`. Same shape as v3.
    V3_1,
}

/// Errors returned by [`parse_block`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SchemeError {
    /// A length-prefix declared more bytes than remained in the
    /// containing slice.
    #[error("length-prefix overflow at offset {at}: declared {declared}, remaining {remaining}")]
    LengthOverflow {
        /// Offset where the overflow was detected.
        at: usize,
        /// Declared length.
        declared: u64,
        /// Bytes remaining in the slice.
        remaining: usize,
    },
    /// A length-prefix is missing (slice too short).
    #[error("truncated length-prefix at offset {at}")]
    Truncated {
        /// Offset of the missing prefix.
        at: usize,
    },
    /// The signers sequence is empty — every block must contain
    /// at least one signer.
    #[error("zero signers in {variant:?} block")]
    NoSigners {
        /// Variant tag.
        variant: SchemeVariant,
    },
    /// `min_sdk`/`max_sdk` mismatch between signer envelope and
    /// signed_data envelope (v3 / v3.1).
    #[error("v3 SDK range mismatch: signer({s_min},{s_max}) != signed_data({d_min},{d_max})")]
    V3SdkRangeMismatch {
        /// Signer-level min.
        s_min: u32,
        /// Signer-level max.
        s_max: u32,
        /// signed_data-level min.
        d_min: u32,
        /// signed_data-level max.
        d_max: u32,
    },
}

/// Parse a v2 block.
pub fn parse_v2(block: &[u8]) -> Result<Vec<Signer>, SchemeError> {
    parse_block(block, SchemeVariant::V2)
}

/// Parse a v3 block.
pub fn parse_v3(block: &[u8]) -> Result<Vec<Signer>, SchemeError> {
    parse_block(block, SchemeVariant::V3)
}

/// Parse a v3.1 block. Same wire shape as v3.
pub fn parse_v3_1(block: &[u8]) -> Result<Vec<Signer>, SchemeError> {
    parse_block(block, SchemeVariant::V3_1)
}

/// Walk a v2/v3/v3.1 block. Layout difference is captured by the
/// `variant` parameter.
pub fn parse_block(block: &[u8], variant: SchemeVariant) -> Result<Vec<Signer>, SchemeError> {
    let signers_seq = take_lp_slice(block, 0)?.0;
    let mut signers = Vec::new();
    let mut off = 0;
    while off < signers_seq.len() {
        let (signer_buf, next) = take_lp_slice(signers_seq, off)?;
        signers.push(parse_signer(signer_buf, variant)?);
        off = next;
    }
    if signers.is_empty() {
        return Err(SchemeError::NoSigners { variant });
    }
    Ok(signers)
}

fn parse_signer(signer: &[u8], variant: SchemeVariant) -> Result<Signer, SchemeError> {
    let (signed_data_slice, mut p) = take_lp_slice(signer, 0)?;
    let signed_data = signed_data_slice.to_vec();

    // v3/v3.1 carry (min_sdk, max_sdk) at the signer envelope
    // BEFORE signatures. v2 jumps straight to signatures.
    let sdk_range_signer = if matches!(variant, SchemeVariant::V3 | SchemeVariant::V3_1) {
        let s_min = take_u32(signer, p)?;
        let s_max = take_u32(signer, p + 4)?;
        p += 8;
        Some((s_min, s_max))
    } else {
        None
    };

    let (sigs_seq, np) = take_lp_slice(signer, p)?;
    p = np;
    let (pk_slice, _) = take_lp_slice(signer, p)?;
    let public_key = pk_slice.to_vec();

    // Walk signed_data
    let (digs_seq, dp) = take_lp_slice(signed_data_slice, 0)?;
    let (certs_seq, dp2) = take_lp_slice(signed_data_slice, dp)?;
    let mut dp_after_certs = dp2;
    let sdk_range_signed = if matches!(variant, SchemeVariant::V3 | SchemeVariant::V3_1) {
        let d_min = take_u32(signed_data_slice, dp_after_certs)?;
        let d_max = take_u32(signed_data_slice, dp_after_certs + 4)?;
        dp_after_certs += 8;
        Some((d_min, d_max))
    } else {
        None
    };
    let (attrs_seq, _) = take_lp_slice(signed_data_slice, dp_after_certs)?;

    if let (Some((s_min, s_max)), Some((d_min, d_max))) = (sdk_range_signer, sdk_range_signed) {
        if s_min != d_min || s_max != d_max {
            return Err(SchemeError::V3SdkRangeMismatch {
                s_min,
                s_max,
                d_min,
                d_max,
            });
        }
    }

    let digests = parse_digest_seq(digs_seq)?;
    let certificates = parse_lp_lp_seq(certs_seq)?;
    let additional_attributes = parse_attribute_seq(attrs_seq)?;
    let signatures = parse_signature_seq(sigs_seq)?;

    Ok(Signer {
        signed_data,
        digests,
        certificates,
        additional_attributes,
        signatures,
        public_key,
        sdk_range: sdk_range_signer,
    })
}

fn parse_digest_seq(seq: &[u8]) -> Result<Vec<DigestEntry>, SchemeError> {
    let mut out = Vec::new();
    let mut off = 0;
    while off < seq.len() {
        let (elt, next) = take_lp_slice(seq, off)?;
        let algorithm_id = take_u32(elt, 0)?;
        let (digest_slice, _) = take_lp_slice(elt, 4)?;
        out.push(DigestEntry {
            algorithm_id,
            algorithm: SignatureAlgorithmId::from_u32(algorithm_id),
            digest: digest_slice.to_vec(),
        });
        off = next;
    }
    Ok(out)
}

fn parse_signature_seq(seq: &[u8]) -> Result<Vec<SignatureEntry>, SchemeError> {
    let mut out = Vec::new();
    let mut off = 0;
    while off < seq.len() {
        let (elt, next) = take_lp_slice(seq, off)?;
        let algorithm_id = take_u32(elt, 0)?;
        let (sig_slice, _) = take_lp_slice(elt, 4)?;
        out.push(SignatureEntry {
            algorithm_id,
            algorithm: SignatureAlgorithmId::from_u32(algorithm_id),
            signature: sig_slice.to_vec(),
        });
        off = next;
    }
    Ok(out)
}

fn parse_attribute_seq(seq: &[u8]) -> Result<Vec<AttributeEntry>, SchemeError> {
    let mut out = Vec::new();
    let mut off = 0;
    while off < seq.len() {
        let (elt, next) = take_lp_slice(seq, off)?;
        let id = take_u32(elt, 0)?;
        out.push(AttributeEntry {
            id,
            value: elt[4..].to_vec(),
        });
        off = next;
    }
    Ok(out)
}

fn parse_lp_lp_seq(seq: &[u8]) -> Result<Vec<Vec<u8>>, SchemeError> {
    let mut out = Vec::new();
    let mut off = 0;
    while off < seq.len() {
        let (elt, next) = take_lp_slice(seq, off)?;
        out.push(elt.to_vec());
        off = next;
    }
    Ok(out)
}

fn take_u32(buf: &[u8], off: usize) -> Result<u32, SchemeError> {
    if off + 4 > buf.len() {
        return Err(SchemeError::Truncated { at: off });
    }
    Ok(u32::from_le_bytes(
        buf[off..off + 4].try_into().expect("4 bytes"),
    ))
}

/// Read a length-prefixed slice at `off`. Returns `(slice, next_off)`
/// where `next_off` is the offset of the byte just past the slice.
fn take_lp_slice(buf: &[u8], off: usize) -> Result<(&[u8], usize), SchemeError> {
    let n = take_u32(buf, off)? as u64;
    let start = off + 4;
    let end = (start as u64)
        .checked_add(n)
        .ok_or(SchemeError::LengthOverflow {
            at: off,
            declared: n,
            remaining: buf.len() - start,
        })?;
    if end > buf.len() as u64 {
        return Err(SchemeError::LengthOverflow {
            at: off,
            declared: n,
            remaining: buf.len() - start,
        });
    }
    let end = end as usize;
    Ok((&buf[start..end], end))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_fixture(rel: &str) -> Vec<u8> {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join(rel);
        std::fs::read(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
    }

    #[test]
    fn parses_v2_signer_from_real_apk() {
        let bytes = read_fixture("corpus/signing/v1-v2/wifiautoff-v1v2.apk");
        let block = crate::locate(&bytes).unwrap().unwrap();
        let v2 = block.v2().expect("v2 entry");
        let signers = parse_v2(v2).expect("parse v2");
        assert_eq!(signers.len(), 1);
        let s = &signers[0];
        assert!(!s.digests.is_empty(), "signer must have digests");
        assert_eq!(
            s.digests[0].algorithm,
            Some(SignatureAlgorithmId::RsaPkcs1Sha256)
        );
        assert_eq!(s.digests[0].digest.len(), 32);
        assert_eq!(s.certificates.len(), 1, "F-Droid v2 fixture: 1 cert");
        assert!(!s.certificates[0].is_empty());
        assert_eq!(s.signatures.len(), 1);
        assert_eq!(
            s.signatures[0].algorithm,
            Some(SignatureAlgorithmId::RsaPkcs1Sha256)
        );
        assert_eq!(
            s.signatures[0].signature.len(),
            256,
            "RSA-2048 → 256 byte sig"
        );
        assert!(!s.public_key.is_empty(), "public key SPKI must be present");
        assert_eq!(s.sdk_range, None, "v2 has no SDK range");
    }

    #[test]
    fn parses_v3_signer_from_real_apk() {
        let bytes = read_fixture("corpus/signing/v1-v2-v3/wifiautoff-v1v2v3.apk");
        let block = crate::locate(&bytes).unwrap().unwrap();
        let v3 = block.v3().expect("v3 entry");
        let signers = parse_v3(v3).expect("parse v3");
        assert_eq!(signers.len(), 1);
        let s = &signers[0];
        assert!(s.sdk_range.is_some(), "v3 must carry SDK range");
        let (s_min, s_max) = s.sdk_range.unwrap();
        assert!(s_min <= s_max, "SDK range malformed: {s_min} > {s_max}");
    }

    #[test]
    fn algorithm_id_round_trip() {
        for raw in [
            0x0101u32, 0x0102, 0x0103, 0x0104, 0x0201, 0x0202, 0x0301, 0x0421, 0x0423, 0x0425,
        ] {
            let alg = SignatureAlgorithmId::from_u32(raw).expect("known id");
            assert_eq!(alg.to_u32(), raw, "round-trip");
        }
    }

    #[test]
    fn unknown_algorithm_id_surfaces_none() {
        assert!(SignatureAlgorithmId::from_u32(0xdead_beef).is_none());
    }

    #[test]
    fn rejects_truncated_block() {
        // Truncated input: 3 bytes only.
        let bs = [0u8, 1, 2];
        assert!(matches!(parse_v2(&bs), Err(SchemeError::Truncated { .. })));
    }

    #[test]
    fn rejects_overflowing_length() {
        // u32 length = 0xffff_ffff but slice is short.
        let bs = [0xff_u8, 0xff, 0xff, 0xff, 0x01, 0x02];
        assert!(matches!(
            parse_v2(&bs),
            Err(SchemeError::LengthOverflow { .. })
        ));
    }
}
