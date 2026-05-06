// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Binary string-pool decoder shared by the AXML and ARSC parsers.
//!
//! The string pool chunk (type 0x0001) appears at the top of every AXML
//! file and inside each ARSC package block. Its wire format is:
//!
//! ```text
//!   u16 type          = 0x0001
//!   u16 header_size   = 28
//!   u32 chunk_size
//!   u32 string_count
//!   u32 style_count
//!   u32 flags         (bit 8 = UTF-8 mode)
//!   u32 strings_start (byte offset from chunk base to string data)
//!   u32 styles_start
//!   u32[string_count] offsets   (relative to strings_start)
//!   ... string data ...
//! ```
//!
//! UTF-16 strings: `u16` char-count + `char-count * u16` codepoints +
//! `u16` null terminator.
//!
//! UTF-8 strings: length-prefix (u8, or u16 with high-bit sentinel) for
//! char count + same for byte count + `byte-count` UTF-8 bytes +
//! `u8` null terminator.

#![allow(clippy::missing_errors_doc)]

/// Errors from string pool decode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StringPoolError {
    /// The chunk bytes are too short to hold the declared header.
    Truncated { need: usize, have: usize },
    /// A string's declared length reaches past the end of the chunk.
    StringOutOfBounds { index: usize, offset: usize },
    /// A UTF-8 string's bytes are not valid UTF-8.
    BadUtf8 { index: usize },
    /// The header_size field is too small (< 28).
    BadHeaderSize { header_size: u16 },
}

impl std::fmt::Display for StringPoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StringPoolError::Truncated { need, have } => {
                write!(f, "string pool truncated: need {need} bytes, have {have}")
            }
            StringPoolError::StringOutOfBounds { index, offset } => {
                write!(f, "string {index} at offset {offset} out of bounds")
            }
            StringPoolError::BadUtf8 { index } => write!(f, "string {index} is not valid UTF-8"),
            StringPoolError::BadHeaderSize { header_size } => {
                write!(f, "string pool header_size {header_size} < 28")
            }
        }
    }
}

impl std::error::Error for StringPoolError {}

const UTF8_FLAG: u32 = 0x0000_0100;

/// Decode the string pool from a raw chunk buffer (the full chunk bytes
/// including the 8-byte common header).
///
/// Returns a `Vec<String>` with one entry per string index.
pub(crate) fn decode(chunk: &[u8]) -> Result<Vec<String>, StringPoolError> {
    if chunk.len() < 28 {
        return Err(StringPoolError::Truncated {
            need: 28,
            have: chunk.len(),
        });
    }
    let header_size = u16::from_le_bytes([chunk[2], chunk[3]]) as usize;
    if header_size < 28 {
        return Err(StringPoolError::BadHeaderSize {
            header_size: header_size as u16,
        });
    }
    let string_count = u32::from_le_bytes([chunk[8], chunk[9], chunk[10], chunk[11]]) as usize;
    let flags = u32::from_le_bytes([chunk[16], chunk[17], chunk[18], chunk[19]]);
    let strings_start = u32::from_le_bytes([chunk[20], chunk[21], chunk[22], chunk[23]]) as usize;
    let utf8 = (flags & UTF8_FLAG) != 0;

    // Offset table starts after the 28-byte pool header.
    let offsets_base = 28usize;
    let offsets_end = offsets_base + string_count * 4;
    if offsets_end > chunk.len() {
        return Err(StringPoolError::Truncated {
            need: offsets_end,
            have: chunk.len(),
        });
    }

    let mut strings = Vec::with_capacity(string_count);
    for i in 0..string_count {
        let off_pos = offsets_base + i * 4;
        let off =
            u32::from_le_bytes([chunk[off_pos], chunk[off_pos + 1], chunk[off_pos + 2], chunk[off_pos + 3]])
                as usize;
        let pos = strings_start + off;
        if pos >= chunk.len() {
            return Err(StringPoolError::StringOutOfBounds { index: i, offset: pos });
        }
        let s = if utf8 {
            decode_utf8_str(chunk, i, pos)?
        } else {
            decode_utf16_str(chunk, i, pos)?
        };
        strings.push(s);
    }
    Ok(strings)
}

fn decode_utf16_str(chunk: &[u8], index: usize, mut pos: usize) -> Result<String, StringPoolError> {
    if pos + 2 > chunk.len() {
        return Err(StringPoolError::StringOutOfBounds { index, offset: pos });
    }
    let mut char_count = u16::from_le_bytes([chunk[pos], chunk[pos + 1]]) as usize;
    pos += 2;
    // High-bit sentinel: if set, the count is a 15-bit value spread across two
    // u16s (high bits in the first, low bits in the second). In practice AXML
    // string pools never exceed 32 767 chars per string, so this is defensive.
    if char_count & 0x8000 != 0 {
        if pos + 2 > chunk.len() {
            return Err(StringPoolError::StringOutOfBounds { index, offset: pos });
        }
        let lo = u16::from_le_bytes([chunk[pos], chunk[pos + 1]]) as usize;
        char_count = ((char_count & 0x7fff) << 16) | lo;
        pos += 2;
    }
    let byte_len = char_count * 2;
    if pos + byte_len > chunk.len() {
        return Err(StringPoolError::StringOutOfBounds { index, offset: pos });
    }
    let utf16: Vec<u16> = chunk[pos..pos + byte_len]
        .chunks_exact(2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect();
    Ok(String::from_utf16_lossy(&utf16).to_string())
}

fn decode_utf8_str(chunk: &[u8], index: usize, mut pos: usize) -> Result<String, StringPoolError> {
    // Char count (in code points) — we skip this; we use the byte count.
    if pos >= chunk.len() {
        return Err(StringPoolError::StringOutOfBounds { index, offset: pos });
    }
    let b0 = chunk[pos] as usize;
    pos += 1;
    if b0 & 0x80 != 0 {
        if pos >= chunk.len() {
            return Err(StringPoolError::StringOutOfBounds { index, offset: pos });
        }
        let _b1 = chunk[pos] as usize;
        pos += 1;
        // char_count = ((b0 & 0x7f) << 8) | b1 — unused for our purposes
    }
    // Byte count.
    if pos >= chunk.len() {
        return Err(StringPoolError::StringOutOfBounds { index, offset: pos });
    }
    let b0 = chunk[pos] as usize;
    pos += 1;
    let byte_count = if b0 & 0x80 != 0 {
        if pos >= chunk.len() {
            return Err(StringPoolError::StringOutOfBounds { index, offset: pos });
        }
        let b1 = chunk[pos] as usize;
        pos += 1;
        ((b0 & 0x7f) << 8) | b1
    } else {
        b0
    };
    if pos + byte_count > chunk.len() {
        return Err(StringPoolError::StringOutOfBounds { index, offset: pos });
    }
    std::str::from_utf8(&chunk[pos..pos + byte_count])
        .map(|s| s.to_owned())
        .map_err(|_| StringPoolError::BadUtf8 { index })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_utf16_pool(strings: &[&str]) -> Vec<u8> {
        // Build the string data section.
        let mut str_data: Vec<u8> = Vec::new();
        let mut offsets: Vec<u32> = Vec::new();
        for s in strings {
            offsets.push(str_data.len() as u32);
            let chars: Vec<u16> = s.encode_utf16().collect();
            str_data.extend_from_slice(&(chars.len() as u16).to_le_bytes());
            for &c in &chars {
                str_data.extend_from_slice(&c.to_le_bytes());
            }
            str_data.extend_from_slice(&0u16.to_le_bytes()); // null terminator
        }
        // Pool header: type(2) + hdr_sz(2) + chunk_sz(4) + str_count(4) +
        //              style_count(4) + flags(4) + strings_start(4) + styles_start(4) = 28
        let str_count = strings.len() as u32;
        let offset_table_len = str_count as usize * 4;
        let strings_start = (28 + offset_table_len) as u32;
        let chunk_size = strings_start as usize + str_data.len();
        let mut out = Vec::new();
        out.extend_from_slice(&0x0001u16.to_le_bytes());  // type
        out.extend_from_slice(&28u16.to_le_bytes());       // header_size
        out.extend_from_slice(&(chunk_size as u32).to_le_bytes());
        out.extend_from_slice(&str_count.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());         // style_count
        out.extend_from_slice(&0u32.to_le_bytes());         // flags (UTF-16)
        out.extend_from_slice(&strings_start.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());         // styles_start
        for &off in &offsets {
            out.extend_from_slice(&off.to_le_bytes());
        }
        out.extend_from_slice(&str_data);
        out
    }

    #[test]
    fn decodes_utf16_pool() {
        let pool = make_utf16_pool(&["manifest", "package", "versionCode"]);
        let strings = decode(&pool).expect("decode");
        assert_eq!(strings, vec!["manifest", "package", "versionCode"]);
    }

    #[test]
    fn decodes_empty_string() {
        let pool = make_utf16_pool(&[""]);
        let strings = decode(&pool).expect("decode");
        assert_eq!(strings, vec![""]);
    }

    #[test]
    fn decodes_unicode() {
        let pool = make_utf16_pool(&["héllo", "世界"]);
        let strings = decode(&pool).expect("decode");
        assert_eq!(strings[0], "héllo");
        assert_eq!(strings[1], "世界");
    }
}
