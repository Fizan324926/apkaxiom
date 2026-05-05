// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `eocd-eval-rust` — P1.9 §IV TV Rust evaluator for the EOCD parser.
//!
//! Mirrors `Apkaxiom.Tv.EocdEval` (Lean) byte-for-byte. The TV
//! validator runs both and asserts byte-identical JSON output
//! across the EOCD corpus.

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

use axiom_zip_ref::eocd::{parse_eocd, ParseError, SIGNATURE};

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
    match e {
        ParseError::ShortFixed => "shortFixed",
        ParseError::BadSignature => "badSignature",
        ParseError::ShortComment => "shortComment",
        ParseError::InconsistentDisks => "inconsistentDisks",
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
    match parse_eocd(&bytes) {
        Ok((e, consumed)) => {
            let comment_hex = hex_encode(&e.comment);
            writeln!(
                w,
                "{{\"i\":{i},\"out\":\"ok\",\"sig\":{sig},\"disk\":{disk},\"cd_disk\":{cd_disk},\"entries_on_disk\":{eod},\"total_entries\":{te},\"cd_size\":{cs},\"cd_offset\":{co},\"clen\":{clen},\"comment\":\"{c}\",\"consumed\":{consumed}}}",
                sig = SIGNATURE,
                disk = e.disk_number,
                cd_disk = e.cd_start_disk,
                eod = e.entries_on_this_disk,
                te = e.total_entries,
                cs = e.cd_size,
                co = e.cd_offset,
                clen = e.comment.len(),
                c = comment_hex,
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
