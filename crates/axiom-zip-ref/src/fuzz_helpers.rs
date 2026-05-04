// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Shared LCG for the test-only fuzz + round-trip modules. Same
//! constants as `tools/zip-corpus-gen` so seed determinism is the
//! same property the differential harness depends on.

// Test-only module; visibility is naturally crate-local. We
// silence the clippy `redundant_pub_crate` lint and the rustc
// `unreachable_pub` lint at module level — both fire because of
// the cfg(test) gating, neither is meaningful here.
#![allow(dead_code, clippy::redundant_pub_crate, unreachable_pub)]

pub(crate) struct Lcg {
    state: u64,
}

impl Lcg {
    pub(crate) const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub(crate) fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.state >> 32) as u32
    }

    pub(crate) fn next_in_range(&mut self, lo: u32, hi: u32) -> u32 {
        debug_assert!(lo < hi);
        lo + (self.next_u32() % (hi - lo))
    }

    pub(crate) fn fill(&mut self, out: &mut [u8]) {
        for byte in out {
            *byte = (self.next_u32() & 0xff) as u8;
        }
    }
}
