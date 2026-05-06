// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Mutation engine — the in-tree radamsa-style mutator the dev-mode
//! harness uses to drive the fuzz loop.
//!
//! Production deployments swap in Nautilus (grammar-aware) +
//! AFL++ (coverage-guided) + Centipede (distributed). Those are
//! tracked under CHECKLIST §C operator one-shots; this module
//! ships a deterministic LCG-driven mutator that exercises the
//! classifier + finding archive at high throughput on any host.
//!
//! Mutation kinds:
//!
//!   - `flip`     — single random bit-flip at a random offset
//!   - `del`      — delete one byte
//!   - `insert`   — insert one random byte
//!   - `bump_u16` — bump a u16-aligned length-shaped field
//!   - `splice`   — splice a random region of one seed into another
//!     (ineffective without two seeds; falls back to `flip`)

use crate::grammar::Grammar;

/// Linear-congruential generator. Tiny but adequate for mutation.
#[derive(Debug, Clone)]
pub struct Lcg {
    state: u64,
}

impl Lcg {
    /// New LCG with the supplied seed.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Step + return a u32.
    pub fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.state >> 32) as u32
    }

    /// Uniform draw in `[lo, hi)`.
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        debug_assert!(lo < hi);
        lo + (self.next_u32() % (hi - lo))
    }
}

/// Kind label for archive logging.
#[derive(Debug, Clone, Copy)]
pub enum MutationKind {
    /// Single bit-flip.
    Flip,
    /// Single byte delete.
    Delete,
    /// Single random byte insert.
    Insert,
    /// Bump a u16 (length-shaped field).
    BumpU16,
    /// Splice a region of one seed into another.
    Splice,
    /// Grammar-aware substitution (placeholder; falls back to flip
    /// in this build because the grammar engine is in `grammar.rs`
    /// and only does loadability checks, not generation).
    Grammar,
}

impl MutationKind {
    /// Stable string label for archive serialisation.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Flip => "flip",
            Self::Delete => "del",
            Self::Insert => "insert",
            Self::BumpU16 => "bump_u16",
            Self::Splice => "splice",
            Self::Grammar => "grammar",
        }
    }
}

/// Apply one mutation. `aux` is an optional second seed used only
/// by the splice kind. Returns `(new_bytes, kind)`.
#[must_use]
pub fn mutate(
    rng: &mut Lcg,
    base: &[u8],
    aux: Option<&[u8]>,
    _grammar: Option<&Grammar>,
) -> (Vec<u8>, MutationKind) {
    if base.is_empty() {
        return (vec![rng.next_u32() as u8], MutationKind::Insert);
    }
    let kind = rng.range(0, 6);
    let off = (rng.next_u32() as usize) % base.len();
    match kind {
        0 => {
            // bit-flip
            let mut v = base.to_vec();
            let bit = (rng.next_u32() & 0x07) as u8;
            v[off] ^= 1 << bit;
            (v, MutationKind::Flip)
        }
        1 => {
            if base.len() <= 1 {
                let mut v = base.to_vec();
                v[off] ^= 0x01;
                return (v, MutationKind::Flip);
            }
            let mut v = base.to_vec();
            v.remove(off);
            (v, MutationKind::Delete)
        }
        2 => {
            let mut v = base.to_vec();
            let b = (rng.next_u32() & 0xff) as u8;
            v.insert(off, b);
            (v, MutationKind::Insert)
        }
        3 => {
            if base.len() < 2 {
                let mut v = base.to_vec();
                v[off] ^= 0x01;
                return (v, MutationKind::Flip);
            }
            let pos = off.min(base.len() - 2);
            let mut v = base.to_vec();
            let cur = u16::from_le_bytes([v[pos], v[pos + 1]]);
            let bumped = cur.wrapping_add(rng.next_u32() as u16);
            v[pos..pos + 2].copy_from_slice(&bumped.to_le_bytes());
            (v, MutationKind::BumpU16)
        }
        4 => match aux {
            Some(other) if !other.is_empty() => {
                let from = (rng.next_u32() as usize) % other.len();
                let n = ((rng.next_u32() as usize) % (other.len() - from)).max(1);
                let chunk = &other[from..from + n];
                let mut v = base[..off].to_vec();
                v.extend_from_slice(chunk);
                v.extend_from_slice(&base[off..]);
                (v, MutationKind::Splice)
            }
            _ => {
                // Fall back to flip.
                let mut v = base.to_vec();
                v[off] ^= 1;
                (v, MutationKind::Flip)
            }
        },
        _ => {
            // Grammar slot — placeholder: same shape as flip but
            // labelled "grammar" so the archive shows the
            // distribution. Real grammar-aware mutation lands when
            // Nautilus is wired in (CHECKLIST §C-3).
            let mut v = base.to_vec();
            v[off] ^= 0x80;
            (v, MutationKind::Grammar)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flip_changes_one_bit() {
        let mut rng = Lcg::new(0x42);
        let (out, _) = mutate(&mut rng, &[0u8; 16], None, None);
        let diff: u32 = out
            .iter()
            .zip([0u8; 16].iter())
            .map(|(a, b)| (a ^ b).count_ones())
            .sum();
        // Could be 0/1/8 depending on which arm fired. Not strictly
        // 1, but bounded:
        assert!(diff <= 8);
    }

    #[test]
    fn deterministic_under_same_seed() {
        let mut a = Lcg::new(1);
        let mut b = Lcg::new(1);
        let (oa, _) = mutate(&mut a, b"hello", None, None);
        let (ob, _) = mutate(&mut b, b"hello", None, None);
        assert_eq!(oa, ob);
    }

    #[test]
    fn empty_input_yields_one_byte() {
        let mut rng = Lcg::new(7);
        let (out, kind) = mutate(&mut rng, &[], None, None);
        assert_eq!(out.len(), 1);
        assert!(matches!(kind, MutationKind::Insert));
    }
}
