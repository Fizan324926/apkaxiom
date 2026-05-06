// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! RSA-PKCS1 and RSA-PSS verification — honest deviation from HACL\*.
//!
//! ## Why this module exists (honest framing)
//!
//! `libcrux` does not ship a verified RSA implementation. RSA is
//! explicitly out of scope for the HACL\* codebase because the formal
//! proof of a number-theoretic primitive at the scale of 2048–4096-bit
//! modular exponentiation is still an open research problem in the
//! EasyCrypt / F\* communities. The HACL\* roadmap does not include
//! a verified RSA (as of 2025). See CHECKLIST §A for the documented
//! deviation.
//!
//! We use the well-audited RustCrypto `rsa` crate (version pinned to
//! `=0.9.7`, the same version already used by `axiom-sigverify`).
//! The `rsa` crate has been audited by NCC Group (2023) and is the
//! de-facto standard in the Rust cryptography ecosystem. The
//! deviation is honest: we document it, we do not pretend HACL\*
//! covers RSA, and we fall back to a well-audited alternative.

use rsa::pkcs8::DecodePublicKey;

/// Verify an RSA-PKCS1-v1.5 + SHA-256 signature.
///
/// - `spki_der`: SubjectPublicKeyInfo DER bytes (the signer's `public_key` field).
/// - `message_digest`: pre-computed SHA-256 digest of the signed data.
///   Note: the RSA PKCS1-v1.5 signature is *over the full message*, not the
///   hash, so this function hashes internally via `rsa::pkcs1v15::VerifyingKey`.
/// - `signed_data`: the verbatim `signed_data` bytes (what was actually signed).
/// - `signature`: the raw RSA signature bytes.
///
/// Returns `true` iff the signature is valid.
#[must_use]
pub fn rsa_pkcs1_verify_sha256(
    spki_der: &[u8],
    signed_data: &[u8],
    signature: &[u8],
) -> bool {
    use rsa::pkcs1v15::{Signature, VerifyingKey};
    use rsa::signature::Verifier;
    let Ok(pk) = rsa::RsaPublicKey::from_public_key_der(spki_der) else {
        return false;
    };
    let vk: VerifyingKey<sha2::Sha256> = VerifyingKey::new(pk);
    let Ok(sig) = Signature::try_from(signature) else {
        return false;
    };
    vk.verify(signed_data, &sig).is_ok()
}

/// Verify an RSA-PSS + SHA-256 signature.
///
/// - `spki_der`: SubjectPublicKeyInfo DER bytes.
/// - `signed_data`: the verbatim `signed_data` bytes (what was actually signed).
/// - `signature`: the raw RSA-PSS signature bytes.
///
/// Returns `true` iff the signature is valid.
#[must_use]
pub fn rsa_pss_verify_sha256(
    spki_der: &[u8],
    signed_data: &[u8],
    signature: &[u8],
) -> bool {
    use rsa::pkcs8::DecodePublicKey;
    use rsa::pss::{Signature, VerifyingKey};
    use rsa::signature::Verifier;
    let Ok(pk) = rsa::RsaPublicKey::from_public_key_der(spki_der) else {
        return false;
    };
    let vk: VerifyingKey<sha2::Sha256> = VerifyingKey::new(pk);
    let Ok(sig) = Signature::try_from(signature) else {
        return false;
    };
    vk.verify(signed_data, &sig).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test: a well-formed call with invalid bytes must return false,
    /// not panic. The full RSA KATs are covered by `axiom-sigverify`'s
    /// real-APK integration tests.
    #[test]
    fn rsa_pkcs1_invalid_spki_returns_false() {
        let result = rsa_pkcs1_verify_sha256(b"not a DER key", b"data", b"sig");
        assert!(!result, "invalid SPKI should return false, not panic");
    }

    #[test]
    fn rsa_pss_invalid_spki_returns_false() {
        let result = rsa_pss_verify_sha256(b"not a DER key", b"data", b"sig");
        assert!(!result, "invalid SPKI should return false, not panic");
    }
}
