// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Pure-std SHA-256 (NIST FIPS 180-4) used to compute the AXIOM-IR
//! schema-freeze hash and the per-module IR-commitment hash.
//!
//! Why hand-roll instead of pulling a crate? Two reasons:
//!
//! 1. **Hermeticity.** The workspace deliberately keeps third-party deps
//!    minimal (`thiserror` + `syn` + `walkdir` only — see
//!    `third-party/rust/Cargo.toml`). Adding `sha2` here would force a
//!    Reindeer regen for a dependency the rest of the project does not
//!    need until P1.10 (where HACL\*-verified BLAKE3 lands).
//!
//! 2. **Self-validating.** [`sha256`] of a buffer is bit-equal to what
//!    `sha256sum < buffer` produces. Tests in this module pin the
//!    well-known NIST test vectors; the corpus harness verifies an in-tree
//!    ↔ external-`sha256sum` agreement. That gives us a strong "the
//!    canonical-bytes encoding is portably-deterministic" property without
//!    a single extra dep.
//!
//! Performance is not the goal here — the schema-freeze hash runs once per
//! corpus regeneration. P1.10's HACL\*-BLAKE3 owns the perf-critical hashing
//! path.

// Single-letter binders (`a`, `b`, … `h`, `s0`, `s1`, `t1`, `t2`, `ch`,
// `maj`) are FIPS-180-4 spec names; renaming would obscure the cross-reference.
// The `for i in 0..N` loop indices are also spec-faithful.
#![allow(
    clippy::many_single_char_names,
    clippy::needless_range_loop,
    clippy::similar_names,
    clippy::unreadable_literal
)]

use std::fmt::Write as _;

const K: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

const H0: [u32; 8] = [
    0x6a09_e667,
    0xbb67_ae85,
    0x3c6e_f372,
    0xa54f_f53a,
    0x510e_527f,
    0x9b05_688c,
    0x1f83_d9ab,
    0x5be0_cd19,
];

/// Full SHA-256 of `bytes`.
///
/// Hand-rolled NIST FIPS 180-4 implementation. Equal to the output of
/// `sha256sum` for every input — verified against published NIST test
/// vectors plus the round-trip check in `tools/ir-corpus`.
///
/// # Panics
/// The internal `try_into()` for the 64-byte block never fails (we slice
/// exactly 64 bytes from a 128-byte buffer). The unreachable
/// `expect`/`unwrap` is left to catch a future refactor that breaks the
/// invariant.
#[must_use]
pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    let mut state = H0;
    let bit_len = (bytes.len() as u64).wrapping_mul(8);

    // Process all complete 64-byte blocks of the input.
    let mut chunks = bytes.chunks_exact(64);
    for chunk in &mut chunks {
        let mut block = [0u8; 64];
        block.copy_from_slice(chunk);
        compress(&mut state, &block);
    }
    let rem = chunks.remainder();

    // Padding: 1-bit (0x80) + zero-pad + 8-byte big-endian length.
    let mut tail = [0u8; 128];
    tail[..rem.len()].copy_from_slice(rem);
    tail[rem.len()] = 0x80;
    let pad_end = if rem.len() < 56 { 64 } else { 128 };
    tail[pad_end - 8..pad_end].copy_from_slice(&bit_len.to_be_bytes());

    compress(&mut state, &tail[..64].try_into().unwrap());
    if pad_end == 128 {
        compress(&mut state, &tail[64..128].try_into().unwrap());
    }

    let mut out = [0u8; 32];
    for (i, word) in state.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

/// Hex-encode a 32-byte digest. Lowercase, no separators.
#[must_use]
pub fn hex(digest: &[u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for b in digest {
        // u8 hex never fails; unwrap is unreachable.
        write!(&mut out, "{b:02x}").unwrap();
    }
    out
}

fn compress(state: &mut [u32; 8], block: &[u8; 64]) {
    let mut w = [0u32; 64];
    for i in 0..16 {
        let off = i * 4;
        w[i] = u32::from_be_bytes([block[off], block[off + 1], block[off + 2], block[off + 3]]);
    }
    for i in 16..64 {
        let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
        let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
        w[i] = w[i - 16]
            .wrapping_add(s0)
            .wrapping_add(w[i - 7])
            .wrapping_add(s1);
    }

    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;

    for i in 0..64 {
        let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let ch = (e & f) ^ ((!e) & g);
        let t1 = h
            .wrapping_add(s1)
            .wrapping_add(ch)
            .wrapping_add(K[i])
            .wrapping_add(w[i]);
        let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let t2 = s0.wrapping_add(maj);
        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(t1);
        d = c;
        c = b;
        b = a;
        a = t1.wrapping_add(t2);
    }

    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
    state[4] = state[4].wrapping_add(e);
    state[5] = state[5].wrapping_add(f);
    state[6] = state[6].wrapping_add(g);
    state[7] = state[7].wrapping_add(h);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// NIST FIPS 180-4 test vectors plus a couple of RFC-style standards.
    #[test]
    fn sha256_known_vectors() {
        // Standard vectors — every cryptographic library on earth must
        // pass these. If we don't, the implementation is broken.
        let cases: &[(&[u8], &str)] = &[
            (
                b"",
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ),
            (
                b"abc",
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            ),
            (
                b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
                "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
            ),
            (
                b"The quick brown fox jumps over the lazy dog",
                "d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592",
            ),
        ];
        for (input, want) in cases {
            assert_eq!(hex(&sha256(input)), *want, "input={input:?}");
        }
    }

    /// A million 'a' characters — the classic FIPS-180 stress vector that
    /// exercises >1 MiB of `compress()` calls. Confirms padding across the
    /// >2^20 boundary works.
    #[test]
    fn sha256_million_a() {
        let buf = vec![b'a'; 1_000_000];
        assert_eq!(
            hex(&sha256(&buf)),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0",
        );
    }

    #[test]
    fn hex_lowercase_no_separators() {
        let h = hex(&sha256(b"hi"));
        assert_eq!(h.len(), 64);
        assert!(h
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
    }
}
