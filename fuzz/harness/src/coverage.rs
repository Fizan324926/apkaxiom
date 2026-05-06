// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Lightweight coverage map — without sancov instrumentation,
//! we approximate "which corner of the parser did this input
//! touch?" by hashing the verdict pair into a bitmap. New
//! `(axiom-l0 verdict, target verdict)` combinations register
//! as new edges.
//!
//! This is the "poor person's AFL" feedback loop: the dev-mode
//! driver biases mutation toward inputs that hit new edges,
//! materially improving exploration vs blind random.
//!
//! For real edge-coverage on the verified parser the operator
//! one-shot is to compile axiom-l0 with `-C
//! instrument-coverage` and feed `prof_data` into LLVM's
//! `__llvm_profile_dump`. That's a separate sub-phase; this
//! lightweight bitmap covers the immediate need.

use std::sync::atomic::{AtomicU32, Ordering};

use axiom_blake3_hacl::{Blake3, Hasher};

use crate::classifier::Verdict;

const BITMAP_SIZE: usize = 65_536;

/// Edge bitmap. One u32 counter per slot (saturating; we don't
/// need full hit counts — just "seen / not seen").
pub struct CoverageMap {
    /// 64K slots indexed by `hash(verdict_pair) % 65536`.
    slots: Vec<AtomicU32>,
}

impl CoverageMap {
    /// Build a fresh empty map.
    #[must_use]
    pub fn new() -> Self {
        let mut v = Vec::with_capacity(BITMAP_SIZE);
        for _ in 0..BITMAP_SIZE {
            v.push(AtomicU32::new(0));
        }
        Self { slots: v }
    }

    /// Record an observation. Returns `true` if this is a
    /// previously-unseen edge (the slot was 0 before).
    pub fn observe(&self, axiom: &Verdict, target: &Verdict) -> bool {
        let key = format!("{}|{}", axiom.label(), target.label());
        let slot = hash_to_slot(key.as_bytes());
        let prev = self.slots[slot].fetch_add(1, Ordering::Relaxed);
        prev == 0
    }

    /// Count of slots with at least one observation.
    #[must_use]
    pub fn distinct_edges(&self) -> usize {
        self.slots
            .iter()
            .filter(|s| s.load(Ordering::Relaxed) > 0)
            .count()
    }

    /// Total observations recorded (sum of all slot counters,
    /// saturated at u32::MAX per slot).
    #[must_use]
    pub fn total_observations(&self) -> u64 {
        self.slots
            .iter()
            .map(|s| u64::from(s.load(Ordering::Relaxed)))
            .sum()
    }
}

impl Default for CoverageMap {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for CoverageMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CoverageMap")
            .field("distinct_edges", &self.distinct_edges())
            .field("total_observations", &self.total_observations())
            .finish()
    }
}

fn hash_to_slot(bytes: &[u8]) -> usize {
    let mut h = Blake3::default();
    h.update(bytes);
    let digest = h.finalize_borrow();
    let v = u32::from_le_bytes([digest[0], digest[1], digest[2], digest[3]]);
    (v as usize) % BITMAP_SIZE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distinct_edge_counted_once() {
        let m = CoverageMap::new();
        assert!(m.observe(&Verdict::Accept, &Verdict::Accept));
        assert!(!m.observe(&Verdict::Accept, &Verdict::Accept));
        assert_eq!(m.distinct_edges(), 1);
        assert_eq!(m.total_observations(), 2);
    }

    #[test]
    fn different_pairs_distinct() {
        let m = CoverageMap::new();
        m.observe(&Verdict::Accept, &Verdict::Accept);
        m.observe(&Verdict::Reject("9".into()), &Verdict::Accept);
        m.observe(&Verdict::Reject("9".into()), &Verdict::Reject("9".into()));
        assert_eq!(m.distinct_edges(), 3);
    }
}
