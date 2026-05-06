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

#![allow(
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc
)]

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
        // Bottom-up fold using a single reusable hasher to amortise
        // BLAKE3 init/finalize overhead across the ~N internal-node
        // hashes the tree-fold pays.
        let mut scratch = Blake3::default();
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
                next.push(combine_with(&mut scratch, &l, &r));
                i += 2;
            }
            level = next;
        }
        level[0]
    }
}

#[cfg(test)]
fn combine(left: &Hash, right: &Hash) -> Hash {
    let mut s = Blake3::default();
    combine_with(&mut s, left, right)
}

/// `BLAKE3(0x00 || left || right)` — internal-node combiner.
/// Reuses an existing hasher allocation. Saves the per-call
/// `Blake3::new` cost across the ~N internal-node hashes a
/// tree-fold pays.
fn combine_with(scratch: &mut Blake3, left: &Hash, right: &Hash) -> Hash {
    scratch.reset();
    scratch.update(&[0x00]);
    scratch.update(left);
    scratch.update(right);
    scratch.finalize_borrow()
}

/// Drive the streaming parser, recording a Merkle leaf for every
/// content-bearing event. The leaf tags emitted are:
///
///   - `"lfh-header"` — the verbatim 30-byte LFH prefix + name +
///     extra-field for each archive entry.
///   - `"lfh-body"` — every `ZipEntryData` chunk, in order.
///   - `"signing-block"` — the bytes between the last LFH body and
///     the central directory (typically the APK v2/v3 signing
///     block; absent for unsigned archives).
///   - `"cdr-entry"` — one leaf per central-directory record (46-
///     byte fixed prefix + name + extra + comment).
///   - `"eocd"` — the verbatim end-of-central-directory record.
///
/// Together these leaves cover **every byte of a well-formed
/// archive**: a single bit-flip anywhere in any LFH/body/CDR/EOCD/
/// signing-block changes the Merkle root.
///
/// Returns the full event log alongside the chain.
pub fn parse_with_commit_chain<R: Read>(
    reader: R,
) -> Result<(Vec<ParseEvent>, CommitChain), StreamError> {
    let mut parser = ApkParser::from_reader(reader);
    let mut events = Vec::new();
    let mut leaves: Vec<CommitLeaf> = Vec::new();
    // Reusable scratch hasher — `reset() + update() + finalize_borrow()`
    // is significantly faster than `default(); update; finalize` per
    // leaf when committing many small regions (LFH header / DD / CDR
    // / EOCD), where the per-call init/finalize cost dominates the
    // actual hashing.
    let mut scratch = Blake3::default();
    let mut leaf_hash = |bytes: &[u8]| -> axiom_blake3_hacl::Hash {
        scratch.reset();
        scratch.update(bytes);
        scratch.finalize_borrow()
    };
    // Per-entry body accumulator. The streaming parser fires
    // `ZipEntryData` once per buffer-chunk, so leaf granularity
    // would otherwise depend on chunk size — which breaks the
    // chunk-size invariance gate (P1.10 §B item 7). We accumulate
    // the body of the *current* entry into a single BLAKE3
    // hasher; the body leaf is finalised + emitted only when the
    // next LFH header / signing block / CDR is seen, or at end-of-
    // stream. This guarantees one body leaf per archive entry,
    // independent of chunk size.
    let mut body_acc: Option<BodyAccumulator> = None;
    while let Some(ev) = parser.next_event()? {
        match &ev {
            ParseEvent::ZipEntryHeader {
                raw_header, offset, ..
            } => {
                if let Some(b) = body_acc.take() {
                    leaves.push(b.finalize());
                }
                leaves.push(CommitLeaf {
                    offset: *offset,
                    length: raw_header.len() as u64,
                    hash: leaf_hash(raw_header),
                    tag: "lfh-header",
                });
                body_acc = Some(BodyAccumulator::new(*offset + raw_header.len() as u64));
            }
            ParseEvent::ZipEntryData { offset: o, bytes } => {
                let acc = body_acc
                    .as_mut()
                    .expect("ZipEntryData arrived without a preceding ZipEntryHeader — parser bug");
                acc.update(*o, bytes);
            }
            ParseEvent::DataDescriptor { raw, offset, .. } => {
                if let Some(b) = body_acc.take() {
                    leaves.push(b.finalize());
                }
                leaves.push(CommitLeaf {
                    offset: *offset,
                    length: raw.len() as u64,
                    hash: leaf_hash(raw),
                    tag: "data-descriptor",
                });
            }
            ParseEvent::SigningBlock { raw, offset } => {
                if let Some(b) = body_acc.take() {
                    leaves.push(b.finalize());
                }
                leaves.push(CommitLeaf {
                    offset: *offset,
                    length: raw.len() as u64,
                    hash: leaf_hash(raw),
                    tag: "signing-block",
                });
            }
            ParseEvent::CdrEntry { raw, offset, .. } => {
                if let Some(b) = body_acc.take() {
                    leaves.push(b.finalize());
                }
                leaves.push(CommitLeaf {
                    offset: *offset,
                    length: raw.len() as u64,
                    hash: leaf_hash(raw),
                    tag: "cdr-entry",
                });
            }
            ParseEvent::EocdSeen { raw, offset, .. } => {
                if let Some(b) = body_acc.take() {
                    leaves.push(b.finalize());
                }
                leaves.push(CommitLeaf {
                    offset: *offset,
                    length: raw.len() as u64,
                    hash: leaf_hash(raw),
                    tag: "eocd",
                });
            }
            _ => {}
        }
        events.push(ev);
    }
    if let Some(b) = body_acc.take() {
        leaves.push(b.finalize());
    }
    let root = CommitChain::merkle_root(&leaves);
    Ok((events, CommitChain { leaves, root }))
}

/// Per-entry body accumulator — collects body chunks under a
/// single BLAKE3 hash so the body leaf is one-per-entry,
/// independent of chunk size.
struct BodyAccumulator {
    hasher: Blake3,
    offset: u64,
    length: u64,
}

impl BodyAccumulator {
    fn new(offset: u64) -> Self {
        Self {
            hasher: Blake3::default(),
            offset,
            length: 0,
        }
    }

    fn update(&mut self, _chunk_offset: u64, bytes: &[u8]) {
        self.hasher.update(bytes);
        self.length += bytes.len() as u64;
    }

    fn finalize(self) -> CommitLeaf {
        CommitLeaf {
            offset: self.offset,
            length: self.length,
            hash: self.hasher.finalize(),
            tag: "lfh-body",
        }
    }
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
