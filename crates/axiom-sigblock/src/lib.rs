// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `axiom-sigblock` — APK signing-block parser.
//!
//! This crate is the byte-format reference for the APK Signing
//! Block — the region between the last LFH body and the central
//! directory of any v2/v3/v3.1-signed APK. The block layout is:
//!
//! ```text
//!   [u64 size_of_block (LE, includes the trailing size + magic)]
//!   [pairs ...]
//!     each pair:
//!       [u64 length (LE) — bytes of (id + value)]
//!       [u32 id     (LE)]
//!       [length - 4 bytes value]
//!   [u64 size_of_block (LE) — must equal the leading size]
//!   [16 bytes magic = b"APK Sig Block 42"]
//! ```
//!
//! The block sits immediately before the central directory; the
//! EOCD's `cd_offset` points at the byte AFTER the magic. Locate
//! by:
//!
//!   1. Find EOCD signature; read `cd_offset`.
//!   2. Read 16 bytes at `cd_offset - 16` — must be the magic.
//!   3. Read `u64` at `cd_offset - 24` — size_of_block.
//!   4. Block starts at `cd_offset - size_of_block - 8`.
//!
//! Known block IDs (per AOSP `tools/apksig`):
//!
//! | ID            | Name                        |
//! |---------------|-----------------------------|
//! | `0x7109871a`  | APK Signature Scheme **v2** |
//! | `0xf05368c0`  | APK Signature Scheme **v3** |
//! | `0x1b93ad61`  | APK Signature Scheme **v3.1** |
//! | `0x6dff800d`  | AOSP zero-padding (block alignment) |
//! | `0x2b09189e`  | Source Stamp v1             |
//! | `0x42726577`  | Source Stamp v2             |
//!
//! Anything else is unknown — the parser surfaces it as
//! [`SignatureBlockEntry::Unknown`] so consumers don't silently
//! drop it.

#![forbid(unsafe_code)]
#![warn(missing_docs, unreachable_pub)]
#![allow(
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::missing_const_for_fn,
    clippy::cast_possible_truncation
)]

pub mod scheme;

/// 16-byte magic at the tail of every APK signing block.
pub const MAGIC: &[u8; 16] = b"APK Sig Block 42";

/// Block ID for APK Signature Scheme v2.
pub const ID_V2: u32 = 0x7109_871a;
/// Block ID for APK Signature Scheme v3.
pub const ID_V3: u32 = 0xf053_68c0;
/// Block ID for APK Signature Scheme v3.1 (rotation-aware v3).
pub const ID_V3_1: u32 = 0x1b93_ad61;
/// AOSP zero-padding block (used to pad the signing block to a
/// 4096-byte boundary; the value is all zeros).
pub const ID_PADDING: u32 = 0x6dff_800d;
/// Source Stamp v1 block.
pub const ID_SOURCE_STAMP_V1: u32 = 0x2b09_189e;
/// Source Stamp v2 block.
pub const ID_SOURCE_STAMP_V2: u32 = 0x4272_6577;

/// One ID-tagged entry inside the signing block.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SignatureBlockEntry {
    /// APK Signature Scheme v2 — `0x7109871a`.
    V2(Vec<u8>),
    /// APK Signature Scheme v3 — `0xf05368c0`.
    V3(Vec<u8>),
    /// APK Signature Scheme v3.1 — `0x1b93ad61`.
    V3_1(Vec<u8>),
    /// AOSP zero-padding block — value is all zeros.
    Padding(Vec<u8>),
    /// Source Stamp v1 — `0x2b09189e`.
    SourceStampV1(Vec<u8>),
    /// Source Stamp v2 — `0x42726577`.
    SourceStampV2(Vec<u8>),
    /// An ID we don't recognise. Kept verbatim so consumers can
    /// inspect / re-serialise without drift.
    Unknown {
        /// 4-byte ID, little-endian as stored on disk.
        id: u32,
        /// Verbatim value bytes.
        value: Vec<u8>,
    },
}

impl SignatureBlockEntry {
    /// Wire ID for this entry.
    #[must_use]
    pub const fn id(&self) -> u32 {
        match self {
            Self::V2(_) => ID_V2,
            Self::V3(_) => ID_V3,
            Self::V3_1(_) => ID_V3_1,
            Self::Padding(_) => ID_PADDING,
            Self::SourceStampV1(_) => ID_SOURCE_STAMP_V1,
            Self::SourceStampV2(_) => ID_SOURCE_STAMP_V2,
            Self::Unknown { id, .. } => *id,
        }
    }

    /// Verbatim value bytes for this entry.
    #[must_use]
    pub fn value(&self) -> &[u8] {
        match self {
            Self::V2(v)
            | Self::V3(v)
            | Self::V3_1(v)
            | Self::Padding(v)
            | Self::SourceStampV1(v)
            | Self::SourceStampV2(v) => v,
            Self::Unknown { value, .. } => value,
        }
    }

    fn from_id(id: u32, value: Vec<u8>) -> Self {
        match id {
            ID_V2 => Self::V2(value),
            ID_V3 => Self::V3(value),
            ID_V3_1 => Self::V3_1(value),
            ID_PADDING => Self::Padding(value),
            ID_SOURCE_STAMP_V1 => Self::SourceStampV1(value),
            ID_SOURCE_STAMP_V2 => Self::SourceStampV2(value),
            _ => Self::Unknown { id, value },
        }
    }
}

/// Fully-parsed APK signing block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureBlock {
    /// Entries in source order.
    pub entries: Vec<SignatureBlockEntry>,
    /// Stream-offset of the leading `size_of_block` u64.
    pub block_offset: u64,
    /// Total bytes from the leading u64 through the trailing magic
    /// (= `size_of_block + 8`).
    pub block_total_size: u64,
}

impl SignatureBlock {
    /// True iff the block has any v2/v3/v3.1 entry.
    #[must_use]
    pub fn has_modern_scheme(&self) -> bool {
        self.entries.iter().any(|e| {
            matches!(
                e,
                SignatureBlockEntry::V2(_)
                    | SignatureBlockEntry::V3(_)
                    | SignatureBlockEntry::V3_1(_)
            )
        })
    }

    /// First v2 block, if any.
    #[must_use]
    pub fn v2(&self) -> Option<&[u8]> {
        self.entries.iter().find_map(|e| match e {
            SignatureBlockEntry::V2(v) => Some(v.as_slice()),
            _ => None,
        })
    }
    /// First v3 block, if any.
    #[must_use]
    pub fn v3(&self) -> Option<&[u8]> {
        self.entries.iter().find_map(|e| match e {
            SignatureBlockEntry::V3(v) => Some(v.as_slice()),
            _ => None,
        })
    }
    /// First v3.1 block, if any.
    #[must_use]
    pub fn v3_1(&self) -> Option<&[u8]> {
        self.entries.iter().find_map(|e| match e {
            SignatureBlockEntry::V3_1(v) => Some(v.as_slice()),
            _ => None,
        })
    }
}

/// Parse errors.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    /// EOCD signature not found.
    #[error("EOCD signature not found")]
    NoEocd,
    /// `cd_offset` field of EOCD points beyond the input bytes.
    #[error("cd_offset {cd_offset} exceeds input length {input_len}")]
    InvalidCdOffset {
        /// `cd_offset` from EOCD.
        cd_offset: u64,
        /// Total input bytes.
        input_len: u64,
    },
    /// The 16-byte tail magic is missing or wrong.
    #[error("missing APK signing block magic at offset {at}")]
    BadMagic {
        /// Offset where magic was expected.
        at: u64,
    },
    /// `size_of_block` field is zero or larger than the input.
    #[error("invalid size_of_block = {size}")]
    InvalidSize {
        /// Declared size.
        size: u64,
    },
    /// Leading and trailing `size_of_block` u64s disagree.
    #[error("size_of_block mismatch: leading {leading} ≠ trailing {trailing}")]
    SizeMismatch {
        /// Leading u64.
        leading: u64,
        /// Trailing u64.
        trailing: u64,
    },
    /// Pair length declared larger than remaining block bytes.
    #[error("pair at offset {at} declares length {length} but only {remaining} bytes remain")]
    PairOverflow {
        /// Pair offset within the block.
        at: u64,
        /// Declared pair length.
        length: u64,
        /// Bytes remaining in the pair region.
        remaining: u64,
    },
    /// Pair length is too small to even hold the 4-byte ID.
    #[error("pair at offset {at} length {length} < 4 (must include 4-byte id)")]
    PairTooShort {
        /// Pair offset.
        at: u64,
        /// Declared length.
        length: u64,
    },
    /// Trailing junk at the end of the pair region.
    #[error("trailing junk: {bytes} bytes after last pair, expected 0")]
    TrailingJunk {
        /// Junk byte count.
        bytes: u64,
    },
}

/// Locate the APK signing block in `apk_bytes`. Returns the
/// fully-parsed block, or `Ok(None)` if the APK is unsigned (no
/// magic at the expected offset — perfectly legal for v1-only
/// JAR-signed APKs).
pub fn locate(apk_bytes: &[u8]) -> Result<Option<SignatureBlock>, ParseError> {
    // Find EOCD by scanning backward for its signature.
    let eocd_off = find_eocd(apk_bytes).ok_or(ParseError::NoEocd)?;
    if eocd_off + 22 > apk_bytes.len() {
        return Err(ParseError::NoEocd);
    }
    let cd_offset = u64::from(read_u32(apk_bytes, eocd_off + 16));
    let input_len = apk_bytes.len() as u64;
    if cd_offset > input_len {
        return Err(ParseError::InvalidCdOffset {
            cd_offset,
            input_len,
        });
    }
    if cd_offset < 24 {
        return Ok(None);
    }
    // 16-byte magic at cd_offset - 16.
    let magic_at = cd_offset - 16;
    if &apk_bytes[magic_at as usize..(magic_at as usize + 16)] != MAGIC {
        return Ok(None);
    }
    // Trailing size_of_block at cd_offset - 24.
    let trailing_sob = read_u64(apk_bytes, (cd_offset - 24) as usize);
    if trailing_sob == 0 || trailing_sob + 8 > cd_offset {
        return Err(ParseError::InvalidSize { size: trailing_sob });
    }
    let block_offset = cd_offset - trailing_sob - 8;
    let leading_sob = read_u64(apk_bytes, block_offset as usize);
    if leading_sob != trailing_sob {
        return Err(ParseError::SizeMismatch {
            leading: leading_sob,
            trailing: trailing_sob,
        });
    }
    // Walk pairs in `[block_offset + 8, cd_offset - 24)`.
    let pair_region_start = (block_offset + 8) as usize;
    let pair_region_end = (cd_offset - 24) as usize;
    let pair_region = &apk_bytes[pair_region_start..pair_region_end];
    let entries = parse_pairs(pair_region)?;
    Ok(Some(SignatureBlock {
        entries,
        block_offset,
        block_total_size: trailing_sob + 8,
    }))
}

/// Walk an in-memory pair region and return all entries. The
/// region must contain only complete pairs (no leading or trailing
/// junk); any leftover bytes flag [`ParseError::TrailingJunk`].
pub fn parse_pairs(region: &[u8]) -> Result<Vec<SignatureBlockEntry>, ParseError> {
    let mut entries = Vec::new();
    let mut cur = 0usize;
    let total = region.len();
    while cur < total {
        if cur + 8 > total {
            return Err(ParseError::TrailingJunk {
                bytes: (total - cur) as u64,
            });
        }
        let length = read_u64(region, cur);
        let pair_at = cur as u64;
        if length < 4 {
            return Err(ParseError::PairTooShort {
                at: pair_at,
                length,
            });
        }
        let total_pair_size = 8u64.saturating_add(length);
        let remaining = (total - cur) as u64;
        if total_pair_size > remaining {
            return Err(ParseError::PairOverflow {
                at: pair_at,
                length,
                remaining: remaining - 8,
            });
        }
        let id = read_u32(region, cur + 8);
        let value_start = cur + 12;
        let value_end = cur + 8 + length as usize;
        let value = region[value_start..value_end].to_vec();
        entries.push(SignatureBlockEntry::from_id(id, value));
        cur = value_end;
    }
    Ok(entries)
}

fn find_eocd(bytes: &[u8]) -> Option<usize> {
    if bytes.len() < 22 {
        return None;
    }
    // Scan from the end backward for the EOCD signature.
    let mut i = bytes.len() - 22;
    loop {
        if read_u32(bytes, i) == 0x0605_4b50 {
            return Some(i);
        }
        if i == 0 {
            return None;
        }
        i -= 1;
    }
}

fn read_u32(bytes: &[u8], off: usize) -> u32 {
    u32::from_le_bytes(bytes[off..off + 4].try_into().expect("4 bytes"))
}

fn read_u64(bytes: &[u8], off: usize) -> u64 {
    u64::from_le_bytes(bytes[off..off + 8].try_into().expect("8 bytes"))
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
    fn locates_block_in_v1v2_apk() {
        let bytes = read_fixture("corpus/signing/v1-v2/wifiautoff-v1v2.apk");
        let block = locate(&bytes).unwrap().expect("v1v2 has signing block");
        assert!(block.v2().is_some(), "v2 entry missing");
        assert!(
            block.v3().is_none(),
            "v3 must NOT be present in v1v2 fixture"
        );
        assert!(block.has_modern_scheme());
    }

    #[test]
    fn locates_block_in_v1v2v3_apk() {
        let bytes = read_fixture("corpus/signing/v1-v2-v3/wifiautoff-v1v2v3.apk");
        let block = locate(&bytes).unwrap().expect("v1v2v3 has signing block");
        assert!(block.v2().is_some(), "v2 entry missing");
        assert!(block.v3().is_some(), "v3 entry missing");
    }

    #[test]
    fn no_block_in_v1_only_apk() {
        let bytes = read_fixture("corpus/signing/v1-only/wifiautoff-v1.apk");
        let block = locate(&bytes).unwrap();
        assert!(block.is_none(), "v1-only APK must have no signing block");
    }

    #[test]
    fn no_block_in_unsigned_minimal_input() {
        // A trivial 22-byte EOCD-only input — cd_offset = 0, no signing block.
        let mut eocd = vec![0u8; 22];
        eocd[0..4].copy_from_slice(&0x0605_4b50u32.to_le_bytes());
        let block = locate(&eocd).unwrap();
        assert!(block.is_none());
    }

    #[test]
    fn rejects_truncated_input() {
        // Empty bytes: NoEocd.
        assert!(matches!(locate(&[]), Err(ParseError::NoEocd)));
    }

    #[test]
    fn block_entries_round_trip_id_and_value_lengths() {
        let bytes = read_fixture("corpus/signing/v1-v2-v3/wifiautoff-v1v2v3.apk");
        let block = locate(&bytes).unwrap().unwrap();
        // Sum of (8-byte length + 8 + length-bytes) = 8 + value_len + 4 ID + 8 length = 12 + value_len
        // Block layout: 8 leading + sum(pairs) + 8 trailing + 16 magic = block_total_size
        let pair_total: u64 = block
            .entries
            .iter()
            .map(|e| 8 + 4 + e.value().len() as u64)
            .sum();
        // block_total_size = 8 (leading) + pair_total + 8 (trailing) + 16 (magic)
        // pair_total here counts (8-byte length-field + 4 + value), and the 4+value = "length payload"
        // So block_total_size = 8 + pair_total + 8 + 16
        let expected = 8 + pair_total + 8 + 16;
        assert_eq!(
            block.block_total_size, expected,
            "block_total_size {} != 8 + pairs({pair_total}) + 8 + 16 = {expected}",
            block.block_total_size
        );
    }
}
