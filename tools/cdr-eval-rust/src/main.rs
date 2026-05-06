// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `cdr-eval-rust` — P1.12 TV Rust evaluator for the CDR parser.
//!
//! Mirrors `Apkaxiom.Tv.CdrEval` (Lean) byte-for-byte. The TV
//! validator runs both and asserts byte-identical JSON output
//! across the CDR corpus.

#![forbid(unsafe_code)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::missing_const_for_fn,
    clippy::too_long_first_doc_paragraph,
    clippy::manual_let_else,
    clippy::doc_markdown
)]

use std::io::{self, Read, Write};

use axiom_zip_ref::cdr::{parse_cdr, ParseError, SIGNATURE};

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

fn err_short_name(e: &ParseError) -> &'static str {
    match e {
        ParseError::ShortHeader => "shortHeader",
        ParseError::BadSignature => "badSignature",
        ParseError::ShortName => "shortName",
        ParseError::ShortExtra => "shortExtra",
        ParseError::ShortComment => "shortComment",
        _ => "unknown",
    }
}

fn err_tag(e: &ParseError) -> u8 {
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
    match parse_cdr(&bytes) {
        Ok((c, consumed)) => {
            writeln!(
                w,
                "{{\"i\":{i},\"out\":\"ok\",\"sig\":{sig},\"vmade\":{vmb},\"vneed\":{vn},\"flags\":{gp},\"method\":{cm},\"time\":{lmt},\"date\":{lmd},\"crc\":{crc},\"csize\":{cs},\"usize\":{us},\"nlen\":{nl},\"elen\":{el},\"clen\":{cl},\"disk\":{dn},\"iattr\":{ia},\"eattr\":{ea},\"lfh_off\":{lo},\"name\":\"{n}\",\"extra\":\"{e}\",\"comment\":\"{co}\",\"consumed\":{consumed}}}",
                sig = SIGNATURE,
                vmb = c.version_made_by,
                vn = c.version_needed,
                gp = c.general_flags,
                cm = c.compression_method,
                lmt = c.last_mod_time,
                lmd = c.last_mod_date,
                crc = c.crc32,
                cs = c.compressed_size,
                us = c.uncompressed_size,
                nl = c.file_name.len(),
                el = c.extra_field.len(),
                cl = c.file_comment.len(),
                dn = c.disk_number_start,
                ia = c.internal_file_attributes,
                ea = c.external_file_attributes,
                lo = c.lfh_offset,
                n = hex_encode(&c.file_name),
                e = hex_encode(&c.extra_field),
                co = hex_encode(&c.file_comment),
            )?;
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
