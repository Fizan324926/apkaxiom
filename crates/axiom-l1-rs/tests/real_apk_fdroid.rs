// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
//
// P1.8 §F-3 — real-APK end-to-end test.

#![allow(
    clippy::many_single_char_names,
    clippy::unreadable_literal,
    clippy::needless_range_loop,
    clippy::cast_possible_truncation
)]
//
// Drives the type-state pipeline (`Apk<Unverified>::from_reader →
// verify_v2 → parse_v2`) against a real, signed, F-Droid APK
// (F-Droid Privileged Extension v2050, 39 214 bytes, SHA-256
// 8d0f5f8351617c99f11156199a281dca6d5fd41c4b8bfeb107dfd60f5c954f5c).
// Checks every gate the pipeline enforces:
//
//   - structural ZIP parse over a real DD-flagged APK,
//   - DEFLATE inflate of META-INF/<key>.RSA, AndroidManifest.xml,
//     resources.arsc (the real APK uses method=8 for the first two,
//     method=0 for resources.arsc),
//   - DER PKCS#7 SignedData magic on the inflated v1-signature
//     carrier,
//   - AXML magic on the inflated AndroidManifest.xml,
//   - ARSC magic on the resources.arsc body,
//   - type-state forward progression from Unverified to
//     FullyParsed<V2>, with `signing_variant_tag()` returning 2.
//
// The fixture is checked into the repo under
// `crates/axiom-l1-rs/tests/fixtures/`. License: GPLv3 (F-Droid
// Privileged Extension is GPL-3.0-or-later, repackaged unchanged
// from f-droid.org/repo). The .apk file is a binary blob; do not
// modify it. To replace, `curl -sL
// https://f-droid.org/repo/org.fdroid.fdroid.privileged_2050.apk >
// fdroid-privileged-2050.apk` and update the SHA-256 below.

use axiom_l1_rs::{Apk, Unverified};

const FIXTURE_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/fdroid-privileged-2050.apk"
);

const FIXTURE_SHA256: [u8; 32] = [
    0x8d, 0x0f, 0x5f, 0x83, 0x51, 0x61, 0x7c, 0x99, 0xf1, 0x11, 0x56, 0x19, 0x9a, 0x28, 0x1d, 0xca,
    0x6d, 0x5f, 0xd4, 0x1c, 0x4b, 0x8b, 0xfe, 0xb1, 0x07, 0xdf, 0xd6, 0x0f, 0x5c, 0x95, 0x4f, 0x5c,
];

fn fixture_bytes() -> Vec<u8> {
    let bytes = std::fs::read(FIXTURE_PATH).expect("fixture present");
    let actual = sha256(&bytes);
    assert_eq!(
        actual, FIXTURE_SHA256,
        "fixture sha256 drifted — replace with the canonical APK"
    );
    bytes
}

#[test]
fn real_fdroid_apk_full_pipeline_v2() {
    let bytes = fixture_bytes();
    // Constructor — structural ZIP parse + body capture for the
    // META-INF/*.RSA, AndroidManifest.xml, resources.arsc entries.
    let apk = Apk::<Unverified>::from_reader(bytes.as_slice()).expect("Apk::from_reader");
    // F-Droid privileged extension has 14 entries.
    assert_eq!(apk.entries().len(), 14);
    let names: Vec<&[u8]> = apk
        .entries()
        .iter()
        .map(|e| e.file_name.as_slice())
        .collect();
    assert!(names.contains(&b"META-INF/CIARANG.RSA".as_ref()));
    assert!(names.contains(&b"AndroidManifest.xml".as_ref()));
    assert!(names.contains(&b"resources.arsc".as_ref()));
    assert!(names.iter().any(|n| n.starts_with(b"classes.dex")));

    // Verify v2 — DER probe on the inflated signature carrier.
    let verified = apk.verify_v2().expect("verify_v2 on real APK");
    assert_eq!(verified.signature_block().variant_tag, 2);
    // Inflated CIARANG.RSA is 1342 bytes (the LFH-declared
    // uncompressed size); the inflate-decoder sized it correctly.
    assert_eq!(verified.signature_block().block_bytes.len(), 1342);
    assert!(
        verified.signature_block().block_bytes.starts_with(&[0x30]),
        "DER SEQUENCE tag must lead the PKCS#7 carrier"
    );

    // Parse v2 — AXML + ARSC magic on the inflated bodies.
    let parsed = verified.parse_v2().expect("parse_v2 on real APK");
    let manifest = parsed.manifest();
    let resources = parsed.resources();

    // Real AndroidManifest.xml: RES_XML_TYPE chunk, header size 0x0008.
    assert_eq!(&manifest.axml_bytes[0..4], &[0x03, 0x00, 0x08, 0x00]);
    assert_eq!(manifest.axml_bytes.len(), 2200);

    // Real resources.arsc: RES_TABLE_TYPE chunk, header size 0x000c.
    assert_eq!(&resources.arsc_bytes[0..4], &[0x02, 0x00, 0x0c, 0x00]);
    assert_eq!(resources.arsc_bytes.len(), 5892);

    // Type-witness on FullyParsed<V2> resolves to 2.
    assert_eq!(parsed.signing_variant_tag(), 2);
    assert_eq!(parsed.state_name(), "fully-parsed");
}

#[test]
fn real_fdroid_apk_full_pipeline_v3() {
    // The F-Droid APK only ships a v1 carrier (META-INF/CIARANG.RSA
    // is JAR-style, not v3-specific). Our placeholder verifier
    // accepts it for any V because real cryptographic verification
    // is P1.10's job. The test confirms the type-witness threads
    // through correctly even when the pipeline succeeds for a
    // "wrong" requested variant — runtime semantics are
    // intentionally permissive at this stage.
    let bytes = fixture_bytes();
    let parsed = Apk::<Unverified>::from_reader(bytes.as_slice())
        .unwrap()
        .verify_v3()
        .unwrap()
        .parse_v3()
        .unwrap();
    assert_eq!(parsed.signing_variant_tag(), 3);
}

#[test]
fn real_fdroid_apk_variant_cross_bind_runtime_check() {
    // verify_v2 → parse_v3 must reject at runtime: the chain's
    // type-witness on FullyParsed<V> says V=V3 but the upstream
    // signature_block.variant_tag was stamped 2.
    let bytes = fixture_bytes();
    let result = Apk::<Unverified>::from_reader(bytes.as_slice())
        .unwrap()
        .verify_v2()
        .unwrap()
        .parse_v3();
    assert!(matches!(
        result,
        Err(axiom_l1_rs::ApkError::SignatureVerify { variant_tag: 3, .. })
    ));
}

// ---------------------------------------------------------------------
// Minimal SHA-256 (so the fixture-integrity check has zero deps).
// ---------------------------------------------------------------------

fn sha256(input: &[u8]) -> [u8; 32] {
    // FIPS-180-4 SHA-256. ~80 lines of pure-Rust; safer than
    // pulling a third-party hash crate just for one assertion.
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (input.len() as u64) * 8;
    let mut msg = input.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes(chunk[i * 4..i * 4 + 4].try_into().unwrap());
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h_] =
            [h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]];
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = h_
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            h_ = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(h_);
    }
    let mut out = [0u8; 32];
    for i in 0..8 {
        out[i * 4..i * 4 + 4].copy_from_slice(&h[i].to_be_bytes());
    }
    out
}

#[test]
fn sha256_self_check() {
    // Smoke: SHA-256 of empty string is the well-known constant.
    let empty: [u8; 32] = [
        0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f, 0xb9,
        0x24, 0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b, 0x78, 0x52,
        0xb8, 0x55,
    ];
    assert_eq!(sha256(b""), empty);
    let abc: [u8; 32] = [
        0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae, 0x22,
        0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61, 0xf2, 0x00,
        0x15, 0xad,
    ];
    assert_eq!(sha256(b"abc"), abc);
}
