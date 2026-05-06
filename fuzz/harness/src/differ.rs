// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Differential driver — runs both arms on a single input and
//! returns a verdict pair.
//!
//! In **dev** mode the target arm is the AOSP libziparchive
//! runtime probe (`target/zip-aosp-runtime-probe`, P1.6). In
//! **real** mode it would be the Cuttlefish CVD via Nyx; that
//! path is unavailable on hosts without `/dev/kvm` (CHECKLIST
//! §C-1) and gated behind `nyx-cuttlefish`.

use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};

use axiom_l0_zip_verified::consistency::{parse_archive, ArchiveError};

use crate::classifier::{classify, Bucket, Verdict};

/// Stable string-tag for `ArchiveError`. Mirrors the parser's
/// `tag()` method but returns a human-readable form so the
/// classifier's `B`-bucket comparison is stable across the
/// `(axiom-l0, AOSP)` pair (both serialise to integer codes
/// already, but the human name helps Grafana panels).
fn axiom_tag(e: ArchiveError) -> String {
    let tag = e.tag();
    let name = match e {
        ArchiveError::NoEocd => "NoEocd",
        ArchiveError::EocdInvalid => "EocdInvalid",
        ArchiveError::CdOutOfRange => "CdOutOfRange",
        ArchiveError::CdrInvalid => "CdrInvalid",
        ArchiveError::CdrCountMismatch => "CdrCountMismatch",
        ArchiveError::LfhOffsetOob => "LfhOffsetOob",
        ArchiveError::LfhInvalid => "LfhInvalid",
        ArchiveError::FilenameMismatch => "FilenameMismatch",
        ArchiveError::FieldMismatch => "FieldMismatch",
        ArchiveError::EocdTooFarFromEof => "EocdTooFarFromEof",
        ArchiveError::CdAfterEocd => "CdAfterEocd",
        ArchiveError::InvalidEntryName => "InvalidEntryName",
        _ => "Unknown",
    };
    format!("{tag}:{name}")
}

/// Run the verified L0 ZIP layer on `input`.
#[must_use]
pub fn run_axiom(input: &[u8]) -> Verdict {
    match parse_archive(input) {
        Ok(_) => Verdict::Accept,
        Err(e) => Verdict::Reject(axiom_tag(e)),
    }
}

/// Run the AOSP libziparchive runtime probe on `input`. Returns
/// `Verdict::Accept` on `ok ...` stdout and
/// `Verdict::Reject(<aosp-zip-error-code>)` on `err 100 <code>`.
///
/// `probe` is the path to the compiled `zip-aosp-runtime-probe`
/// binary. Build it with `make p16-aosp-runtime-probe`.
pub fn run_aosp_runtime(probe: &Path, input: &[u8], timeout: Duration) -> std::io::Result<Verdict> {
    let mut child = Command::new(probe)
        .arg("--archive-runtime")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input)?;
    }
    // Best-effort timeout: poll wait_with_output and kill on
    // expiry. We don't pull tokio in for this — a polling loop
    // is sufficient at the harness's iteration frequency.
    let start = std::time::Instant::now();
    loop {
        match child.try_wait()? {
            Some(_) => break,
            None => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    return Err(std::io::Error::other("aosp probe timeout"));
                }
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }
    let out = child.wait_with_output()?;
    let s = String::from_utf8_lossy(&out.stdout);
    if s.starts_with("ok ") {
        Ok(Verdict::Accept)
    } else if s.starts_with("err ") {
        // Format: `err <reject-tag> <aosp-code>`
        let rest = s.trim_end().strip_prefix("err ").unwrap_or("");
        let code = rest.split_whitespace().nth(1).unwrap_or("?");
        Ok(Verdict::Reject(format!("aosp:{code}")))
    } else {
        Err(std::io::Error::other(format!(
            "unrecognised probe output: {}",
            s.lines().next().unwrap_or("")
        )))
    }
}

/// Run both arms and classify. Returns the verdict pair plus the
/// classifier bucket. Errors only on I/O failure of the target
/// probe (axiom-l0 is in-process and infallible at this layer).
pub fn run_diff(
    input: &[u8],
    probe: &Path,
    timeout: Duration,
) -> std::io::Result<(Verdict, Verdict, Bucket)> {
    let axiom = run_axiom(input);
    let target = run_aosp_runtime(probe, input, timeout)?;
    let bucket = classify(&axiom, &target);
    Ok((axiom, target, bucket))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classifier::Bucket;

    /// Minimal well-formed archive (98 bytes) — `parse_archive` must accept.
    fn minimal_archive() -> Vec<u8> {
        // Same shape as `axiom-l0-zip-verified::tests::minimal_archive`.
        // Inlined to keep this test self-contained.
        let mut v = Vec::with_capacity(98);
        // LFH 30 bytes
        v.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        v.extend_from_slice(&[0x14, 0x00]);
        v.extend_from_slice(&[0u8; 20]);
        v.extend_from_slice(&0u16.to_le_bytes());
        v.extend_from_slice(&0u16.to_le_bytes());
        // CDR 46 bytes
        v.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        v.extend_from_slice(&[0x14, 0x00, 0x14, 0x00]);
        v.extend_from_slice(&[0u8; 8]);
        v.extend_from_slice(&[0u8; 12]);
        v.extend_from_slice(&0u16.to_le_bytes());
        v.extend_from_slice(&0u16.to_le_bytes());
        v.extend_from_slice(&0u16.to_le_bytes());
        v.extend_from_slice(&[0u8; 8]);
        v.extend_from_slice(&0u32.to_le_bytes());
        // EOCD 22 bytes
        v.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        v.extend_from_slice(&[0u8; 4]);
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&46u32.to_le_bytes());
        v.extend_from_slice(&30u32.to_le_bytes());
        v.extend_from_slice(&0u16.to_le_bytes());
        v
    }

    #[test]
    fn run_axiom_accepts_minimal() {
        assert_eq!(run_axiom(&minimal_archive()), Verdict::Accept);
    }

    #[test]
    fn run_axiom_rejects_garbage() {
        let v = run_axiom(b"not a zip");
        assert!(matches!(v, Verdict::Reject(_)));
    }

    #[test]
    fn axiom_tag_is_stable() {
        assert_eq!(axiom_tag(ArchiveError::FieldMismatch), "9:FieldMismatch");
        assert_eq!(axiom_tag(ArchiveError::NoEocd), "1:NoEocd");
    }

    #[test]
    fn classify_smoke() {
        let acc = Verdict::Accept;
        let rej = Verdict::Reject("3:CdOutOfRange".into());
        assert_eq!(classify(&acc, &acc), Bucket::A);
        assert_eq!(classify(&acc, &rej), Bucket::D);
        assert_eq!(classify(&rej, &acc), Bucket::E);
        assert_eq!(classify(&rej, &rej), Bucket::B);
    }
}
