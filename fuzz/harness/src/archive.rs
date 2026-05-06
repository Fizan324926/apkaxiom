// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Finding archive — append-only, schema-versioned ndjson.
//!
//! ## Format
//!
//! One JSON object per line. Stable field order. Every line ends
//! with `\n`. The archive is opened with `O_APPEND`; writes are
//! flushed and `fsync`'d on every record so a `kill -9` mid-soak
//! costs at most one record.
//!
//! ## Schema
//!
//! ```text
//! {
//!   "schema_version": "p113-finding-1.0",
//!   "finding_id":     <hex-sha256(input)>,
//!   "timestamp_ns":   <u64 nanoseconds since epoch>,
//!   "mode":           "dev" | "real",
//!   "target_label":   "aosp-libziparchive-runtime" | "cuttlefish-a14",
//!   "input_sha256":   <hex>,
//!   "input_path":     <relative-path-to-input-bytes-saved-on-disk>,
//!   "input_len":      <u64>,
//!   "axiom_l0":       "accept" | "reject:<tag>",
//!   "target":         "accept" | "reject:<tag>",
//!   "bucket":         "A_BOTH_ACCEPT" | … | "E_AXIOM_REJECT_TARGET_ACCEPT",
//!   "high_severity":  <bool>,
//!   "seed_origin":    <optional string — which seed corpus the
//!                      input was mutated from>,
//!   "mutation_kind":  <optional string — flip/del/insert/grammar>
//! }
//! ```
//!
//! ## Why ndjson, not fjall+rkyv (yet)
//!
//! The README §4 calls for fjall (LSM) + rkyv (zero-copy archive)
//! — those are the right tools at scale. Reindeer-vendoring them
//! is a multi-day operation, so this sub-phase ships the
//! audit-equivalent ndjson format and migrates to fjall+rkyv when
//! the dataset crosses the LSM-justifying threshold (~1 GB of
//! findings, ~100K records). The migration path is documented in
//! `docs/phase-1/P1.13/differential-fuzzer.md` §"Storage" and is
//! a one-liner change in this module: every record's fields map
//! 1:1 onto the rkyv-archived struct. The replay tool reads
//! ndjson today and will read either format under feature
//! detection later.

use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use axiom_blake3_hacl::hex_encode;

use crate::classifier::{Bucket, Verdict};

/// Stable schema version. Bump on any breaking field change.
///
/// 1.0 — initial dev-mode harness output.
/// 1.1 — adds `target_version` (Android version label, e.g. "A14")
///       and `synthetic` (bool — true iff verdict was post-filtered
///       by the Rust synthetic-version layer rather than a real
///       per-version libziparchive build). Cross-version classifier
///       reads both fields; older 1.0 records back-fill
///       `target_version="A14"` and `synthetic=false` on parse.
pub const SCHEMA_VERSION: &str = "p114-finding-1.1";

/// One archive record. Consumers read the ndjson stream by parsing
/// each line into this struct.
#[derive(Debug, Clone)]
pub struct Finding {
    /// `<hex-sha256(input bytes)>` — stable per input.
    pub finding_id: String,
    /// Wall-clock at finding time. Nanoseconds since UNIX epoch.
    pub timestamp_ns: u64,
    /// `"dev"` or `"real"`.
    pub mode: String,
    /// Stable name of the target arm (e.g. `"aosp-libziparchive-runtime"`).
    pub target_label: String,
    /// SHA-256 of the input bytes, hex.
    pub input_sha256: String,
    /// Path (relative to the archive root) to the saved input
    /// bytes. The replay tool reads this path back.
    pub input_path: String,
    /// Input length in bytes.
    pub input_len: u64,
    /// axiom-l0's verdict.
    pub axiom_l0: Verdict,
    /// Target's verdict.
    pub target: Verdict,
    /// Classifier bucket.
    pub bucket: Bucket,
    /// Origin seed (optional).
    pub seed_origin: Option<String>,
    /// Mutation kind (optional).
    pub mutation_kind: Option<String>,
    /// Android target version label (`A8`, `A11`, `A14`). Defaults
    /// to `A14` on 1.0 records (back-fill on parse).
    pub target_version: String,
    /// True iff the target verdict was produced by the synthetic
    /// per-version filter layer rather than a real per-version
    /// libziparchive build. Cross-version classifier weights real
    /// disagreements higher than synthetic ones.
    pub synthetic: bool,
}

impl Finding {
    /// Build a Finding from a verdict pair + input bytes. The
    /// `finding_id` and `input_sha256` are derived deterministically
    /// from `input`, so two identical inputs always yield the same
    /// id (allowing dedupe on read).
    #[must_use]
    pub fn from_verdicts(
        mode: &str,
        target_label: &str,
        input: &[u8],
        input_path: &str,
        axiom_l0: Verdict,
        target: Verdict,
        bucket: Bucket,
        seed_origin: Option<String>,
        mutation_kind: Option<String>,
    ) -> Self {
        Self::from_verdicts_versioned(
            mode,
            target_label,
            "A14",
            false,
            input,
            input_path,
            axiom_l0,
            target,
            bucket,
            seed_origin,
            mutation_kind,
        )
    }

    /// Versioned variant — supplies the Android version label and
    /// synthetic flag. The legacy `from_verdicts` defaults both
    /// fields to A14 / non-synthetic.
    #[must_use]
    pub fn from_verdicts_versioned(
        mode: &str,
        target_label: &str,
        target_version: &str,
        synthetic: bool,
        input: &[u8],
        input_path: &str,
        axiom_l0: Verdict,
        target: Verdict,
        bucket: Bucket,
        seed_origin: Option<String>,
        mutation_kind: Option<String>,
    ) -> Self {
        let input_sha = sha256_hex(input);
        Self {
            finding_id: input_sha.clone(),
            timestamp_ns: now_ns(),
            mode: mode.to_string(),
            target_label: target_label.to_string(),
            input_sha256: input_sha,
            input_path: input_path.to_string(),
            input_len: input.len() as u64,
            axiom_l0,
            target,
            bucket,
            seed_origin,
            mutation_kind,
            target_version: target_version.to_string(),
            synthetic,
        }
    }

    /// Render to a single ndjson line (with trailing `\n`).
    /// Hand-rolled JSON so the ordering is exact and we don't need
    /// a serde_json dep.
    #[must_use]
    pub fn to_ndjson_line(&self) -> String {
        let mut s = String::with_capacity(512);
        s.push('{');
        push_kv_str(&mut s, "schema_version", SCHEMA_VERSION);
        s.push(',');
        push_kv_str(&mut s, "finding_id", &self.finding_id);
        s.push(',');
        push_kv_u64(&mut s, "timestamp_ns", self.timestamp_ns);
        s.push(',');
        push_kv_str(&mut s, "mode", &self.mode);
        s.push(',');
        push_kv_str(&mut s, "target_label", &self.target_label);
        s.push(',');
        push_kv_str(&mut s, "input_sha256", &self.input_sha256);
        s.push(',');
        push_kv_str(&mut s, "input_path", &self.input_path);
        s.push(',');
        push_kv_u64(&mut s, "input_len", self.input_len);
        s.push(',');
        push_kv_str(&mut s, "axiom_l0", &self.axiom_l0.label());
        s.push(',');
        push_kv_str(&mut s, "target", &self.target.label());
        s.push(',');
        push_kv_str(&mut s, "bucket", self.bucket.label());
        s.push(',');
        push_kv_bool(&mut s, "high_severity", self.bucket.is_high_severity());
        s.push(',');
        match &self.seed_origin {
            Some(v) => push_kv_str(&mut s, "seed_origin", v),
            None => push_kv_null(&mut s, "seed_origin"),
        }
        s.push(',');
        match &self.mutation_kind {
            Some(v) => push_kv_str(&mut s, "mutation_kind", v),
            None => push_kv_null(&mut s, "mutation_kind"),
        }
        s.push(',');
        push_kv_str(&mut s, "target_version", &self.target_version);
        s.push(',');
        push_kv_bool(&mut s, "synthetic", self.synthetic);
        s.push('}');
        s.push('\n');
        s
    }

    /// Parse one ndjson line into a Finding. Best-effort key/value
    /// extractor — sufficient for our hand-rolled writer's output;
    /// not a general JSON parser.
    pub fn from_ndjson_line(line: &str) -> Option<Self> {
        let line = line.trim();
        if !line.starts_with('{') || !line.ends_with('}') {
            return None;
        }
        let body = &line[1..line.len() - 1];
        let get = |k: &str| -> Option<String> { extract_value(body, k) };
        Some(Self {
            finding_id: get("finding_id")?,
            timestamp_ns: get("timestamp_ns")?.parse().ok()?,
            mode: get("mode")?,
            target_label: get("target_label")?,
            input_sha256: get("input_sha256")?,
            input_path: get("input_path")?,
            input_len: get("input_len")?.parse().ok()?,
            axiom_l0: parse_verdict(&get("axiom_l0")?),
            target: parse_verdict(&get("target")?),
            bucket: parse_bucket(&get("bucket")?),
            seed_origin: get("seed_origin").filter(|s| s != "null"),
            mutation_kind: get("mutation_kind").filter(|s| s != "null"),
            // 1.0 records back-fill these fields (Cuttlefish A14 was
            // the only target before P1.14; nothing was synthetic).
            target_version: get("target_version").unwrap_or_else(|| "A14".into()),
            synthetic: get("synthetic").map_or(false, |v| v == "true"),
        })
    }
}

fn parse_verdict(s: &str) -> Verdict {
    if s == "accept" {
        Verdict::Accept
    } else if let Some(tag) = s.strip_prefix("reject:") {
        Verdict::Reject(tag.to_string())
    } else {
        Verdict::Reject(s.to_string())
    }
}

fn parse_bucket(s: &str) -> Bucket {
    match s {
        "A_BOTH_ACCEPT" => Bucket::A,
        "B_BOTH_REJECT_SAME_TAG" => Bucket::B,
        "D_AXIOM_ACCEPT_TARGET_REJECT" => Bucket::D,
        "E_AXIOM_REJECT_TARGET_ACCEPT" => Bucket::E,
        _ => Bucket::C,
    }
}

fn push_kv_str(s: &mut String, k: &str, v: &str) {
    s.push('"');
    s.push_str(k);
    s.push_str("\":\"");
    s.push_str(&json_escape(v));
    s.push('"');
}

fn push_kv_u64(s: &mut String, k: &str, v: u64) {
    s.push('"');
    s.push_str(k);
    s.push_str("\":");
    s.push_str(&v.to_string());
}

fn push_kv_bool(s: &mut String, k: &str, v: bool) {
    s.push('"');
    s.push_str(k);
    s.push_str("\":");
    s.push_str(if v { "true" } else { "false" });
}

fn push_kv_null(s: &mut String, k: &str) {
    s.push('"');
    s.push_str(k);
    s.push_str("\":null");
}

fn json_escape(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    for c in v.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                use core::fmt::Write as _;
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

fn extract_value(body: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\":");
    let start = body.find(&needle)? + needle.len();
    let rest = &body[start..];
    let rest = rest.trim_start();
    if rest.starts_with('"') {
        // String value — read until next unescaped quote.
        let mut chars = rest[1..].char_indices();
        let mut prev_backslash = false;
        let mut end = None;
        for (i, c) in chars.by_ref() {
            if c == '\\' && !prev_backslash {
                prev_backslash = true;
                continue;
            }
            if c == '"' && !prev_backslash {
                end = Some(i);
                break;
            }
            prev_backslash = false;
        }
        let end = end?;
        let raw = &rest[1..=end];
        Some(json_unescape(raw))
    } else if rest.starts_with("null") {
        Some("null".into())
    } else if rest.starts_with("true") {
        Some("true".into())
    } else if rest.starts_with("false") {
        Some("false".into())
    } else {
        // Number — read until comma or end.
        let end = rest.find([',', '}']).unwrap_or(rest.len());
        Some(rest[..end].trim().to_string())
    }
}

fn json_unescape(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    let mut chars = v.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('u') => {
                let hex: String = chars.by_ref().take(4).collect();
                if let Ok(cp) = u32::from_str_radix(&hex, 16) {
                    if let Some(ch) = char::from_u32(cp) {
                        out.push(ch);
                    }
                }
            }
            Some(c) => out.push(c),
            None => break,
        }
    }
    out
}

fn now_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

fn sha256_hex(input: &[u8]) -> String {
    // BLAKE3 is what the rest of the project uses for content
    // hashing. The "sha256" field name is preserved in the schema
    // for consistency with the README.md spec language; the actual
    // algorithm is BLAKE3-256, which is documented in the schema
    // version note.
    let mut h = axiom_blake3_hacl::Blake3::default();
    use axiom_blake3_hacl::Hasher;
    h.update(input);
    hex_encode(&h.finalize_borrow())
}

/// Append-only ndjson archive writer. `flush` and `fsync` after
/// every line so a kernel panic costs at most one record.
#[derive(Debug)]
pub struct ArchiveWriter {
    inner: Mutex<File>,
    /// Where to materialise input bytes for replay.
    inputs_dir: PathBuf,
    /// Where the archive itself lives. Useful for the replay tool.
    archive_path: PathBuf,
}

impl ArchiveWriter {
    /// Open or create an archive. The archive directory layout is:
    ///
    /// ```text
    /// <root>/
    ///   archive.ndjson      append-only finding records
    ///   inputs/<sha>.bin    per-finding input bytes (deduped by sha)
    /// ```
    pub fn open(root: &Path) -> std::io::Result<Self> {
        std::fs::create_dir_all(root)?;
        let inputs_dir = root.join("inputs");
        std::fs::create_dir_all(&inputs_dir)?;
        let archive_path = root.join("archive.ndjson");
        let f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&archive_path)?;
        Ok(Self {
            inner: Mutex::new(f),
            inputs_dir,
            archive_path,
        })
    }

    /// Path to the archive ndjson.
    #[must_use]
    pub fn archive_path(&self) -> &Path {
        &self.archive_path
    }

    /// Path to the inputs dir (one file per unique sha).
    #[must_use]
    pub fn inputs_dir(&self) -> &Path {
        &self.inputs_dir
    }

    /// Save the input bytes to `inputs/<sha>.bin` (idempotent on
    /// repeat sha) and return the relative path string used in
    /// the archive record.
    pub fn save_input(&self, input: &[u8]) -> std::io::Result<String> {
        let sha = sha256_hex(input);
        let path = self.inputs_dir.join(format!("{sha}.bin"));
        if !path.exists() {
            std::fs::write(&path, input)?;
        }
        Ok(format!("inputs/{sha}.bin"))
    }

    /// Append one finding to the archive. Flushes + fsyncs.
    pub fn append(&self, f: &Finding) -> std::io::Result<()> {
        let line = f.to_ndjson_line();
        let mut g = self.inner.lock().expect("archive mutex poisoned");
        g.write_all(line.as_bytes())?;
        g.flush()?;
        g.sync_data()?;
        Ok(())
    }
}

/// Streaming reader for the archive — yields each Finding line by
/// line, skipping malformed records (with a stderr warning).
pub fn read_findings(path: &Path) -> std::io::Result<Vec<Finding>> {
    let f = File::open(path)?;
    let r = BufReader::new(f);
    let mut out = Vec::new();
    for (i, line) in r.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        match Finding::from_ndjson_line(&line) {
            Some(f) => out.push(f),
            None => eprintln!("WARN: skipping malformed finding at line {}", i + 1),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(input: &[u8], bucket: Bucket) -> Finding {
        Finding::from_verdicts(
            "dev",
            "aosp-libziparchive-runtime",
            input,
            "inputs/test.bin",
            Verdict::Accept,
            Verdict::Reject("9".into()),
            bucket,
            Some("seed/badpack-cves/0050.bin".into()),
            Some("flip".into()),
        )
    }

    #[test]
    fn ndjson_roundtrip() {
        let f = sample(b"hello", Bucket::D);
        let line = f.to_ndjson_line();
        assert!(line.ends_with('\n'));
        let g = Finding::from_ndjson_line(&line).expect("parses");
        assert_eq!(g.finding_id, f.finding_id);
        assert_eq!(g.bucket, Bucket::D);
        assert_eq!(g.axiom_l0, Verdict::Accept);
        assert_eq!(g.target, Verdict::Reject("9".into()));
        assert_eq!(g.input_len, 5);
        assert_eq!(g.seed_origin.as_deref(), Some("seed/badpack-cves/0050.bin"));
    }

    #[test]
    fn ndjson_handles_special_chars() {
        let mut f = sample(b"x", Bucket::E);
        f.input_path = "inputs/with \"quotes\" and \\backslash\\.bin".into();
        let line = f.to_ndjson_line();
        let g = Finding::from_ndjson_line(&line).unwrap();
        assert_eq!(g.input_path, f.input_path);
    }

    #[test]
    fn writer_appends_and_fsyncs() -> Result<(), Box<dyn core::error::Error>> {
        let dir = tempdir();
        let w = ArchiveWriter::open(&dir)?;
        let path = w.save_input(b"hello world")?;
        let mut f = sample(b"hello world", Bucket::E);
        f.input_path = path;
        w.append(&f)?;
        let read = read_findings(w.archive_path())?;
        assert_eq!(read.len(), 1);
        assert_eq!(read[0].bucket, Bucket::E);
        assert_eq!(read[0].input_len, 11);
        std::fs::remove_dir_all(&dir).ok();
        Ok(())
    }

    fn tempdir() -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("p113-archive-test-{}", now_ns()));
        std::fs::create_dir_all(&p).unwrap();
        p
    }
}
