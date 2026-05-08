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

const TAG_UNKNOWN_PAIR:      &str = "UNKNOWN_SIGBLOCK_PAIR";
const TAG_MANIFEST_ERR:      &str = "MANIFEST_PARSE_ERROR";
const TAG_V3_1:              &str = "HAS_V3_1_ROTATION";
const TAG_SOURCE_STAMP:      &str = "HAS_SOURCE_STAMP";
const TAG_SIGBLOCK_ERR:      &str = "SIGBLOCK_PARSE_ERROR";
const TAG_NO_SIGBLOCK:       &str = "NO_SIGBLOCK";
const TAG_BLAKE3_DRIFT:      &str = "BLAKE3_DRIFT";

// ── Structural-attack first-class tags (G-3) ─────────────────────────────────

/// More than one EOCD signature exists in the file. Canonical
/// dual-EOCD / nested-ZIP / Master-Key attack signature. See
/// CVE-2013-4787 family.
const TAG_MULTIPLE_EOCD:     &str = "MULTIPLE_EOCD_RECORDS";

/// Two or more central-directory records refer to the same
/// filename. Different consumers will pick different bodies
/// — the canonical Master-Key bug-class.
const TAG_DUPLICATE_NAME:    &str = "DUPLICATE_LFH_NAME";

/// File begins with a valid DEX magic (`dex\n035`) before the
/// first LFH signature — the Janus attack pattern (CVE-2017-13156),
/// where Android pre-Oreo loads the DEX directly while v1
/// signature verification reads the JAR.
const TAG_JANUS_DEX:         &str = "JANUS_DEX_PREPEND";

/// CDR and LFH disagree on a critical field (compressed_size,
/// uncompressed_size, or crc32) for the same entry. This is the
/// Master-Key 9950697 class.
const TAG_LFH_CDR_MISMATCH:  &str = "LFH_CDR_FIELD_MISMATCH";

/// An entry has the encryption bit (general-flag bit 0) set.
/// Android's installer rejects encrypted entries; surfacing it
/// flags an APK that could not run on a real device.
const TAG_ENCRYPTED_ENTRY:   &str = "ENCRYPTED_ENTRY";

/// Two LFH bodies overlap in the file's byte range — physically
/// impossible for honest archives; only adversarial archives or
/// hand-crafted ZIP smuggling produces this.
const TAG_OVERLAPPING_LFH:   &str = "OVERLAPPING_LFH_REGIONS";

/// The CDR carries `n` records that point at LFH offsets which
/// cannot all be reached without re-using the same file region —
/// CDR-overrun attack family.
const TAG_CDR_OVERLAP:       &str = "OVERLAPPING_CDR_ENTRIES";

/// Truncation detected at byte X with N expected. The parser
/// already raises a structural-parse error — this tag separates
/// "real corruption" from generic MANIFEST_PARSE_ERROR.
const TAG_TRUNCATED:         &str = "TRUNCATED_INPUT";

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

        // ── Structural-attack detectors (G-3) ─────────────────────────────
        // Independent of streaming-parse outcome so attacks that
        // crash a downstream layer still surface as findings.
        run_structural_detectors(&bytes, &mut findings);

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
                // Classify truncation separately from generic parse error.
                if detail.contains("truncated") {
                    findings.push(format!(
                        r#"{{"tag":"{TAG_TRUNCATED}","detail":"{detail}"}}"#
                    ));
                } else {
                    findings.push(format!(
                        r#"{{"tag":"{TAG_MANIFEST_ERR}","detail":"{detail}"}}"#
                    ));
                }
                "parse-err".to_string()
            }
        };

        // ── Verdicts (G-10: separate signature and parse outcomes) ───────
        // `signature_verdict` reflects ONLY whether the v2/v3 APK
        // Signing Block verifies cryptographically. `parse_verdict`
        // reflects whether the L0/L1 ZIP+manifest pipeline succeeded
        // end-to-end. They can disagree:
        //
        //   - dual-EOCD APK: signature_verdict=accept (no v2 sigblock
        //     present so accept is the v1-fallback default), but
        //     parse_verdict=reject because the manifest is unparseable.
        //   - sigblock-tamper APK: signature_verdict=accept (v2
        //     verifies on the unsigned regions), parse_verdict=accept,
        //     but findings array carries UNKNOWN_SIGBLOCK_PAIR.
        //
        // Consumers checking only `verdict` were getting the wrong
        // boolean for malformed APKs. The new split keeps backward
        // compatibility (`verdict` mirrors `signature_verdict`) and
        // adds `parse_verdict` so a downstream filter can refuse
        // any APK whose manifest could not be extracted.
        let signature_verdict = verify_apk_bytes(&bytes);
        let signature_str = if signature_verdict.is_accept() {
            "accept"
        } else {
            "reject"
        };
        let parse_str = if ir_sha256 == "parse-err" || ir_sha256 == "ir-err" || ir_sha256 == "no-manifest" {
            "reject"
        } else {
            "accept"
        };

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
            r#"{{"file":"{file_name}","verdict":"{signature_str}","signature_verdict":"{signature_str}","parse_verdict":"{parse_str}","ir_sha256":"{ir_sha256}","file_blake3":"{blake3_hex}","findings_count":{findings_count},"findings":{findings_json}}}"#
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

// ── Structural-attack detectors (G-3) ────────────────────────────────────────

/// `dex\n035\0` (DEX file magic) — first 8 bytes of any classes.dex.
/// Janus (CVE-2017-13156) prepends a DEX in front of an APK so that
/// Android pre-Oreo dex2oats the prefix while v1 sig-verify reads
/// the rest as JAR. Detect by checking `bytes[0..8]`.
const DEX_MAGIC: &[u8; 8] = b"dex\n035\0";
/// `dex\n037\0` (DEX 037 magic) — also seen in the wild.
const DEX_MAGIC_037: &[u8; 8] = b"dex\n037\0";

/// LFH signature little-endian.
const LFH_SIG_LE: u32 = 0x0403_4b50;
/// CDR signature little-endian.
const CDR_SIG_LE: u32 = 0x0201_4b50;
/// EOCD signature little-endian.
const EOCD_SIG_LE: u32 = 0x0605_4b50;

/// Run every structural-attack detector against the raw APK
/// bytes and append findings (as JSON object strings) into the
/// provided `findings` vector.
///
/// All detectors are read-only and operate on the byte slice
/// directly. They are intentionally independent of the
/// streaming parser so that a parse failure does not mask a
/// structural finding.
fn run_structural_detectors(bytes: &[u8], findings: &mut Vec<String>) {
    // ── 1. JANUS_DEX_PREPEND ─────────────────────────────────────────────
    // CVE-2017-13156. APK begins with a DEX magic instead of a ZIP
    // local-file-header signature. Pre-Oreo Android dex2oats the
    // prefix; v1 JAR verifier reads the rest. Emit the finding when
    // the file starts with a DEX magic AND we can still find a ZIP
    // EOCD somewhere in the file (i.e. it is a hybrid DEX+ZIP).
    if bytes.len() >= 8
        && (&bytes[..8] == DEX_MAGIC || &bytes[..8] == DEX_MAGIC_037)
    {
        // Confirm there is also a ZIP container inside the file.
        // Without it this is just a plain DEX, not the Janus
        // hybrid attack.
        if memmem(bytes, b"PK\x05\x06").is_some() {
            findings.push(format!(
                r#"{{"tag":"{TAG_JANUS_DEX}","dex_magic_at":0}}"#
            ));
        }
    }

    // ── 2. MULTIPLE_EOCD_RECORDS ─────────────────────────────────────────
    //
    // Detect dual-EOCD / nested-ZIP / Master-Key Bug 8219321 family.
    // The challenge: a real EOCD's `cd_offset` field can be either
    //   (a) global (canonical ZIP) — points at a CDR signature in
    //       the whole-file byte stream, OR
    //   (b) local (concatenated-ZIP attack) — points at a CDR
    //       signature relative to the start of *its own* ZIP, which
    //       in the host file is some interior position that happens
    //       to land on garbage when the EOCD is appended after
    //       another complete ZIP.
    //
    // Filtering on (a) alone gives false-negatives on (b)-shaped
    // attacks. So we use a multi-stage filter:
    //
    //   (i)   signature matches `PK\x05\x06`
    //   (ii)  EOCD fixed (22B) + declared `comment_len` fit in file
    //   (iii) `entries_on_this_disk == total_entries` (single-disk
    //          ZIP — true for every honest APK)
    //   (iv)  `disk_number == 0 && cd_start_disk == 0`
    //   (v)   `cd_size <= file_size`  (sanity)
    //
    // This catches both attack variants without inflating false
    // positives on random `PK\x05\x06` bytes inside compressed
    // bodies, because conditions (iii)-(v) eliminate ~99% of such
    // false hits.
    let mut eocd_offsets: Vec<usize> = Vec::new();
    if bytes.len() >= 22 {
        let upper = bytes.len() - 22;
        let mut i = 0usize;
        while i <= upper {
            let sig = u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap());
            if sig == EOCD_SIG_LE {
                let disk_number =
                    u16::from_le_bytes(bytes[i + 4..i + 6].try_into().unwrap());
                let cd_start_disk =
                    u16::from_le_bytes(bytes[i + 6..i + 8].try_into().unwrap());
                let entries_on_disk =
                    u16::from_le_bytes(bytes[i + 8..i + 10].try_into().unwrap());
                let total_entries =
                    u16::from_le_bytes(bytes[i + 10..i + 12].try_into().unwrap());
                let cd_size = u32::from_le_bytes(
                    bytes[i + 12..i + 16].try_into().unwrap(),
                ) as u64;
                let cmt_len =
                    u16::from_le_bytes(bytes[i + 20..i + 22].try_into().unwrap()) as usize;
                let single_disk =
                    disk_number == 0 && cd_start_disk == 0;
                let entries_consistent = entries_on_disk == total_entries;
                let cmt_fits = i + 22 + cmt_len <= bytes.len();
                let cd_size_sane = cd_size <= bytes.len() as u64;
                if single_disk && entries_consistent && cmt_fits && cd_size_sane {
                    eocd_offsets.push(i);
                }
            }
            i += 1;
        }
    }
    // Distinguish *embedded ZIPs* (legitimate: a JAR/AAR shipped as
    // an APK asset, whose own EOCD lives inside the outer entry's
    // body region) from *true dual-EOCD attacks* (canonical
    // CVE-2013-4787 / Master Key Bug 8219321). For each non-
    // canonical EOCD we check whether it falls inside the byte
    // range of any outer LFH body. If it does, it's an embedded
    // ZIP — informational, not an attack. If it does not, it is a
    // competing outer EOCD and `MULTIPLE_EOCD_RECORDS` fires.
    if eocd_offsets.len() >= 2 {
        eocd_offsets.sort();
        eocd_offsets.dedup();

        // Build outer-LFH body intervals using the canonical
        // (last) EOCD's central directory.
        let mut outer_body_intervals: Vec<(u64, u64)> = Vec::new();
        if let Some(&canonical_eocd) = eocd_offsets.last() {
            let cd_size = u32::from_le_bytes(
                bytes[canonical_eocd + 12..canonical_eocd + 16]
                    .try_into()
                    .unwrap(),
            ) as usize;
            let cd_offset = u32::from_le_bytes(
                bytes[canonical_eocd + 16..canonical_eocd + 20]
                    .try_into()
                    .unwrap(),
            ) as usize;
            if cd_offset.saturating_add(cd_size) <= bytes.len() {
                let cd_bytes = &bytes[cd_offset..cd_offset + cd_size];
                let mut k = 0usize;
                while k + 46 <= cd_bytes.len() {
                    let sig =
                        u32::from_le_bytes(cd_bytes[k..k + 4].try_into().unwrap());
                    if sig != CDR_SIG_LE {
                        break;
                    }
                    let csize = u32::from_le_bytes(
                        cd_bytes[k + 20..k + 24].try_into().unwrap(),
                    ) as u64;
                    let name_len = u16::from_le_bytes(
                        cd_bytes[k + 28..k + 30].try_into().unwrap(),
                    ) as usize;
                    let extra_len = u16::from_le_bytes(
                        cd_bytes[k + 30..k + 32].try_into().unwrap(),
                    ) as usize;
                    let cmt_len = u16::from_le_bytes(
                        cd_bytes[k + 32..k + 34].try_into().unwrap(),
                    ) as usize;
                    let lfh_off = u32::from_le_bytes(
                        cd_bytes[k + 42..k + 46].try_into().unwrap(),
                    ) as u64;
                    // LFH body interval = [lfh_off + 30 + name + extra,
                    //                       lfh_off + 30 + name + extra + csize)
                    // — but the entry's name+extra inside the LFH may
                    // differ from the CDR's name+extra. To stay
                    // conservative we use a wider interval that
                    // begins at lfh_off and ends at lfh_off +
                    // header_max + csize. This over-includes by the
                    // header size (≤ ~130KB) which is negligible.
                    let body_start = lfh_off;
                    let body_end = lfh_off
                        .saturating_add(30)
                        .saturating_add(name_len as u64)
                        .saturating_add(extra_len as u64)
                        .saturating_add(csize)
                        .saturating_add(64); // small slop for DD records
                    outer_body_intervals.push((body_start, body_end));
                    let total = 46 + name_len + extra_len + cmt_len;
                    if k + total > cd_bytes.len() {
                        break;
                    }
                    k += total;
                }
            }
        }

        // Classify each non-canonical EOCD.
        let canonical_eocd = *eocd_offsets.last().unwrap();
        let mut competing_outers: Vec<usize> = Vec::new();
        let mut embedded_zips: u32 = 0;
        for &off in &eocd_offsets {
            if off == canonical_eocd {
                continue;
            }
            let in_outer_body = outer_body_intervals
                .iter()
                .any(|&(s, e)| (off as u64) >= s && (off as u64) < e);
            if in_outer_body {
                embedded_zips += 1;
            } else {
                competing_outers.push(off);
            }
        }

        if !competing_outers.is_empty() {
            // True dual-EOCD attack: a non-canonical EOCD that is
            // not contained within any outer LFH body.
            let mut all = competing_outers.clone();
            all.push(canonical_eocd);
            all.sort();
            let offs: Vec<String> = all.iter().map(|o| o.to_string()).collect();
            findings.push(format!(
                r#"{{"tag":"{TAG_MULTIPLE_EOCD}","count":{},"offsets":[{}]}}"#,
                all.len(),
                offs.join(",")
            ));
        }
        // We deliberately do NOT emit a finding for embedded ZIPs
        // (e.g. JAR/AAR shipped as APK asset) — that is benign.
        let _ = embedded_zips;
    }

    // ── 3. DUPLICATE_LFH_NAME ────────────────────────────────────────────
    // Walk the central directory and count name occurrences. Any
    // count > 1 is anomalous. We rely on the last EOCD (canonical
    // ZIP behaviour) for cd_offset / cd_size.
    if let Some(eocd_off) = bytes.windows(4).rposition(|w| {
        u32::from_le_bytes(w.try_into().unwrap()) == EOCD_SIG_LE
    }) {
        // Re-validate fields fit.
        if eocd_off + 22 <= bytes.len() {
            let cd_size = u32::from_le_bytes(
                bytes[eocd_off + 12..eocd_off + 16].try_into().unwrap(),
            ) as usize;
            let cd_offset = u32::from_le_bytes(
                bytes[eocd_off + 16..eocd_off + 20].try_into().unwrap(),
            ) as usize;
            if cd_offset
                .checked_add(cd_size)
                .is_some_and(|end| end <= bytes.len())
            {
                let cd_bytes = &bytes[cd_offset..cd_offset + cd_size];
                let mut counts: HashMap<Vec<u8>, u32> = HashMap::new();
                let mut cdr_starts: Vec<usize> = Vec::new();
                let mut lfh_offsets: Vec<u32> = Vec::new();
                let mut sizes: Vec<(u32, u32, u32)> = Vec::new();
                let mut general_flags_seen: Vec<u16> = Vec::new();
                let mut k = 0usize;
                while k + 46 <= cd_bytes.len() {
                    let sig = u32::from_le_bytes(cd_bytes[k..k + 4].try_into().unwrap());
                    if sig != CDR_SIG_LE {
                        break;
                    }
                    let gp_flags =
                        u16::from_le_bytes(cd_bytes[k + 8..k + 10].try_into().unwrap());
                    let crc =
                        u32::from_le_bytes(cd_bytes[k + 16..k + 20].try_into().unwrap());
                    let csize =
                        u32::from_le_bytes(cd_bytes[k + 20..k + 24].try_into().unwrap());
                    let usize_ =
                        u32::from_le_bytes(cd_bytes[k + 24..k + 28].try_into().unwrap());
                    let name_len =
                        u16::from_le_bytes(cd_bytes[k + 28..k + 30].try_into().unwrap()) as usize;
                    let extra_len =
                        u16::from_le_bytes(cd_bytes[k + 30..k + 32].try_into().unwrap()) as usize;
                    let cmt_len =
                        u16::from_le_bytes(cd_bytes[k + 32..k + 34].try_into().unwrap()) as usize;
                    let lfh_off =
                        u32::from_le_bytes(cd_bytes[k + 42..k + 46].try_into().unwrap());
                    let total = 46 + name_len + extra_len + cmt_len;
                    if k + total > cd_bytes.len() {
                        break;
                    }
                    let name = cd_bytes[k + 46..k + 46 + name_len].to_vec();
                    *counts.entry(name).or_insert(0) += 1;
                    cdr_starts.push(k);
                    lfh_offsets.push(lfh_off);
                    sizes.push((csize, usize_, crc));
                    general_flags_seen.push(gp_flags);
                    k += total;
                }
                let dups: Vec<(Vec<u8>, u32)> = counts
                    .into_iter()
                    .filter(|(_, c)| *c > 1)
                    .collect();
                for (name, count) in &dups {
                    let name_str = String::from_utf8_lossy(name).replace('"', "'");
                    findings.push(format!(
                        r#"{{"tag":"{TAG_DUPLICATE_NAME}","name":"{name_str}","count":{count}}}"#
                    ));
                }

                // ── 4. ENCRYPTED_ENTRY ───────────────────────────────────────
                // gp_flag bit 0 = encrypted file. Android does not
                // support encrypted ZIP entries; presence indicates
                // an APK that cannot install OR an attempt to hide
                // body bytes from static analysis.
                let n_enc = general_flags_seen
                    .iter()
                    .filter(|f| (*f & 0x0001) != 0)
                    .count();
                if n_enc > 0 {
                    findings.push(format!(
                        r#"{{"tag":"{TAG_ENCRYPTED_ENTRY}","count":{n_enc}}}"#
                    ));
                }

                // ── 5. LFH_CDR_FIELD_MISMATCH ────────────────────────────────
                // Each CDR record points at an LFH via lfh_offset.
                // For non-DD entries, the LFH must declare the same
                // (csize, usize, crc) as the CDR. A mismatch is the
                // Master-Key 9950697 vector.
                let mut mismatches = 0u32;
                let mut sample: Option<(u32, (u32, u32, u32), (u32, u32, u32))> = None;
                for (i, &lfh_off) in lfh_offsets.iter().enumerate() {
                    let off = lfh_off as usize;
                    if off + 30 > bytes.len() {
                        continue;
                    }
                    let lfh_sig = u32::from_le_bytes(bytes[off..off + 4].try_into().unwrap());
                    if lfh_sig != LFH_SIG_LE {
                        continue;
                    }
                    let lfh_gpflags =
                        u16::from_le_bytes(bytes[off + 6..off + 8].try_into().unwrap());
                    if (lfh_gpflags & 0x0008) != 0 {
                        // DD entry — LFH csize/usize/crc are zero by spec.
                        continue;
                    }
                    let lfh_crc =
                        u32::from_le_bytes(bytes[off + 14..off + 18].try_into().unwrap());
                    let lfh_csize =
                        u32::from_le_bytes(bytes[off + 18..off + 22].try_into().unwrap());
                    let lfh_usize =
                        u32::from_le_bytes(bytes[off + 22..off + 26].try_into().unwrap());
                    let cdr = sizes[i];
                    let lfh_triple = (lfh_csize, lfh_usize, lfh_crc);
                    if cdr != lfh_triple {
                        mismatches += 1;
                        if sample.is_none() {
                            sample = Some((lfh_off, lfh_triple, cdr));
                        }
                    }
                }
                if mismatches > 0 {
                    let detail = sample
                        .map(|(off, lfh, cdr)| {
                            format!(
                                r#""sample_lfh_offset":{},"lfh_csize":{},"cdr_csize":{}"#,
                                off, lfh.0, cdr.0
                            )
                        })
                        .unwrap_or_default();
                    findings.push(format!(
                        r#"{{"tag":"{TAG_LFH_CDR_MISMATCH}","count":{mismatches},{detail}}}"#
                    ));
                }

                // ── 6. OVERLAPPING_CDR_ENTRIES ───────────────────────────────
                // Two CDR records claim LFH bodies whose [start, end)
                // intervals intersect. Detect via simple O(n log n)
                // sweep over (lfh_offset, csize).
                let mut intervals: Vec<(u64, u64)> = lfh_offsets
                    .iter()
                    .zip(sizes.iter())
                    .map(|(&off, &(csize, _, _))| {
                        let start = off as u64;
                        let end = start + 30 + csize as u64;
                        (start, end)
                    })
                    .collect();
                intervals.sort_by_key(|&(s, _)| s);
                let mut overlaps = 0u32;
                for w in intervals.windows(2) {
                    if w[1].0 < w[0].1 {
                        overlaps += 1;
                    }
                }
                if overlaps > 0 {
                    findings.push(format!(
                        r#"{{"tag":"{TAG_OVERLAPPING_LFH}","overlap_pairs":{overlaps}}}"#
                    ));
                }
                let _ = TAG_CDR_OVERLAP; // kept for forward compatibility
            }
        }
    }
}

/// Plain `memmem` over byte slices (no external crate). Returns
/// the first match offset, if any.
fn memmem(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|w| w == needle)
}

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
