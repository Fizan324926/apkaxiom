# P2.15 — Layer 3.2: AXML Compiler Provenance Fingerprint + Classifier (G4)

> Identify which toolchain compiled this AXML. Single-sample repackaging detection: META-INF claims aapt2 but AXML structure says apktool → proven repackaging.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §8.2](../../../README.md#layer-3)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P2.15 |
| Owner(s) | G4 |
| Duration | Weeks 8–16 |
| Critical-path | no, but feeds Phase-2 KPI gate |
| Hard prerequisites | P2.5 (verified AXML — extract structural micro-features) |

## 2. Goal & Scope

Build a structural fingerprint of binary AXML that identifies the compiling toolchain (`aapt`, `aapt2`, `apktool`, `axmlpp`, Chinese-compiler variants, etc.). When the META-INF claims one toolchain but the AXML structural fingerprint says another, we have proven repackaging from a single sample.

### In scope
- Reference corpus: identical manifests compiled by every known toolchain (~10 toolchains × 500 manifests = 5,000 samples)
- Structural micro-feature extractor — string-pool ordering, attribute sort order, chunk padding, alignment quirks
- Classifier (rules + small statistical model)
- META-INF inconsistency detector
- Per-finding evidence (which features differ from claimed compiler's signature)

### Out of scope
- Native-code provenance (Phase 5)
- DEX provenance (Phase 5)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P2.5** | Verified AXML — structural micro-features extracted |
| **P2.9** | AXIOM-IR-v0.2 frozen |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Rust** | 1.95 | Implementation |
| **xgboost** (Rust binding via `xgboost-rs`) | 2.0+ | Classifier (rules-first; ML for tie-breaking) |
| **Python (scikit-learn)** | 1.3+ | Reference classifier training |
| **aapt2 / aapt / apktool** | latest / HAVE / HAVE | Reference toolchains |
| **axmlpp** | latest | Reference toolchain (alternative compiler) |
| **Chinese AXML compilers** | various | Variants used by some packers (Ai Jia Mi, 360, Jiagubao, ...) |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **aapt / aapt2** | binaries | **Free** | already installed | |
| **apktool** | binary | **Free** OSS | already installed | |
| **axmlpp** | binary | **Free** OSS | https://github.com/iBotPeaches/Apktool (or standalone variants) | |
| **Chinese AXML compiler binaries** | research / community | **Free** to obtain | various sources (some require online sample collection) | We use binary outputs only; no licensing concerns at our scale |
| **GitHub model releases (xgboost)** | model artifacts | **Free** OSS | https://github.com/dmlc/xgboost | |

**No new API keys.**

## 6. System Inventory — Have vs Need

### Already present
- ✅ aapt2, apktool
- ✅ Rust + Python + scikit-learn (already installed in P1.18)

### Missing — must install
- ❌ **xgboost-rs** — Cargo dep
- ❌ **axmlpp** — clone + build
- ❌ Chinese AXML compiler samples (research collection)

```bash
# axmlpp
git clone https://github.com/dx7/axmlpp third-party/axmlpp
cd third-party/axmlpp && make

# Chinese compilers — collected as binaries; documented in reference corpus README
```

## 7. Features & Functions Delivered (Comprehensive)

### Reference corpus
- 500 hand-curated benchmark manifests covering all common patterns (simple, complex, nested, intent-filter-rich, permission-rich, large-string-pool, ...)
- Each manifest compiled by ~10 known toolchains → 5,000 reference samples
- Features extracted per sample (~50 structural features)
- Distributional fingerprint per toolchain stored

### Public Rust API
- `pub fn fingerprint(axml: &AxmlDocument) -> Fingerprint`
- `pub fn classify(fp: &Fingerprint) -> Vec<(Toolchain, f64)>` — sorted by confidence
- `pub fn detect_inconsistency(meta_inf: &MetaInf, axml_fp: &Fingerprint) -> Option<RepackagingEvidence>`
- `pub struct Fingerprint { features: HashMap<FeatureName, FeatureValue> }`
- `pub enum Toolchain { Aapt, Aapt2, Apktool, Axmlpp, Chinese360, ChineseJiagubao, ChineseAJM, Other(String) }`

### Structural features (~50)
- String-pool ordering (sorted? alpha vs. usage-order?)
- Attribute sort order (resourceId, name, namespace, value)
- Chunk padding/alignment (4-byte? 8-byte? toolchain-specific quirks)
- Empty namespace declarations vs. compact
- Resource map sparse-encoding usage
- UTF-8 vs UTF-16 string-pool encoding patterns
- "Sorted" string-pool flag handling
- Whitespace handling (preserve / collapse / drop)
- Comment chunk presence
- ...

### Classifier
- Rules-first: if any single feature is decisive, no ML needed
- xgboost for tie-breaking on close cases
- Confidence threshold for "unknown" (avoid overconfident misidentification)

### META-INF inconsistency detector
- Reads `META-INF/MANIFEST.MF` for `Created-By:` field
- Reads APK signing block for tooling hints
- Cross-checks with structural fingerprint
- Inconsistency = strong repackaging evidence

### Repackaging evidence output
- Claimed toolchain (from META-INF / signing block)
- Detected toolchain (from structural fingerprint)
- Per-feature differences contributing to detection
- Confidence + p-value

### Documentation
- `docs/forensics-axml-provenance.md` — feature taxonomy, reference corpus methodology, classifier evaluation

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Toolchain identification accuracy on reference corpus | ≥ 95 % | ≥ 99 % |
| Misidentification rate | < 5 % | < 1 % |
| Throughput | ≥ 300 APKs/sec/16-core | ≥ 800 APKs/sec |
| Per-pass p99 latency | ≤ 30 ms | ≤ 10 ms |
| Repackaging detection precision (claimed-vs-actual mismatch on Repack-2K) | ≥ 95 % | ≥ 99 % |
| Reference corpus size | ≥ 5,000 samples | ≥ 10,000 |
| Toolchains covered | ≥ 8 | ≥ 12 |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-l3-axml-provenance/
│       ├── Cargo.toml
│       ├── BUCK
│       └── src/
│           ├── lib.rs
│           ├── features.rs              # ~50 structural feature extractors
│           ├── fingerprint.rs
│           ├── classifier.rs            # rules + xgboost tie-breaker
│           └── meta_inf.rs              # META-INF inconsistency detector
├── corpus/axml-provenance-reference/
│   ├── manifests/                       # 500 hand-curated source manifests
│   ├── compiled/                        # 5K compiled outputs across toolchains
│   └── corpus.toml                      # manifest with toolchain labels
├── tools/
│   └── compile-corpus/                  # automation to compile corpus
│       └── compile.py
├── tests/forensics/
│   └── axml-provenance-eval.rs
└── docs/
    └── forensics-axml-provenance.md     # NEW
```

## 10. Standalone Output

```bash
buck2 build //crates/axiom-l3-axml-provenance --release
buck2 run //tools/cli -- axml-provenance /path/to/apk.apk
# Outputs claimed vs detected toolchain, feature differences
buck2 test //tests/forensics:axml-provenance-eval
# "Toolchain ID accuracy: 97.3% on 5K reference; misid 2.7%"
```

## 11. End-to-End Test

```bash
buck2 test //crates/axiom-l3-axml-provenance:full-eval
# - Toolchain ID ≥ 95% accuracy (HARD)
# - Misid < 5% (HARD)
# - Throughput ≥ 300 APKs/sec/16-core (HARD)
# - Repackaging precision ≥ 95% on Repack-2K (HARD)
```

## 12. Exit Checklist

- [ ] Reference corpus compiled (≥ 5,000 samples, ≥ 8 toolchains)
- [ ] All ~50 structural features implemented
- [ ] Classifier trained and validated
- [ ] META-INF inconsistency detector
- [ ] Toolchain ID accuracy ≥ 95 % (HARD)
- [ ] Misidentification < 5 % (HARD)
- [ ] Throughput ≥ 300 APKs/sec/16-core (HARD)
- [ ] p99 ≤ 30 ms (HARD)
- [ ] Repackaging precision ≥ 95 % on Repack-2K (HARD)
- [ ] `docs/forensics-axml-provenance.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P2.18** | Provenance findings part of E2E pipeline output |
| **Phase 4 / G7** | Findings shipped as part of `.axc` certificate |
| **Bug-bounty pilot** | "Prove repackaging from single sample" is a flagship capability |
