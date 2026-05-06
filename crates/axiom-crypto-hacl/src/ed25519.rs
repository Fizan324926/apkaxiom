// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Ed25519 signature verification backed by `libcrux-ed25519`
//! (HACL\*-extracted Rust, formally verified).
//!
//! `libcrux_ed25519::verify` is the direct Rust extraction of the
//! HACL\* Ed25519 implementation. The HACL\* proof covers
//! correctness (verify accepts iff (pk, msg, sig) is a valid
//! Ed25519 triple per RFC 8032) and security (EUF-CMA under the
//! Schnorr-framework reduction). The Rust extraction carries no
//! `unsafe` and inherits the proof under the MSC-2023 lowering
//! assumption.
//!
//! API: `libcrux_ed25519::verify(payload, public_key, signature)`
//! where payload is the message, public_key is the 32-byte Ed25519
//! public key, and signature is the 64-byte Ed25519 signature.

/// Verify an Ed25519 signature via `libcrux-ed25519` (HACL\*-backed).
///
/// - `public_key`: 32-byte compressed Edwards point (RFC 8032 §5.1.3).
/// - `message`: signed payload (arbitrary bytes).
/// - `signature`: 64-byte Ed25519 signature (R‖S).
///
/// Returns `true` iff the signature is valid.
#[must_use]
pub fn ed25519_verify(public_key: &[u8; 32], message: &[u8], signature: &[u8; 64]) -> bool {
    // libcrux-ed25519 API: verify(payload, public_key, signature) -> Result<(), Error>
    libcrux_ed25519::verify(message, public_key, signature).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    // RFC 8032 §6.1 test vectors for Ed25519.
    // https://datatracker.ietf.org/doc/html/rfc8032#section-6.1

    fn from_hex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn ed25519_rfc8032_test_vector_1() {
        // RFC 8032, Test Vector 1 (empty message).
        let pk_bytes = from_hex("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a");
        let pk: [u8; 32] = pk_bytes.try_into().unwrap();
        let msg: &[u8] = b"";
        let sig_bytes = from_hex(
            "e5564300c360ac729086e2cc806e828a\
             84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b",
        );
        let sig: [u8; 64] = sig_bytes.try_into().unwrap();
        assert!(
            ed25519_verify(&pk, msg, &sig),
            "RFC 8032 test vector 1 (empty msg) should verify"
        );
    }

    #[test]
    fn ed25519_rfc8032_test_vector_2() {
        // RFC 8032, Test Vector 2 (1-byte message).
        let pk_bytes = from_hex("3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c");
        let pk: [u8; 32] = pk_bytes.try_into().unwrap();
        let msg: &[u8] = &[0x72u8];
        let sig_bytes = from_hex(
            "92a009a9f0d4cab8720e820b5f642540\
             a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00",
        );
        let sig: [u8; 64] = sig_bytes.try_into().unwrap();
        assert!(
            ed25519_verify(&pk, msg, &sig),
            "RFC 8032 test vector 2 (1-byte msg) should verify"
        );
    }

    #[test]
    fn ed25519_bad_signature_rejected() {
        // Flip one bit in the valid RFC 8032 vector 1 signature.
        let pk_bytes = from_hex("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a");
        let pk: [u8; 32] = pk_bytes.try_into().unwrap();
        let msg: &[u8] = b"";
        let mut sig_bytes = from_hex(
            "e5564300c360ac729086e2cc806e828a\
             84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b",
        );
        sig_bytes[0] ^= 0x01; // flip a bit
        let sig: [u8; 64] = sig_bytes.try_into().unwrap();
        assert!(
            !ed25519_verify(&pk, msg, &sig),
            "tampered signature must be rejected"
        );
    }

    #[test]
    fn ed25519_wrong_message_rejected() {
        // Valid key + signature but different message.
        let pk_bytes = from_hex("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a");
        let pk: [u8; 32] = pk_bytes.try_into().unwrap();
        let msg: &[u8] = b"wrong message"; // was empty
        let sig_bytes = from_hex(
            "e5564300c360ac729086e2cc806e828a\
             84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b",
        );
        let sig: [u8; 64] = sig_bytes.try_into().unwrap();
        assert!(
            !ed25519_verify(&pk, msg, &sig),
            "signature over different message must be rejected"
        );
    }
}
