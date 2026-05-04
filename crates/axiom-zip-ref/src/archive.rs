// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Whole-archive ZIP parser.
//!
//! Connects the LFH, CDR, and EOCD parsers into a single
//! `parse_archive` driver that proves cross-record consistency.
//! Mirrors `theorems/Apkaxiom/Zip/Consistency.lean::parseArchive`
//! byte-for-byte.
//!
//! The driver:
//!
//!   1. Locates the EOCD via [`crate::eocd::find_eocd`].
//!   2. Validates that the central directory (`cdOffset` + `cdSize`)
//!      lies within the byte stream.
//!   3. Parses the CDR sequence in the central-directory region.
//!   4. Asserts the parsed CDR count matches the EOCD's
//!      `total_entries`.
//!   5. For every CDR: validates `lfh_offset` is in-bounds, parses the
//!      LFH there, and asserts the CDR's `file_name` byte sequence
//!      equals the LFH's `file_name`.
//!
//! Any deviation rejects with one of the eight [`ArchiveError`]
//! variants. Tag bytes match the Lean
//! `Apkaxiom.Zip.Consistency.ArchiveError.tag` enumeration so the
//! differential harness can compare across languages numerically.

use crate::{cdr, eocd, lfh};

/// Successfully parsed archive. The `cdrs` and `lfhs` lists are
/// paired by index: `lfhs[i]` is the LFH referenced by
/// `cdrs[i].lfh_offset`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Archive {
    /// Parsed CDR records, in CD-order.
    pub cdrs: Vec<cdr::Cdr>,
    /// Parsed LFHs, paired with `cdrs` by index.
    pub lfhs: Vec<lfh::Lfh>,
    /// EOCD record.
    pub eocd: eocd::Eocd,
}

/// Whole-archive parse failures.
///
/// Tag bytes match the Lean `ArchiveError.tag` enumeration (1..=8).
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum ArchiveError {
    /// No EOCD record located in the byte stream.
    #[error("noEocd")]
    NoEocd,
    /// Bytes at the located EOCD offset failed to parse as an EOCD.
    #[error("eocdInvalid")]
    EocdInvalid,
    /// `cd_offset + cd_size` exceeds the byte-stream length.
    #[error("cdOutOfRange")]
    CdOutOfRange,
    /// A CDR record inside the central-directory region failed to
    /// parse.
    #[error("cdrInvalid")]
    CdrInvalid,
    /// Parsed CDR count differs from EOCD's `total_entries`.
    #[error("cdrCountMismatch")]
    CdrCountMismatch,
    /// A CDR's `lfh_offset` (or its 30-byte LFH fixed prefix) runs
    /// past EOF.
    #[error("lfhOffsetOob")]
    LfhOffsetOob,
    /// Bytes at a CDR's `lfh_offset` failed to parse as an LFH.
    #[error("lfhInvalid")]
    LfhInvalid,
    /// CDR's `file_name` differs from the referenced LFH's
    /// `file_name`.
    #[error("filenameMismatch")]
    FilenameMismatch,
    /// CDR's structural fields disagree with the referenced LFH's.
    /// Checked: `crc32`, `compressed_size`, `uncompressed_size`,
    /// `compression_method`. APPNOTE.TXT requires byte-identity on
    /// these fields between the two records; BadPack-class evasions
    /// commonly smuggle a mismatch past filename-only checkers.
    #[error("fieldMismatch")]
    FieldMismatch,
}

impl ArchiveError {
    /// Cross-language tag byte. Mirrors the Lean
    /// `Apkaxiom.Zip.Consistency.ArchiveError.tag`.
    #[must_use]
    pub const fn tag(&self) -> u8 {
        match self {
            Self::NoEocd => 1,
            Self::EocdInvalid => 2,
            Self::CdOutOfRange => 3,
            Self::CdrInvalid => 4,
            Self::CdrCountMismatch => 5,
            Self::LfhOffsetOob => 6,
            Self::LfhInvalid => 7,
            Self::FilenameMismatch => 8,
            Self::FieldMismatch => 9,
        }
    }
}

/// Bitmask for the APPNOTE.TXT §4.4.4 "data descriptor present" flag.
///
/// General-purpose bit 3. When set on the LFH, the LFH's `crc32`,
/// `compressed_size`, `uncompressed_size` are zero and the real
/// values trail in a data-descriptor record after the file body.
pub const GPB_DATA_DESCRIPTOR_MASK: u16 = 0x0008;

/// Whether the LFH's general-flag bit 3 is set.
const fn lfh_has_data_descriptor(lfh_record: &lfh::Lfh) -> bool {
    (lfh_record.general_flags & GPB_DATA_DESCRIPTOR_MASK) != 0
}

/// Structural-field equality between a CDR and its referenced LFH.
///
/// Two cases (mirrors `Apkaxiom.Zip.Consistency.cdrLfhFieldsAgree`):
///
///   1. **No data descriptor** (LFH bit 3 unset): `crc32` /
///      `compressed_size` / `uncompressed_size` / `compression_method`
///      must be byte-identical between CDR and LFH (APPNOTE.TXT §4.4).
///
///   2. **Data descriptor present** (LFH bit 3 set): the LFH's
///      `crc32` / `compressed_size` / `uncompressed_size` are
///      *defined to be zero*; the CDR carries the canonical values.
///      `compression_method` must still agree.
const fn cdr_lfh_fields_agree(cdr_record: &cdr::Cdr, lfh_record: &lfh::Lfh) -> bool {
    if lfh_has_data_descriptor(lfh_record) {
        // DD branch: LFH fields must be zero, compression method must agree.
        lfh_record.crc32 == 0
            && lfh_record.compressed_size == 0
            && lfh_record.uncompressed_size == 0
            && cdr_record.compression_method == lfh_record.compression_method
    } else {
        // Strict-equality branch (the common case for APKs).
        cdr_record.crc32 == lfh_record.crc32
            && cdr_record.compressed_size == lfh_record.compressed_size
            && cdr_record.uncompressed_size == lfh_record.uncompressed_size
            && cdr_record.compression_method == lfh_record.compression_method
    }
}

/// Whole-archive driver. Mirrors
/// `Apkaxiom.Zip.Consistency.parseArchive`.
///
/// # Errors
/// Returns one of [`ArchiveError`]'s variants on the first integrity
/// failure. The error category matches the Lean theorem statements
/// at `Apkaxiom.Zip.Consistency.badpack_*_rejected`.
pub fn parse_archive(bs: &[u8]) -> Result<Archive, ArchiveError> {
    // (1) Locate the EOCD.
    let eocd_off = eocd::find_eocd(bs).ok_or(ArchiveError::NoEocd)?;
    // (2) Parse the EOCD record.
    let (eocd_record, _) =
        eocd::parse_eocd(&bs[eocd_off..]).map_err(|_| ArchiveError::EocdInvalid)?;
    // (3) Validate cdOffset + cdSize are in-bounds.
    let cd_start = eocd_record.cd_offset as usize;
    let cd_size = eocd_record.cd_size as usize;
    let cd_end = cd_start
        .checked_add(cd_size)
        .ok_or(ArchiveError::CdOutOfRange)?;
    if cd_end > bs.len() {
        return Err(ArchiveError::CdOutOfRange);
    }
    // (4) Parse the CDR sequence.
    let cdrs =
        cdr::parse_cdr_sequence(&bs[cd_start..cd_end]).map_err(|_| ArchiveError::CdrInvalid)?;
    // (5) The CDR count must match `total_entries`.
    if cdrs.len() != eocd_record.total_entries as usize {
        return Err(ArchiveError::CdrCountMismatch);
    }
    // (6) Per-CDR consistency check.
    let mut lfhs = Vec::with_capacity(cdrs.len());
    for cdr_record in &cdrs {
        let lo = cdr_record.lfh_offset as usize;
        let lo_end = lo
            .checked_add(lfh::FIXED_SIZE)
            .ok_or(ArchiveError::LfhOffsetOob)?;
        if lo_end > bs.len() {
            return Err(ArchiveError::LfhOffsetOob);
        }
        let (lfh_record, _) = lfh::parse_lfh(&bs[lo..]).map_err(|_| ArchiveError::LfhInvalid)?;
        if cdr_record.file_name != lfh_record.file_name {
            return Err(ArchiveError::FilenameMismatch);
        }
        if !cdr_lfh_fields_agree(cdr_record, &lfh_record) {
            return Err(ArchiveError::FieldMismatch);
        }
        lfhs.push(lfh_record);
    }
    Ok(Archive {
        cdrs,
        lfhs,
        eocd: eocd_record,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cdr::minimal_cdr;
    use crate::lfh::SIGNATURE as LFH_SIG;

    /// Minimal well-formed archive.
    ///
    /// 1 LFH (30 bytes, offset 0), 1 CDR (46 bytes, offset 30), 1 EOCD
    /// (22 bytes, offset 76). Total 98 bytes. Mirrors
    /// `Apkaxiom.Zip.Consistency.minimalArchiveBytes`.
    fn minimal_archive() -> Vec<u8> {
        let mut v = Vec::with_capacity(98);
        // LFH at offset 0
        v.extend_from_slice(&LFH_SIG.to_le_bytes());
        v.extend_from_slice(&[0x14, 0x00]); // versionNeeded = 20
        v.extend_from_slice(&[0x00; 2]); // generalFlags
        v.extend_from_slice(&[0x00; 2]); // compressionMethod
        v.extend_from_slice(&[0x00; 4]); // lastMod time/date
        v.extend_from_slice(&[0x00; 4]); // crc32
        v.extend_from_slice(&[0x00; 4]); // compressedSize
        v.extend_from_slice(&[0x00; 4]); // uncompressedSize
        v.extend_from_slice(&[0x00; 2]); // nameLen
        v.extend_from_slice(&[0x00; 2]); // extraLen
        debug_assert_eq!(v.len(), 30);
        // CDR at offset 30
        v.extend_from_slice(&minimal_cdr());
        debug_assert_eq!(v.len(), 76);
        // EOCD at offset 76
        v.extend_from_slice(&eocd::SIGNATURE.to_le_bytes());
        v.extend_from_slice(&[0x00; 4]); // diskNumber + cdStartDisk
        v.extend_from_slice(&[0x01, 0x00]); // entriesOnThisDisk = 1
        v.extend_from_slice(&[0x01, 0x00]); // totalEntries = 1
        v.extend_from_slice(&46u32.to_le_bytes()); // cdSize = 46
        v.extend_from_slice(&30u32.to_le_bytes()); // cdOffset = 30
        v.extend_from_slice(&[0x00, 0x00]); // commentLen = 0
        debug_assert_eq!(v.len(), 98);
        v
    }

    #[test]
    fn minimal_archive_parses() {
        let bytes = minimal_archive();
        let archive = parse_archive(&bytes).unwrap();
        assert_eq!(archive.cdrs.len(), 1);
        assert_eq!(archive.lfhs.len(), 1);
        assert_eq!(archive.eocd.total_entries, 1);
        assert_eq!(archive.eocd.cd_offset, 30);
        assert_eq!(archive.eocd.cd_size, 46);
    }

    #[test]
    fn no_eocd_rejected() {
        // Just a minimal LFH, no EOCD trailer.
        let mut bytes = Vec::with_capacity(30);
        bytes.extend_from_slice(&LFH_SIG.to_le_bytes());
        bytes.extend_from_slice(&[0x00; 26]);
        assert_eq!(parse_archive(&bytes), Err(ArchiveError::NoEocd));
    }

    #[test]
    fn cd_out_of_range_rejected() {
        let mut bytes = minimal_archive();
        // Patch EOCD's cdOffset (offset 76+16 = 92) to 0xff_ff_ff_ff.
        let cd_offset_pos = 76 + 16;
        bytes[cd_offset_pos..cd_offset_pos + 4].copy_from_slice(&0xffff_ffffu32.to_le_bytes());
        assert_eq!(parse_archive(&bytes), Err(ArchiveError::CdOutOfRange));
    }

    #[test]
    fn cdr_count_mismatch_rejected() {
        let mut bytes = minimal_archive();
        // Patch EOCD's totalEntries (offset 76+10 = 86) to 2.
        let te_pos = 76 + 10;
        bytes[te_pos..te_pos + 2].copy_from_slice(&2u16.to_le_bytes());
        // Also patch entriesOnThisDisk to keep them consistent.
        let eotd_pos = 76 + 8;
        bytes[eotd_pos..eotd_pos + 2].copy_from_slice(&2u16.to_le_bytes());
        assert_eq!(parse_archive(&bytes), Err(ArchiveError::CdrCountMismatch));
    }

    #[test]
    fn lfh_offset_oob_rejected() {
        let mut bytes = minimal_archive();
        // Patch CDR's lfhOffset (offset 30+42 = 72) to 0xff_ff_ff_ff.
        let lo_pos = 30 + 42;
        bytes[lo_pos..lo_pos + 4].copy_from_slice(&0xffff_ffffu32.to_le_bytes());
        assert_eq!(parse_archive(&bytes), Err(ArchiveError::LfhOffsetOob));
    }

    #[test]
    fn lfh_magic_mismatch_rejected() {
        let mut bytes = minimal_archive();
        // Patch CDR's lfhOffset to 1, so the LFH parser sees bytes
        // starting one byte into the LFH (no magic).
        let lo_pos = 30 + 42;
        bytes[lo_pos..lo_pos + 4].copy_from_slice(&1u32.to_le_bytes());
        assert_eq!(parse_archive(&bytes), Err(ArchiveError::LfhInvalid));
    }

    #[test]
    fn filename_mismatch_rejected() {
        // Construct an archive where the CDR claims nameLen=1 with
        // filename "A" but the LFH at lfh_offset=0 has nameLen=0.
        let mut v = Vec::new();
        // LFH (nameLen = 0)
        v.extend_from_slice(&LFH_SIG.to_le_bytes());
        v.extend_from_slice(&[0x14, 0x00]);
        v.extend_from_slice(&[0x00; 2]);
        v.extend_from_slice(&[0x00; 2]);
        v.extend_from_slice(&[0x00; 4]);
        v.extend_from_slice(&[0x00; 4]);
        v.extend_from_slice(&[0x00; 4]);
        v.extend_from_slice(&[0x00; 4]);
        v.extend_from_slice(&[0x00; 2]); // nameLen = 0
        v.extend_from_slice(&[0x00; 2]);
        debug_assert_eq!(v.len(), 30);
        // CDR (nameLen = 1, filename = "A", lfh_offset = 0)
        v.extend_from_slice(&cdr::SIGNATURE.to_le_bytes());
        v.extend_from_slice(&[0x14, 0x00, 0x14, 0x00]);
        v.extend_from_slice(&[0x00; 8]);
        v.extend_from_slice(&[0x00; 4]); // crc32
        v.extend_from_slice(&[0x00; 4]); // compressedSize
        v.extend_from_slice(&[0x00; 4]); // uncompressedSize
        v.extend_from_slice(&[0x01, 0x00]); // nameLen = 1
        v.extend_from_slice(&[0x00, 0x00]); // extraLen
        v.extend_from_slice(&[0x00, 0x00]); // commentLen
        v.extend_from_slice(&[0x00, 0x00]); // diskNumberStart
        v.extend_from_slice(&[0x00, 0x00]); // internalAttrs
        v.extend_from_slice(&[0x00; 4]); // externalAttrs
        v.extend_from_slice(&[0x00; 4]); // lfhOffset
        v.push(0x41); // filename "A"
        debug_assert_eq!(v.len(), 30 + 47);
        // EOCD (cdSize = 47, cdOffset = 30, totalEntries = 1)
        v.extend_from_slice(&eocd::SIGNATURE.to_le_bytes());
        v.extend_from_slice(&[0x00; 4]); // diskNumber + cdStartDisk
        v.extend_from_slice(&[0x01, 0x00]);
        v.extend_from_slice(&[0x01, 0x00]);
        v.extend_from_slice(&47u32.to_le_bytes());
        v.extend_from_slice(&30u32.to_le_bytes());
        v.extend_from_slice(&[0x00, 0x00]);
        assert_eq!(parse_archive(&v), Err(ArchiveError::FilenameMismatch));
    }

    #[test]
    fn tag_bytes_match_lean() {
        assert_eq!(ArchiveError::NoEocd.tag(), 1);
        assert_eq!(ArchiveError::EocdInvalid.tag(), 2);
        assert_eq!(ArchiveError::CdOutOfRange.tag(), 3);
        assert_eq!(ArchiveError::CdrInvalid.tag(), 4);
        assert_eq!(ArchiveError::CdrCountMismatch.tag(), 5);
        assert_eq!(ArchiveError::LfhOffsetOob.tag(), 6);
        assert_eq!(ArchiveError::LfhInvalid.tag(), 7);
        assert_eq!(ArchiveError::FilenameMismatch.tag(), 8);
        assert_eq!(ArchiveError::FieldMismatch.tag(), 9);
    }

    #[test]
    fn field_mismatch_rejected() {
        // Patch the CDR's crc32 (offset 30+16 = 46) to non-zero while
        // the LFH's crc32 stays zero.
        let mut bytes = minimal_archive();
        let crc_pos = 30 + 16;
        bytes[crc_pos..crc_pos + 4].copy_from_slice(&0xdead_beefu32.to_le_bytes());
        assert_eq!(parse_archive(&bytes), Err(ArchiveError::FieldMismatch));
    }

    #[test]
    fn data_descriptor_branch_accepts_zero_lfh_fields_with_nonzero_cdr() {
        // Set LFH + CDR general-flag bit 3 (DD mode) and patch the CDR's
        // crc32 to a non-zero value. The LFH keeps zero crc32 / sizes
        // (per APPNOTE.TXT §4.4.4) — field-set check must accept.
        let mut bytes = minimal_archive();
        bytes[6] = 0x08;
        bytes[7] = 0x00;
        bytes[30 + 8] = 0x08;
        bytes[30 + 9] = 0x00;
        bytes[46..50].copy_from_slice(&0xdead_beefu32.to_le_bytes());
        assert!(parse_archive(&bytes).is_ok());
    }

    #[test]
    fn data_descriptor_branch_rejects_nonzero_lfh_fields() {
        // DD flag set, but LFH's crc32 is non-zero — violates
        // APPNOTE.TXT (LFH must be zero in DD mode). Reject with
        // FieldMismatch.
        let mut bytes = minimal_archive();
        bytes[6] = 0x08;
        bytes[7] = 0x00;
        bytes[30 + 8] = 0x08;
        bytes[30 + 9] = 0x00;
        bytes[14..18].copy_from_slice(&0xdead_beefu32.to_le_bytes());
        assert_eq!(parse_archive(&bytes), Err(ArchiveError::FieldMismatch));
    }
}
