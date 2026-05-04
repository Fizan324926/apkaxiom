// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! ZIP central directory record (CDR) parser. Layout per APPNOTE.TXT
//! 6.3.10 §4.3.12 — see `theorems/Apkaxiom/Zip/CentralDirectory.lean`
//! for the Lean reflection.

/// Magic bytes at the start of every CDR ("PK\x01\x02").
pub const SIGNATURE: u32 = 0x0201_4b50;

/// Fixed-size portion of the CDR: 46 bytes.
pub const FIXED_SIZE: usize = 46;

/// Parsed CDR structure. Field names mirror APPNOTE.TXT.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cdr {
    /// Tool / OS version that made the archive.
    pub version_made_by: u16,
    /// Tool version needed to extract.
    pub version_needed: u16,
    /// General-purpose bit flag.
    pub general_flags: u16,
    /// Compression method (0 = stored, 8 = deflate, …).
    pub compression_method: u16,
    /// Last-modification time (DOS format).
    pub last_mod_time: u16,
    /// Last-modification date (DOS format).
    pub last_mod_date: u16,
    /// CRC-32 of the uncompressed data.
    pub crc32: u32,
    /// Size of the compressed data.
    pub compressed_size: u32,
    /// Size of the uncompressed data.
    pub uncompressed_size: u32,
    /// Disk number this entry's data starts on (always 0 for
    /// single-volume APKs).
    pub disk_number_start: u16,
    /// Internal-file-attributes bitfield.
    pub internal_file_attributes: u16,
    /// External-file-attributes bitfield (Unix mode bits in the top
    /// half on Unix-made archives).
    pub external_file_attributes: u32,
    /// Relative offset of the local file header for this entry, from
    /// the beginning of the byte stream.
    pub lfh_offset: u32,
    /// File name.
    pub file_name: Vec<u8>,
    /// Extra-field bytes.
    pub extra_field: Vec<u8>,
    /// File-comment bytes.
    pub file_comment: Vec<u8>,
}

/// Parse failure modes.
///
/// Tag bytes match the Lean
/// `Apkaxiom.Zip.CentralDirectory.ParseError.tag` enumeration — the
/// differential harness compares numerically. Five variants vs
/// LFH/EOCD's four because the CDR has three variable-length regions.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseError {
    /// Input shorter than the 46-byte fixed prefix.
    #[error("shortHeader")]
    ShortHeader,
    /// Magic bytes are not `0x02014b50`.
    #[error("badSignature")]
    BadSignature,
    /// Filename region runs past EOF.
    #[error("shortName")]
    ShortName,
    /// Extra-field region runs past EOF.
    #[error("shortExtra")]
    ShortExtra,
    /// File-comment region runs past EOF.
    #[error("shortComment")]
    ShortComment,
}

impl ParseError {
    /// Cross-language tag byte. Mirrors the Lean `ParseError.tag`.
    #[must_use]
    pub const fn tag(&self) -> u8 {
        match self {
            Self::ShortHeader => 1,
            Self::BadSignature => 2,
            Self::ShortName => 3,
            Self::ShortExtra => 4,
            Self::ShortComment => 5,
        }
    }
}

/// Result of a successful parse: the structured record plus the
/// number of bytes consumed (so callers can resume on the next
/// entry).
pub type ParseOk = (Cdr, usize);

/// Read a little-endian `u16` from `bs[o..o+2]`.
fn read_u16(bs: &[u8], o: usize) -> Option<u16> {
    let bytes = bs.get(o..o + 2)?;
    let arr: [u8; 2] = bytes.try_into().ok()?;
    Some(u16::from_le_bytes(arr))
}

/// Read a little-endian `u32` from `bs[o..o+4]`.
fn read_u32(bs: &[u8], o: usize) -> Option<u32> {
    let bytes = bs.get(o..o + 4)?;
    let arr: [u8; 4] = bytes.try_into().ok()?;
    Some(u32::from_le_bytes(arr))
}

/// Slice `bs[o..o+len]` to a fresh `Vec<u8>`.
fn slice_at(bs: &[u8], o: usize, len: usize) -> Option<Vec<u8>> {
    Some(bs.get(o..o + len)?.to_vec())
}

/// Reference parser. Mirrors
/// `theorems/Apkaxiom/Zip/CentralDirectory.lean::parseCdr`
/// byte-for-byte.
///
/// # Errors
/// Returns one of [`ParseError`]'s variants if the input is too
/// short, has the wrong magic, or any of its three declared
/// variable-length regions runs past EOF.
pub fn parse_cdr(bs: &[u8]) -> Result<ParseOk, ParseError> {
    if bs.len() < FIXED_SIZE {
        return Err(ParseError::ShortHeader);
    }
    let sig = read_u32(bs, 0).ok_or(ParseError::ShortHeader)?;
    if sig != SIGNATURE {
        return Err(ParseError::BadSignature);
    }
    let version_made_by = read_u16(bs, 4).ok_or(ParseError::ShortHeader)?;
    let version_needed = read_u16(bs, 6).ok_or(ParseError::ShortHeader)?;
    let general_flags = read_u16(bs, 8).ok_or(ParseError::ShortHeader)?;
    let compression_method = read_u16(bs, 10).ok_or(ParseError::ShortHeader)?;
    let last_mod_time = read_u16(bs, 12).ok_or(ParseError::ShortHeader)?;
    let last_mod_date = read_u16(bs, 14).ok_or(ParseError::ShortHeader)?;
    let crc32 = read_u32(bs, 16).ok_or(ParseError::ShortHeader)?;
    let compressed_size = read_u32(bs, 20).ok_or(ParseError::ShortHeader)?;
    let uncompressed_size = read_u32(bs, 24).ok_or(ParseError::ShortHeader)?;
    let name_len = read_u16(bs, 28).ok_or(ParseError::ShortHeader)?;
    let extra_len = read_u16(bs, 30).ok_or(ParseError::ShortHeader)?;
    let comment_len = read_u16(bs, 32).ok_or(ParseError::ShortHeader)?;
    let disk_number_start = read_u16(bs, 34).ok_or(ParseError::ShortHeader)?;
    let internal_file_attributes = read_u16(bs, 36).ok_or(ParseError::ShortHeader)?;
    let external_file_attributes = read_u32(bs, 38).ok_or(ParseError::ShortHeader)?;
    let lfh_offset = read_u32(bs, 42).ok_or(ParseError::ShortHeader)?;
    let file_name = slice_at(bs, FIXED_SIZE, name_len as usize).ok_or(ParseError::ShortName)?;
    let extra_field = slice_at(bs, FIXED_SIZE + name_len as usize, extra_len as usize)
        .ok_or(ParseError::ShortExtra)?;
    let file_comment = slice_at(
        bs,
        FIXED_SIZE + name_len as usize + extra_len as usize,
        comment_len as usize,
    )
    .ok_or(ParseError::ShortComment)?;
    let cdr = Cdr {
        version_made_by,
        version_needed,
        general_flags,
        compression_method,
        last_mod_time,
        last_mod_date,
        crc32,
        compressed_size,
        uncompressed_size,
        disk_number_start,
        internal_file_attributes,
        external_file_attributes,
        lfh_offset,
        file_name,
        extra_field,
        file_comment,
    };
    let consumed = FIXED_SIZE + name_len as usize + extra_len as usize + comment_len as usize;
    Ok((cdr, consumed))
}

/// Walk a contiguous byte slice (typically the central-directory
/// region carved out by the EOCD's `cdOffset` + `cdSize`) and parse
/// one CDR after another. Stops on the first error.
///
/// # Errors
/// Returns the first per-record [`ParseError`] encountered.
pub fn parse_cdr_sequence(cd_bytes: &[u8]) -> Result<Vec<Cdr>, ParseError> {
    let mut out = Vec::new();
    let mut off = 0usize;
    while off < cd_bytes.len() {
        let (cdr, n) = parse_cdr(&cd_bytes[off..])?;
        if n == 0 {
            // Defensive: parse_cdr always consumes ≥ 46 bytes on
            // success, so this branch is unreachable in practice.
            out.push(cdr);
            break;
        }
        out.push(cdr);
        off += n;
    }
    Ok(out)
}

/// Test-only helper: build the canonical "minimal CDR" byte sequence
/// (zero filename / extra / comment, all attribute fields zero,
/// `lfh_offset = 0`). Used both by this module's unit tests and by
/// the `archive` module's tests.
#[cfg(test)]
pub(crate) fn minimal_cdr() -> Vec<u8> {
    let mut v = Vec::with_capacity(FIXED_SIZE);
    v.extend_from_slice(&SIGNATURE.to_le_bytes()); // signature
    v.extend_from_slice(&[0x14, 0x00]); // versionMadeBy
    v.extend_from_slice(&[0x14, 0x00]); // versionNeeded
    v.extend_from_slice(&[0x00; 2]); // generalFlags
    v.extend_from_slice(&[0x00; 2]); // compressionMethod
    v.extend_from_slice(&[0x00; 4]); // lastMod time/date
    v.extend_from_slice(&[0x00; 4]); // crc32
    v.extend_from_slice(&[0x00; 4]); // compressedSize
    v.extend_from_slice(&[0x00; 4]); // uncompressedSize
    v.extend_from_slice(&[0x00; 2]); // nameLen
    v.extend_from_slice(&[0x00; 2]); // extraLen
    v.extend_from_slice(&[0x00; 2]); // commentLen
    v.extend_from_slice(&[0x00; 2]); // diskNumberStart
    v.extend_from_slice(&[0x00; 2]); // internalFileAttributes
    v.extend_from_slice(&[0x00; 4]); // externalFileAttributes
    v.extend_from_slice(&[0x00; 4]); // lfhOffset
    debug_assert_eq!(v.len(), FIXED_SIZE);
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_short_header() {
        assert_eq!(parse_cdr(&[]), Err(ParseError::ShortHeader));
    }

    #[test]
    fn three_bytes_short_header() {
        assert_eq!(parse_cdr(&[0x50, 0x4b, 0x01]), Err(ParseError::ShortHeader));
    }

    #[test]
    fn forty_five_bytes_short_header() {
        let bytes = vec![0u8; 45];
        assert_eq!(parse_cdr(&bytes), Err(ParseError::ShortHeader));
    }

    #[test]
    fn minimal_cdr_parses() {
        let bytes = minimal_cdr();
        let (cdr, n) = parse_cdr(&bytes).unwrap();
        assert_eq!(n, FIXED_SIZE);
        assert_eq!(cdr.version_made_by, 20);
        assert_eq!(cdr.version_needed, 20);
        assert_eq!(cdr.lfh_offset, 0);
        assert!(cdr.file_name.is_empty());
        assert!(cdr.extra_field.is_empty());
        assert!(cdr.file_comment.is_empty());
    }

    #[test]
    fn wrong_magic_bad_signature() {
        let mut bytes = minimal_cdr();
        bytes[0] = 0xff;
        bytes[1] = 0xff;
        bytes[2] = 0xff;
        bytes[3] = 0xff;
        assert_eq!(parse_cdr(&bytes), Err(ParseError::BadSignature));
    }

    #[test]
    fn nonzero_filename_parses() {
        let mut bytes = minimal_cdr();
        // nameLen = 5
        bytes[28] = 5;
        bytes[29] = 0;
        bytes.extend_from_slice(b"hello");
        let (cdr, n) = parse_cdr(&bytes).unwrap();
        assert_eq!(n, FIXED_SIZE + 5);
        assert_eq!(cdr.file_name, b"hello");
    }

    #[test]
    fn name_runs_past_eof() {
        let mut bytes = minimal_cdr();
        bytes[28] = 100;
        bytes[29] = 0;
        assert_eq!(parse_cdr(&bytes), Err(ParseError::ShortName));
    }

    #[test]
    fn extra_runs_past_eof() {
        let mut bytes = minimal_cdr();
        bytes[30] = 10;
        bytes[31] = 0;
        assert_eq!(parse_cdr(&bytes), Err(ParseError::ShortExtra));
    }

    #[test]
    fn comment_runs_past_eof() {
        let mut bytes = minimal_cdr();
        bytes[32] = 10;
        bytes[33] = 0;
        assert_eq!(parse_cdr(&bytes), Err(ParseError::ShortComment));
    }

    #[test]
    fn region_priority_name_first() {
        // nameLen=1, extraLen=1, commentLen=1 — all three would flunk
        // but ShortName is reported first.
        let mut bytes = minimal_cdr();
        bytes[28] = 1; // nameLen
        bytes[30] = 1; // extraLen
        bytes[32] = 1; // commentLen
        assert_eq!(parse_cdr(&bytes), Err(ParseError::ShortName));
    }

    #[test]
    fn region_priority_extra_before_comment() {
        // nameLen=0, extraLen=1, commentLen=1 — both would flunk but
        // ShortExtra is reported first.
        let mut bytes = minimal_cdr();
        bytes[30] = 1; // extraLen
        bytes[32] = 1; // commentLen
        assert_eq!(parse_cdr(&bytes), Err(ParseError::ShortExtra));
    }

    #[test]
    fn lfh_offset_decoded_as_u32_le() {
        let mut bytes = minimal_cdr();
        // lfhOffset = 0x12_34_56_78 at offset 42..46
        bytes[42] = 0x78;
        bytes[43] = 0x56;
        bytes[44] = 0x34;
        bytes[45] = 0x12;
        let (cdr, _) = parse_cdr(&bytes).unwrap();
        assert_eq!(cdr.lfh_offset, 0x1234_5678);
    }

    #[test]
    fn tag_bytes_match_lean() {
        assert_eq!(ParseError::ShortHeader.tag(), 1);
        assert_eq!(ParseError::BadSignature.tag(), 2);
        assert_eq!(ParseError::ShortName.tag(), 3);
        assert_eq!(ParseError::ShortExtra.tag(), 4);
        assert_eq!(ParseError::ShortComment.tag(), 5);
    }

    #[test]
    fn sequence_parses_two_back_to_back() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&minimal_cdr());
        bytes.extend_from_slice(&minimal_cdr());
        let cdrs = parse_cdr_sequence(&bytes).unwrap();
        assert_eq!(cdrs.len(), 2);
    }

    #[test]
    fn sequence_rejects_garbage_tail() {
        let mut bytes = minimal_cdr();
        bytes.extend_from_slice(&[0x00, 0x01, 0x02]); // garbage
                                                      // The garbage tail starts a new parse attempt that fails with
                                                      // ShortHeader (only 3 bytes left).
        assert_eq!(parse_cdr_sequence(&bytes), Err(ParseError::ShortHeader));
    }
}
