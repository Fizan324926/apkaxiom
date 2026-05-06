// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
//
// P1.10 §10 row 4 (HARD) — Merkle root reproducibility.
//
// Runs `parse_with_commit_chain` against the four real-APK
// fixtures (committed under crates/axiom-l1-rs/tests/fixtures/)
// twice and asserts the produced Merkle roots are bit-identical
// across runs. Also commits the canonical roots into a snapshot
// file so any future change to the parser's commit-emit hooks
// flips the snapshot — and that flip is reviewable.

use axiom_l1_rs::commit_chain::parse_with_commit_chain;

/// Canonical Merkle roots — KAT regression. These are the
/// content-determined commitment receipts for the four real
/// F-Droid APK fixtures committed under `tests/fixtures/`.
/// Any change to the parser, leaf-formation rule, body
/// accumulator, or tree-fold that perturbs a single bit-flip
/// in any fixture flips the corresponding root and fails this
/// test — even on a clean reproducibility test that only
/// asserts run-1 == run-2. The KAT is the production
/// regression seal.
///
/// To re-stamp these (after an intentional protocol change):
///   1. Implement the change.
///   2. Run `make p110-reproducibility` to print live roots.
///   3. Update both this constant array AND the matching
///      table in `docs/phase-1/P1.10/CHECKLIST.md` §D.
///   4. The matching test below asserts the live computation
///      equals these committed values — both must move together.
const KAT_FIXTURES: &[(&str, usize, [u8; 32])] = &[
    (
        "fdroid-privileged-2050.apk",
        50,
        [
            0x89, 0x30, 0x8c, 0x49, 0x01, 0xeb, 0xc3, 0x45, 0xf8, 0x0a, 0xe4, 0xdd, 0x9b, 0xe4,
            0x21, 0x90, 0x57, 0x48, 0x17, 0x17, 0x58, 0x6d, 0xbc, 0x26, 0xc2, 0x03, 0x46, 0x14,
            0x27, 0x05, 0x10, 0x9b,
        ],
    ),
    (
        "clipboard.apk",
        36,
        [
            0x11, 0x88, 0x8d, 0xa7, 0xe1, 0xaf, 0x12, 0x88, 0x4b, 0x8c, 0x7a, 0x6f, 0x56, 0x75,
            0xb4, 0xa0, 0xb7, 0xcf, 0x59, 0xf7, 0xec, 0x25, 0xa7, 0x55, 0x32, 0xd1, 0x16, 0x5e,
            0x6c, 0xf8, 0x8c, 0x45,
        ],
    ),
    (
        "tickytacky-mirror.apk",
        35,
        [
            0x5a, 0x30, 0x4a, 0x81, 0xb9, 0x82, 0xc6, 0xba, 0xae, 0x01, 0xbb, 0xd1, 0xc4, 0xd8,
            0xdb, 0x88, 0x8a, 0x16, 0xf8, 0xcd, 0x00, 0x40, 0x51, 0x33, 0xcd, 0xd2, 0x91, 0x74,
            0x6c, 0xaa, 0x3c, 0xe6,
        ],
    ),
    (
        "wifiautoff.apk",
        27,
        [
            0x38, 0xbd, 0xb9, 0x59, 0xb7, 0xed, 0x8e, 0xee, 0x46, 0x2a, 0x59, 0xbe, 0x8b, 0x9d,
            0x42, 0x3f, 0x3a, 0x07, 0x0e, 0x75, 0x7a, 0xfd, 0xc6, 0x0d, 0x52, 0xcb, 0x8e, 0xef,
            0xed, 0x35, 0x7e, 0x99,
        ],
    ),
];

const FIXTURE_NAMES: &[&str] = &[
    "fdroid-privileged-2050.apk",
    "clipboard.apk",
    "tickytacky-mirror.apk",
    "wifiautoff.apk",
];

fn fixture_path(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

const fn nibble_hex(n: u8) -> char {
    if n < 10 {
        (b'0' + n) as char
    } else {
        (b'a' + n - 10) as char
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(nibble_hex(b >> 4));
        out.push(nibble_hex(b & 0x0f));
    }
    out
}

#[test]
fn merkle_root_is_reproducible_on_real_apks() {
    for name in FIXTURE_NAMES {
        let bytes =
            std::fs::read(fixture_path(name)).unwrap_or_else(|e| panic!("{name}: read err {e}"));
        let (_, c1) = parse_with_commit_chain(bytes.as_slice())
            .unwrap_or_else(|e| panic!("{name}: pass1 {e}"));
        let (_, c2) = parse_with_commit_chain(bytes.as_slice())
            .unwrap_or_else(|e| panic!("{name}: pass2 {e}"));
        assert_eq!(
            c1.root, c2.root,
            "{name}: Merkle root not reproducible across runs"
        );
        assert_eq!(c1.leaves.len(), c2.leaves.len(), "{name}: leaf count");
        for (i, (a, b)) in c1.leaves.iter().zip(c2.leaves.iter()).enumerate() {
            assert_eq!(a.hash, b.hash, "{name}: leaf #{i} hash diverged");
            assert_eq!(a.offset, b.offset, "{name}: leaf #{i} offset");
            assert_eq!(a.length, b.length, "{name}: leaf #{i} length");
        }
        eprintln!(
            "{name}: leaves={} root={}",
            c1.leaves.len(),
            hex_encode(&c1.root)
        );
    }
}

#[test]
fn merkle_root_changes_when_input_changes() {
    let mut bytes = std::fs::read(fixture_path("clipboard.apk")).unwrap();
    let (_, c1) = parse_with_commit_chain(bytes.as_slice()).unwrap();
    // Flip a single bit in a body byte (offset chosen to land in a
    // ZipEntryData range, not in the LFH header — anywhere ≥ 1024
    // works for these fixtures).
    let mut mutated = std::mem::take(&mut bytes);
    mutated[1024] ^= 0x01;
    let (_, c2) = parse_with_commit_chain(mutated.as_slice()).unwrap();
    assert_ne!(c1.root, c2.root, "single-bit flip must change Merkle root");
}

/// KAT regression — assert live Merkle roots match the committed
/// constants in [`KAT_FIXTURES`]. This is the load-bearing
/// regression gate: any silent change to the chain protocol
/// fails the build, not just markdown drift.
#[test]
fn merkle_root_kat_regression_on_four_apks() {
    for &(name, expected_leaves, expected_root) in KAT_FIXTURES {
        let bytes =
            std::fs::read(fixture_path(name)).unwrap_or_else(|e| panic!("{name}: read err {e}"));
        let (_, chain) = parse_with_commit_chain(bytes.as_slice())
            .unwrap_or_else(|e| panic!("{name}: parse {e}"));
        assert_eq!(
            chain.leaves.len(),
            expected_leaves,
            "{name}: leaf count drift — committed {expected_leaves}, got {} (live root {})",
            chain.leaves.len(),
            hex_encode(&chain.root)
        );
        assert_eq!(
            chain.root,
            expected_root,
            "{name}: KAT regression — committed root {} ≠ live root {}",
            hex_encode(&expected_root),
            hex_encode(&chain.root)
        );
    }
}
