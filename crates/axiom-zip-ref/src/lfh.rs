// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! ZIP local file header (LFH) parser. Layout per APPNOTE.TXT 6.3.10
//! §4.3.7 — see `theorems/Apkaxiom/Zip/LocalHeader.lean` for the
//! Lean reflection.

/// Magic bytes at the start of every LFH ("PK\x03\x04").
pub const SIGNATURE: u32 = 0x0403_4b50;

/// Fixed-size portion of the LFH: 30 bytes.
pub const FIXED_SIZE: usize = 30;

/// Parsed LFH structure. Field names mirror APPNOTE.TXT.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lfh {
    /// Version of the ZIP spec needed to extract.
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
    /// File name.
    pub file_name: Vec<u8>,
    /// Extra-field bytes.
    pub extra_field: Vec<u8>,
}

/// Parse failure modes. Tag bytes match the Lean `ParseError.tag`
/// enumeration — the differential harness compares numerically.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseError {
    /// Input shorter than the 30-byte fixed prefix.
    #[error("shortHeader")]
    ShortHeader,
    /// Magic bytes are not `0x04034b50`.
    #[error("badSignature")]
    BadSignature,
    /// Filename region runs past EOF.
    #[error("shortName")]
    ShortName,
    /// Extra-field region runs past EOF.
    #[error("shortExtra")]
    ShortExtra,
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
        }
    }
}

/// Result of a successful parse: the structured record plus the
/// number of bytes consumed (so callers can resume on the next
/// entry).
pub type ParseOk = (Lfh, usize);

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

/// Reference parser. Mirrors `theorems/Apkaxiom/Zip/LocalHeader.lean`
/// `parseLfh` byte-for-byte.
///
/// # Errors
/// Returns one of [`ParseError`]'s variants if the input is too
/// short, has the wrong magic, or its declared filename / extra-
/// field regions run past EOF.
pub fn parse_lfh(bs: &[u8]) -> Result<ParseOk, ParseError> {
    if bs.len() < FIXED_SIZE {
        return Err(ParseError::ShortHeader);
    }
    let sig = read_u32(bs, 0).ok_or(ParseError::ShortHeader)?;
    if sig != SIGNATURE {
        return Err(ParseError::BadSignature);
    }
    let version_needed = read_u16(bs, 4).ok_or(ParseError::ShortHeader)?;
    let general_flags = read_u16(bs, 6).ok_or(ParseError::ShortHeader)?;
    let compression_method = read_u16(bs, 8).ok_or(ParseError::ShortHeader)?;
    let last_mod_time = read_u16(bs, 10).ok_or(ParseError::ShortHeader)?;
    let last_mod_date = read_u16(bs, 12).ok_or(ParseError::ShortHeader)?;
    let crc32 = read_u32(bs, 14).ok_or(ParseError::ShortHeader)?;
    let compressed_size = read_u32(bs, 18).ok_or(ParseError::ShortHeader)?;
    let uncompressed_size = read_u32(bs, 22).ok_or(ParseError::ShortHeader)?;
    let name_len = read_u16(bs, 26).ok_or(ParseError::ShortHeader)?;
    let extra_len = read_u16(bs, 28).ok_or(ParseError::ShortHeader)?;
    let file_name = slice_at(bs, 30, name_len as usize).ok_or(ParseError::ShortName)?;
    let extra_field =
        slice_at(bs, 30 + name_len as usize, extra_len as usize).ok_or(ParseError::ShortExtra)?;
    let lfh = Lfh {
        version_needed,
        general_flags,
        compression_method,
        last_mod_time,
        last_mod_date,
        crc32,
        compressed_size,
        uncompressed_size,
        file_name,
        extra_field,
    };
    let consumed = FIXED_SIZE + name_len as usize + extra_len as usize;
    Ok((lfh, consumed))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_lfh() -> Vec<u8> {
        let mut v = Vec::with_capacity(FIXED_SIZE);
        v.extend_from_slice(&SIGNATURE.to_le_bytes()); // signature
        v.extend_from_slice(&[0x14, 0x00]); // versionNeeded = 20
        v.extend_from_slice(&[0x00; 2]); // generalFlags
        v.extend_from_slice(&[0x00; 2]); // compressionMethod
        v.extend_from_slice(&[0x00; 4]); // lastMod time/date
        v.extend_from_slice(&[0x00; 4]); // crc32
        v.extend_from_slice(&[0x00; 4]); // compressedSize
        v.extend_from_slice(&[0x00; 4]); // uncompressedSize
        v.extend_from_slice(&[0x00; 2]); // nameLen
        v.extend_from_slice(&[0x00; 2]); // extraLen
        v
    }

    #[test]
    fn empty_input_short_header() {
        assert_eq!(parse_lfh(&[]), Err(ParseError::ShortHeader));
    }

    #[test]
    fn three_bytes_short_header() {
        assert_eq!(parse_lfh(&[0x50, 0x4b, 0x03]), Err(ParseError::ShortHeader));
    }

    #[test]
    fn minimal_lfh_parses() {
        let bytes = minimal_lfh();
        let (lfh, n) = parse_lfh(&bytes).unwrap();
        assert_eq!(n, FIXED_SIZE);
        assert_eq!(lfh.version_needed, 20);
        assert!(lfh.file_name.is_empty());
        assert!(lfh.extra_field.is_empty());
    }

    #[test]
    fn wrong_magic_bad_signature() {
        let mut bytes = minimal_lfh();
        bytes[0] = 0xff;
        bytes[1] = 0xff;
        bytes[2] = 0xff;
        bytes[3] = 0xff;
        assert_eq!(parse_lfh(&bytes), Err(ParseError::BadSignature));
    }

    #[test]
    fn nonzero_filename_parses() {
        let mut bytes = minimal_lfh();
        // nameLen = 5
        bytes[26] = 5;
        bytes[27] = 0;
        bytes.extend_from_slice(b"hello");
        let (lfh, n) = parse_lfh(&bytes).unwrap();
        assert_eq!(n, FIXED_SIZE + 5);
        assert_eq!(lfh.file_name, b"hello");
    }

    #[test]
    fn name_runs_past_eof() {
        let mut bytes = minimal_lfh();
        bytes[26] = 100; // declared 100-byte name, but no body bytes
        bytes[27] = 0;
        assert_eq!(parse_lfh(&bytes), Err(ParseError::ShortName));
    }

    #[test]
    fn extra_runs_past_eof() {
        let mut bytes = minimal_lfh();
        bytes[28] = 10; // declared 10-byte extra, but no body bytes
        bytes[29] = 0;
        assert_eq!(parse_lfh(&bytes), Err(ParseError::ShortExtra));
    }

    #[test]
    fn tag_bytes_match_lean() {
        // These constants are the contract with
        // theorems/Apkaxiom/Zip/LocalHeader.lean's ParseError.tag.
        assert_eq!(ParseError::ShortHeader.tag(), 1);
        assert_eq!(ParseError::BadSignature.tag(), 2);
        assert_eq!(ParseError::ShortName.tag(), 3);
        assert_eq!(ParseError::ShortExtra.tag(), 4);
    }
}
