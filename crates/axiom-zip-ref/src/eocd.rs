// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! ZIP end-of-central-directory (EOCD) parser. Layout per
//! APPNOTE.TXT 6.3.10 §4.3.16 — see
//! `theorems/Apkaxiom/Zip/Eocd.lean` for the Lean reflection.

/// Magic bytes at the start of every EOCD ("PK\x05\x06").
pub const SIGNATURE: u32 = 0x0605_4b50;

/// Fixed-size portion of the EOCD: 22 bytes.
pub const FIXED_SIZE: usize = 22;

/// Parsed EOCD record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Eocd {
    /// This-disk number. Single-volume archives always carry 0.
    pub disk_number: u16,
    /// Disk number where the central directory begins.
    pub cd_start_disk: u16,
    /// CD entries on this disk.
    pub entries_on_this_disk: u16,
    /// Total CD entries.
    pub total_entries: u16,
    /// Size of the central directory.
    pub cd_size: u32,
    /// Offset of the central directory from start-of-archive.
    pub cd_offset: u32,
    /// Trailing comment.
    pub comment: Vec<u8>,
}

/// Parse failure modes. Tag bytes match the Lean
/// `Apkaxiom.Zip.Eocd.ParseError.tag` enumeration.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseError {
    /// Input shorter than the 22-byte fixed prefix.
    #[error("shortFixed")]
    ShortFixed,
    /// Magic bytes are not `0x06054b50`.
    #[error("badSignature")]
    BadSignature,
    /// Comment region runs past EOF.
    #[error("shortComment")]
    ShortComment,
    /// `disk_number ≠ cd_start_disk` — multi-volume archive (out of
    /// scope for v0.1; ZIP64 multi-volume support tracked under
    /// ADR-0017).
    #[error("inconsistentDisks")]
    InconsistentDisks,
}

impl ParseError {
    /// Cross-language tag byte. Mirrors the Lean enumeration.
    #[must_use]
    pub const fn tag(&self) -> u8 {
        match self {
            Self::ShortFixed => 1,
            Self::BadSignature => 2,
            Self::ShortComment => 3,
            Self::InconsistentDisks => 4,
        }
    }
}

/// Result of a successful parse.
pub type ParseOk = (Eocd, usize);

fn read_u16(bs: &[u8], o: usize) -> Option<u16> {
    let bytes = bs.get(o..o + 2)?;
    let arr: [u8; 2] = bytes.try_into().ok()?;
    Some(u16::from_le_bytes(arr))
}

fn read_u32(bs: &[u8], o: usize) -> Option<u32> {
    let bytes = bs.get(o..o + 4)?;
    let arr: [u8; 4] = bytes.try_into().ok()?;
    Some(u32::from_le_bytes(arr))
}

/// Reference parser. Operates on the byte sequence starting at the
/// EOCD signature; callers locate the EOCD by scanning backwards
/// from EOF (the suffix-locator helper [`find_eocd`] below).
///
/// Mirrors `theorems/Apkaxiom/Zip/Eocd.lean` `parseEocd` byte-for-byte.
///
/// # Errors
/// One of [`ParseError`]'s variants.
pub fn parse_eocd(bs: &[u8]) -> Result<ParseOk, ParseError> {
    if bs.len() < FIXED_SIZE {
        return Err(ParseError::ShortFixed);
    }
    let sig = read_u32(bs, 0).ok_or(ParseError::ShortFixed)?;
    if sig != SIGNATURE {
        return Err(ParseError::BadSignature);
    }
    let disk_number = read_u16(bs, 4).ok_or(ParseError::ShortFixed)?;
    let cd_start_disk = read_u16(bs, 6).ok_or(ParseError::ShortFixed)?;
    let entries_on_this_disk = read_u16(bs, 8).ok_or(ParseError::ShortFixed)?;
    let total_entries = read_u16(bs, 10).ok_or(ParseError::ShortFixed)?;
    let cd_size = read_u32(bs, 12).ok_or(ParseError::ShortFixed)?;
    let cd_offset = read_u32(bs, 16).ok_or(ParseError::ShortFixed)?;
    let comment_len = read_u16(bs, 20).ok_or(ParseError::ShortFixed)?;
    if disk_number != cd_start_disk {
        return Err(ParseError::InconsistentDisks);
    }
    let comment_end = 22 + comment_len as usize;
    if comment_end > bs.len() {
        return Err(ParseError::ShortComment);
    }
    let comment = bs[22..comment_end].to_vec();
    let eocd = Eocd {
        disk_number,
        cd_start_disk,
        entries_on_this_disk,
        total_entries,
        cd_size,
        cd_offset,
        comment,
    };
    Ok((eocd, comment_end))
}

/// Maximum legal comment length (16-bit field).
pub const MAX_COMMENT_LEN: usize = 0xffff;

/// Locate the EOCD by scanning backwards from EOF for the signature.
/// Returns the byte offset of the signature, or `None` if no
/// candidate fits in the trailing `MAX_COMMENT_LEN + FIXED_SIZE`
/// bytes.
///
/// Mirrors `theorems/Apkaxiom/Zip/Eocd.lean` `findEocd`.
#[must_use]
pub fn find_eocd(bs: &[u8]) -> Option<usize> {
    let len = bs.len();
    if len < FIXED_SIZE {
        return None;
    }
    let scan_from = len.saturating_sub(MAX_COMMENT_LEN + FIXED_SIZE);
    (scan_from..=len - FIXED_SIZE)
        .rev()
        .find(|&off| read_u32(bs, off) == Some(SIGNATURE))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_eocd() -> Vec<u8> {
        let mut v = Vec::with_capacity(FIXED_SIZE);
        v.extend_from_slice(&SIGNATURE.to_le_bytes());
        v.extend_from_slice(&[0x00; 2]); // diskNumber
        v.extend_from_slice(&[0x00; 2]); // cdStartDisk
        v.extend_from_slice(&[0x00; 2]); // entriesOnThisDisk
        v.extend_from_slice(&[0x00; 2]); // totalEntries
        v.extend_from_slice(&[0x00; 4]); // cdSize
        v.extend_from_slice(&[0x00; 4]); // cdOffset
        v.extend_from_slice(&[0x00; 2]); // commentLen
        v
    }

    #[test]
    fn empty_input_short_fixed() {
        assert_eq!(parse_eocd(&[]), Err(ParseError::ShortFixed));
    }

    #[test]
    fn three_bytes_short_fixed() {
        assert_eq!(parse_eocd(&[0x50, 0x4b, 0x05]), Err(ParseError::ShortFixed));
    }

    #[test]
    fn minimal_eocd_parses() {
        let bytes = minimal_eocd();
        let (eocd, n) = parse_eocd(&bytes).unwrap();
        assert_eq!(n, FIXED_SIZE);
        assert_eq!(eocd.disk_number, 0);
        assert_eq!(eocd.total_entries, 0);
        assert!(eocd.comment.is_empty());
    }

    #[test]
    fn wrong_magic_bad_signature() {
        let mut bytes = minimal_eocd();
        bytes[0] = 0xff;
        bytes[1] = 0xff;
        bytes[2] = 0xff;
        bytes[3] = 0xff;
        assert_eq!(parse_eocd(&bytes), Err(ParseError::BadSignature));
    }

    #[test]
    fn multi_volume_inconsistent_disks() {
        let mut bytes = minimal_eocd();
        bytes[4] = 1; // diskNumber = 1
        assert_eq!(parse_eocd(&bytes), Err(ParseError::InconsistentDisks));
    }

    #[test]
    fn comment_truncation_short_comment() {
        let mut bytes = minimal_eocd();
        bytes[20] = 100; // commentLen = 100, but no comment body
        bytes[21] = 0;
        assert_eq!(parse_eocd(&bytes), Err(ParseError::ShortComment));
    }

    #[test]
    fn nonzero_comment_parses() {
        let mut bytes = minimal_eocd();
        bytes[20] = 5;
        bytes[21] = 0;
        bytes.extend_from_slice(b"hello");
        let (eocd, n) = parse_eocd(&bytes).unwrap();
        assert_eq!(n, FIXED_SIZE + 5);
        assert_eq!(eocd.comment, b"hello");
    }

    #[test]
    fn find_eocd_at_end_of_minimal() {
        let bytes = minimal_eocd();
        assert_eq!(find_eocd(&bytes), Some(0));
    }

    #[test]
    fn find_eocd_with_prefix_padding() {
        let mut bytes = vec![0u8; 100];
        bytes.extend_from_slice(&minimal_eocd());
        assert_eq!(find_eocd(&bytes), Some(100));
    }

    #[test]
    fn find_eocd_no_signature() {
        let bytes = vec![0u8; 50];
        assert_eq!(find_eocd(&bytes), None);
    }

    #[test]
    fn tag_bytes_match_lean() {
        assert_eq!(ParseError::ShortFixed.tag(), 1);
        assert_eq!(ParseError::BadSignature.tag(), 2);
        assert_eq!(ParseError::ShortComment.tag(), 3);
        assert_eq!(ParseError::InconsistentDisks.tag(), 4);
    }
}
