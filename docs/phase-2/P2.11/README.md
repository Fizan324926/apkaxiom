# P2.11 — App Bundle (AAB) Parser — Base + ABI/Density/Language Splits

> Parse Android App Bundle (.aab) files end-to-end. Base APK + every split type (ABI, density, language). Differential against bundletool. Coverage ≥ 99 % on Bundles-5K.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §7](../../../README.md#layer-2)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P2.11 |
| Owner(s) | G2 + G3 |
| Duration | Weeks 8–14 |
| Critical-path | yes |
| Hard prerequisites | P2.5, P2.6 (verified AXML + ARSC ready) |

## 2. Goal & Scope

A Rust parser for the `.aab` format: top-level zip envelope, `BundleConfig.pb` (Protocol Buffer), per-module structure (manifest/, dex/, res/, lib/), split-config metadata, asset-pack delivery descriptors. Produces an `AppBundle` typed structure ready for the `⊕` operator (P2.12 implementation).

### In scope
- `crates/axiom-aab-parser` — full AAB parser
- BundleConfig.pb decoding (Google's protobuf schema)
- Per-module structure parsing (base, feature modules, asset packs)
- Split metadata extraction (ABI splits, density splits, language splits)
- Differential vs `bundletool extract-all` on Bundles-5K corpus
- Streaming-aware (extends Layer 0 streaming spine)

### Out of scope
- Dynamic feature module on-device installation behavior (P2.12)
- Schrödinger BehaviorSet construction (P2.12 implements the ⊕ operator)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P2.5** | Verified AXML parser for module manifests |
| **P2.6** | Verified ARSC parser for module resources |
| **P2.10** | Schrödinger Lean formalism — defines what we must extract |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Rust** | 1.95 | Implementation |
| **prost** | 0.13+ | Protobuf compiler/runtime for BundleConfig.pb |
| **protobuf** schema files | from bundletool | Schema for BundleConfig + native build configs |
| **bundletool** | latest | Reference oracle |
| **Glommio** | from P1.7 | Streaming runtime |
| **deku / scroll** | from P1.7 | Binary parser combinators |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **bundletool** | reference | **Free** OSS | https://github.com/google/bundletool | Already installed in P2.10 |
| **AAB format spec** | reference | **Free** | https://developer.android.com/guide/app-bundle/format | |
| **BundleConfig.proto schema** | reference | **Free** OSS | https://github.com/google/bundletool/blob/master/src/main/proto/config.proto | Vendored into our schema/ |
| **Android Asset Packaging Tool (aapt2)** | reference | **Free** | already installed | Used to inspect AABs at lower level |
| **Bundles-5K AAB corpus** | corpus | **Free academic** | sourced from AndroZoo + F-Droid + Play Store crawl | API keys from P1.3 |

**No new API keys.**

## 6. System Inventory — Have vs Need

### Already present
- ✅ Rust + Glommio + deku
- ✅ bundletool (P2.10)
- ✅ AAA aapt2

### Missing — must install
- ❌ **prost** — `cargo add prost --features prost-derive` (Cargo dep)
- ❌ **protoc** — `sudo apt-get install -y protobuf-compiler`

```bash
sudo apt-get install -y protobuf-compiler
# Cargo dep
# crates/axiom-aab-parser/Cargo.toml:
#   prost = "0.13"
#   prost-derive = "0.13"
#   prost-build = "0.13"  # build.rs
```

## 7. Features & Functions Delivered (Comprehensive)

### Public Rust API
- `pub fn parse_aab(bytes: &[u8]) -> Result<AppBundle, AabError>`
- `pub fn parse_aab_streaming<R: Read>(reader: R) -> impl Stream<Item = AabEvent>`
- `pub struct AppBundle { config, base_module, feature_modules, asset_packs }`
- `pub struct BaseModule { manifest, resources, dex, native_libs }`
- `pub struct FeatureModule { name, delivery, manifest, resources, dex, native_libs }`
- `pub struct AssetPack { name, delivery, assets }`
- `pub struct DeliveryConfig { install_time, on_demand, fast_follow }`
- `pub struct Splits { abi: Vec<AbiSplit>, density: Vec<DensitySplit>, language: Vec<LangSplit> }`

### BundleConfig.pb decoding
- Compression strategy, split dimensions, optimization mode
- Native libs split-by-ABI configuration
- Master split declarations
- API-level split-disable rules

### Split extraction
- ABI splits: `armeabi-v7a`, `arm64-v8a`, `x86`, `x86_64`
- Density splits: `mdpi`, `hdpi`, `xhdpi`, `xxhdpi`, `xxxhdpi`, `nodpi`
- Language splits: per-locale, with fallback rules
- All splits emit AXIOM-IR with split-aware references

### Streaming awareness
- `parse_aab_streaming` emits events as base module → splits arrive
- Glommio runtime, no full-file load
- Wire-speed compatible

### Differential testing
- For each AAB: extract via our parser → extract via `bundletool extract-all` → diff
- Per-module byte-identity check on extracted APKs (with documented exceptions for compression normalization)
- Per-config feasibility check (we agree on which configs the bundle declares)

### Error handling
- `AabError` enum covering all failure modes; never panics
- All errors carry byte-range location

### Documentation
- `docs/aab-parser.md` — design, BundleConfig.pb schema notes, split-resolution rules

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Coverage on Bundles-5K (parses without error) | ≥ 99 % | ≥ 99.9 % |
| Bundle parse throughput (single-core) | ≥ 30 AABs/sec | ≥ 80 AABs/sec |
| Streaming first-event latency | ≤ 50 ms after byte 0 | ≤ 15 ms |
| Differential vs bundletool extract-all | ≥ 99.5 % per-module byte-identical | ≥ 99.9 % |
| All split dimensions enumerated correctly | 100 % | 100 % |
| Memory peak per AAB parse | ≤ 200 MB | ≤ 100 MB |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-aab-parser/
│       ├── Cargo.toml
│       ├── BUCK
│       ├── build.rs                       # prost-build: BundleConfig.pb → Rust
│       └── src/
│           ├── lib.rs
│           ├── streaming.rs
│           ├── bundle_config.rs           # generated from .proto
│           ├── module.rs
│           ├── splits.rs
│           ├── asset_pack.rs
│           └── error.rs
├── schema/
│   └── bundle-config.proto                # vendored from bundletool
├── corpus/
│   └── bundles-5k/                        # 5K real-world AABs (AndroZoo + F-Droid + Play Store crawl)
├── tests/aab-differential/
│   └── src/main.rs                        # diff vs bundletool
└── docs/
    └── aab-parser.md                      # NEW
```

## 10. Standalone Output

```bash
nix develop
buck2 build //crates/axiom-aab-parser --release
buck2 test //tests/aab-differential
# "4980/5000 AABs parsed; 99.6% byte-identical extraction vs bundletool"
```

## 11. End-to-End Test

```bash
buck2 test //crates/axiom-aab-parser:bundles-5k
# - Coverage ≥ 99% (HARD)
# - Differential ≥ 99.5% byte-identical (HARD)
# - All split dimensions enumerated (HARD)
# - Throughput ≥ 30 AABs/sec/core (HARD)
# - Memory ≤ 200 MB peak (HARD)
```

## 12. Exit Checklist

- [ ] `axiom-aab-parser` crate compiles
- [ ] BundleConfig.pb fully decoded
- [ ] All split dimensions handled
- [ ] Streaming entrypoint with Glommio
- [ ] Coverage ≥ 99 % on Bundles-5K (HARD)
- [ ] Differential ≥ 99.5 % byte-identical vs bundletool (HARD)
- [ ] Throughput ≥ 30 AABs/sec/core (HARD)
- [ ] Memory ≤ 200 MB peak per AAB (HARD)
- [ ] `docs/aab-parser.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P2.12** | `AppBundle` typed structure as input to ⊕ resolver |
| **P2.13** | AABs ready for differential testing vs AOSP install |
| **P2.18** | Bundles-5K corpus measured in E2E |
| **Phase 3 / G5** | Bundle structure + BehaviorSet for cross-config symbolic reasoning |
