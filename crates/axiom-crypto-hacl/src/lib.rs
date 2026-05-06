// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `axiom-crypto-hacl` — unified crypto surface for APKAXIOM.
//!
//! Routes each primitive to the best-available formally verified
//! implementation. SHA-256, Ed25519, and ECDSA-P256 go through
//! `libcrux` (HACL\*-extracted Rust). RSA is an honest deviation
//! (no verified RSA exists in libcrux; documented in CHECKLIST §A).
//!
//! ## Primitive routing
//!
//! | Primitive | Backend | Formal proof |
//! |---|---|---|
//! | SHA-256/384/512 | `libcrux-sha2 0.0.6` | HACL\* (F\*) |
//! | Ed25519 verify | `libcrux-ed25519 0.0.7` | HACL\* (F\*) |
//! | ECDSA-P256 verify | `libcrux-ecdsa 0.0.6` | HACL\* (F\*) |
//! | RSA-PKCS1 verify | RustCrypto `rsa 0.9.7` | Audited (NCC 2023) |
//! | RSA-PSS verify | RustCrypto `rsa 0.9.7` | Audited (NCC 2023) |
//! | BLAKE3 | `blake3 1.5.5` via `axiom-blake3-hacl` | No verified BLAKE3 in HACL\* (ADR-0028) |
//!
//! ## BLAKE3 re-export
//!
//! This crate re-exports the BLAKE3 surface from `axiom-blake3-hacl`
//! so callers can import a single crate for all APKAXIOM crypto
//! primitives.

#![forbid(unsafe_code)]
#![warn(missing_docs, unreachable_pub)]
#![allow(
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::too_long_first_doc_paragraph,
    clippy::module_name_repetitions,
    clippy::missing_panics_doc
)]

pub mod ecdsa;
pub mod ed25519;
pub mod rsa_compat;
pub mod sha256;

// Re-export BLAKE3 surface.
pub use axiom_blake3_hacl as blake3_hacl;

// Re-export the most common primitives at the top level for ergonomic use.

/// SHA-256 via HACL\* (`libcrux-sha2`). Returns the 32-byte digest.
#[must_use]
pub fn sha256(data: &[u8]) -> [u8; 32] {
    sha256::sha256(data)
}

/// Ed25519 verification via HACL\* (`libcrux-ed25519`).
///
/// - `public_key`: 32-byte Ed25519 public key.
/// - `message`: signed payload.
/// - `signature`: 64-byte signature.
///
/// Returns `true` iff valid.
#[must_use]
pub fn ed25519_verify(public_key: &[u8; 32], message: &[u8], signature: &[u8; 64]) -> bool {
    ed25519::ed25519_verify(public_key, message, signature)
}

/// ECDSA-P256 verification via HACL\* (`libcrux-ecdsa`) over a pre-hashed digest.
///
/// - `public_key_uncompressed`: 65-byte uncompressed SEC1 point (`0x04 ‖ X ‖ Y`).
/// - `digest`: 32-byte SHA-256 digest of the signed payload.
/// - `r`, `s`: 32-byte big-endian signature components.
///
/// Returns `true` iff valid.
#[must_use]
pub fn ecdsa_p256_verify(
    public_key_uncompressed: &[u8; 65],
    digest: &[u8; 32],
    r: &[u8; 32],
    s: &[u8; 32],
) -> bool {
    ecdsa::ecdsa_p256_verify(public_key_uncompressed, digest, r, s)
}

/// RSA-PKCS1-v1.5 + SHA-256 verification (honest deviation — RustCrypto).
///
/// - `spki_der`: SubjectPublicKeyInfo DER.
/// - `signed_data`: verbatim signed bytes (not the pre-hash).
/// - `signature`: raw RSA signature bytes.
///
/// Returns `true` iff valid.
#[must_use]
pub fn rsa_pkcs1_verify_sha256(spki_der: &[u8], signed_data: &[u8], signature: &[u8]) -> bool {
    rsa_compat::rsa_pkcs1_verify_sha256(spki_der, signed_data, signature)
}

/// RSA-PSS + SHA-256 verification (honest deviation — RustCrypto).
///
/// - `spki_der`: SubjectPublicKeyInfo DER.
/// - `signed_data`: verbatim signed bytes.
/// - `signature`: raw RSA-PSS signature bytes.
///
/// Returns `true` iff valid.
#[must_use]
pub fn rsa_pss_verify_sha256(spki_der: &[u8], signed_data: &[u8], signature: &[u8]) -> bool {
    rsa_compat::rsa_pss_verify_sha256(spki_der, signed_data, signature)
}
