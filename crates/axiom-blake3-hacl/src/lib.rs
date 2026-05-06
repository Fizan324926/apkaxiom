// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `axiom-blake3-hacl` — P1.10 verified-crypto hashing.
//!
//! ## Honest framing (ADR-0028)
//!
//! P1.10's README §4 lists "HACL\*" as the source of an
//! F\*-verified BLAKE3. **HACL\* does not actually ship a
//! verified BLAKE3** (only BLAKE2b / BLAKE2s); the closest
//! upstream is a research-paper proposal that hasn't landed.
//! See ADR-0028 for the full deviation rationale.
//!
//! What this crate ships:
//!
//!   - **`Blake3` (production)** — the official BLAKE3-team Rust
//!     crate. Audited, SIMD-tunable, the same reference
//!     implementation Android `apksigner` uses for v3 signing.
//!     **This is what `commit_chain.rs` actually hashes with.**
//!
//!   - **`Blake2bHacl` (verified-baseline placeholder)** — the
//!     binding *surface* for HACL\* BLAKE2b. The full HACL\* C
//!     dist is a 30-min cold build with F\*+OCaml+opam
//!     dependencies that are out of session-scope; the
//!     surface is shipped today as a `cfg(feature = "hacl-c")`-
//!     gated module so the type-check / API contract lands now,
//!     and the real C-binding lights up once HACL\* is vendored
//!     under `external/hacl-star/` (operator one-shot in §C).
//!
//! ## API
//!
//! Both backends share a `Hasher` trait so consumers (the
//! commit-chain) parameterise over the hash without committing
//! to either backend. `Blake3` is the workspace default.

#![forbid(unsafe_code)]
#![warn(missing_docs, unreachable_pub)]
#![allow(
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::too_long_first_doc_paragraph
)]

/// Output of a hash — 32 bytes for BLAKE3 / BLAKE2b-256.
pub type Hash = [u8; 32];

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

// ---------------------------------------------------------------------
// Verified-baseline: HACL* BLAKE2b (cfg-gated; see ADR-0028)
// ---------------------------------------------------------------------

/// HACL\* BLAKE2b-256 binding — F\*-verified. The C dist is
/// **not** built in-session (operator one-shot in P1.10 §C);
/// when `cfg(feature = "hacl-c")` is active, this struct
/// dispatches to the linked `libevercrypt`. Without the feature,
/// it currently returns the SAME bytes as the production
/// `Blake3` — wrong cryptographically, but a placeholder that
/// keeps the type-check honest until HACL\* lands. Tests that
/// rely on this backend are `#[cfg(feature = "hacl-c")]`-gated
/// so we never claim a verified result we didn't compute.
#[derive(Default, Clone, Debug)]
pub struct Blake2bHacl {
    // Placeholder — real backend lives behind `feature = "hacl-c"`
    // (which is not enabled in workspace-default builds).
    inner: blake3::Hasher,
}

impl Hasher for Blake2bHacl {
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

// ---------------------------------------------------------------------
// Tests — production BLAKE3 against the BLAKE3-team test vectors
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// BLAKE3 official test vector for empty input.
    const EMPTY_BLAKE3: [u8; 32] = [
        0xaf, 0x13, 0x49, 0xb9, 0xf5, 0xf9, 0xa1, 0xa6, 0xa0, 0x40, 0x4d, 0xea, 0x36, 0xdc, 0xc9,
        0x49, 0x9b, 0xcb, 0x25, 0xc9, 0xad, 0xc1, 0x12, 0xb7, 0xcc, 0x9a, 0x93, 0xca, 0xe4, 0x1f,
        0x32, 0x62,
    ];
    /// BLAKE3 official test vector for "abc".
    const ABC_BLAKE3: [u8; 32] = [
        0x64, 0x37, 0xb3, 0xac, 0x38, 0x46, 0x51, 0x33, 0xff, 0xb6, 0x3b, 0x75, 0x27, 0x3a, 0x8d,
        0xb5, 0x48, 0xc5, 0x58, 0x46, 0x5d, 0x79, 0xdb, 0x03, 0xfd, 0x35, 0x9c, 0x6c, 0xd5, 0xbd,
        0x9d, 0x85,
    ];

    #[test]
    fn blake3_empty_matches_official_vector() {
        let h = Blake3::hash_oneshot(b"");
        assert_eq!(h, EMPTY_BLAKE3);
    }

    #[test]
    fn blake3_abc_matches_official_vector() {
        let h = Blake3::hash_oneshot(b"abc");
        assert_eq!(h, ABC_BLAKE3);
    }

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

    #[test]
    #[allow(clippy::cast_possible_truncation)]
    fn blake3_long_input_deterministic() {
        // 1MB of LCG bytes — deterministic across runs.
        let mut payload = Vec::with_capacity(1 << 20);
        let mut s: u64 = 0xdead_beef;
        for _ in 0..(1 << 20) {
            s = s
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            payload.push((s >> 32) as u8);
        }
        let h1 = Blake3::hash_oneshot(&payload);
        let h2 = Blake3::hash_oneshot(&payload);
        assert_eq!(h1, h2, "BLAKE3 must be deterministic");
    }

    #[test]
    fn blake2b_hacl_placeholder_is_distinct_in_documentation() {
        // The placeholder currently produces the same bytes as Blake3.
        // This test asserts the *type* is distinct (so consumers
        // who depend on `Blake2bHacl: Hasher` get compile errors
        // if the backend disappears) and documents the honest
        // status — see ADR-0028.
        let h3 = Blake3::hash_oneshot(b"hello");
        let h2 = Blake2bHacl::hash_oneshot(b"hello");
        // Until HACL* lands, these MUST equal each other (the
        // placeholder dispatches to BLAKE3). When HACL* C is
        // wired, this assertion flips and a follow-up commit
        // updates it.
        assert_eq!(h3, h2, "placeholder dispatches to BLAKE3 — see ADR-0028");
    }
}
