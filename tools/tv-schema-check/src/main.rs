// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `tv-schema-check` — P1.9 §V item 6.
//!
//! Validates one or more JSON-line streams (from the Lean/Rust/
//! extracted evaluators) against the canonical schema in
//! `docs/phase-1/P1.9/lfh-output-schema.json`. Hand-rolled
//! validator (no third-party JSON Schema crate) so we don't
//! pull a heavy dep tree just for this one gate.
//!
//! What "validates" means here:
//!   - Each line is a syntactically well-formed JSON object.
//!   - The object has exactly one of the two shapes in the
//!     schema's `oneOf`.
//!   - Numeric fields fit their declared `minimum`..`maximum`.
//!   - String fields match their `pattern` / `enum`.
//!   - No extra properties.
//!
//! Run protocol:
//!
//!   tv-schema-check < evaluator-output.txt
//!
//! Exits 0 on success; non-zero with a per-line failure
//! description on the first invalid line.

#![forbid(unsafe_code)]
#![allow(
    clippy::too_long_first_doc_paragraph,
    clippy::cast_lossless,
    clippy::needless_range_loop,
    clippy::single_match_else,
    clippy::manual_let_else,
    clippy::redundant_pattern_matching,
    clippy::significant_drop_in_scrutinee,
    renamed_and_removed_lints,
    unknown_lints
)]

use std::io::{self, Read};
use std::process::ExitCode;

#[derive(Debug)]
enum Variant {
    Ok,
    Err,
}

fn validate_line(line: &str) -> Result<Variant, String> {
    let obj = parse_json_obj(line.trim())?;
    let out = obj
        .iter()
        .find(|(k, _)| *k == "out")
        .map(|(_, v)| v.as_str())
        .ok_or_else(|| "missing `out` field".to_string())?;
    match out.trim_matches('"') {
        "ok" => validate_ok(&obj).map(|()| Variant::Ok),
        "err" => validate_err(&obj).map(|()| Variant::Err),
        other => Err(format!("`out` must be \"ok\" or \"err\", got {other:?}")),
    }
}

fn validate_ok(obj: &[(String, String)]) -> Result<(), String> {
    let required: &[&str] = &[
        "i", "out", "sig", "vers", "flags", "method", "time", "date", "crc", "csize", "usize",
        "nlen", "elen", "name", "extra", "consumed",
    ];
    for k in required {
        if !obj.iter().any(|(kk, _)| kk == k) {
            return Err(format!("ok-shape: missing field `{k}`"));
        }
    }
    if obj.len() != required.len() {
        return Err(format!(
            "ok-shape: extra properties (have {}, expected {})",
            obj.len(),
            required.len()
        ));
    }
    check_uint(obj, "i", 0, u64::MAX)?;
    check_uint(obj, "sig", 0, u32::MAX as u64)?;
    check_uint(obj, "vers", 0, u16::MAX as u64)?;
    check_uint(obj, "flags", 0, u16::MAX as u64)?;
    check_uint(obj, "method", 0, u16::MAX as u64)?;
    check_uint(obj, "time", 0, u16::MAX as u64)?;
    check_uint(obj, "date", 0, u16::MAX as u64)?;
    check_uint(obj, "crc", 0, u32::MAX as u64)?;
    check_uint(obj, "csize", 0, u32::MAX as u64)?;
    check_uint(obj, "usize", 0, u32::MAX as u64)?;
    check_uint(obj, "nlen", 0, u16::MAX as u64)?;
    check_uint(obj, "elen", 0, u16::MAX as u64)?;
    check_uint(obj, "consumed", 0, u64::MAX)?;
    check_lower_hex(obj, "name")?;
    check_lower_hex(obj, "extra")?;
    Ok(())
}

fn validate_err(obj: &[(String, String)]) -> Result<(), String> {
    let required: &[&str] = &["i", "out", "tag", "name"];
    for k in required {
        if !obj.iter().any(|(kk, _)| kk == k) {
            return Err(format!("err-shape: missing field `{k}`"));
        }
    }
    if obj.len() != required.len() {
        return Err(format!(
            "err-shape: extra properties (have {}, expected {})",
            obj.len(),
            required.len()
        ));
    }
    check_uint(obj, "i", 0, u64::MAX)?;
    check_uint(obj, "tag", 1, 255)?;
    let name = field_str(obj, "name")?;
    if !matches!(
        name,
        "shortHeader" | "badSignature" | "shortName" | "shortExtra" | "hexDecode"
    ) {
        return Err(format!("err-shape: unknown name {name:?}"));
    }
    Ok(())
}

fn field_str<'a>(obj: &'a [(String, String)], key: &str) -> Result<&'a str, String> {
    let raw = obj
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
        .ok_or_else(|| format!("missing field `{key}`"))?;
    let trimmed = raw.trim();
    if !trimmed.starts_with('"') || !trimmed.ends_with('"') {
        return Err(format!("field `{key}` is not a JSON string ({trimmed:?})"));
    }
    Ok(&trimmed[1..trimmed.len() - 1])
}

fn check_uint(obj: &[(String, String)], key: &str, min: u64, max: u64) -> Result<(), String> {
    let raw = obj
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
        .ok_or_else(|| format!("missing field `{key}`"))?;
    let n: u64 = raw
        .trim()
        .parse()
        .map_err(|e| format!("field `{key}` is not a u64: {raw:?} ({e})"))?;
    if n < min || n > max {
        return Err(format!("field `{key}` = {n} out of range [{min}, {max}]"));
    }
    Ok(())
}

fn check_lower_hex(obj: &[(String, String)], key: &str) -> Result<(), String> {
    let s = field_str(obj, key)?;
    if s.len() % 2 != 0 {
        return Err(format!("field `{key}` has odd length: {s:?}"));
    }
    if !s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
        return Err(format!("field `{key}` is not lowercase hex: {s:?}"));
    }
    Ok(())
}

/// Hand-rolled flat-object JSON parser. Exact enough for our
/// shape (no nested objects, no arrays, no escaped quotes inside
/// keys); robust to whitespace.
fn parse_json_obj(line: &str) -> Result<Vec<(String, String)>, String> {
    let s = line.trim();
    if !s.starts_with('{') || !s.ends_with('}') {
        return Err(format!("line is not a JSON object: {s:?}"));
    }
    let inner = &s[1..s.len() - 1];
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    for ch in inner.chars() {
        if esc {
            buf.push(ch);
            esc = false;
            continue;
        }
        if ch == '\\' && in_str {
            buf.push(ch);
            esc = true;
            continue;
        }
        if ch == '"' {
            in_str = !in_str;
            buf.push(ch);
            continue;
        }
        if !in_str {
            match ch {
                '{' | '[' => depth += 1,
                '}' | ']' => depth -= 1,
                ',' if depth == 0 => {
                    push_pair(&mut out, &buf)?;
                    buf.clear();
                    continue;
                }
                _ => {}
            }
        }
        buf.push(ch);
    }
    if !buf.is_empty() {
        push_pair(&mut out, &buf)?;
    }
    Ok(out)
}

fn push_pair(out: &mut Vec<(String, String)>, raw: &str) -> Result<(), String> {
    let raw = raw.trim();
    let colon = raw
        .find(':')
        .ok_or_else(|| format!("expected `:` in pair {raw:?}"))?;
    let key_raw = raw[..colon].trim();
    let val_raw = raw[colon + 1..].trim();
    if !key_raw.starts_with('"') || !key_raw.ends_with('"') || key_raw.len() < 2 {
        return Err(format!("key is not a JSON string: {key_raw:?}"));
    }
    let key = key_raw[1..key_raw.len() - 1].to_string();
    out.push((key, val_raw.to_string()));
    Ok(())
}

fn main() -> ExitCode {
    let mut buf = String::new();
    if let Err(e) = io::stdin().lock().read_to_string(&mut buf) {
        eprintln!("read stdin: {e}");
        return ExitCode::from(1);
    }
    let mut total = 0u64;
    let mut ok_count = 0u64;
    let mut err_count = 0u64;
    for (i, line) in buf.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        total += 1;
        match validate_line(line) {
            Ok(Variant::Ok) => ok_count += 1,
            Ok(Variant::Err) => err_count += 1,
            Err(msg) => {
                eprintln!("FAIL line {i}: {msg}\n  line: {line}");
                return ExitCode::from(1);
            }
        }
    }
    println!("tv-schema-check: {total} lines validated ({ok_count} ok-shape, {err_count} err-shape) — PASS");
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok_line_validates() {
        let l = "{\"i\":0,\"out\":\"ok\",\"sig\":67324752,\"vers\":20,\"flags\":0,\"method\":0,\"time\":0,\"date\":0,\"crc\":0,\"csize\":0,\"usize\":0,\"nlen\":0,\"elen\":0,\"name\":\"\",\"extra\":\"\",\"consumed\":30}";
        assert!(matches!(validate_line(l), Ok(Variant::Ok)));
    }

    #[test]
    fn err_line_validates() {
        let l = "{\"i\":0,\"out\":\"err\",\"tag\":1,\"name\":\"shortHeader\"}";
        assert!(matches!(validate_line(l), Ok(Variant::Err)));
    }

    #[test]
    fn extra_field_rejected() {
        let l = "{\"i\":0,\"out\":\"err\",\"tag\":1,\"name\":\"shortHeader\",\"extra\":\"oops\"}";
        assert!(validate_line(l).is_err());
    }

    #[test]
    fn out_of_range_rejected() {
        let l = "{\"i\":0,\"out\":\"ok\",\"sig\":99999999999,\"vers\":0,\"flags\":0,\"method\":0,\"time\":0,\"date\":0,\"crc\":0,\"csize\":0,\"usize\":0,\"nlen\":0,\"elen\":0,\"name\":\"\",\"extra\":\"\",\"consumed\":30}";
        assert!(validate_line(l).is_err());
    }

    #[test]
    fn upper_hex_rejected() {
        let l = "{\"i\":0,\"out\":\"ok\",\"sig\":67324752,\"vers\":0,\"flags\":0,\"method\":0,\"time\":0,\"date\":0,\"crc\":0,\"csize\":0,\"usize\":0,\"nlen\":2,\"elen\":0,\"name\":\"FF\",\"extra\":\"\",\"consumed\":32}";
        assert!(validate_line(l).is_err());
    }
}
