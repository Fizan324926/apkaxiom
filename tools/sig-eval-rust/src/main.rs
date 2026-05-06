// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `sig-eval-rust` — Rust mirror of the Lean signing-block parser.
//!
//! Reads APK file paths on stdin (one per line); for each APK,
//! emits a single JSON line summarising the parsed signing block:
//!
//! ```json
//! {"path": "...", "signed": true,
//!  "block_offset": 12288, "block_total_size": 4096,
//!  "entries": [
//!    {"id": "0x7109871a", "tag": "v2", "len": 1497},
//!    {"id": "0xf05368c0", "tag": "v3", "len": 1497},
//!    ...
//!  ]}
//! ```
//!
//! Or on parse error / unsigned input:
//!
//! ```json
//! {"path": "...", "signed": false, "error": "..." }
//! ```
//!
//! The differential harness runs this tool + a Lean equivalent
//! over `corpus/signing/` and asserts byte-identical output —
//! the same translation-validation pattern P1.9 used.

#![forbid(unsafe_code)]
#![allow(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::manual_let_else,
    clippy::single_match_else,
    clippy::missing_const_for_fn
)]

use std::io::{self, BufRead, Write};

use axiom_sigblock::{locate, scheme};

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut i: usize = 0;
    for line in stdin.lock().lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            i += 1;
            continue;
        }
        let bytes = match hex_decode(trimmed) {
            Some(b) => b,
            None => {
                writeln!(
                    out,
                    "{{\"i\":{i},\"out\":\"err\",\"tag\":255,\"name\":\"hex-decode\"}}"
                )?;
                i += 1;
                continue;
            }
        };
        writeln!(out, "{}", evaluate(i, &bytes))?;
        i += 1;
    }
    Ok(())
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    for chunk in bytes.chunks_exact(2) {
        let hi = hex_nibble(chunk[0])?;
        let lo = hex_nibble(chunk[1])?;
        out.push((hi << 4) | lo);
    }
    Some(out)
}

fn hex_nibble(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// Format mirrors `theorems/Apkaxiom/Tv/SigEval.lean::evalOne`:
/// indexed lines, JSON-shape pinned, decimal numeric IDs.
fn evaluate(i: usize, bytes: &[u8]) -> String {
    match locate(bytes) {
        Ok(None) => format!("{{\"i\":{i},\"out\":\"unsigned\"}}"),
        Ok(Some(block)) => {
            let mut entries_json = String::from("[");
            for (j, entry) in block.entries.iter().enumerate() {
                if j > 0 {
                    entries_json.push(',');
                }
                entries_json.push_str(&format!(
                    "{{\"id\":{},\"len\":{}}}",
                    entry.id(),
                    entry.value().len()
                ));
            }
            entries_json.push(']');
            let mut detail = String::new();
            if let Some(v2) = block.v2() {
                if let Ok(signers) = scheme::parse_v2(v2) {
                    detail.push_str(&format!(",\"v2\":{}", signers_to_json(&signers, "v2")));
                }
            }
            if let Some(v3) = block.v3() {
                if let Ok(signers) = scheme::parse_v3(v3) {
                    detail.push_str(&format!(",\"v3\":{}", signers_to_json(&signers, "v3")));
                }
            }
            if let Some(v3_1) = block.v3_1() {
                if let Ok(signers) = scheme::parse_v3_1(v3_1) {
                    detail.push_str(&format!(",\"v3_1\":{}", signers_to_json(&signers, "v3_1")));
                }
            }
            format!(
                "{{\"i\":{i},\"out\":\"ok\",\"blockOffset\":{},\"blockTotalSize\":{},\"entries\":{}{}}}",
                block.block_offset, block.block_total_size, entries_json, detail
            )
        }
        Err(e) => {
            // Map parse error to Lean's tag enumeration.
            let tag: u8 = match e {
                axiom_sigblock::ParseError::NoEocd => 1,
                axiom_sigblock::ParseError::InvalidCdOffset { .. } => 2,
                axiom_sigblock::ParseError::BadMagic { .. } => 3,
                axiom_sigblock::ParseError::InvalidSize { .. } => 4,
                axiom_sigblock::ParseError::SizeMismatch { .. } => 5,
                axiom_sigblock::ParseError::PairOverflow { .. } => 6,
                axiom_sigblock::ParseError::PairTooShort { .. } => 7,
                axiom_sigblock::ParseError::TrailingJunk { .. } => 8,
                _ => 255,
            };
            let name = match e {
                axiom_sigblock::ParseError::NoEocd => "noEocd",
                axiom_sigblock::ParseError::InvalidCdOffset { .. } => "invalidCdOffset",
                axiom_sigblock::ParseError::BadMagic { .. } => "badMagic",
                axiom_sigblock::ParseError::InvalidSize { .. } => "invalidSize",
                axiom_sigblock::ParseError::SizeMismatch { .. } => "sizeMismatch",
                axiom_sigblock::ParseError::PairOverflow { .. } => "pairOverflow",
                axiom_sigblock::ParseError::PairTooShort { .. } => "pairTooShort",
                axiom_sigblock::ParseError::TrailingJunk { .. } => "trailingJunk",
                _ => "unknown",
            };
            format!("{{\"i\":{i},\"out\":\"err\",\"tag\":{tag},\"name\":\"{name}\"}}")
        }
    }
}

fn signers_to_json(signers: &[scheme::Signer], variant: &str) -> String {
    let mut out = String::from("[");
    for (i, s) in signers.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        // Lean's encodeSigner emits aggregate counts only — match its shape.
        let sdk = s
            .sdk_range
            .map(|(a, b)| format!(",\"sdk_min\":{a},\"sdk_max\":{b}"))
            .unwrap_or_default();
        out.push_str(&format!(
            "{{\"variant\":\"{variant}\",\"signed_data_len\":{},\"public_key_len\":{},\"certs\":{},\"digests\":{},\"signatures\":{}{}}}",
            s.signed_data.len(),
            s.public_key.len(),
            s.certificates.len(),
            s.digests.len(),
            s.signatures.len(),
            sdk
        ));
    }
    out.push(']');
    out
}
