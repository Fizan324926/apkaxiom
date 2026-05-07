// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `novelty-proof` — structural-finding scanner.
//!
//! Runs the full APKAXIOM pipeline on every APK in a corpus and emits one
//! NDJSON record per APK containing ALL findings that signature-only tools
//! (Androguard, apksigner) cannot detect:
//!
//! - `UNKNOWN_SIGBLOCK_PAIR` — an unrecognised ID injected into the APK
//!   Signing Block. apksigner ignores it; APKAXIOM surfaces it verbatim.
//! - `MANIFEST_PARSE_ERROR` — the ZIP parses but the AXML manifest cannot
//!   be decoded (dual-EOCD / overlapping-entry attacks).
//! - `HAS_V3_1_ROTATION` — APK carries a v3.1 rotation lineage entry;
//!   Androguard / apkanalyzer do not verify v3.1 at all.
//! - `HAS_SOURCE_STAMP` — SourceStamp v1/v2 detected in the signing block.
//! - `SIGBLOCK_PARSE_ERROR` — APK signing block is structurally malformed.
//! - `NO_SIGBLOCK` — no APK Signing Block found (v1-only or unsigned APK).
//! - `BLAKE3_DRIFT` — whole-file BLAKE3 hash differs from a provided baseline
//!   (detects supply-chain tamper in regions v2/v3 signatures don't cover).
//!
//! ## Usage
//!
//! ```text
//! novelty-proof --corpus <dir> [--baseline <ndjson>] [--json-out <file>]
//! ```

#![forbid(unsafe_code)]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::print_stdout,
    clippy::too_many_lines
)]

use std::collections::HashMap;
use std::io::{self, Write as IoWrite};
use std::path::{Path, PathBuf};

use axiom_blake3_hacl::{Blake3, Hasher as _};
use axiom_l1_rs::ir::emit as ir_emit;
use axiom_l1_rs::{Apk, Manifest, Unverified};
use axiom_l1_signing_verified::verify_apk_bytes;
use axiom_sigblock::{locate as sigblock_locate, SignatureBlockEntry};
use walkdir::WalkDir;

// ── Finding tags ──────────────────────────────────────────────────────────────

const TAG_UNKNOWN_PAIR: &str = "UNKNOWN_SIGBLOCK_PAIR";
const TAG_MANIFEST_ERR: &str = "MANIFEST_PARSE_ERROR";
const TAG_V3_1: &str = "HAS_V3_1_ROTATION";
const TAG_SOURCE_STAMP: &str = "HAS_SOURCE_STAMP";
const TAG_SIGBLOCK_ERR: &str = "SIGBLOCK_PARSE_ERROR";
const TAG_NO_SIGBLOCK: &str = "NO_SIGBLOCK";
const TAG_BLAKE3_DRIFT: &str = "BLAKE3_DRIFT";

// ── Entry ─────────────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let corpus_dir = parse_flag(&args, "--corpus").unwrap_or_else(|| {
        eprintln!(
            "Usage: novelty-proof --corpus <dir> [--baseline <ndjson>] [--json-out <file>]"
        );
        std::process::exit(1);
    });
    let baseline_path = parse_flag(&args, "--baseline");
    let json_out_path = parse_flag(&args, "--json-out");

    let apks = collect_apks(Path::new(&corpus_dir));
    if apks.is_empty() {
        eprintln!("No APK files found in {corpus_dir}");
        std::process::exit(1);
    }

    let baseline: HashMap<String, String> = baseline_path
        .as_deref()
        .map(load_baseline)
        .unwrap_or_default();

    eprintln!("corpus: {} APKs in {corpus_dir}", apks.len());

    let mut records: Vec<String> = Vec::with_capacity(apks.len());
    let mut total_findings = 0usize;
    let mut apks_with_findings = 0usize;

    for apk_path in &apks {
        let file_name = apk_path.file_name().unwrap().to_string_lossy().to_string();

        let bytes = match std::fs::read(apk_path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("  SKIP {}: {e}", apk_path.display());
                continue;
            }
        };

        let mut findings: Vec<String> = Vec::new();

        // ── BLAKE3 whole-file hash ────────────────────────────────────────
        let file_blake3 = Blake3::hash_oneshot(&bytes);
        let blake3_hex = hex_32(&file_blake3);

        // ── Baseline drift check ──────────────────────────────────────────
        if let Some(baseline_hash) = baseline.get(&file_name) {
            if *baseline_hash != blake3_hex {
                findings.push(format!(
                    r#"{{"tag":"{TAG_BLAKE3_DRIFT}","baseline":"{baseline_hash}","observed":"{blake3_hex}"}}"#
                ));
            }
        }

        // ── APK Signing Block structural scan ─────────────────────────────
        match sigblock_locate(&bytes) {
            Ok(None) => {
                findings.push(format!(r#"{{"tag":"{TAG_NO_SIGBLOCK}"}}"#));
            }
            Ok(Some(block)) => {
                for entry in &block.entries {
                    match entry {
                        SignatureBlockEntry::Unknown { id, value } => {
                            findings.push(format!(
                                r#"{{"tag":"{TAG_UNKNOWN_PAIR}","id":"0x{id:08x}","value_len":{}}}"#,
                                value.len()
                            ));
                        }
                        SignatureBlockEntry::V3_1(_) => {
                            findings.push(format!(r#"{{"tag":"{TAG_V3_1}"}}"#));
                        }
                        SignatureBlockEntry::SourceStampV1(_)
                        | SignatureBlockEntry::SourceStampV2(_) => {
                            findings.push(format!(r#"{{"tag":"{TAG_SOURCE_STAMP}"}}"#));
                        }
                        _ => {}
                    }
                }
            }
            Err(e) => {
                let detail = e.to_string().replace('"', "'");
                findings.push(format!(
                    r#"{{"tag":"{TAG_SIGBLOCK_ERR}","detail":"{detail}"}}"#
                ));
            }
        }

        // ── ZIP parse + IR emit ───────────────────────────────────────────
        let ir_sha256 = match Apk::<Unverified>::from_reader(std::io::Cursor::new(&bytes)) {
            Ok(apk) => compute_ir_sha256(&apk, &mut findings),
            Err(e) => {
                let detail = e.to_string().replace('"', "'");
                findings.push(format!(
                    r#"{{"tag":"{TAG_MANIFEST_ERR}","detail":"{detail}"}}"#
                ));
                "parse-err".to_string()
            }
        };

        // ── Signature verification verdict ────────────────────────────────
        let verdict = verify_apk_bytes(&bytes);
        let verdict_str = if verdict.is_accept() { "accept" } else { "reject" };

        // ── Emit record ───────────────────────────────────────────────────
        let findings_count = findings.len();
        total_findings += findings_count;
        if findings_count > 0 {
            apks_with_findings += 1;
        }

        let findings_json = if findings.is_empty() {
            "[]".to_string()
        } else {
            format!("[{}]", findings.join(","))
        };

        records.push(format!(
            r#"{{"file":"{file_name}","verdict":"{verdict_str}","ir_sha256":"{ir_sha256}","file_blake3":"{blake3_hex}","findings_count":{findings_count},"findings":{findings_json}}}"#
        ));
    }

    // ── Output ────────────────────────────────────────────────────────────────
    let mut out: Box<dyn IoWrite> = match json_out_path {
        Some(ref p) => {
            let f = std::fs::File::create(p).expect("create json-out");
            Box::new(f)
        }
        None => Box::new(io::stdout()),
    };

    for rec in &records {
        writeln!(out, "{rec}").unwrap();
    }

    let n = records.len();
    eprintln!();
    eprintln!("  APKs scanned:       {n}");
    eprintln!("  APKs with findings: {apks_with_findings}");
    eprintln!(
        "  Total findings:     {total_findings}  ({:.2} per APK avg)",
        if n == 0 { 0.0 } else { total_findings as f64 / n as f64 }
    );
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn compute_ir_sha256(apk: &Apk<Unverified>, findings: &mut Vec<String>) -> String {
    let Some(mb) = apk.manifest_bytes() else {
        findings.push(format!(r#"{{"tag":"{TAG_MANIFEST_ERR}","detail":"no-manifest"}}"#));
        return "no-manifest".to_string();
    };
    let manifest = Manifest { axml_bytes: mb.to_vec() };
    match ir_emit::emit_manifest(&manifest) {
        Ok(ir) => {
            let reencoded = ir_emit::reencode_manifest(&ir);
            let digest = axiom_crypto_hacl::sha256(&reencoded);
            hex_32(&digest)
        }
        Err(_) => {
            findings.push(format!(r#"{{"tag":"{TAG_MANIFEST_ERR}","detail":"ir-emit-err"}}"#));
            "ir-err".to_string()
        }
    }
}

fn parse_flag(args: &[String], flag: &str) -> Option<String> {
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == flag {
            return it.next().cloned();
        }
    }
    None
}

fn collect_apks(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = WalkDir::new(dir)
        .min_depth(1)
        .max_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case("apk"))
        })
        .map(|e| e.path().to_path_buf())
        .collect();
    v.sort();
    v
}

fn load_baseline(path: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Ok(contents) = std::fs::read_to_string(path) else {
        return map;
    };
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let (Some(file), Some(hash)) =
            (extract_json_str(line, "file"), extract_json_str(line, "file_blake3"))
        {
            map.insert(file, hash);
        }
    }
    map
}

fn extract_json_str(line: &str, key: &str) -> Option<String> {
    let needle = format!(r#""{key}":""#);
    let start = line.find(&needle)? + needle.len();
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn hex_32(bytes: &[u8; 32]) -> String {
    const CHARS: &[u8] = b"0123456789abcdef";
    let mut s = String::with_capacity(64);
    for &b in bytes {
        s.push(CHARS[(b >> 4) as usize] as char);
        s.push(CHARS[(b & 0xf) as usize] as char);
    }
    s
}
