// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `axiom-blake3-hacl` — APKAXIOM's BLAKE3 hashing surface.
//!
//! ## Honest framing (ADR-0028)
//!
//! The crate name preserves an aspirational link to HACL\* (the
//! F\*-verified crypto library), but **HACL\* does not currently
//! ship a verified BLAKE3** (it covers BLAKE2b/2s, SHA-2/3,
//! ChaCha20-Poly1305, Curve25519, P-256, Ed25519). The
//! upstream-proposed verified BLAKE3 is a research paper that
//! has not landed. See [ADR-0028](../../../docs/phase-1/P1.10/ADR-0028-hacl-blake3-deviation.md).
//!
//! Earlier drafts shipped a `Blake2bHacl` placeholder type that
//! dispatched to BLAKE3 — that was a Potemkin abstraction (a
//! `Hasher` named after BLAKE2b/HACL\* that was neither). It is
//! deleted. This crate now ships exactly one backend:
//!
//!   - **`Blake3`** — the official BLAKE3-team Rust crate
//!     (`blake3 = 1.5.5`, `pure` feature). Audited, the same
//!     reference implementation Android `apksigner` uses for v3
//!     signing. This is the one and only hash backend
//!     `commit_chain.rs` builds against.
//!
//! When HACL\* upstream merges a verified BLAKE3, this crate
//! grows a `Blake3Hacl` backend behind a real Cargo feature and
//! an honest `Hasher`-trait routing layer. Until then we ship
//! one truthful backend and refuse to fake the second.

#![forbid(unsafe_code)]
#![warn(missing_docs, unreachable_pub)]
#![allow(
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::too_long_first_doc_paragraph
)]

#[allow(missing_docs)]
pub mod vectors;

#[allow(missing_docs)]
pub mod cross_impl;

/// Output of a hash — 32 bytes for BLAKE3.
pub type Hash = [u8; 32];

/// Generate the canonical BLAKE3 test-vector input for `len` bytes:
/// `[i % 251 for i in 0..len]`. Matches the BLAKE3-team's
/// `paint_test_input` helper used by the official suite.
#[must_use]
pub fn paint_test_input(len: usize) -> Vec<u8> {
    // `i % 251 < 256`, so the `as u8` truncation is safe by construction.
    #[allow(clippy::cast_possible_truncation)]
    (0..len).map(|i| (i % 251) as u8).collect()
}

/// Common hashing surface implemented by every backend.
pub trait Hasher: Default {
    /// Update the hasher with a byte slice. Streaming-friendly.
    fn update(&mut self, bytes: &[u8]);
    /// Finalise and return the 32-byte digest.
    fn finalize(self) -> Hash;
    /// One-shot convenience — `update` once and `finalize`.
    fn hash_oneshot(bytes: &[u8]) -> Hash {
        let mut h = Self::default();
        h.update(bytes);
        h.finalize()
    }
}

// ---------------------------------------------------------------------
// Production: BLAKE3 (BLAKE3-team Rust reference)
// ---------------------------------------------------------------------

/// BLAKE3 hasher backed by the official BLAKE3-team Rust crate
/// (`blake3 = 1.5.5`, audited, the same code path Android
/// `apksigner` uses for v3 signing).
#[derive(Default, Clone, Debug)]
pub struct Blake3 {
    inner: blake3::Hasher,
}

impl Hasher for Blake3 {
    fn update(&mut self, bytes: &[u8]) {
        self.inner.update(bytes);
    }
    fn finalize(self) -> Hash {
        let h = self.inner.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(h.as_bytes());
        out
    }
}

impl Blake3 {
    /// Reset the hasher's internal state so the same allocation
    /// can be reused for another digest. Saves a `Hasher::new()`
    /// allocation on the hot per-leaf path.
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    /// Finalise without consuming `self`. Combined with
    /// [`Self::reset`] this enables hot-loop reuse — significantly
    /// faster than `default(); update; finalize` per leaf when
    /// committing many small regions (LFH/CDR/EOCD).
    #[must_use]
    pub fn finalize_borrow(&self) -> Hash {
        let h = self.inner.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(h.as_bytes());
        out
    }
}

/// Keyed BLAKE3 — used by the BLAKE3-team's official
/// `keyed_hash` test-vector suite and useful for downstream
/// consumers that want a per-tenant root key.
#[must_use]
pub fn blake3_keyed_oneshot(key: &[u8; 32], bytes: &[u8]) -> Hash {
    let mut h = blake3::Hasher::new_keyed(key);
    h.update(bytes);
    let mut out = [0u8; 32];
    out.copy_from_slice(h.finalize().as_bytes());
    out
}

/// BLAKE3 `derive_key` mode — used by the BLAKE3-team's official
/// `derive_key` test-vector suite. `context` is a non-secret
/// global identifier; `key_material` is hashed under that
/// context to produce the 32-byte derived key.
#[must_use]
pub fn blake3_derive_key_oneshot(context: &str, key_material: &[u8]) -> Hash {
    let mut h = blake3::Hasher::new_derive_key(context);
    h.update(key_material);
    let mut out = [0u8; 32];
    out.copy_from_slice(h.finalize().as_bytes());
    out
}

/// XOF (extendable output) variant of BLAKE3 hash. Used by the
/// BLAKE3-team's official vector suite, which specifies a 1056-byte
/// output beyond the standard 32-byte digest.
pub fn blake3_xof_oneshot(bytes: &[u8], out: &mut [u8]) {
    let mut h = blake3::Hasher::new();
    h.update(bytes);
    let mut reader = h.finalize_xof();
    reader.fill(out);
}

/// XOF variant of `keyed_hash`. Mirrors the BLAKE3 reference
/// `keyed_hash_xof` test vector.
pub fn blake3_keyed_xof_oneshot(key: &[u8; 32], bytes: &[u8], out: &mut [u8]) {
    let mut h = blake3::Hasher::new_keyed(key);
    h.update(bytes);
    let mut reader = h.finalize_xof();
    reader.fill(out);
}

/// XOF variant of `derive_key`. Mirrors the BLAKE3 reference
/// `derive_key_xof` test vector.
pub fn blake3_derive_key_xof_oneshot(context: &str, key_material: &[u8], out: &mut [u8]) {
    let mut h = blake3::Hasher::new_derive_key(context);
    h.update(key_material);
    let mut reader = h.finalize_xof();
    reader.fill(out);
}

// ---------------------------------------------------------------------
// Tests — production BLAKE3 against the BLAKE3-team test vectors
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::vectors::{VECTORS, VECTOR_CONTEXT, VECTOR_KEY};
    use super::*;

    #[test]
    fn blake3_streaming_matches_oneshot() {
        let payload = b"the quick brown fox jumps over the lazy dog".repeat(100);
        let oneshot = Blake3::hash_oneshot(&payload);
        let mut h = Blake3::default();
        for chunk in payload.chunks(7) {
            h.update(chunk);
        }
        assert_eq!(h.finalize(), oneshot);
    }

    /// All 35 official BLAKE3 vectors, `hash` mode, 32-byte digest.
    #[test]
    fn official_hash_digest_all_35() {
        for v in VECTORS {
            let input = paint_test_input(v.input_len);
            let got = Blake3::hash_oneshot(&input);
            let want = &v.hash_xof[..32];
            assert_eq!(
                got.as_slice(),
                want,
                "input_len={} hash digest mismatch",
                v.input_len
            );
        }
    }

    /// All 35 official BLAKE3 vectors, `hash` mode, full 131-byte XOF.
    #[test]
    fn official_hash_xof_all_35() {
        for v in VECTORS {
            let input = paint_test_input(v.input_len);
            let mut got = vec![0u8; v.hash_xof.len()];
            blake3_xof_oneshot(&input, &mut got);
            assert_eq!(got, v.hash_xof, "input_len={} XOF mismatch", v.input_len);
        }
    }

    /// All 35 official BLAKE3 vectors, `keyed_hash` mode, 32-byte digest.
    #[test]
    fn official_keyed_digest_all_35() {
        for v in VECTORS {
            let input = paint_test_input(v.input_len);
            let got = blake3_keyed_oneshot(&VECTOR_KEY, &input);
            let want = &v.keyed_xof[..32];
            assert_eq!(
                got.as_slice(),
                want,
                "input_len={} keyed digest mismatch",
                v.input_len
            );
        }
    }

    /// All 35 official BLAKE3 vectors, `keyed_hash` mode, full 131-byte XOF.
    #[test]
    fn official_keyed_xof_all_35() {
        for v in VECTORS {
            let input = paint_test_input(v.input_len);
            let mut got = vec![0u8; v.keyed_xof.len()];
            blake3_keyed_xof_oneshot(&VECTOR_KEY, &input, &mut got);
            assert_eq!(
                got, v.keyed_xof,
                "input_len={} keyed XOF mismatch",
                v.input_len
            );
        }
    }

    /// All 35 official BLAKE3 vectors, `derive_key` mode, 32-byte digest.
    #[test]
    fn official_derive_key_digest_all_35() {
        for v in VECTORS {
            let input = paint_test_input(v.input_len);
            let got = blake3_derive_key_oneshot(VECTOR_CONTEXT, &input);
            let want = &v.derive_key_xof[..32];
            assert_eq!(
                got.as_slice(),
                want,
                "input_len={} derive_key digest mismatch",
                v.input_len
            );
        }
    }

    /// All 35 official BLAKE3 vectors, `derive_key` mode, full 131-byte XOF.
    #[test]
    fn official_derive_key_xof_all_35() {
        for v in VECTORS {
            let input = paint_test_input(v.input_len);
            let mut got = vec![0u8; v.derive_key_xof.len()];
            blake3_derive_key_xof_oneshot(VECTOR_CONTEXT, &input, &mut got);
            assert_eq!(
                got, v.derive_key_xof,
                "input_len={} derive_key XOF mismatch",
                v.input_len
            );
        }
    }

    /// Streaming `update` in arbitrary chunks reproduces the
    /// one-shot digest on every official-vector input length.
    #[test]
    fn streaming_chunked_matches_oneshot_on_all_35() {
        let chunk_sizes = [1usize, 7, 31, 64, 64 + 1, 1024, 1024 + 1, 4096, 16384];
        for v in VECTORS {
            let input = paint_test_input(v.input_len);
            let oneshot = Blake3::hash_oneshot(&input);
            for &cs in &chunk_sizes {
                let mut h = Blake3::default();
                for chunk in input.chunks(cs) {
                    h.update(chunk);
                }
                assert_eq!(
                    h.finalize(),
                    oneshot,
                    "input_len={} chunk_size={cs} streaming != oneshot",
                    v.input_len
                );
            }
        }
    }
}
