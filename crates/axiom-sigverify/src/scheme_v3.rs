// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Real verifier for APK Signature Schemes v3 + v3.1.
//!
//! v3 / v3.1 share the wire format and the cryptographic
//! verification path with v2. The deltas are:
//!
//!   - Per-signer `(min_sdk, max_sdk)` SDK range, mirrored in
//!     `signed_data`. The parser already enforces these match.
//!   - Different block IDs (v3 = 0xf05368c0, v3.1 = 0x1b93ad61)
//!     so devices that don't understand the scheme leave the
//!     block alone (forward compat).
//!   - Optional proof-of-rotation lineage in
//!     `additional_attributes` (id 0x3ba06f8c).
//!
//! For verifier *soundness* the algorithm is identical to v2:
//! recompute the chunked digest, verify the public-key cross-bind,
//! verify each signature under the leaf cert. We delegate to
//! `scheme_v2::verify_signers` and just thread the v3-specific
//! coexistence/rotation invariants in the dispatcher.

use axiom_sigblock::scheme::Signer;

use crate::Verdict;

/// Verify every v3 signer. Identical algorithm to v2; the v3-
/// specific SDK-range invariant is already enforced by the parser.
pub fn verify_signers(
    signers: &[Signer],
    apk: &[u8],
    signing_block_start: u64,
    signing_block_end: u64,
    cd_offset: u64,
    eocd_offset: u64,
) -> Verdict {
    crate::scheme_v2::verify_signers(
        signers,
        apk,
        signing_block_start,
        signing_block_end,
        cd_offset,
        eocd_offset,
    )
}

/// Verify every v3.1 signer. Same algorithm as v3.
pub fn verify_signers_v3_1(
    signers: &[Signer],
    apk: &[u8],
    signing_block_start: u64,
    signing_block_end: u64,
    cd_offset: u64,
    eocd_offset: u64,
) -> Verdict {
    verify_signers(
        signers,
        apk,
        signing_block_start,
        signing_block_end,
        cd_offset,
        eocd_offset,
    )
}

/// True iff every present scheme accepts AND v3 / v3.1 coexistence
/// is well-formed AND at least one scheme is present. Mirrors
/// `Apkaxiom.Signing.Dispatch.dispatchAcceptCondition`.
#[must_use]
pub fn dispatch_verify(apk: &[u8]) -> Verdict {
    let block = match axiom_sigblock::locate(apk) {
        Ok(Some(b)) => b,
        Ok(None) => return Verdict::NotPresent,
        Err(e) => return Verdict::Malformed(format!("block: {e}")),
    };
    // Find EOCD bounds.
    let eocd_off = match find_eocd(apk) {
        Some(o) => o as u64,
        None => return Verdict::Malformed("no EOCD".into()),
    };
    let cd_offset = u32::from_le_bytes(
        apk[eocd_off as usize + 16..eocd_off as usize + 20]
            .try_into()
            .unwrap_or([0u8; 4]),
    ) as u64;
    let sb_start = block.block_offset;
    let sb_end = cd_offset;
    // v2.
    if let Some(v2) = block.v2() {
        let signers = match axiom_sigblock::scheme::parse_v2(v2) {
            Ok(s) => s,
            Err(e) => return Verdict::Malformed(format!("v2 parse: {e}")),
        };
        let verdict =
            crate::scheme_v2::verify_signers(&signers, apk, sb_start, sb_end, cd_offset, eocd_off);
        if verdict != Verdict::Accept {
            return verdict;
        }
    }
    // v3.
    if let Some(v3) = block.v3() {
        let signers = match axiom_sigblock::scheme::parse_v3(v3) {
            Ok(s) => s,
            Err(e) => return Verdict::Malformed(format!("v3 parse: {e}")),
        };
        let verdict = verify_signers(&signers, apk, sb_start, sb_end, cd_offset, eocd_off);
        if verdict != Verdict::Accept {
            return verdict;
        }
    }
    // v3.1 (coexistence: must come with v3).
    if let Some(v3_1) = block.v3_1() {
        if block.v3().is_none() {
            return Verdict::Malformed("v3.1 present without v3 (downgrade attempt)".into());
        }
        let signers = match axiom_sigblock::scheme::parse_v3_1(v3_1) {
            Ok(s) => s,
            Err(e) => return Verdict::Malformed(format!("v3.1 parse: {e}")),
        };
        let verdict = verify_signers_v3_1(&signers, apk, sb_start, sb_end, cd_offset, eocd_off);
        if verdict != Verdict::Accept {
            return verdict;
        }
    }
    if block.v2().is_none() && block.v3().is_none() && block.v3_1().is_none() {
        return Verdict::NotPresent;
    }
    Verdict::Accept
}

fn find_eocd(bytes: &[u8]) -> Option<usize> {
    if bytes.len() < 22 {
        return None;
    }
    let mut i = bytes.len() - 22;
    loop {
        if u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap()) == 0x0605_4b50 {
            return Some(i);
        }
        if i == 0 {
            return None;
        }
        i -= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn read_fixture(rel: &str) -> Vec<u8> {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join(rel);
        std::fs::read(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
    }

    #[test]
    fn dispatch_verify_real_v2_v3_apk_accepts() {
        let apk = read_fixture("corpus/signing/v1-v2-v3/wifiautoff-v1v2v3.apk");
        let v = dispatch_verify(&apk);
        assert!(matches!(v, Verdict::Accept), "v1+v2+v3 fixture: {:?}", v);
    }

    #[test]
    fn dispatch_verify_real_v3_1_apk_accepts() {
        let apk = read_fixture("corpus/signing/v1-v2-v3-v31/wifiautoff-v1v2v3v31.apk");
        let v = dispatch_verify(&apk);
        assert!(
            matches!(v, Verdict::Accept),
            "v1+v2+v3+v3.1 fixture: {:?}",
            v
        );
    }

    #[test]
    fn dispatch_verify_v1_only_returns_not_present() {
        let apk = read_fixture("corpus/signing/v1-only/wifiautoff-v1.apk");
        let v = dispatch_verify(&apk);
        assert!(
            matches!(v, Verdict::NotPresent),
            "v1-only fixture: expected NotPresent, got {:?}",
            v
        );
    }
}
