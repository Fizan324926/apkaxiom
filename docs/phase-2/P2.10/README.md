# P2.10 — Schrödinger APK Formal Semantics (Lean) — Bundle Composition Operator `⊕`

> Mechanize what an APK *is* in the App Bundle era. Bundle composition operator `⊕` formalized in Lean 4. BehaviorSet inclusion theorems. The mathematical foundation for the entire bundle-era pipeline.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §7 (Layer 2: Schrödinger)](../../../README.md#layer-2)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P2.10 |
| Owner(s) | G1 + G3 |
| Duration | Weeks 6–12 |
| Critical-path | yes |
| Hard prerequisites | P2.5, P2.6 (verified AXML + ARSC parsers — required to formalize what splits *contain*) |

## 2. Goal & Scope

The single most consequential intellectual contribution of Phase 2. Define formally what an App Bundle's behavior set is, mechanically:

- **Bundle composition operator** `⊕ : BaseAPK × Splits × DynamicFeatures × AssetPacks → BehaviorSet`
- **BehaviorSet inclusion theorems** — for each `(abi, density, language, modules)` configuration, the resulting program is well-defined and the behaviorSet contains it.
- **Differential property** — the bundle ⊕-composition's behavior matches what AOSP `installapex`/`pm` would actually install on each device configuration.

### In scope
- `theorems/Apkaxiom/Bundle/Compose.lean` — the `⊕` operator
- `theorems/Apkaxiom/Bundle/BehaviorSet.lean` — typed BehaviorSet definition
- `theorems/Apkaxiom/Bundle/Soundness.lean` — install-on-device theorem
- `theorems/Apkaxiom/Bundle/Configuration.lean` — feasible-configuration enumeration
- Examples + small theorems suite proving the operator behaves as expected

### Out of scope
- Rust implementation (P2.11, P2.12)
- Differential testing (P2.13)
- All Phase-3 reasoning (G5 lifts BehaviorSet for symbolic resolution)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P2.5** | Verified AXML parser — splits contain manifests we must formalize |
| **P2.6** | Verified ARSC parser — splits contribute resources |
| **P2.7** | DEX parser — splits contribute classes |
| **P2.9** | AXIOM-IR-v0.2 frozen types |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Lean 4 + mathlib4** | pinned | Theorem prover |
| **mathlib4 set theory + finite-product machinery** | from mathlib | Behavior sets are unions over finite products |
| **AOSP `frameworks/base/services/core/.../pm`** | per Android version | Reference for install behavior |
| **bundletool** (Google's Play Store reference) | latest | Reference for bundle composition |
| **Z3** (HAVE) | 4.12 | Side-tool for some configuration-space queries |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **bundletool** | reference | **Free** OSS (Apache 2.0) | https://github.com/google/bundletool | Google's reference implementation; we cross-check against it |
| **AAB format spec (Android docs)** | reference | **Free** | https://developer.android.com/guide/app-bundle/format | Authoritative |
| **AOSP `frameworks/base/services/core/.../pm` (PackageManagerService et al.)** | reference | **Free** OSS | already synced | Install-time behavior |
| **Zenodo** | DOI for the formalization | **Free** | https://zenodo.org | Publish formalization artifact |
| **Software Heritage** | source archival | **Free** | https://www.softwareheritage.org | Permanent archival |

**No new API keys.**

## 6. System Inventory — Have vs Need

### Already present
- ✅ Lean / Lake / mathlib4 cache
- ✅ Z3 4.12

### Missing — must install
- ❌ **bundletool** — `wget https://github.com/google/bundletool/releases/latest/download/bundletool-all.jar`

```bash
mkdir -p ~/tools/bundletool
curl -L https://github.com/google/bundletool/releases/latest/download/bundletool-all.jar \
  -o ~/tools/bundletool/bundletool.jar
echo 'alias bundletool="java -jar ~/tools/bundletool/bundletool.jar"' >> ~/.bashrc
```

## 7. Features & Functions Delivered (Comprehensive)

### Lean theorems

#### Configuration space
- `inductive ConfigSpace where | mk : abi : AbiSet × density : DensitySet × language : LangSet × modules : ModuleSet → ConfigSpace`
- `feasible_configurations : Bundle → Finset ConfigSpace`
- `device_compatible : ConfigSpace → DeviceState → Bool`

#### Bundle composition
- `compose : BaseAPK → Splits → DynamicFeatures → AssetPacks → ConfigSpace → MaterializedAPK`
- `⊕` notation for `compose`
- Lemma: `materialized_apk_well_formed : ∀ b s d a c, (b ⊕ s ⊕ d ⊕ a) c |> wellFormed`

#### BehaviorSet
- `BehaviorSet := { (c, m) | c ∈ feasible_configurations b ∧ m = b ⊕ s ⊕ d ⊕ a c }`
- `BehaviorSet.contains : BehaviorSet → MaterializedAPK → Prop`

#### Soundness theorems
- `bundle_install_sound : ∀ b s d a c, (b ⊕ s ⊕ d ⊕ a) c = m → AOSP_v.install (b, s, d, a) on (device_with c) ≡ install m`
- `behavior_set_complete : ∀ device, install_outcome b s d a device ∈ BehaviorSet b s d a`

#### Configuration helpers
- `enumerate_abi_splits` — given an AAB, list ABI splits
- `enumerate_density_splits`, `enumerate_language_splits`
- `enumerate_dynamic_features` — including delivery type (install-time / on-demand / fast-follow)

#### Adversarial reasoning
- `bundle_evasion_lemma` — formalizes the kind of evasion that hides malware in a dynamic feature module: not visible without ⊕ over the module's configuration

### Documentation
- `docs/lean-schrodinger.md` — design notes, theorem statements, proof sketches, undecidable cases (lifted to `Approx<T>` typed approximation)
- Worked examples — three end-to-end App Bundles formalized

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Cumulative Lean LOC (Bundle modules) | ≥ 1,500 | ≥ 2,500 |
| Theorem re-verify on CI | ≤ 30 min | ≤ 18 min |
| ⊕ operator formalized | yes | yes |
| BehaviorSet inclusion theorems proved | yes | yes |
| Soundness theorem proved (with respect to AOSP install) | yes | yes |
| Worked examples with full proofs | ≥ 3 | ≥ 5 |
| Reviewer sign-off (G1, G3 leads + external formal-methods reviewer) | yes | + community Zulip review |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── theorems/Apkaxiom/Bundle/
│   ├── Compose.lean                      # NEW — ~600 LOC
│   ├── BehaviorSet.lean                  # NEW — ~400 LOC
│   ├── Configuration.lean                # NEW — ~300 LOC
│   ├── Soundness.lean                    # NEW — ~400 LOC
│   └── Examples.lean                     # NEW — worked examples
├── corpus/bundles/
│   ├── examples/                         # 3+ AABs with hand-formalized behavior sets
│   └── adversarial-bundle-attacks/       # 100+ malware-in-dynamic-feature samples
└── docs/
    └── lean-schrodinger.md               # NEW
```

## 10. Standalone Output

A self-contained Lean library + paper-ready formalization:

```bash
nix develop
buck2 build //theorems:bundle-all
buck2 test //tests/bundle-examples
# All worked examples build cleanly; theorems re-verify
```

The formalization itself is a publishable artifact — Phase-2 paper directly cites it.

## 11. End-to-End Test

The three worked examples must:
1. Build cleanly in Lean.
2. Have explicit BehaviorSet enumeration.
3. Match AOSP install on Cuttlefish for each enumerated configuration.

```bash
buck2 test //tests/bundle-examples
# - All examples build (HARD)
# - BehaviorSet matches Cuttlefish install on each config (HARD)
```

## 12. Exit Checklist

- [ ] `⊕` operator formalized in Lean (HARD)
- [ ] BehaviorSet typed definition + inclusion theorems
- [ ] Soundness theorem proved against AOSP install
- [ ] ≥ 3 worked examples build with full proofs (HARD)
- [ ] Cumulative Lean LOC ≥ 1,500 (HARD)
- [ ] Theorem re-verify on CI ≤ 30 min (HARD)
- [ ] G1 + G3 lead sign-off (HARD)
- [ ] Adversarial bundle-evasion lemma stated and proved
- [ ] `docs/lean-schrodinger.md` published
- [ ] Formalization artifact prepared for Zenodo upload (in P2.19)

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P2.11** | App Bundle parser implements the ⊕ operator's input side |
| **P2.12** | Bundle resolver implements the ⊕ operator in Rust |
| **P2.13** | Differential testing checks ⊕ matches AOSP install behavior |
| **P2.18** | E2E pipeline operates on BehaviorSet, not single-APK |
| **P2.19** | Phase-2 paper builds on this formalization |
| **Phase 3 / G5** | Symbolic resolver reasons over BehaviorSet rather than single APKs |
