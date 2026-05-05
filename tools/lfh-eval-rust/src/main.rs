// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `lfh-eval-rust` — P1.9 translation-validation Rust evaluator.
//!
//! Mirrors `Apkaxiom.Tv.LfhEval` (the Lean evaluator). Reads a list
//! of `<hex-blob>\n` records on stdin, runs
//! [`axiom_zip_ref::lfh::parse_lfh`] on each, and emits one
//! stable-shape JSON-line per input on stdout. The
//! `tools/translation-validator` binary asserts the two evaluators
//! produce **byte-identical output** for every input in the corpus.
//!
//! Output line shape — *must* match the Lean side byte-for-byte:
//!
//! ```text
//! {"i":<index>,"out":"ok","sig":<u32>,"vers":<u16>,"flags":<u16>,
//!  "method":<u16>,"time":<u16>,"date":<u16>,"crc":<u32>,
//!  "csize":<u32>,"usize":<u32>,"nlen":<u16>,"elen":<u16>,
//!  "name":"<hex>","extra":"<hex>","consumed":<usize>}
//!
//! {"i":<index>,"out":"err","tag":<u8>,"name":"<short-name>"}
//! ```
//!
//! Hex encoding is lowercase, fixed-width (no `0x` prefix), so the
//! output is byte-deterministic across Rust toolchain versions.

#![forbid(unsafe_code)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::needless_range_loop,
    clippy::single_match_else,
    clippy::too_long_first_doc_paragraph,
    clippy::needless_pass_by_value,
    clippy::missing_const_for_fn,
    clippy::single_match,
    clippy::manual_let_else
)]

use std::io::{self, Read, Write};

use axiom_zip_ref::lfh::{parse_lfh, ParseError, SIGNATURE};

fn nibble_hex(n: u8) -> u8 {
    if n < 10 {
        b'0' + n
    } else {
        b'a' + (n - 10)
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(nibble_hex(b >> 4) as char);
        out.push(nibble_hex(b & 0x0f) as char);
    }
    out
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

fn err_short_name(e: ParseError) -> &'static str {
    // Exhaustive over the 4 variants the Lean side enumerates.
    // `axiom-zip-ref::lfh::ParseError` is `#[non_exhaustive]` for
    // forward-compat; we crash loudly if a new variant lands without
    // the Lean side adding it too — which is exactly the
    // translation-validation invariant.
    match e {
        ParseError::ShortHeader => "shortHeader",
        ParseError::BadSignature => "badSignature",
        ParseError::ShortName => "shortName",
        ParseError::ShortExtra => "shortExtra",
        _ => unreachable!("new ParseError variant without TV-side coverage"),
    }
}

fn emit_one(i: usize, line: &str, w: &mut impl Write) -> io::Result<()> {
    let bytes = match hex_decode(line) {
        Some(b) => b,
        None => {
            writeln!(
                w,
                "{{\"i\":{i},\"out\":\"err\",\"tag\":255,\"name\":\"hexDecode\"}}"
            )?;
            return Ok(());
        }
    };
    match parse_lfh(&bytes) {
        Ok((lfh, consumed)) => {
            let name_hex = hex_encode(&lfh.file_name);
            let extra_hex = hex_encode(&lfh.extra_field);
            writeln!(
                w,
                "{{\"i\":{i},\"out\":\"ok\",\"sig\":{sig},\"vers\":{vers},\"flags\":{flags},\"method\":{method},\"time\":{time},\"date\":{date},\"crc\":{crc},\"csize\":{csize},\"usize\":{usize},\"nlen\":{nlen},\"elen\":{elen},\"name\":\"{name}\",\"extra\":\"{extra}\",\"consumed\":{consumed}}}",
                sig = SIGNATURE,
                vers = lfh.version_needed,
                flags = lfh.general_flags,
                method = lfh.compression_method,
                time = lfh.last_mod_time,
                date = lfh.last_mod_date,
                crc = lfh.crc32,
                csize = lfh.compressed_size,
                usize = lfh.uncompressed_size,
                nlen = lfh.file_name.len(),
                elen = lfh.extra_field.len(),
                name = name_hex,
                extra = extra_hex,
            )?;
        }
        Err(e) => {
            writeln!(
                w,
                "{{\"i\":{i},\"out\":\"err\",\"tag\":{tag},\"name\":\"{name}\"}}",
                tag = e.tag(),
                name = err_short_name(e),
            )?;
        }
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let mut stdin_buf = String::new();
    io::stdin().lock().read_to_string(&mut stdin_buf)?;
    let stdout = io::stdout();
    let mut w = stdout.lock();
    for (i, line) in stdin_buf.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        emit_one(i, line, &mut w)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_roundtrip() {
        let bs = [0x00u8, 0xff, 0x42, 0xde, 0xad, 0xbe, 0xef];
        assert_eq!(hex_encode(&bs), "00ff42deadbeef");
        assert_eq!(hex_decode("00ff42deadbeef").unwrap(), bs);
    }

    #[test]
    fn rejects_odd_length() {
        assert!(hex_decode("0").is_none());
        assert!(hex_decode("0a1").is_none());
    }

    #[test]
    fn emit_err_shape() {
        let mut buf = Vec::new();
        // 32 zero bytes — fixed prefix length but bad signature.
        let zeros_hex = "00".repeat(32);
        emit_one(0, &zeros_hex, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        // Whatever the verdict, output must be a single JSON line.
        assert_eq!(s.lines().count(), 1);
        assert!(s.starts_with("{\"i\":0,"));
        assert!(s.ends_with("}\n"));
    }

    #[test]
    fn emit_ok_shape_minimal_lfh() {
        // Hand-crafted minimal LFH: signature + 26 zero bytes (so
        // name_len=0, extra_len=0, all other fields zero).
        let mut bytes = vec![0x50, 0x4b, 0x03, 0x04];
        bytes.extend(std::iter::repeat_n(0u8, 26));
        let hex = hex_encode(&bytes);
        let mut buf = Vec::new();
        emit_one(7, &hex, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("\"i\":7"));
        assert!(s.contains("\"out\":\"ok\""));
        assert!(s.contains("\"sig\":67324752"));
        assert!(s.contains("\"name\":\"\""));
        assert!(s.contains("\"extra\":\"\""));
        assert!(s.contains("\"consumed\":30"));
    }
}
