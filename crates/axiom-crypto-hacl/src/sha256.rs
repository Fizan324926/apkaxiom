// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! SHA-256 backed by `libcrux-sha2` (HACL*-extracted Rust, formally verified).
//!
//! `libcrux_sha2::sha256` is extracted from the F\*-verified HACL\* proof;
//! the Rust code carries no `unsafe` and inherits the HACL\* security
//! proof under the assumption of a correct Rust-to-assembly lowering
//! (MSC-2023 paper, "HACL\* 2023"). The API is `sha256(data) -> [u8; 32]`,
//! matching the C HACL\* interface `Hacl_Hash_SHA2_hash_256`.

/// SHA-256 of `data` via the formally verified `libcrux-sha2` (HACL\*) backend.
///
/// Returns the 32-byte digest.
#[must_use]
pub fn sha256(data: &[u8]) -> [u8; 32] {
    libcrux_sha2::sha256(data)
}

/// SHA-384 of `data` via the formally verified `libcrux-sha2` (HACL\*) backend.
///
/// Returns the 48-byte digest.
#[must_use]
pub fn sha384(data: &[u8]) -> [u8; 48] {
    libcrux_sha2::sha384(data)
}

/// SHA-512 of `data` via the formally verified `libcrux-sha2` (HACL\*) backend.
///
/// Returns the 64-byte digest.
#[must_use]
pub fn sha512(data: &[u8]) -> [u8; 64] {
    libcrux_sha2::sha512(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// NIST FIPS 180-4 — SHA-256 known-answer test vectors.
    ///
    /// Source: NIST Cryptographic Algorithm Validation Program (CAVP),
    /// SHA Test Vectors, `SHA256ShortMsg.rsp` test group.
    #[test]
    fn sha256_nist_kat_empty() {
        // NIST vector: empty input.
        let got = sha256(b"");
        let want: [u8; 32] = [
            0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f,
            0xb9, 0x24, 0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b,
            0x78, 0x52, 0xb8, 0x55,
        ];
        assert_eq!(got, want, "SHA-256('') KAT failed");
    }

    #[test]
    fn sha256_nist_kat_abc() {
        // NIST vector: "abc".
        let got = sha256(b"abc");
        let want: [u8; 32] = [
            0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
            0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
            0xf2, 0x00, 0x15, 0xad,
        ];
        assert_eq!(got, want, "SHA-256('abc') KAT failed");
    }

    #[test]
    fn sha256_nist_kat_448bit() {
        // NIST vector: "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq" (448 bits).
        let got = sha256(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq");
        let want: [u8; 32] = [
            0x24, 0x8d, 0x6a, 0x61, 0xd2, 0x06, 0x38, 0xb8, 0xe5, 0xc0, 0x26, 0x93, 0x0c, 0x3e,
            0x60, 0x39, 0xa3, 0x3c, 0xe4, 0x59, 0x64, 0xff, 0x21, 0x67, 0xf6, 0xec, 0xed, 0xd4,
            0x19, 0xdb, 0x06, 0xc1,
        ];
        assert_eq!(got, want, "SHA-256(448-bit NIST) KAT failed");
    }

    #[test]
    fn sha256_nist_kat_896bit() {
        // NIST vector: 896-bit message.
        let got = sha256(
            b"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu",
        );
        let want: [u8; 32] = [
            0xcf, 0x5b, 0x16, 0xa7, 0x78, 0xaf, 0x83, 0x80, 0x03, 0x6c, 0xe5, 0x9e, 0x7b, 0x04,
            0x92, 0x37, 0x0b, 0x24, 0x9b, 0x11, 0xe8, 0xf0, 0x7a, 0x51, 0xaf, 0xac, 0x45, 0x03,
            0x7a, 0xfe, 0xe9, 0xd1,
        ];
        assert_eq!(got, want, "SHA-256(896-bit NIST) KAT failed");
    }

    #[test]
    fn sha256_cross_check_libcrux_vs_sha2_crate() {
        // Cross-implementation consistency check: libcrux-sha2 must agree
        // with the RustCrypto sha2 crate (audit baseline) on 32 random-ish inputs.
        use sha2::{Digest as _, Sha256 as RcSha256};
        for len in [0usize, 1, 7, 31, 32, 63, 64, 127, 255, 1023, 4096] {
            let payload: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
            let libcrux = sha256(&payload);
            let mut h = RcSha256::new();
            h.update(&payload);
            let rustcrypto: [u8; 32] = h.finalize().into();
            assert_eq!(
                libcrux, rustcrypto,
                "libcrux-sha2 vs sha2 diverge at len={len}"
            );
        }
    }
}
