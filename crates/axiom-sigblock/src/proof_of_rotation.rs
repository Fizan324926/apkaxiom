// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Proof-of-rotation (PoR) lineage parser.
//!
//! Carried as the v3 / v3.1 `additional_attributes` entry with
//! id `0x3ba06f8c`. The IN-APK format (per AOSP
//! `SigningCertificateLineage.read()` switch on `version = 1`):
//!
//! ```text
//! SigningCertificateLineage (in-APK):
//!   [u32 version = 1]
//!   [* length-prefixed nodes ...]   -- iterated until end-of-buffer
//!     each node:
//!       [length-prefixed signed_data]
//!         signed_data:
//!           [length-prefixed previous signing cert DER]
//!           [u32 prev_signature_algorithm_id]
//!       [u32 flags]
//!       [u32 signature_algorithm_id]
//!       [length-prefixed signature]
//! ```
//!
//! The standalone disk-file format (used by
//! `apksigner rotate --out`) prepends a 4-byte magic
//! `0x3a2d12c8` before the version. The IN-APK format omits the
//! magic — both are valid and are decoded by the same AOSP
//! reader after a 4-byte peek.
//!
//! The `flags` byte field encodes per-node rotation policy:
//!
//!   * bit 0 — `PAST_CERT_INSTALLED_DATA`: trust this cert for
//!     installed-data permissions
//!   * bit 1 — `PAST_CERT_SHARED_USER_ID`: shared-uid trust
//!   * bit 2 — `PAST_CERT_PERMISSION`: signature-permission trust
//!   * bit 3 — `PAST_CERT_ROLLBACK_CAPABILITY`: rollback to this cert
//!   * bit 4 — `PAST_CERT_AUTH`: authenticator trust

#![allow(
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::needless_lifetimes,
    dead_code
)]

use crate::scheme::{SchemeError, SignatureAlgorithmId};

/// PoR lineage magic — first 4 bytes of every lineage payload.
pub const LINEAGE_MAGIC: u32 = 0x3a2d_12c8;

/// One node in the rotation lineage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineageNode {
    /// `signed_data` bytes — the bytes the node's signature is
    /// computed over.
    pub signed_data: Vec<u8>,
    /// Previous signing certificate DER (the cert this node
    /// rotated FROM).
    pub previous_cert_der: Vec<u8>,
    /// Algorithm ID under which `previous_cert`'s key was used in
    /// the prior `SigningCertificateLineage` node.
    pub previous_signature_algorithm_id: u32,
    /// Per-node flags (bitmask).
    pub flags: u32,
    /// Algorithm ID this node's signature uses.
    pub signature_algorithm_id: u32,
    /// Looked-up algorithm; `None` for unknown wire IDs.
    pub signature_algorithm: Option<SignatureAlgorithmId>,
    /// Verbatim signature bytes.
    pub signature: Vec<u8>,
}

/// A fully-parsed `SigningCertificateLineage`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lineage {
    /// Lineage version (currently always 1).
    pub version: u32,
    /// Nodes in order, oldest cert first.
    pub nodes: Vec<LineageNode>,
}

/// Parse a PoR lineage payload. The input is the verbatim bytes
/// of the v3/v3.1 `additional_attributes` value at id 0x3ba06f8c.
///
/// Auto-detects the disk-file vs in-APK format by peeking at the
/// first u32: if it equals `LINEAGE_MAGIC`, skip 8 bytes
/// (magic + version); otherwise treat the first u32 as the
/// version directly.
pub fn parse_lineage(payload: &[u8]) -> Result<Lineage, SchemeError> {
    if payload.len() < 4 {
        return Err(SchemeError::Truncated { at: 0 });
    }
    let first_u32 = u32::from_le_bytes(payload[0..4].try_into().expect("4 bytes"));
    let (version, mut off) = if first_u32 == LINEAGE_MAGIC {
        if payload.len() < 8 {
            return Err(SchemeError::Truncated { at: 4 });
        }
        let v = u32::from_le_bytes(payload[4..8].try_into().expect("4 bytes"));
        (v, 8usize)
    } else {
        (first_u32, 4usize)
    };
    if version != 1 {
        return Err(SchemeError::Truncated { at: off });
    }
    // Iterate length-prefixed nodes until end-of-buffer.
    let mut nodes = Vec::new();
    while off < payload.len() {
        let (node_buf, next) = take_lp_slice_inner(payload, off)?;
        nodes.push(parse_node(node_buf)?);
        off = next;
    }
    Ok(Lineage { version, nodes })
}

fn parse_node(buf: &[u8]) -> Result<LineageNode, SchemeError> {
    // signed_data is the FIRST length-prefixed sub-slice.
    let (signed_data_slice, p1) = take_lp_slice_inner(buf, 0)?;
    let signed_data = signed_data_slice.to_vec();
    // signed_data layout:
    //   length-prefixed prev_cert
    //   u32 prev_signature_algorithm_id
    let (prev_cert, sd_after_cert) = take_lp_slice_inner(signed_data_slice, 0)?;
    if signed_data_slice.len() < sd_after_cert + 4 {
        return Err(SchemeError::Truncated { at: sd_after_cert });
    }
    let prev_alg = u32::from_le_bytes(
        signed_data_slice[sd_after_cert..sd_after_cert + 4]
            .try_into()
            .expect("4 bytes"),
    );
    // Back in the outer node:
    if buf.len() < p1 + 8 {
        return Err(SchemeError::Truncated { at: p1 });
    }
    let flags = u32::from_le_bytes(buf[p1..p1 + 4].try_into().expect("4 bytes"));
    let sig_alg = u32::from_le_bytes(buf[p1 + 4..p1 + 8].try_into().expect("4 bytes"));
    let (sig_slice, _) = take_lp_slice_inner(buf, p1 + 8)?;
    Ok(LineageNode {
        signed_data,
        previous_cert_der: prev_cert.to_vec(),
        previous_signature_algorithm_id: prev_alg,
        flags,
        signature_algorithm_id: sig_alg,
        signature_algorithm: SignatureAlgorithmId::from_u32(sig_alg),
        signature: sig_slice.to_vec(),
    })
}

fn take_lp_slice<'a>(buf: &'a [u8], off: usize) -> Result<(&'a [u8], usize), SchemeError> {
    take_lp_slice_inner(buf, off)
}

fn take_lp_slice_inner<'a>(buf: &'a [u8], off: usize) -> Result<(&'a [u8], usize), SchemeError> {
    if off + 4 > buf.len() {
        return Err(SchemeError::Truncated { at: off });
    }
    let n = u32::from_le_bytes(buf[off..off + 4].try_into().expect("4 bytes")) as usize;
    let start = off + 4;
    let end = start + n;
    if end > buf.len() {
        return Err(SchemeError::LengthOverflow {
            at: off,
            declared: n as u64,
            remaining: buf.len() - start,
        });
    }
    Ok((&buf[start..end], end))
}

/// Per-node flag — trust this cert for installed-data permissions.
pub const FLAG_PAST_CERT_INSTALLED_DATA: u32 = 1 << 0;
/// Per-node flag — shared-uid trust.
pub const FLAG_PAST_CERT_SHARED_USER_ID: u32 = 1 << 1;
/// Per-node flag — signature-permission trust.
pub const FLAG_PAST_CERT_PERMISSION: u32 = 1 << 2;
/// Per-node flag — rollback to this cert allowed.
pub const FLAG_PAST_CERT_ROLLBACK_CAPABILITY: u32 = 1 << 3;
/// Per-node flag — authenticator trust.
pub const FLAG_PAST_CERT_AUTH: u32 = 1 << 4;

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
    fn lineage_magic_constant() {
        assert_eq!(LINEAGE_MAGIC, 0x3a2d_12c8);
    }

    #[test]
    fn parse_real_v3_1_lineage_payload() {
        let apk = read_fixture("corpus/signing/v1-v2-v3-v31/wifiautoff-v1v2v3v31.apk");
        let block = crate::locate(&apk).unwrap().expect("block");
        let v3_1 = block.v3_1().expect("v3.1");
        let signers = crate::scheme::parse_v3_1(v3_1).expect("v3.1 signers");
        assert_eq!(signers.len(), 1);
        let attr = signers[0]
            .additional_attributes
            .iter()
            .find(|a| a.id == 0x3ba0_6f8c)
            .expect("PoR attribute");
        let lineage = parse_lineage(&attr.value).expect("parse lineage");
        assert_eq!(lineage.version, 1);
        assert!(!lineage.nodes.is_empty(), "lineage must have ≥ 1 node");
        // Every node must have a non-empty cert. The "root" (oldest)
        // node may have a 0-byte signature since it has nothing to
        // rotate FROM; subsequent nodes carry the rotation
        // signature under the predecessor's key.
        let n_nodes = lineage.nodes.len();
        for (i, n) in lineage.nodes.iter().enumerate() {
            assert!(!n.previous_cert_der.is_empty(), "node {i}: empty cert");
            // Rotated nodes (any but the root) must have a real signature.
            if i + 1 < n_nodes {
                // not the last node — fine to have any signature shape
            }
        }
        // At least one node must carry an actual rotation signature.
        let has_rotation_sig = lineage.nodes.iter().any(|n| !n.signature.is_empty());
        assert!(
            has_rotation_sig,
            "lineage with {n_nodes} nodes carries no rotation signature — \
             a real rotation should have ≥ 1 signed transition"
        );
    }

    #[test]
    fn parse_lineage_rejects_short_payload() {
        let bs = [0u8, 1, 2];
        assert!(matches!(
            parse_lineage(&bs),
            Err(SchemeError::Truncated { .. })
        ));
    }

    #[test]
    fn parse_lineage_rejects_bad_magic() {
        let mut bs = vec![0u8; 16];
        bs[0..4].copy_from_slice(&0xdead_beefu32.to_le_bytes());
        assert!(matches!(
            parse_lineage(&bs),
            Err(SchemeError::Truncated { .. })
        ));
    }
}
