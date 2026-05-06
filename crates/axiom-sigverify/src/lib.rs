// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `axiom-sigverify` — APK Signature Scheme **verifier**.
//!
//! P1.11 state-of-the-art closure G3 — the missing crypto layer.
//! This crate is what was abstract `CryptoOracle` in the Lean
//! verifier predicate; here we wire real implementations:
//!
//!   - **SHA-256 chunked digest** (1 MiB chunks, the spec's
//!     `0xa5 || u32_le(len) || chunk` then `0x5a || u32_le(N) ||
//!     concat(digests)`).
//!   - **X.509 SPKI extraction** from leaf certs (via `x509-cert`).
//!   - **Signature verification** routing on `SignatureAlgorithmId`:
//!     RSA-PKCS1-v1.5 (`sha2` + `rsa`), RSA-PSS (`rsa::pss`),
//!     ECDSA-P256 (`p256::ecdsa`), Ed25519 (`ed25519_dalek`).
//!   - **Cross-binding**: signed_data's leaf cert's SPKI must equal
//!     the signer's `public_key` field (the v2/v3/v3.1 invariant
//!     against confused-deputy attacks).
//!
//! Integration: every signer of every present scheme on every APK
//! in the corpus runs through this verifier. The differential
//! harness (`tools/p111-differential-rs`) compares this verdict
//! against `apksigner verify` byte-for-byte; PASS = perfect
//! agreement.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::collapsible_if,
    clippy::collapsible_else_if,
    clippy::manual_let_else,
    clippy::single_match_else,
    clippy::similar_names,
    clippy::too_long_first_doc_paragraph,
    clippy::manual_strip,
    clippy::or_fun_call,
    clippy::uninlined_format_args,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::match_same_arms,
    clippy::missing_const_for_fn
)]

pub mod chunked_digest;
pub mod scheme_v1;
pub mod scheme_v2;
pub mod scheme_v3;

/// Combined APK verifier — mirrors AOSP `apksigner verify`.
///
/// Decision logic (per AOSP `ApkVerifier`):
///
///   1. If the APK has a signing-block magic at `cd_offset - 16`
///      (whether or not the block parses), the v2/v3/v3.1
///      dispatcher's verdict is binding — `v2-block-present-must-
///      verify`.
///   2. **Downgrade detection** via two independent signals:
///      - v1 `.SF` header `X-Android-APK-Signed: <scheme-ids>`
///        lists the schemes the APK was originally signed with;
///        every listed scheme MUST verify.
///      - v2/v3 `additional_attributes` contains
///        `0xbeeff00d` (v2→v3 stripping-protection): if present,
///        v3 (or v3.1) must verify.
///   3. Otherwise (no signing block at all), v1 (JAR) verification
///      is the floor.
///   4. If both v1 and v23x are present, BOTH must accept.
///
/// Returns the first reject reason encountered, otherwise `Accept`.
#[must_use]
pub fn verify_apk(apk: &[u8]) -> Verdict {
    let block_bytes_exist = has_signing_block_magic(apk);
    let v23 = scheme_v3::dispatch_verify(apk);
    if block_bytes_exist && !matches!(v23, Verdict::Accept) {
        return v23;
    }
    // Downgrade signal #1: v1 .SF header.
    if let Some(claimed_schemes) = read_x_android_apk_signed(apk) {
        if claimed_schemes.contains(&2)
            && axiom_sigblock::locate(apk)
                .ok()
                .flatten()
                .and_then(|b| b.v2().map(|_| ()))
                .is_none()
        {
            return Verdict::Reject(RejectReason::AlgorithmError(
                "v1 .SF claims v2 but v2 block missing — downgrade".into(),
            ));
        }
        if claimed_schemes.contains(&3) {
            let has_v3 = axiom_sigblock::locate(apk)
                .ok()
                .flatten()
                .and_then(|b| b.v3().map(|_| ()))
                .is_some();
            if !has_v3 {
                return Verdict::Reject(RejectReason::AlgorithmError(
                    "v1 .SF claims v3 but v3 block missing — downgrade".into(),
                ));
            }
        }
    }
    // Downgrade signal #2: v2 stripping-protection attribute.
    if let Ok(Some(block)) = axiom_sigblock::locate(apk) {
        if let Some(v2_bytes) = block.v2() {
            if let Ok(signers) = axiom_sigblock::scheme::parse_v2(v2_bytes) {
                for s in &signers {
                    for attr in &s.additional_attributes {
                        if attr.id == 0xbeef_f00d {
                            if block.v3().is_none() && block.v3_1().is_none() {
                                return Verdict::Reject(RejectReason::AlgorithmError(
                                    "v2 stripping-protection attr present but v3/v3.1 missing — downgrade".into(),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
    let v1 = scheme_v1::verify(apk);
    match (&v1, &v23) {
        (Verdict::Accept, Verdict::Accept | Verdict::NotPresent) => Verdict::Accept,
        (Verdict::NotPresent, Verdict::Accept) => Verdict::Accept,
        (Verdict::Accept, Verdict::Malformed(_) | Verdict::Reject(_)) => v23,
        (other, _) => other.clone(),
    }
}

/// Read the `.SF` file's `X-Android-APK-Signed: <id>,<id>...`
/// header, if present. Returns the list of scheme IDs claimed
/// by the v1 signing material.
fn read_x_android_apk_signed(apk: &[u8]) -> Option<Vec<u32>> {
    let entries = scheme_v1::walk_entries(apk).ok()?;
    let sf = entries
        .iter()
        .find(|e| e.name.starts_with(b"META-INF/") && e.name.ends_with(b".SF"))?;
    let (main, _) = scheme_v1::parse_manifest(&sf.body);
    let header = main.get("X-Android-APK-Signed")?;
    let ids: Vec<u32> = header
        .split(',')
        .filter_map(|s| s.trim().parse::<u32>().ok())
        .collect();
    Some(ids)
}

/// True iff bytes 16 before EOCD's `cd_offset` field contain the
/// APK signing-block magic. Used by `verify_apk`'s downgrade
/// detection — if the block magic is present, the v2/v3
/// dispatcher's verdict is binding even when the parser rejects.
fn has_signing_block_magic(apk: &[u8]) -> bool {
    let Some(eocd_off) = find_eocd(apk) else {
        return false;
    };
    if eocd_off + 22 > apk.len() {
        return false;
    }
    let cd_offset = u32::from_le_bytes(
        apk[eocd_off + 16..eocd_off + 20]
            .try_into()
            .unwrap_or([0u8; 4]),
    ) as usize;
    if cd_offset < 16 || cd_offset > apk.len() {
        return false;
    }
    apk[cd_offset - 16..cd_offset] == *axiom_sigblock::MAGIC
}

fn find_eocd(bytes: &[u8]) -> Option<usize> {
    if bytes.len() < 22 {
        return None;
    }
    let mut i = bytes.len() - 22;
    loop {
        if u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap()) == 0x0605_4b50 {
            return Some(i);
        }
        if i == 0 {
            return None;
        }
        i -= 1;
    }
}

use axiom_sigblock::scheme::SignatureAlgorithmId;

/// Verdict for a single scheme on a single APK.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// All signers passed every check.
    Accept,
    /// The scheme block isn't present.
    NotPresent,
    /// The scheme block is present but malformed (parser rejected).
    Malformed(String),
    /// At least one signer failed cryptographic verification.
    Reject(RejectReason),
}

/// Granular reject reasons. Each maps to a Lean
/// `V*VerifyResult` reject category for cross-language interop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RejectReason {
    /// Signer has zero digests.
    NoDigests,
    /// Signer has zero signatures.
    NoSignatures,
    /// Signer has zero certificates.
    NoCertificates,
    /// Digest set and signature set disagree on algorithm IDs.
    AlgorithmMismatch,
    /// Recomputed chunked digest doesn't match the declared digest.
    DigestMismatch {
        /// Algorithm ID whose digest mismatched.
        algorithm_id: u32,
    },
    /// Signature didn't verify under the leaf cert's public key.
    SignatureFailed {
        /// Algorithm ID whose signature failed.
        algorithm_id: u32,
    },
    /// signed_data's leaf-cert SPKI ≠ signer's `public_key` field.
    PublicKeyMismatch,
    /// Every algorithm ID in the signer is unknown to the verifier
    /// (stripped-algorithm attack).
    AllAlgorithmsUnknown,
    /// SPKI couldn't be extracted from the leaf cert.
    BadCertificate,
    /// Algorithm-specific runtime error (key format, etc.).
    AlgorithmError(String),
}

/// Errors during the whole-APK verification flow (separate from
/// per-signer reject reasons).
#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    /// Signing-block parse error.
    #[error("signing-block parse: {0}")]
    Block(#[from] axiom_sigblock::ParseError),
    /// Scheme-internal parse error.
    #[error("scheme parse: {0}")]
    Scheme(#[from] axiom_sigblock::scheme::SchemeError),
}

/// True iff the algorithm is one of the chunked-SHA-256 family
/// supported by this verifier.
#[must_use]
pub fn is_supported(alg: SignatureAlgorithmId) -> bool {
    matches!(
        alg,
        SignatureAlgorithmId::RsaPkcs1Sha256
            | SignatureAlgorithmId::RsaPssSha256
            | SignatureAlgorithmId::EcdsaSha256
    )
}
