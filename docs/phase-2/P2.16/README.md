# P2.16 — Layer 3.3: Negative-Space Resource Anomaly Detector (G4)

> Treat resource tables as statistical objects. Detect injection without a malware corpus to compare against. Steganalysis applied to APK resources.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §8.3](../../../README.md#layer-3)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P2.16 |
| Owner(s) | G4 |
| Duration | Weeks 10–17 |
| Critical-path | no |
| Hard prerequisites | P2.6 (verified ARSC parser) |

## 2. Goal & Scope

A pure-statistics detector that flags resource-table anomalies suggestive of injection. No malware corpus required. Treats resource distributions as natural objects (like steganalysis treats images): legitimate apps have predictable distributions; malware-injected resources stand out.

### In scope
- Distributional priors over benign resource tables (computed from F-Droid + AndroZoo benign corpus)
- Per-anomaly scoring functions
- Configurable sensitivity thresholds
- Per-finding output with statistical confidence
- Streaming-aware analysis

### Out of scope
- DEX-level statistical anomaly (Phase 5)
- Cross-app statistical analysis (Phase 3 / G5)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P2.6** | Verified ARSC parses → analyze structural distributions |
| **P2.9** | AXIOM-IR-v0.2 |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Rust** | 1.95 | Implementation |
| **statrs / nalgebra** | latest | Statistical primitives |
| **DuckDB / Apache Arrow** | latest | Compute distributional priors over benign corpus |
| **Polars** (Rust DataFrames) | 0.40+ | Per-feature analytical pipelines |
| **Python (scipy.stats)** | latest | Reference scoring + threshold calibration |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **F-Droid archive** | benign corpus | **Free** | already provisioned | "Normal-looking" reference |
| **AndroZoo** | wider corpus | **Free academic** | already provisioned | Captures distributional variance |

**No new API keys.**

## 6. System Inventory — Have vs Need

### Already present
- ✅ Rust + DuckDB + Arrow

### Missing — must install
- ❌ **Polars** (Rust crate) — `cargo add polars --features lazy`

```bash
# Cargo dep, no system install
```

## 7. Features & Functions Delivered (Comprehensive)

### Distributional priors
- Computed over 50K+ benign APK resource tables (F-Droid + AndroZoo curated benign)
- Per-locale string-pool distributions (English-dominant apps have specific Russian/Chinese ratios)
- Per-density drawable distributions (apps target specific density buckets)
- Resource-ID range distributions (legitimate apps have contiguous ID ranges)
- Type-table cardinality distributions (typical app has N drawable, M string, K layout entries — distribution known)

### Public Rust API
- `pub fn detect(behavior_set: &BehaviorSet) -> Vec<NegativeSpaceFinding>`
- `pub struct NegativeSpaceFinding { kind: NegSpaceKind, score: f64, p_value: f64, location: ResourceRef, rationale: String }`
- `pub enum NegSpaceKind { LocaleStringOutlier, DensityDrawableOutlier, ResourceIdGap, TypeCardinalityOutlier, AssetSizeOutlier, FloatingResourceId, ... }`

### Anomaly types (~10 detector kinds)
1. **Locale-string outlier** — "English-only app with 1 Russian string" — flagged by chi-square test against locale-distribution prior.
2. **Density-drawable outlier** — "Single drawable in `drawable-anydpi` while everything else is `drawable-mdpi`."
3. **Resource-ID gap** — "Sole resource at 0x7f0099aa floating in empty 0x7f00xxxx range."
4. **Type-cardinality outlier** — "App with 1 layout, 1 string, 200 raws (highly atypical)."
5. **Asset-size outlier** — "Single `.png` with megabyte-scale unexplained padding."
6. **Floating resource ID** — non-contiguous ID assignment.
7. **String-pool dedup ratio anomaly** — typical apps have ~30% string pool overlap; injection breaks this.
8. **UTF-8/UTF-16 mix anomaly** — legitimate apps tend to be monolithically encoded; mixed encoding suggests injection.
9. **Public-resource ratio anomaly** — too few or too many public resources.
10. **Sparse-encoding inversion** — type table claims sparse but density is high (or vice versa).

### Streaming awareness
- Detectors operate on the BehaviorSet without re-parsing
- Per-config detection (a config-tagged finding can flag a config-specific anomaly)

### Per-finding output
- Score (raw)
- p-value (calibrated against benign prior)
- Resource reference + rationale
- Severity hint (low / medium / high)

### Threshold calibration
- Tuned to FP rate < 20 % on benign corpus (HARD per PHASE_GATES.md §6 — looser than other forensic passes because this is the most exploratory)
- Operator-tunable via configuration

### Documentation
- `docs/forensics-negative-space.md` — design, distributional model, calibration

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Negative-Space FP rate on benign 10K | < 20 % | < 8 % |
| Detection rate on Repack-2K | ≥ 60 % | ≥ 80 % |
| Throughput | ≥ 300 APKs/sec/16-core | ≥ 800 APKs/sec |
| Per-pass p99 latency | ≤ 80 ms | ≤ 30 ms |
| All anomaly kinds carry calibrated p-values | yes | yes |
| Reference distributional priors derived from ≥ 50K benign APKs | yes | ≥ 100K |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-l3-negative-space/
│       ├── Cargo.toml
│       ├── BUCK
│       └── src/
│           ├── lib.rs
│           ├── locale_string.rs
│           ├── density_drawable.rs
│           ├── resource_id_gap.rs
│           ├── type_cardinality.rs
│           ├── asset_size.rs
│           └── string_pool.rs
├── corpus/
│   └── benign-distributional-prior/    # 50K+ resource tables, columnar Arrow
├── tools/
│   └── distribution-fit/                # tool to recompute priors quarterly
└── docs/
    └── forensics-negative-space.md       # NEW
```

## 10. Standalone Output

```bash
buck2 build //crates/axiom-l3-negative-space --release
buck2 run //tools/cli -- negative-space /path/to/apk.apk
# Outputs JSON anomaly list + p-values
```

## 11. End-to-End Test

```bash
buck2 test //crates/axiom-l3-negative-space:full-eval
# - FP < 20% on benign 10K (HARD)
# - Detection ≥ 60% on Repack-2K (HARD)
# - Throughput ≥ 300 APKs/sec/16-core (HARD)
# - p99 ≤ 80 ms (HARD)
```

## 12. Exit Checklist

- [ ] All 10+ anomaly types implemented
- [ ] Distributional priors computed from ≥ 50K benign APKs
- [ ] FP < 20 % on benign 10K (HARD)
- [ ] Detection ≥ 60 % on Repack-2K (HARD)
- [ ] Throughput ≥ 300 APKs/sec/16-core (HARD)
- [ ] p99 ≤ 80 ms (HARD)
- [ ] Per-finding p-value calibrated
- [ ] Quarterly distribution-recompute runbook in place
- [ ] `docs/forensics-negative-space.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P2.18** | Negative-Space findings part of E2E pipeline |
| **Phase 4 / G7** | Findings shipped in `.axc` certificate |
| **Phase 5 / G11** | ML-model integrity work uses similar steganalysis approach |
