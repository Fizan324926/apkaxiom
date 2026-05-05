// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `lfh-eval-extracted` — P1.9 §IV LFH evaluator using the
//! *auto-extracted* Rust parser.
//!
//! Mirrors [`lfh-eval-rust`](../lfh-eval-rust) byte-for-byte on the
//! output side, but the parsing call goes through
//! `axiom_l0_zip_lfh_extracted::parse_lfh` — Rust code generated
//! from `theorems/Apkaxiom/Zip/LocalHeader.lean` by
//! `tools/lean-to-rust`.
//!
//! The translation-validator runs both of these (plus the Lean
//! `lfh-eval` binary) and asserts byte-identical output across all
//! three arms. If they ever diverge, either:
//!
//!   1. the extractor introduced a semantic regression, OR
//!   2. the Lean source changed without the extractor being
//!      re-run, OR
//!   3. the hand-written `axiom_zip_ref::lfh::parse_lfh` drifted
//!      from the Lean reference.
//!
//! All three are real bugs the gate catches.

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

use axiom_l0_zip_lfh_extracted::{parse_lfh, ParseError, LFH_SIGNATURE};

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
        ParseError::ShortHeader => "shortHeader",
        ParseError::BadSignature => "badSignature",
        ParseError::ShortName => "shortName",
        ParseError::ShortExtra => "shortExtra",
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
                sig = LFH_SIGNATURE,
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
