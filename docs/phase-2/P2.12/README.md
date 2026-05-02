# P2.12 — Bundle Resolver: Dynamic Feature Modules + Asset Packs (G3)

> Implement the ⊕ operator in Rust. Resolves App Bundle into BehaviorSet covering full configuration space. Dynamic-feature-module discovery rate ≥ 95%. The Schrödinger semantics in production code.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §7](../../../README.md#layer-2)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P2.12 |
| Owner(s) | G3 |
| Duration | Weeks 12–17 |
| Critical-path | yes |
| Hard prerequisites | P2.10 (Lean ⊕), P2.11 (AAB parser) |

## 2. Goal & Scope

The Layer 2 bundle resolver in Rust. Given an `AppBundle`, computes the full `BehaviorSet` over all feasible configurations. Handles dynamic feature modules (install-time / on-demand / fast-follow), asset packs, and module-fusing rules. Output drives all Phase-2+ analysis.

### In scope
- `crates/axiom-l2-bundle-resolver`
- ⊕ operator implementation (mirrors Lean P2.10)
- BehaviorSet construction with config-tagged components
- Dynamic feature module enumeration (including on-demand modules fetched from developer endpoint)
- Asset pack handling (install-time / fast-follow / on-demand)
- Configuration-tagged finding propagation
- Translation validation against Lean reference (key theorems)

### Out of scope
- Differential testing vs AOSP install (P2.13)
- Forensic passes consuming BehaviorSet (P2.14, P2.15, P2.16)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P2.10** | ⊕ operator Lean reference |
| **P2.11** | AAB parser produces input |
| **P2.9** | AXIOM-IR-v0.2 frozen — BehaviorSet uses these types |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Rust** | 1.95 | Implementation |
| **bumpalo** (from P2.8) | 3.x | Arena allocation for transient BehaviorSet construction |
| **rkyv** | 0.7+ | Persistent BehaviorSet representation |
| **ahash** | 0.8+ | Fast non-crypto hashing for config-key index |
| **Translation validator** (from P1.9) | latest | Spot-check ⊕ behavior matches Lean |
| **bundletool** (from P2.10) | latest | Cross-check on dynamic features |
| **HACL\* SHA-256** (from P1.10) | | For fingerprinting dynamic-feature module manifests |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **Google Play Store Asset Delivery API** | service | **Free** for read-only public bundles | (varies; commonly via Play Store) | Used to fetch on-demand modules for analysis (consent gate) |
| **Play Store crawl** | corpus | **Paid** if at scale via aggregators; **Free** at small scale via direct APK download | varies | Used to source AABs that ship dynamic features |
| **F-Droid bundles** | corpus | **Free** | https://f-droid.org/archive/ | Reference clean bundles |
| **bundletool** | reference | **Free** OSS | already installed | Cross-check |

**API key (potential):** Google Play credentials if we crawl Play Store at scale. For Phase 2, we use AndroZoo's bundle subset + F-Droid bundles (no Play credentials needed).

## 6. System Inventory — Have vs Need

### Already present
- ✅ Rust + bumpalo + rkyv + bundletool

### Missing
- Just Cargo deps (`ahash`, `rkyv`).

## 7. Features & Functions Delivered (Comprehensive)

### Public Rust API
- `pub fn resolve(bundle: AppBundle) -> Result<BehaviorSet, ResolverError>`
- `pub fn resolve_for_device(bundle: AppBundle, device: &DeviceState) -> Result<MaterializedAPK, ResolverError>`
- `pub struct BehaviorSet { configurations: Vec<(ConfigKey, MaterializedAPK)>, fused_modules: Vec<ModuleId>, asset_packs: Vec<AssetPackId> }`
- `pub struct ConfigKey { abi, density, language, dynamic_features: BitSet, asset_packs: BitSet }`
- `pub fn enumerate_feasible_configurations(bundle: AppBundle) -> impl Iterator<Item = ConfigKey>`

### ⊕ operator implementation
- Mirrors `theorems/Apkaxiom/Bundle/Compose.lean`
- Translation-validation hooks: spot-check on a 100-bundle test set that Rust ⊕ matches Lean reference
- Memory-efficient: BehaviorSet uses arena allocator + deduplication of shared content

### Dynamic feature module handling
- **Install-time** modules: included in every BehaviorSet element
- **On-demand** modules: tagged but not auto-included; analysis can request them
- **Fast-follow** modules: included in delayed BehaviorSet variants
- **Conditional** modules: based on device feature requirements
- On-demand fetching with **consent gating** — never fetches without explicit operator approval (security guardrail)

### Asset pack handling
- Install-time / fast-follow / on-demand asset packs
- Asset-pack-delivered resources lifted into resource dialect with `runtime_loaded_string` markers

### Module fusing
- Standalone vs fused module variants (per BundleConfig)
- Fusing rules formalized: which modules merge into base for older Android versions

### Configuration-tagged findings
- Every IR operation in BehaviorSet carries the ConfigKey it originates from
- Downstream forensic passes can filter "occurs only in config X" findings

### BehaviorSet memory efficiency
- Shared content (base manifest, base DEX) deduplicated
- Per-config delta encoded as patch over the base
- Memory representation ≤ 2.5× raw bundle size (HARD per PHASE_GATES.md §6)

### Documentation
- `docs/bundle-resolver.md` — design, ⊕ operator, configuration enumeration, security guardrails on on-demand fetching

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Bundle resolution throughput | ≥ 20 AABs/sec/16-core | ≥ 60 AABs/sec/16-core |
| Bundle resolution p99 (20-split bundle) | ≤ 3 s | ≤ 1 s |
| Dynamic-feature-module discovery rate | ≥ 95 % on Bundles-5K | 100 % |
| BehaviorSet memory representation | ≤ 2.5× raw bundle | ≤ 1.8× |
| Bundle resolution overhead vs single-APK | ≤ 60 % | ≤ 30 % |
| Translation-validation against Lean ⊕ | 100 % on 100-bundle test set | 100 % |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-l2-bundle-resolver/
│       ├── Cargo.toml
│       ├── BUCK
│       └── src/
│           ├── lib.rs
│           ├── compose.rs                # ⊕ operator
│           ├── behavior_set.rs           # arena-backed BehaviorSet
│           ├── config_enum.rs            # configuration enumeration
│           ├── dynamic_feature.rs
│           ├── asset_pack.rs
│           └── on_demand.rs              # consent-gated on-demand fetch
├── tests/bundle-resolver/
│   ├── translation-validation.rs         # vs Lean ⊕
│   └── bundles-5k.rs                     # full Bundles-5K eval
└── docs/
    └── bundle-resolver.md                # NEW
```

## 10. Standalone Output

```bash
nix develop
buck2 build //crates/axiom-l2-bundle-resolver --release
buck2 test //tests/bundle-resolver:translation-validation
# "100/100 BehaviorSets axiom-l2 ↔ Lean ⊕ agree"
buck2 run //bench:bundle-resolution -- --corpus bundles-5k
# "Throughput: 23 AABs/sec/16-core; p99: 2.4s; memory: 2.1× raw"
```

## 11. End-to-End Test

```bash
buck2 test //crates/axiom-l2-bundle-resolver:bundles-5k
# - Throughput ≥ 20 AABs/sec/16-core (HARD)
# - p99 ≤ 3 s (HARD)
# - Dynamic-feature discovery ≥ 95% (HARD)
# - Memory ≤ 2.5× raw (HARD)
# - Translation validation 100% (HARD)
```

## 12. Exit Checklist

- [ ] ⊕ operator implementation lands (HARD)
- [ ] BehaviorSet typed structure with arena backing
- [ ] Dynamic feature module handling (all three delivery types)
- [ ] Asset pack handling (all three delivery types)
- [ ] Configuration-tagged findings propagate
- [ ] Translation validation 100 % vs Lean (HARD)
- [ ] Throughput ≥ 20 AABs/sec/16-core (HARD)
- [ ] p99 ≤ 3 s (HARD)
- [ ] Discovery rate ≥ 95 % (HARD)
- [ ] Memory ≤ 2.5× raw (HARD)
- [ ] On-demand fetching is consent-gated
- [ ] `docs/bundle-resolver.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P2.13** | BehaviorSets ready for differential testing vs AOSP install |
| **P2.14, P2.15, P2.16** | BehaviorSet as input to forensic passes |
| **P2.17** | Differential fuzzer extends to AABs |
| **P2.18** | E2E pipeline operates on BehaviorSet |
| **Phase 3 / G5** | Symbolic resolver reasons over BehaviorSet |
