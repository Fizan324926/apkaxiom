// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `archive-eval-rust` — P1.12 TV Rust evaluator for the
//! whole-archive (Consistency) parser.
//!
//! Mirrors `Apkaxiom.Tv.ArchiveEval` (Lean) byte-for-byte.

#![forbid(unsafe_code)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::missing_const_for_fn,
    clippy::too_long_first_doc_paragraph,
    clippy::manual_let_else
)]

use std::io::{self, Read, Write};

use axiom_zip_ref::archive::{parse_archive, ArchiveError};

fn nibble_hex(n: u8) -> u8 {
    if n < 10 {
        b'0' + n
    } else {
        b'a' + (n - 10)
    }
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return None;
    }
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(s.len() / 2);
    for chunk in bytes.chunks_exact(2) {
        let hi = decode_nibble(chunk[0])?;
        let lo = decode_nibble(chunk[1])?;
        out.push((hi << 4) | lo);
    }
    Some(out)
}

fn decode_nibble(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    }
}

fn err_short_name(e: &ArchiveError) -> &'static str {
    match e {
        ArchiveError::NoEocd => "noEocd",
        ArchiveError::EocdInvalid => "eocdInvalid",
        ArchiveError::CdOutOfRange => "cdOutOfRange",
        ArchiveError::CdrInvalid => "cdrInvalid",
        ArchiveError::CdrCountMismatch => "cdrCountMismatch",
        ArchiveError::LfhOffsetOob => "lfhOffsetOob",
        ArchiveError::LfhInvalid => "lfhInvalid",
        ArchiveError::FilenameMismatch => "filenameMismatch",
        ArchiveError::FieldMismatch => "fieldMismatch",
        ArchiveError::EocdTooFarFromEof => "eocdTooFarFromEof",
        ArchiveError::CdAfterEocd => "cdAfterEocd",
        ArchiveError::InvalidEntryName => "invalidEntryName",
        _ => "unknown",
    }
}

fn err_tag(e: &ArchiveError) -> u8 {
    e.tag()
}

fn emit_one(i: usize, line: &str, w: &mut impl Write) -> io::Result<()> {
    let Some(bytes) = hex_decode(line) else {
        writeln!(
            w,
            "{{\"i\":{i},\"out\":\"err\",\"tag\":255,\"name\":\"hexDecode\"}}"
        )?;
        return Ok(());
    };
    match parse_archive(&bytes) {
        Ok(a) => {
            writeln!(
                w,
                "{{\"i\":{i},\"out\":\"ok\",\"cdrs\":{cdrs},\"lfhs\":{lfhs},\"total_entries\":{te},\"cd_offset\":{co},\"cd_size\":{cs}}}",
                cdrs = a.cdrs.len(),
                lfhs = a.lfhs.len(),
                te = a.eocd.total_entries,
                co = a.eocd.cd_offset,
                cs = a.eocd.cd_size,
            )?;
            // Hex-encoder lint suppression — used inline elsewhere.
            let _ = nibble_hex;
        }
        Err(e) => {
            writeln!(
                w,
                "{{\"i\":{i},\"out\":\"err\",\"tag\":{tag},\"name\":\"{name}\"}}",
                tag = err_tag(&e),
                name = err_short_name(&e),
            )?;
        }
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let mut buf = String::new();
    io::stdin().lock().read_to_string(&mut buf)?;
    let stdout = io::stdout();
    let mut w = stdout.lock();
    for (i, line) in buf.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        emit_one(i, line, &mut w)?;
    }
    Ok(())
}
