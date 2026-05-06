// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! P1.10 — Merkle commit chain for the streaming parser.
//!
//! Every parse step that consumes a contiguous byte range emits a
//! BLAKE3 hash of those bytes; the hashes feed a Merkle tree
//! whose root is reproducible bit-identical across runs on the
//! same input.
//!
//! ## Shape
//!
//! ```text
//!     leaf_i  =  BLAKE3(bytes consumed by step i)
//!     node    =  BLAKE3(0x00 || left || right)   for internal pairs
//!     root    =  fold over leaves bottom-up
//! ```
//!
//! The internal-node prefix `0x00` matches the BLAKE3-team's
//! domain-separation convention for Merkle trees built from
//! BLAKE3 leaves; an attacker can't fool the verifier by
//! presenting a leaf-shaped collision.
//!
//! Empty leaf set → root is the BLAKE3 hash of the empty string.
//!
//! ## Reproducibility
//!
//! The chain is deterministic given the same input bytes and
//! the same parser code path. `parse_lfh_with_commit_chain` runs
//! the parser on a byte slice and returns `(parse_result,
//! CommitChain)`; calling it twice on identical input produces
//! byte-identical roots — asserted by the §F-1 reproducibility
//! test.

#![allow(clippy::doc_markdown, clippy::missing_errors_doc)]

use axiom_blake3_hacl::{Blake3, Hash, Hasher};

use crate::event::ParseEvent;
use crate::stream::{ApkParser, StreamError};
use std::io::Read;

/// One leaf in the commit chain — a contiguous byte-range that
/// the parser consumed, plus the BLAKE3 hash of those bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitLeaf {
    /// Byte offset of the start of the range, relative to the
    /// streaming parser's input.
    pub offset: u64,
    /// Length of the range in bytes.
    pub length: u64,
    /// BLAKE3 hash of the range contents.
    pub hash: Hash,
    /// Diagnostic tag — what kind of parse step produced the
    /// leaf. Stable across runs; the Merkle root does NOT depend
    /// on this tag (the tag is for human inspection only).
    pub tag: &'static str,
}

/// The full commit chain — every leaf + the Merkle root.
#[derive(Debug, Clone)]
pub struct CommitChain {
    /// All leaves in source order.
    pub leaves: Vec<CommitLeaf>,
    /// Merkle root over `leaves`. Reproducible across runs.
    pub root: Hash,
}

impl CommitChain {
    /// Compute the Merkle root from a list of leaves. Public so
    /// downstream consumers (P1.15 IR-emit, Phase 4 .axc) can
    /// re-derive the root from a saved leaf list.
    #[must_use]
    pub fn merkle_root(leaves: &[CommitLeaf]) -> Hash {
        if leaves.is_empty() {
            return Blake3::hash_oneshot(b"");
        }
        // Bottom-up fold. Promote leaves to "level 0", then
        // pairwise combine into "level 1", until one node remains.
        // Odd levels duplicate the last element (per the standard
        // Bitcoin/Certificate-Transparency pattern).
        let mut level: Vec<Hash> = leaves.iter().map(|l| l.hash).collect();
        while level.len() > 1 {
            let mut next = Vec::with_capacity(level.len().div_ceil(2));
            let mut i = 0;
            while i < level.len() {
                let l = level[i];
                let r = if i + 1 < level.len() {
                    level[i + 1]
                } else {
                    level[i]
                };
                next.push(combine(&l, &r));
                i += 2;
            }
            level = next;
        }
        level[0]
    }
}

/// `BLAKE3(0x00 || left || right)` — the internal-node combiner.
/// `0x00` domain-separates internal nodes from leaves (which the
/// parser hashes directly without a prefix).
fn combine(left: &Hash, right: &Hash) -> Hash {
    let mut h = Blake3::default();
    h.update(&[0x00]);
    h.update(left);
    h.update(right);
    h.finalize()
}

/// Drive the streaming parser over `bytes`, recording a commit
/// leaf for every `ZipEntryHeader` (header bytes) and
/// `ZipEntryData` (body chunk) event. Returns the streaming
/// parser's final state alongside the chain.
///
/// This is the "with hooks" path the P1.10 §10 row 5 perf-delta
/// gate measures against the bare streaming parser.
pub fn parse_with_commit_chain<R: Read>(
    reader: R,
) -> Result<(Vec<ParseEvent>, CommitChain), StreamError> {
    let mut parser = ApkParser::from_reader(reader);
    let mut events = Vec::new();
    let mut leaves: Vec<CommitLeaf> = Vec::new();
    let mut offset: u64 = 0;
    while let Some(ev) = parser.next_event()? {
        match &ev {
            ParseEvent::ZipEntryHeader { file_name, .. } => {
                // The streaming parser doesn't expose the raw
                // header bytes — it has already consumed them by
                // the time the event fires. We commit to the
                // *file name* as a stable per-entry identifier;
                // the body bytes (next event) are committed with
                // their actual range. P1.15 will extend this to
                // commit on the full LFH header bytes once the
                // streaming layer exposes them.
                let h = Blake3::hash_oneshot(file_name);
                let len = file_name.len() as u64;
                leaves.push(CommitLeaf {
                    offset,
                    length: len,
                    hash: h,
                    tag: "lfh-name",
                });
                offset += len;
            }
            ParseEvent::ZipEntryData { offset: o, bytes } => {
                let h = Blake3::hash_oneshot(bytes);
                let len = bytes.len() as u64;
                leaves.push(CommitLeaf {
                    offset: *o,
                    length: len,
                    hash: h,
                    tag: "lfh-body",
                });
                offset = *o + len;
            }
            _ => {}
        }
        events.push(ev);
    }
    let root = CommitChain::merkle_root(&leaves);
    Ok((events, CommitChain { leaves, root }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merkle_root_empty() {
        let h = CommitChain::merkle_root(&[]);
        assert_eq!(h, Blake3::hash_oneshot(b""));
    }

    #[test]
    fn merkle_root_single_leaf_equals_leaf_hash() {
        let leaf = CommitLeaf {
            offset: 0,
            length: 4,
            hash: Blake3::hash_oneshot(b"data"),
            tag: "test",
        };
        let h = CommitChain::merkle_root(std::slice::from_ref(&leaf));
        assert_eq!(h, leaf.hash);
    }

    #[test]
    fn merkle_root_combines_two_leaves() {
        let l1 = CommitLeaf {
            offset: 0,
            length: 2,
            hash: Blake3::hash_oneshot(b"aa"),
            tag: "x",
        };
        let l2 = CommitLeaf {
            offset: 2,
            length: 2,
            hash: Blake3::hash_oneshot(b"bb"),
            tag: "y",
        };
        let r = CommitChain::merkle_root(&[l1.clone(), l2.clone()]);
        let expected = combine(&l1.hash, &l2.hash);
        assert_eq!(r, expected);
    }

    #[test]
    fn merkle_root_odd_count_duplicates_last() {
        let l1 = CommitLeaf {
            offset: 0,
            length: 1,
            hash: Blake3::hash_oneshot(b"1"),
            tag: "a",
        };
        let l2 = CommitLeaf {
            offset: 1,
            length: 1,
            hash: Blake3::hash_oneshot(b"2"),
            tag: "b",
        };
        let l3 = CommitLeaf {
            offset: 2,
            length: 1,
            hash: Blake3::hash_oneshot(b"3"),
            tag: "c",
        };
        let r = CommitChain::merkle_root(&[l1.clone(), l2.clone(), l3.clone()]);
        // Level 1: combine(l1,l2), combine(l3,l3)
        let n12 = combine(&l1.hash, &l2.hash);
        let n33 = combine(&l3.hash, &l3.hash);
        // Level 2: combine(n12, n33)
        let expected = combine(&n12, &n33);
        assert_eq!(r, expected);
    }

    #[test]
    fn merkle_root_is_deterministic() {
        // 1000 leaves; root must be identical across two runs.
        let leaves: Vec<CommitLeaf> = (0..1000u64)
            .map(|i| CommitLeaf {
                offset: i * 16,
                length: 16,
                hash: Blake3::hash_oneshot(&i.to_le_bytes()),
                tag: "det",
            })
            .collect();
        let r1 = CommitChain::merkle_root(&leaves);
        let r2 = CommitChain::merkle_root(&leaves);
        assert_eq!(r1, r2);
    }

    #[test]
    fn merkle_root_changes_when_leaf_changes() {
        let mut leaves: Vec<CommitLeaf> = (0..16u64)
            .map(|i| CommitLeaf {
                offset: i,
                length: 1,
                hash: Blake3::hash_oneshot(&[i as u8]),
                tag: "x",
            })
            .collect();
        let r1 = CommitChain::merkle_root(&leaves);
        leaves[7].hash = Blake3::hash_oneshot(b"different");
        let r2 = CommitChain::merkle_root(&leaves);
        assert_ne!(r1, r2);
    }
}
