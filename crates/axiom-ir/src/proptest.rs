// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Deterministic seeded generator for property-based round-trip testing.
//!
//! Hand-rolled, no `proptest` / `quickcheck` dep — those would force a
//! Reindeer regen for a test-only crate. The shape of this module is:
//!
//!   * [`Rng`] — minimal SplitMix64 PRNG (hand-rolled, deterministic).
//!   * [`gen_manifest`] / [`gen_resource`] / [`gen_module`] — build a
//!     valid IR value from a `u64` seed.
//!
//! The properties exercised in [`crate::canonical`]'s `proptests` test
//! module are:
//!
//!   * **Round-trip**: `decode(encode(m)) == m` for all generators, all
//!     seeds in `0..=10_000`.
//!   * **Idempotent encode**: `encode(decode(encode(m))) == encode(m)`
//!     (i.e. canonical bytes are a fixed point of the round-trip).
//!   * **Deterministic encode**: `encode(m) == encode(m.clone())`.
//!   * **Hash determinism**: `commitment_hash(m) == commitment_hash(m)`.
//!
//! Test budget: 10,000 seeds × 3 properties × 3 generators = 90,000
//! property checks per `cargo test`. Runtime ≈ 1 s on dev profile.
//!
//! This module is `pub(crate)` and `cfg(test)`-only — it is not part of
//! the v0.1 public API. The generator output shape may evolve without
//! a schema bump.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::redundant_pub_crate,
    clippy::missing_docs_in_private_items,
    clippy::doc_markdown
)]

use crate::core::Tribool;
use crate::manifest::{
    Component, ComponentKind, DataAuthority, DataFilter, IntentFilter, ManifestModule, Permission,
    ProtectionLevel,
};
use crate::resource::{
    Configuration, ResourceEntry, ResourceId, ResourceRef, ResourceTable, ResourceType,
    ResourceValue, StringPool,
};

// ---------------------------------------------------------------------------
// SplitMix64 — tiny, deterministic, well-distributed PRNG.
// Reference: Steele/Lea/Flood, "Fast Splittable Pseudorandom Number
// Generators", OOPSLA'14. We use the canonical 64-bit step constants.
// ---------------------------------------------------------------------------

pub(crate) struct Rng {
    state: u64,
}

impl Rng {
    pub(crate) const fn new(seed: u64) -> Self {
        // Avoid the all-zero state which makes the first two outputs degenerate.
        Self {
            state: seed.wrapping_add(0x9E37_79B9_7F4A_7C15),
        }
    }

    pub(crate) fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub(crate) fn range(&mut self, lo: u32, hi: u32) -> u32 {
        debug_assert!(lo < hi);
        let span = u64::from(hi - lo);
        lo + ((self.next_u64() % span) as u32)
    }

    pub(crate) fn bool_with(&mut self, p_numerator: u32, p_denominator: u32) -> bool {
        self.next_u64() % u64::from(p_denominator) < u64::from(p_numerator)
    }

    pub(crate) fn pick<'a, T>(&mut self, slice: &'a [T]) -> &'a T {
        let idx = self.range(0, slice.len() as u32) as usize;
        &slice[idx]
    }

    pub(crate) fn ascii_word(&mut self, len: u32) -> String {
        const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789_.";
        let mut s = String::with_capacity(len as usize);
        for _ in 0..len {
            let i = self.range(0, CHARS.len() as u32) as usize;
            s.push(CHARS[i] as char);
        }
        s
    }

    /// Convenience: pick a length in `[lo, hi)` then a word of that length.
    pub(crate) fn word_in(&mut self, lo: u32, hi: u32) -> String {
        let len = self.range(lo, hi);
        self.ascii_word(len)
    }
}

// ---------------------------------------------------------------------------
// Tribool / ProtectionLevel / ComponentKind / ResourceType pickers.
// ---------------------------------------------------------------------------

pub(crate) fn gen_tribool(rng: &mut Rng) -> Tribool {
    *rng.pick(&[Tribool::True, Tribool::False, Tribool::Default])
}

pub(crate) fn gen_component_kind(rng: &mut Rng) -> ComponentKind {
    *rng.pick(&[
        ComponentKind::Activity,
        ComponentKind::Service,
        ComponentKind::Receiver,
        ComponentKind::Provider,
    ])
}

pub(crate) fn gen_protection(rng: &mut Rng) -> ProtectionLevel {
    *rng.pick(&[
        ProtectionLevel::Normal,
        ProtectionLevel::Dangerous,
        ProtectionLevel::Signature,
        ProtectionLevel::SignatureOrSystem,
        ProtectionLevel::Internal,
    ])
}

pub(crate) fn gen_resource_type(rng: &mut Rng) -> ResourceType {
    *rng.pick(&[
        ResourceType::String,
        ResourceType::Drawable,
        ResourceType::Layout,
        ResourceType::Color,
        ResourceType::Dimen,
        ResourceType::Style,
        ResourceType::Bool,
        ResourceType::Integer,
        ResourceType::Raw,
    ])
}

// ---------------------------------------------------------------------------
// Manifest generator.
// ---------------------------------------------------------------------------

pub(crate) fn gen_data_filter(rng: &mut Rng) -> DataFilter {
    DataFilter {
        scheme: opt_string(rng, 1, 8),
        host: opt_string(rng, 3, 12),
        port: opt_string(rng, 1, 5),
        path: opt_string(rng, 1, 16),
        path_prefix: opt_string(rng, 1, 8),
        path_pattern: opt_string(rng, 1, 12),
        mime_type: opt_string(rng, 3, 24),
    }
}

pub(crate) fn gen_intent_filter(rng: &mut Rng) -> IntentFilter {
    let n_actions = rng.range(0, 4);
    let n_categories = rng.range(0, 4);
    let n_data = rng.range(0, 3);
    let actions = (0..n_actions).map(|_| rng.word_in(3, 16)).collect();
    let categories = (0..n_categories).map(|_| rng.word_in(3, 16)).collect();
    let data = (0..n_data).map(|_| gen_data_filter(rng)).collect();
    IntentFilter {
        actions,
        categories,
        data,
        priority: rng.next_u64() as i32,
    }
}

pub(crate) fn gen_authority(rng: &mut Rng) -> DataAuthority {
    let host = rng.word_in(4, 20);
    DataAuthority {
        host,
        port: opt_string(rng, 1, 5),
    }
}

pub(crate) fn gen_component(rng: &mut Rng) -> Component {
    let kind = gen_component_kind(rng);
    let n_filters = rng.range(0, 4);
    let n_authorities = if matches!(kind, ComponentKind::Provider) {
        rng.range(0, 3)
    } else {
        0
    };
    let name_body = rng.word_in(3, 16);
    let exported = gen_tribool(rng);
    let enabled = gen_tribool(rng);
    let permission = opt_string(rng, 4, 32);
    let intent_filters = (0..n_filters).map(|_| gen_intent_filter(rng)).collect();
    let authorities = (0..n_authorities).map(|_| gen_authority(rng)).collect();
    Component {
        kind,
        name: format!(".{name_body}"),
        exported,
        enabled,
        permission,
        intent_filters,
        authorities,
    }
}

pub(crate) fn gen_permission(rng: &mut Rng) -> Permission {
    let name = rng.word_in(8, 32);
    Permission {
        name,
        protection: gen_protection(rng),
        group: opt_string(rng, 4, 16),
    }
}

pub(crate) fn gen_manifest(seed: u64) -> ManifestModule {
    let mut rng = Rng::new(seed);
    let n_components = rng.range(0, 6);
    let n_permissions = rng.range(0, 4);
    let n_uses = rng.range(0, 5);
    let pkg = rng.word_in(4, 16);
    let target_sdk = rng.range(21, 36) as u8;
    let min_sdk = rng.range(21, 36) as u8;
    let application_label = opt_string(&mut rng, 4, 32);
    let components = (0..n_components).map(|_| gen_component(&mut rng)).collect();
    let permissions = (0..n_permissions)
        .map(|_| gen_permission(&mut rng))
        .collect();
    let uses_permissions = (0..n_uses).map(|_| rng.word_in(8, 32)).collect();
    ManifestModule {
        package: format!("com.apkaxiom.{pkg}"),
        target_sdk,
        min_sdk,
        application_label,
        components,
        permissions,
        uses_permissions,
    }
}

// ---------------------------------------------------------------------------
// Resource generator.
// ---------------------------------------------------------------------------

pub(crate) fn gen_string_pool(rng: &mut Rng) -> StringPool {
    let n = rng.range(0, 16);
    let mut p = StringPool::new();
    for _ in 0..n {
        let s = rng.word_in(1, 32);
        let _ = p.intern(s);
    }
    p
}

pub(crate) fn gen_configuration(rng: &mut Rng) -> Configuration {
    let qualifier = rng.word_in(4, 24);
    Configuration {
        qualifier,
        density_dpi: rng.range(0, 700),
        locale: opt_string(rng, 2, 8),
        orientation: opt_string(rng, 3, 6),
        min_sdk: rng.range(21, 36) as u8,
    }
}

pub(crate) fn gen_resource_ref(rng: &mut Rng) -> ResourceRef {
    let r#type = gen_resource_type(rng);
    let id = ResourceId(rng.next_u64() as u32);
    let name = rng.word_in(2, 24);
    ResourceRef { r#type, id, name }
}

pub(crate) fn gen_resource_value(rng: &mut Rng) -> ResourceValue {
    match rng.range(0, 4) {
        0 => {
            let s = rng.word_in(0, 64);
            ResourceValue::String(s)
        }
        1 => ResourceValue::Int(rng.next_u64() as i32),
        2 => ResourceValue::Bool(rng.bool_with(1, 2)),
        _ => ResourceValue::Ref(gen_resource_ref(rng)),
    }
}

pub(crate) fn gen_resource_table(seed: u64) -> ResourceTable {
    let mut rng = Rng::new(seed);
    let n_configs = rng.range(0, 5);
    let n_entries = rng.range(0, 8);
    let pkg = rng.word_in(4, 16);
    let string_pool = gen_string_pool(&mut rng);
    let configurations = (0..n_configs)
        .map(|_| gen_configuration(&mut rng))
        .collect();
    let entries = (0..n_entries)
        .map(|_| ResourceEntry {
            ref_: gen_resource_ref(&mut rng),
            value: gen_resource_value(&mut rng),
        })
        .collect();
    ResourceTable {
        package: format!("com.apkaxiom.{pkg}"),
        string_pool,
        configurations,
        entries,
    }
}

// ---------------------------------------------------------------------------
// Helpers.
// ---------------------------------------------------------------------------

fn opt_string(rng: &mut Rng, lo: u32, hi: u32) -> Option<String> {
    if rng.bool_with(1, 2) {
        Some(rng.word_in(lo, hi))
    } else {
        None
    }
}
