// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Cross-version probe registry — lets the driver run the same
//! input through multiple Android-version targets and emit
//! cross-version disagreements as findings (P1.14 §B taxonomy
//! row "AOSP CVE candidate / cross-version evasion").
//!
//! ## Two probe sources
//!
//! 1. **Real probes** — separate `zip-aosp-runtime-probe`-shaped
//!    binaries built against vendored A8 / A11 / A14 libziparchive
//!    sources. Spawned exactly like the existing primary probe
//!    (length-prefixed `--archive-runtime-server` protocol). On a
//!    KVM-enabled host the operator builds these from the matching
//!    AOSP trees; their paths are passed as
//!    `--probes A14:target/probe-a14,A11:target/probe-a11,A8:target/probe-a8`.
//!
//! 2. **Synthetic probes** — for hosts without KVM and without
//!    A8/A11 libziparchive source builds (this dev box), a thin
//!    Rust wrapper applies a documented filter list on top of the
//!    real A14 probe to model historical AOSP behavioural deltas.
//!    Used **only** for end-to-end validation of the cross-version
//!    classifier infrastructure; flagged in archive output via the
//!    `synthetic = true` field so downstream filters can excise
//!    them when real probes are wired in.
//!
//! The synthetic deltas are intentionally narrow and well-grounded:
//!
//! | Version | Synthetic delta vs A14 |
//! |---|---|
//! | A14 | (none — pass-through) |
//! | A11 | reject inputs containing the ZIP64 EOCD-locator signature `PK\x06\x07` (older libziparchive's ZIP64 path was less permissive about locator placement) |
//! | A8  | A11 deltas + reject inputs whose CDR general-purpose bit 11 (UTF-8 filename, RFC 7159) is set on any entry (Oreo predates UTF-8 filename support) |
//!
//! These are documented as approximate stand-ins, NOT historically
//! exact. A real differential needs the real A8/A11 builds (§C-1).
//! The stand-ins exist so the rest of the pipeline (scheduler,
//! classifier, archive schema, dashboard) can be validated end-to-
//! end on this host.

use std::path::Path;
use std::time::Duration;

use crate::classifier::Verdict;
use crate::probe::PersistentProbe;

/// Android target version label. Mirrors the AOSP letter naming
/// (Oreo=A8, RedVelvetCake=A11, UpsideDownCake=A14).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AndroidVersion {
    /// Android 8 (Oreo) — ZIP64 + UTF-8-filename deltas vs A14.
    A8,
    /// Android 11 (RedVelvetCake) — ZIP64-locator delta vs A14.
    A11,
    /// Android 14 (UpsideDownCake) — current-generation baseline.
    A14,
}

impl AndroidVersion {
    /// Stable string label used in archive `target_version` field.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            AndroidVersion::A8 => "A8",
            AndroidVersion::A11 => "A11",
            AndroidVersion::A14 => "A14",
        }
    }

    /// Parse from the CSV form (`A8`, `A11`, `A14`).
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "A8" | "a8" => Some(AndroidVersion::A8),
            "A11" | "a11" => Some(AndroidVersion::A11),
            "A14" | "a14" => Some(AndroidVersion::A14),
            _ => None,
        }
    }
}

/// One probe in the cross-version registry. Wraps a real
/// `PersistentProbe` plus an optional synthetic-filter layer.
pub struct VersionedProbe {
    /// Stable label for the archive `target_label` field.
    pub label: String,
    /// Android version this probe stands in for.
    pub version: AndroidVersion,
    /// Underlying persistent probe. For real probes this is the
    /// per-version binary; for synthetic probes it's the A14 base
    /// probe shared across all entries.
    pub base: PersistentProbe,
    /// True iff the verdict is post-filtered by a synthetic rule.
    pub synthetic: bool,
}

impl VersionedProbe {
    /// Real probe — distinct binary per version. The archive flags
    /// this as `synthetic = false` so cross-version disagreements
    /// it surfaces are honest.
    pub fn real(
        version: AndroidVersion,
        binary: &Path,
        timeout: Duration,
    ) -> std::io::Result<Self> {
        let label = format!("aosp-libziparchive-{}", version.label().to_lowercase());
        let base = PersistentProbe::spawn(&label, binary)?.with_timeout(timeout);
        Ok(Self {
            label,
            version,
            base,
            synthetic: false,
        })
    }

    /// Synthetic probe — wraps an existing A14 base probe and
    /// applies the documented per-version filter list. Used only
    /// for classifier validation where real per-version binaries
    /// aren't available (no-KVM dev hosts).
    #[must_use]
    pub fn synthetic_layer(version: AndroidVersion, base: PersistentProbe) -> Self {
        let label = format!("synthetic-{}", version.label().to_lowercase());
        Self {
            label,
            version,
            base,
            synthetic: true,
        }
    }

    /// Run one input through this probe and apply any synthetic
    /// post-filter. The base probe's verdict is preserved for
    /// real probes; synthetic probes apply their version's filter
    /// rules and may downgrade an Accept to a Reject.
    pub fn run_one(&self, input: &[u8]) -> std::io::Result<Verdict> {
        let base_verdict = self.base.run_one(input)?;
        if !self.synthetic {
            return Ok(base_verdict);
        }
        Ok(apply_synthetic_rules(self.version, &base_verdict, input))
    }

    /// Number of inputs the underlying base probe killed by the
    /// per-call watchdog (Gap-10 / D'-2 closure).
    #[must_use]
    pub fn timed_out(&self) -> u64 {
        self.base.timed_out()
    }
}

impl std::fmt::Debug for VersionedProbe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VersionedProbe")
            .field("label", &self.label)
            .field("version", &self.version)
            .field("synthetic", &self.synthetic)
            .finish()
    }
}

/// Apply the documented per-version synthetic rule list to a
/// base A14 verdict. See module docs for the rule table.
fn apply_synthetic_rules(version: AndroidVersion, base: &Verdict, input: &[u8]) -> Verdict {
    match version {
        AndroidVersion::A14 => base.clone(),
        AndroidVersion::A11 => {
            if has_zip64_locator(input) {
                Verdict::Reject("synthetic-a11:zip64-locator".into())
            } else {
                base.clone()
            }
        }
        AndroidVersion::A8 => {
            if has_zip64_locator(input) {
                Verdict::Reject("synthetic-a8:zip64-locator".into())
            } else if has_utf8_filename_flag(input) {
                Verdict::Reject("synthetic-a8:utf8-filename-flag".into())
            } else {
                base.clone()
            }
        }
    }
}

/// True iff the input contains the ZIP64 EOCD-locator signature
/// `PK\x06\x07` (0x07064b50 LE). Conservative byte-search; the
/// real A11 path would scan the structured CDR/EOCD region only.
fn has_zip64_locator(input: &[u8]) -> bool {
    input.windows(4).any(|w| w == b"PK\x06\x07")
}

/// True iff any LFH or CDR entry in the input has general-purpose
/// bit 11 (UTF-8 filename) set. Bit 11 is at byte offset
/// `+0x06` (LFH) or `+0x08` (CDR) of the record, low byte; the
/// flag is bit 0x08 in `flags >> 8`. We do a permissive scan: any
/// LFH signature `PK\x03\x04` whose +6/+7 word has bit 11 set, or
/// any CDR signature `PK\x01\x02` whose +8/+9 word has bit 11 set,
/// trips the rule.
fn has_utf8_filename_flag(input: &[u8]) -> bool {
    // LFH: signature at +0, generalFlags at +6 (u16 LE).
    for i in 0..input.len().saturating_sub(8) {
        if &input[i..i + 4] == b"PK\x03\x04" {
            let flags = u16::from_le_bytes([input[i + 6], input[i + 7]]);
            if flags & 0x0800 != 0 {
                return true;
            }
        }
    }
    // CDR: signature at +0, generalFlags at +8 (u16 LE).
    for i in 0..input.len().saturating_sub(10) {
        if &input[i..i + 4] == b"PK\x01\x02" {
            let flags = u16::from_le_bytes([input[i + 8], input[i + 9]]);
            if flags & 0x0800 != 0 {
                return true;
            }
        }
    }
    false
}

/// Parse the `--probes` CSV. Format:
///
/// ```text
///   A14:path/to/probe-a14,A11:path/to/probe-a11,A8:path/to/probe-a8
/// ```
///
/// Returns `(version, path)` pairs. Unknown version labels are
/// ignored (with a warning to stderr).
#[must_use]
pub fn parse_probes_csv(csv: &str) -> Vec<(AndroidVersion, std::path::PathBuf)> {
    let mut out = Vec::new();
    for entry in csv.split(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        let (label, path) = match entry.split_once(':') {
            Some(p) => p,
            None => {
                eprintln!("WARN ignoring --probes entry without ':' separator: {entry}");
                continue;
            }
        };
        match AndroidVersion::parse(label) {
            Some(v) => out.push((v, std::path::PathBuf::from(path))),
            None => eprintln!("WARN ignoring --probes entry with unknown version: {label}"),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_parse_round_trip() {
        for v in [AndroidVersion::A8, AndroidVersion::A11, AndroidVersion::A14] {
            assert_eq!(AndroidVersion::parse(v.label()), Some(v));
        }
        assert!(AndroidVersion::parse("A99").is_none());
    }

    #[test]
    fn zip64_locator_detected() {
        let mut buf = vec![0u8; 100];
        buf[40..44].copy_from_slice(b"PK\x06\x07");
        assert!(has_zip64_locator(&buf));
        // Without the marker, no detection.
        let clean = vec![0u8; 100];
        assert!(!has_zip64_locator(&clean));
    }

    #[test]
    fn utf8_filename_flag_detected_in_lfh() {
        // Build a minimal LFH with bit 11 set in generalFlags.
        let mut buf = Vec::new();
        buf.extend_from_slice(b"PK\x03\x04"); // signature
        buf.extend_from_slice(&20u16.to_le_bytes()); // versionNeeded
        buf.extend_from_slice(&0x0800u16.to_le_bytes()); // generalFlags bit 11
        // pad
        buf.extend(std::iter::repeat(0u8).take(20));
        assert!(has_utf8_filename_flag(&buf));
    }

    #[test]
    fn utf8_filename_flag_detected_in_cdr() {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"PK\x01\x02"); // signature
        buf.extend_from_slice(&20u16.to_le_bytes()); // versionMadeBy
        buf.extend_from_slice(&20u16.to_le_bytes()); // versionNeeded
        buf.extend_from_slice(&0x0800u16.to_le_bytes()); // generalFlags bit 11
        buf.extend(std::iter::repeat(0u8).take(40));
        assert!(has_utf8_filename_flag(&buf));
    }

    #[test]
    fn synthetic_a8_reject_zip64() {
        let mut buf = vec![0u8; 80];
        buf[10..14].copy_from_slice(b"PK\x06\x07");
        let v = apply_synthetic_rules(AndroidVersion::A8, &Verdict::Accept, &buf);
        assert!(matches!(v, Verdict::Reject(s) if s.contains("zip64")));
    }

    #[test]
    fn synthetic_a14_passthrough() {
        let buf = vec![0u8; 50];
        let v = apply_synthetic_rules(AndroidVersion::A14, &Verdict::Accept, &buf);
        assert!(matches!(v, Verdict::Accept));
        let r = Verdict::Reject("aosp:-3".into());
        let v = apply_synthetic_rules(AndroidVersion::A14, &r, &buf);
        assert_eq!(v, r);
    }

    #[test]
    fn parse_probes_csv_basic() {
        let pairs = parse_probes_csv("A14:p14,A11:p11,A8:p8");
        assert_eq!(pairs.len(), 3);
        assert_eq!(pairs[0].0, AndroidVersion::A14);
        assert_eq!(pairs[0].1.to_str().unwrap(), "p14");
        assert_eq!(pairs[2].0, AndroidVersion::A8);
    }

    #[test]
    fn parse_probes_csv_skips_malformed() {
        let pairs = parse_probes_csv("A14:p14,malformed,A99:bad");
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, AndroidVersion::A14);
    }
}
