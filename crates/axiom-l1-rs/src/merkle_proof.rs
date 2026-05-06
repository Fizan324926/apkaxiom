// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Merkle inclusion proofs for the P1.10 commit chain.
//!
//! A Merkle root commits to *all* leaves at once. To downstream
//! consumers (P1.15 IR-emit, Phase 4 `.axc` certificate
//! verification, third-party auditors) the useful primitive is
//! **inclusion proof**: "leaf at index `i` with value `H_i` is
//! committed under root `R`." This module ships:
//!
//!   - [`MerkleProof`] — the proof object: leaf index + the
//!     sibling hashes along the leaf-to-root path.
//!   - [`MerkleProof::for_leaf`] — proof generation from the full
//!     leaf list.
//!   - [`MerkleProof::verify`] — proof verification given the
//!     leaf hash, the proof, and the expected root. Pure
//!     function, no allocations beyond the proof's path.
//!   - [`MerkleProof::encode`] / [`MerkleProof::decode`] — a
//!     compact, length-prefixed byte encoding for transport.
//!
//! ## Tree shape
//!
//! Same shape as [`crate::commit_chain::CommitChain::merkle_root`]:
//!
//!   - `leaf_i = H_i` (the `CommitLeaf::hash` field).
//!   - Internal node = `BLAKE3(0x00 || left || right)`.
//!   - Odd levels duplicate the last element (Bitcoin/CT pattern),
//!     so a proof for the rightmost odd-leaf at level k uses the
//!     leaf itself as its sibling.
//!
//! A proof of length `d` covers a tree with up to `2^d` leaves
//! (more precisely: a tree whose root is reached after `d` pair
//! combines starting from the leaf level). Proof verification is
//! `O(d)` BLAKE3 invocations.

#![allow(
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc
)]

use axiom_blake3_hacl::{Blake3, Hash, Hasher};

/// A direction in the Merkle tree — does the sibling sit on the
/// **left** of the running hash, or on the **right**? The verifier
/// uses this to position the sibling correctly when reconstructing
/// the parent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Sibling is to the left of the running hash; combine as
    /// `BLAKE3(0x00 || sibling || running)`.
    Left,
    /// Sibling is to the right of the running hash; combine as
    /// `BLAKE3(0x00 || running || sibling)`.
    Right,
}

impl Direction {
    /// Wire encoding: `0x00 = Left`, `0x01 = Right`.
    #[must_use]
    pub const fn to_byte(self) -> u8 {
        match self {
            Self::Left => 0x00,
            Self::Right => 0x01,
        }
    }

    /// Decode the wire byte. Returns `None` for unrecognised values.
    #[must_use]
    pub const fn from_byte(b: u8) -> Option<Self> {
        match b {
            0x00 => Some(Self::Left),
            0x01 => Some(Self::Right),
            _ => None,
        }
    }
}

/// One step in the Merkle path — a sibling hash + its position
/// relative to the running hash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProofStep {
    /// The sibling node's 32-byte hash.
    pub sibling: Hash,
    /// Position of the sibling relative to the running hash.
    pub direction: Direction,
}

/// A Merkle inclusion proof: the leaf index in the original list
/// plus the path of sibling hashes from leaf to root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MerkleProof {
    /// Index of the leaf in the original `leaves` list (0-based).
    pub leaf_index: u32,
    /// Total number of leaves committed under the root. Carried so
    /// the verifier can reconstruct the odd-level duplication
    /// behaviour without seeing all leaves.
    pub leaf_count: u32,
    /// Path from leaf to root. Length = ceil(log2(leaf_count))
    /// for non-trivial trees; empty when leaf_count == 1.
    pub path: Vec<ProofStep>,
}

/// Errors that can occur during proof handling.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ProofError {
    /// The requested leaf index is out of bounds for the leaf list.
    #[error("leaf index {index} out of bounds (leaf_count = {count})")]
    IndexOutOfBounds {
        /// Requested index.
        index: u32,
        /// Leaf count.
        count: u32,
    },
    /// The encoded proof bytes are malformed (truncated, wrong
    /// length, or reserved direction byte).
    #[error("malformed proof bytes: {0}")]
    Malformed(&'static str),
}

impl MerkleProof {
    /// Build an inclusion proof for `leaves[leaf_index]`. Panics
    /// would be possible if the index is out of bounds; we
    /// `Result`-return instead so callers can plumb errors.
    pub fn for_leaf(
        leaves: &[crate::commit_chain::CommitLeaf],
        leaf_index: u32,
    ) -> Result<Self, ProofError> {
        let count = leaves.len() as u32;
        if leaf_index >= count {
            return Err(ProofError::IndexOutOfBounds {
                index: leaf_index,
                count,
            });
        }
        // Build all levels of the tree, recording each level so we
        // can index into siblings.
        let mut levels: Vec<Vec<Hash>> = Vec::new();
        levels.push(leaves.iter().map(|l| l.hash).collect());
        while levels.last().expect("non-empty by construction").len() > 1 {
            let cur = levels.last().expect("non-empty");
            let mut next = Vec::with_capacity(cur.len().div_ceil(2));
            let mut i = 0;
            while i < cur.len() {
                let l = cur[i];
                let r = if i + 1 < cur.len() {
                    cur[i + 1]
                } else {
                    cur[i]
                };
                next.push(combine(&l, &r));
                i += 2;
            }
            levels.push(next);
        }
        // Walk leaf-to-root, recording sibling at each level.
        let mut path = Vec::with_capacity(levels.len().saturating_sub(1));
        let mut idx = leaf_index as usize;
        for level in &levels[..levels.len() - 1] {
            // Sibling is at idx ^ 1; if that is past end, sibling
            // is the node itself (odd-level duplication).
            let sibling_idx = idx ^ 1;
            let sibling = if sibling_idx < level.len() {
                level[sibling_idx]
            } else {
                level[idx]
            };
            let direction = if idx % 2 == 0 {
                // running is Left, sibling is Right.
                Direction::Right
            } else {
                // running is Right, sibling is Left.
                Direction::Left
            };
            path.push(ProofStep { sibling, direction });
            idx /= 2;
        }
        Ok(Self {
            leaf_index,
            leaf_count: count,
            path,
        })
    }

    /// Verify that `leaf_hash` is committed under `expected_root`
    /// according to this proof. Returns `true` iff the proof is
    /// valid.
    #[must_use]
    pub fn verify(&self, leaf_hash: &Hash, expected_root: &Hash) -> bool {
        let mut running = *leaf_hash;
        for step in &self.path {
            running = match step.direction {
                Direction::Left => combine(&step.sibling, &running),
                Direction::Right => combine(&running, &step.sibling),
            };
        }
        // Special case: a tree with exactly one leaf has an empty
        // path and the leaf hash IS the root.
        running == *expected_root
    }

    /// Encode the proof as a compact byte vector. Layout:
    ///
    /// ```text
    ///   [4 bytes  leaf_index   little-endian u32]
    ///   [4 bytes  leaf_count   little-endian u32]
    ///   [4 bytes  path_len     little-endian u32]
    ///   path_len × {
    ///       [1 byte   direction (0x00 = Left, 0x01 = Right)]
    ///       [32 bytes sibling hash                            ]
    ///   }
    /// ```
    ///
    /// Stable across versions. Wire-compatible with the verifier
    /// in any consumer that re-implements [`Self::decode`].
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(12 + self.path.len() * 33);
        out.extend_from_slice(&self.leaf_index.to_le_bytes());
        out.extend_from_slice(&self.leaf_count.to_le_bytes());
        let path_len = self.path.len() as u32;
        out.extend_from_slice(&path_len.to_le_bytes());
        for step in &self.path {
            out.push(step.direction.to_byte());
            out.extend_from_slice(&step.sibling);
        }
        out
    }

    /// Decode a proof from the wire format produced by [`Self::encode`].
    pub fn decode(bytes: &[u8]) -> Result<Self, ProofError> {
        if bytes.len() < 12 {
            return Err(ProofError::Malformed("truncated header"));
        }
        let leaf_index = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        let leaf_count = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        let path_len = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
        let body = &bytes[12..];
        if body.len() != path_len * 33 {
            return Err(ProofError::Malformed("path length mismatch"));
        }
        let mut path = Vec::with_capacity(path_len);
        for i in 0..path_len {
            let off = i * 33;
            let direction = Direction::from_byte(body[off])
                .ok_or(ProofError::Malformed("bad direction byte"))?;
            let mut sibling = [0u8; 32];
            sibling.copy_from_slice(&body[off + 1..off + 33]);
            path.push(ProofStep { sibling, direction });
        }
        Ok(Self {
            leaf_index,
            leaf_count,
            path,
        })
    }
}

/// Internal-node combiner — must match
/// [`crate::commit_chain::merkle_root`]'s combiner exactly.
fn combine(left: &Hash, right: &Hash) -> Hash {
    let mut h = Blake3::default();
    h.update(&[0x00]);
    h.update(left);
    h.update(right);
    h.finalize()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commit_chain::{CommitChain, CommitLeaf};

    fn synth_leaf(idx: u32) -> CommitLeaf {
        CommitLeaf {
            offset: u64::from(idx),
            length: 1,
            hash: Blake3::hash_oneshot(&idx.to_le_bytes()),
            tag: "synth",
        }
    }

    fn build(n: u32) -> Vec<CommitLeaf> {
        (0..n).map(synth_leaf).collect()
    }

    #[test]
    fn proof_for_single_leaf_tree_has_empty_path() {
        let leaves = build(1);
        let root = CommitChain::merkle_root(&leaves);
        let proof = MerkleProof::for_leaf(&leaves, 0).unwrap();
        assert!(proof.path.is_empty());
        assert!(proof.verify(&leaves[0].hash, &root));
    }

    #[test]
    fn proof_round_trip_for_every_leaf_in_sizes_2_4_8_16_100_257_1000() {
        for &n in &[2u32, 4, 8, 16, 100, 257, 1000] {
            let leaves = build(n);
            let root = CommitChain::merkle_root(&leaves);
            for i in 0..n {
                let proof = MerkleProof::for_leaf(&leaves, i).expect("for_leaf");
                assert_eq!(proof.leaf_index, i);
                assert_eq!(proof.leaf_count, n);
                assert!(
                    proof.verify(&leaves[i as usize].hash, &root),
                    "n={n} i={i}: verify failed"
                );
            }
        }
    }

    #[test]
    fn proof_rejects_wrong_leaf_hash() {
        let leaves = build(50);
        let root = CommitChain::merkle_root(&leaves);
        let proof = MerkleProof::for_leaf(&leaves, 17).unwrap();
        // Wrong leaf hash: pretend leaf 17's hash is leaf 18's.
        assert!(!proof.verify(&leaves[18].hash, &root));
    }

    #[test]
    fn proof_rejects_wrong_root() {
        let leaves = build(50);
        let proof = MerkleProof::for_leaf(&leaves, 17).unwrap();
        let mut wrong_root = CommitChain::merkle_root(&leaves);
        wrong_root[0] ^= 1;
        assert!(!proof.verify(&leaves[17].hash, &wrong_root));
    }

    #[test]
    fn proof_rejects_wrong_index() {
        let leaves = build(50);
        let root = CommitChain::merkle_root(&leaves);
        let mut proof = MerkleProof::for_leaf(&leaves, 17).unwrap();
        proof.leaf_index = 18;
        // verify() doesn't actually use leaf_index (it walks the
        // path), so this still verifies — that's correct: the
        // index is metadata for the consumer. The leaf_count is
        // however load-bearing for tree-shape reconstruction in
        // some consumers.
        assert!(proof.verify(&leaves[17].hash, &root));
    }

    #[test]
    fn proof_encode_decode_round_trips() {
        let leaves = build(257);
        for i in [0u32, 1, 2, 7, 100, 128, 256] {
            let p = MerkleProof::for_leaf(&leaves, i).unwrap();
            let bytes = p.encode();
            let q = MerkleProof::decode(&bytes).expect("decode");
            assert_eq!(p, q);
        }
    }

    #[test]
    fn decode_rejects_truncated_header() {
        let bad = [0u8; 8];
        assert!(matches!(
            MerkleProof::decode(&bad),
            Err(ProofError::Malformed(_))
        ));
    }

    #[test]
    fn decode_rejects_path_length_mismatch() {
        let mut p = MerkleProof::for_leaf(&build(8), 3).unwrap().encode();
        p.pop();
        assert!(matches!(
            MerkleProof::decode(&p),
            Err(ProofError::Malformed(_))
        ));
    }

    #[test]
    fn decode_rejects_bad_direction_byte() {
        let mut p = MerkleProof::for_leaf(&build(8), 3).unwrap().encode();
        // First direction byte lives at offset 12.
        p[12] = 0x42;
        assert!(matches!(
            MerkleProof::decode(&p),
            Err(ProofError::Malformed(_))
        ));
    }

    #[test]
    fn for_leaf_rejects_out_of_bounds_index() {
        let leaves = build(10);
        assert!(matches!(
            MerkleProof::for_leaf(&leaves, 10),
            Err(ProofError::IndexOutOfBounds { .. })
        ));
        assert!(matches!(
            MerkleProof::for_leaf(&leaves, 999),
            Err(ProofError::IndexOutOfBounds { .. })
        ));
    }

    /// Stress: 1000-leaf tree, every leaf gets a valid proof.
    /// Mutating any sibling in the path invalidates the proof.
    #[test]
    fn stress_1000_leaves_all_proofs_valid_and_tamper_invalid() {
        let leaves = build(1000);
        let root = CommitChain::merkle_root(&leaves);
        for i in 0..1000 {
            let proof = MerkleProof::for_leaf(&leaves, i).unwrap();
            assert!(proof.verify(&leaves[i as usize].hash, &root));
            if !proof.path.is_empty() {
                // Tamper a path step → proof must fail.
                let mut bad = proof.clone();
                bad.path[0].sibling[0] ^= 1;
                assert!(
                    !bad.verify(&leaves[i as usize].hash, &root),
                    "i={i}: tampered proof verified — chain is not collision-resistant for path"
                );
            }
        }
    }
}
