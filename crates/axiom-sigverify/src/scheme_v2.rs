// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Real verifier for APK Signature Scheme v2.
//!
//! This is the production crypto layer the Lean verifier
//! predicate is parameterised over. We support:
//!
//!   - **RSA-PKCS1-v1.5 + SHA-256** (algorithm `0x0103`).
//!   - **RSA-PSS + SHA-256** (algorithm `0x0101`).
//!   - **ECDSA over P-256 + SHA-256** (algorithm `0x0201`).
//!
//! Per AOSP `tools/apksig/V2SchemeVerifier.java` the digest is
//! the chunked SHA-256 over `zip_entries || central_directory ||
//! patched_eocd`. The signature is computed over the verbatim
//! `signed_data` bytes (NOT the digest input — `signed_data`
//! itself contains the digest).
//!
//! Verifier soundness invariant (per spec):
//!
//!   1. Every signer must have at least one (digest, signature)
//!      pair with a *known* algorithm ID.
//!   2. The digest declared in `signed_data` must equal the
//!      recomputed chunked SHA-256 of the digest input.
//!   3. Each signature must verify under the leaf cert's public
//!      key over the verbatim `signed_data` bytes.
//!   4. The public key embedded in `signed_data.public_key` must
//!      equal the leaf cert's SubjectPublicKeyInfo DER (the
//!      cross-binding gate against confused-deputy attacks).

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::needless_pass_by_value
)]

use axiom_sigblock::scheme::{SignatureAlgorithmId, Signer};

use crate::chunked_digest::apk_chunked_sha256;
use crate::{RejectReason, Verdict};

/// Verify every signer in the v2 block. Returns `Accept` iff every
/// signer passes; otherwise the first reject reason.
pub fn verify_signers(
    signers: &[Signer],
    apk: &[u8],
    signing_block_start: u64,
    _signing_block_end: u64,
    cd_offset: u64,
    eocd_offset: u64,
) -> Verdict {
    if signers.is_empty() {
        return Verdict::Reject(RejectReason::NoDigests);
    }
    let digest = apk_chunked_sha256(apk, signing_block_start, cd_offset, eocd_offset);
    for signer in signers {
        match verify_one_signer(signer, &digest) {
            Verdict::Accept => {}
            other => return other,
        }
    }
    Verdict::Accept
}

fn verify_one_signer(signer: &Signer, digest_input_sha256: &[u8; 32]) -> Verdict {
    if signer.digests.is_empty() {
        return Verdict::Reject(RejectReason::NoDigests);
    }
    if signer.signatures.is_empty() {
        return Verdict::Reject(RejectReason::NoSignatures);
    }
    if signer.certificates.is_empty() {
        return Verdict::Reject(RejectReason::NoCertificates);
    }
    // Cross-bind: leaf cert's SPKI DER must equal signer.public_key.
    let leaf_cert = &signer.certificates[0];
    match extract_spki_der(leaf_cert) {
        Some(spki) => {
            if spki != signer.public_key {
                return Verdict::Reject(RejectReason::PublicKeyMismatch);
            }
        }
        None => return Verdict::Reject(RejectReason::BadCertificate),
    }
    // Algorithm-pairing: digest set ↔ signature set on algorithm IDs.
    let dig_ids: std::collections::BTreeSet<u32> =
        signer.digests.iter().map(|d| d.algorithm_id).collect();
    let sig_ids: std::collections::BTreeSet<u32> =
        signer.signatures.iter().map(|s| s.algorithm_id).collect();
    if dig_ids != sig_ids {
        return Verdict::Reject(RejectReason::AlgorithmMismatch);
    }
    // All-unknown gate.
    if signer.digests.iter().all(|d| d.algorithm.is_none())
        && signer.signatures.iter().all(|s| s.algorithm.is_none())
    {
        return Verdict::Reject(RejectReason::AllAlgorithmsUnknown);
    }
    // For each known algorithm, verify digest + signature.
    for sig_entry in &signer.signatures {
        let alg = match sig_entry.algorithm {
            Some(a) => a,
            None => continue, // unknown — rule §1 already filtered all-unknown
        };
        // Find paired digest.
        let dig = signer
            .digests
            .iter()
            .find(|d| d.algorithm_id == sig_entry.algorithm_id);
        let dig = match dig {
            Some(d) => d,
            None => return Verdict::Reject(RejectReason::AlgorithmMismatch),
        };
        // Recompute digest from input under the algorithm's hash kind.
        let recomputed: Vec<u8> = match alg.digest_kind() {
            axiom_sigblock::scheme::DigestKind::Sha256 => digest_input_sha256.to_vec(),
            axiom_sigblock::scheme::DigestKind::Sha512 => {
                // Spec requires a separate SHA-512 chunked digest;
                // we don't precompute it here — the corpus uses
                // SHA-256 algorithms (0x0103) so this branch is
                // exercised only by SHA-512 fixtures we don't ship.
                return Verdict::Reject(RejectReason::AlgorithmError(
                    "SHA-512 chunked digest not yet wired".into(),
                ));
            }
        };
        if recomputed != dig.digest {
            return Verdict::Reject(RejectReason::DigestMismatch {
                algorithm_id: sig_entry.algorithm_id,
            });
        }
        // Signature verification.
        let ok = verify_signature(
            alg,
            &signer.public_key,
            &signer.signed_data,
            &sig_entry.signature,
        );
        match ok {
            Ok(true) => {}
            Ok(false) => {
                return Verdict::Reject(RejectReason::SignatureFailed {
                    algorithm_id: sig_entry.algorithm_id,
                })
            }
            Err(e) => return Verdict::Reject(RejectReason::AlgorithmError(e)),
        }
    }
    Verdict::Accept
}

/// Extract the verbatim SubjectPublicKeyInfo DER from an X.509
/// leaf certificate's DER bytes. Returns `None` on parse error.
pub fn extract_spki_der(leaf_cert_der: &[u8]) -> Option<Vec<u8>> {
    use der::Decode;
    use der::Encode;
    let cert = x509_cert::Certificate::from_der(leaf_cert_der).ok()?;
    let spki = cert.tbs_certificate.subject_public_key_info;
    spki.to_der().ok()
}

/// Verify a single signature under an algorithm + public-key DER.
pub fn verify_signature(
    alg: SignatureAlgorithmId,
    public_key_spki_der: &[u8],
    signed_data: &[u8],
    signature: &[u8],
) -> Result<bool, String> {
    match alg {
        SignatureAlgorithmId::RsaPkcs1Sha256 => {
            verify_rsa_pkcs1_sha256(public_key_spki_der, signed_data, signature)
        }
        SignatureAlgorithmId::RsaPssSha256 => {
            verify_rsa_pss_sha256(public_key_spki_der, signed_data, signature)
        }
        SignatureAlgorithmId::EcdsaSha256 => {
            verify_ecdsa_sha256(public_key_spki_der, signed_data, signature)
        }
        SignatureAlgorithmId::RsaPssSha512
        | SignatureAlgorithmId::RsaPkcs1Sha512
        | SignatureAlgorithmId::EcdsaSha512
        | SignatureAlgorithmId::DsaSha256
        | SignatureAlgorithmId::VerityRsaPkcs1Sha256
        | SignatureAlgorithmId::VerityEcdsaSha256
        | SignatureAlgorithmId::VerityDsaSha256 => Err(format!(
            "algorithm {:?} not yet wired (operator one-shot)",
            alg
        )),
    }
}

fn verify_rsa_pkcs1_sha256(
    spki_der: &[u8],
    signed_data: &[u8],
    signature: &[u8],
) -> Result<bool, String> {
    use rsa::pkcs1v15::{Signature, VerifyingKey};
    use rsa::pkcs8::DecodePublicKey;
    use rsa::signature::Verifier;
    let pk = rsa::RsaPublicKey::from_public_key_der(spki_der)
        .map_err(|e| format!("RSA public key: {e}"))?;
    let vk: VerifyingKey<sha2::Sha256> = VerifyingKey::new(pk);
    let sig = Signature::try_from(signature).map_err(|e| format!("RSA signature: {e}"))?;
    Ok(vk.verify(signed_data, &sig).is_ok())
}

fn verify_rsa_pss_sha256(
    spki_der: &[u8],
    signed_data: &[u8],
    signature: &[u8],
) -> Result<bool, String> {
    use rsa::pkcs8::DecodePublicKey;
    use rsa::pss::{Signature, VerifyingKey};
    use rsa::signature::Verifier;
    let pk = rsa::RsaPublicKey::from_public_key_der(spki_der)
        .map_err(|e| format!("RSA-PSS public key: {e}"))?;
    let vk: VerifyingKey<sha2::Sha256> = VerifyingKey::new(pk);
    let sig = Signature::try_from(signature).map_err(|e| format!("RSA-PSS signature: {e}"))?;
    Ok(vk.verify(signed_data, &sig).is_ok())
}

fn verify_ecdsa_sha256(
    spki_der: &[u8],
    signed_data: &[u8],
    signature: &[u8],
) -> Result<bool, String> {
    use p256::ecdsa::signature::Verifier;
    use p256::ecdsa::{DerSignature, VerifyingKey};
    use p256::pkcs8::DecodePublicKey;
    let vk = VerifyingKey::from_public_key_der(spki_der)
        .map_err(|e| format!("ECDSA public key: {e}"))?;
    let sig = DerSignature::try_from(signature).map_err(|e| format!("ECDSA signature: {e}"))?;
    Ok(vk.verify(signed_data, &sig).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn read_fixture(rel: &str) -> Vec<u8> {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join(rel);
        std::fs::read(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
    }

    #[test]
    fn verify_real_v2_signed_apk_accepts() {
        let apk = read_fixture("corpus/signing/v1-v2/wifiautoff-v1v2.apk");
        let block = axiom_sigblock::locate(&apk).unwrap().expect("v2 block");
        let v2_bytes = block.v2().expect("v2");
        let signers = axiom_sigblock::scheme::parse_v2(v2_bytes).expect("parse v2");
        // Locate EOCD + signing-block bounds.
        let eocd_off = find_eocd(&apk).expect("eocd");
        let cd_offset =
            u32::from_le_bytes(apk[eocd_off + 16..eocd_off + 20].try_into().unwrap()) as u64;
        let sb_start = block.block_offset;
        let sb_end = cd_offset;
        let verdict = verify_signers(&signers, &apk, sb_start, sb_end, cd_offset, eocd_off as u64);
        assert!(
            matches!(verdict, Verdict::Accept),
            "v2 verifier on real APK: {:?}",
            verdict
        );
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
}
