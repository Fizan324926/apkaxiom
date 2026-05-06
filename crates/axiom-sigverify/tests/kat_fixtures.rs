// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
#![allow(clippy::doc_markdown)]
//
// P1.11 G9 + G10 — KAT regression + cross-implementation SHA-256.
//
// Pinned hex constants per APK fixture in the signing corpus.
// The values were computed by Python `hashlib.sha256(...)` (an
// independent SHA-256 implementation backed by OpenSSL). Live
// computation via the RustCrypto `sha2` crate must equal these
// values byte-for-byte. Any drift either:
//
//   - Changes the fixture bytes (re-run `apksigner` to regenerate
//     the corpus and update this constant table — but document
//     why), OR
//   - Reveals a Rust `sha2` regression vs OpenSSL/Python (load-
//     bearing — fail the build).

use sha2::{Digest, Sha256};

/// Fixture path → (expected_size_bytes, sha256_hex).
const KAT: &[(&str, usize, &str)] = &[
    (
        "corpus/signing/v1-only/wifiautoff-v1.apk",
        11317,
        "953642485ccf77964d3824c7c2cfd8a260115e5696dff80edf87285493460704",
    ),
    (
        "corpus/signing/v1-v2/wifiautoff-v1v2.apk",
        16866,
        "f6119d461d164147d6389a33709eec834040456d082fc009af122419bad1008d",
    ),
    (
        "corpus/signing/v1-v2-v3/wifiautoff-v1v2v3.apk",
        16866,
        "33010a5f128d7be3339c72d92d6a49b3a0121417a3108827cc197fe558b12e8e",
    ),
    (
        "corpus/signing/v1-v2-v3-v31/wifiautoff-v1v2v3v31.apk",
        20962,
        "f03fd6071380eac98aab567f94692ecbe4e67d6a840527f21eb28b2ea34e8808",
    ),
    (
        "crates/axiom-l1-rs/tests/fixtures/clipboard.apk",
        14310,
        "9783901de30f7ce5b0048ea014e9a4a9177f75ce954161b72c27124b62a42c30",
    ),
    (
        "crates/axiom-l1-rs/tests/fixtures/fdroid-privileged-2050.apk",
        39214,
        "8d0f5f8351617c99f11156199a281dca6d5fd41c4b8bfeb107dfd60f5c954f5c",
    ),
    (
        "crates/axiom-l1-rs/tests/fixtures/tickytacky-mirror.apk",
        7036,
        "abd4696ed450d1baef3c4fc53d4307e4a1faced26091d406d3ddf65a34059ec4",
    ),
    (
        "crates/axiom-l1-rs/tests/fixtures/wifiautoff.apk",
        11419,
        "d3d95a012eefdd1e88996b95c6eca70c5dfaa1703ed8b81f3a59f9d1011c92a4",
    ),
];

fn workspace_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn hex_decode(s: &str) -> Vec<u8> {
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).expect("hex"))
        .collect()
}

#[test]
fn kat_fixtures_size_and_sha256_match_python_hashlib() {
    for (rel, expected_len, expected_sha) in KAT {
        let path = workspace_root().join(rel);
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        assert_eq!(
            bytes.len(),
            *expected_len,
            "{rel}: size drift — expected {expected_len}, got {}",
            bytes.len()
        );
        let sha = Sha256::digest(&bytes);
        let expected = hex_decode(expected_sha);
        assert_eq!(
            sha.as_slice(),
            expected.as_slice(),
            "{rel}: RustCrypto sha2 disagrees with Python hashlib reference",
        );
    }
}
