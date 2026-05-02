# P2.14 — Layer 3.1: Shadow Stack — Forensic Deletion Detection (G4)

> Treat APK as forensic artifact. Detect what was *deleted* during repackaging — gaps in offsets, orphaned references, dangling indices. Single-sample repackaging detection.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §8.1](../../../README.md#layer-3)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P2.14 |
| Owner(s) | G4 (Structural Forensics) |
| Duration | Weeks 6–14 |
| Critical-path | no, but feeds Phase-2 KPI gate |
| Hard prerequisites | P2.5, P2.6 (need verified parses to detect what was deleted from them) |

## 2. Goal & Scope

The first of G4's three forensic passes. Treats the APK as a forensic artifact and detects what's *missing* — telltale signs of repackaging that current tools ignore.

### In scope
- `crates/axiom-l3-shadow-stack`
- ZIP-level forensics: gaps in entry offsets, stale CDH timestamps, sequence anomalies
- Manifest-level forensics: orphaned string-pool references, unreferenced resource IDs
- DEX-level forensics: dangling type/method/field index references
- Statistical anomaly model derived from a benign reference corpus
- Per-finding probability bound (Bayesian)
- Findings cite exact byte ranges + IR references

### Out of scope
- AXML provenance fingerprinting (P2.15)
- Negative-space resource detection (P2.16)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P2.5** | Verified AXML parses → orphaned-reference detection |
| **P2.6** | Verified ARSC parses → unreferenced-resource detection |
| **P2.8** | Verified DEX parses → dangling-index detection |
| **P2.9** | AXIOM-IR-v0.2 frozen — input to forensic passes |
| **P2.12** | BehaviorSet → run forensics over the union of configs |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Rust** | 1.95 | Implementation |
| **statrs** / **nalgebra** | latest | Statistical model |
| **DuckDB** (from P1.18 or now) | latest | Compute reference distributions |
| **Apache Arrow** | latest | Efficient corpus analytics |
| **rkyv** | 0.7+ | Persist findings |
| **HDR Histogram** | from P1.18 | Distribution capture |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **F-Droid archive** (reference benign corpus) | corpus | **Free** | already provisioned | Source for "what does benign look like?" |
| **AndroZoo** | corpus | **Free academic** | already provisioned | Wider distribution |
| **DREBIN labeled malware** | corpus | **Free research** | TU Braunschweig | For ground-truth tuning of detection thresholds |
| **Repack-2K corpus** | corpus | self-curated | Phase 2 | Curated repackaging pairs |

**No new API keys.**

## 6. System Inventory — Have vs Need

### Already present
- ✅ Rust + DuckDB + Arrow + rkyv

### Missing
- ❌ **statrs / nalgebra** — Cargo deps; just add to `Cargo.toml`

## 7. Features & Functions Delivered (Comprehensive)

### Public Rust API
- `pub fn detect(behavior_set: &BehaviorSet) -> Vec<ShadowFinding>`
- `pub struct ShadowFinding { kind: ShadowKind, location: ByteRange, probability: f64, rationale: String }`
- `pub enum ShadowKind { ZipOffsetGap, StaleCdhTimestamp, EntrySequenceAnomaly, OrphanedStringPoolRef, UnreferencedResourceId, DanglingDexTypeIdx, DanglingDexMethodIdx, DanglingDexFieldIdx, ResourceIdOutsideContiguousRange, ... }`

### ZIP-level signals (~10 signal types)
- Offset gaps (extra unreferenced bytes between entries)
- Stale Central Directory Header timestamps
- Out-of-order entry sequencing
- Local-file-header timestamps inconsistent with CDH
- "Extra field" data inconsistencies
- Suspicious filename encodings (UTF-8 misuse, zero-prefixed names)

### Manifest-level signals (~8 signal types)
- Orphaned string-pool indices (string allocated but never used)
- AXML attributes pointing into unallocated string-pool ranges
- Resource references to IDs that don't exist in resources.arsc

### Resource-table signals (~6 signal types)
- Unreferenced resource IDs
- Resource IDs in non-contiguous ranges (suspicious "floating" IDs)
- Type-table sparse-encoding inconsistencies

### DEX-level signals (~12 signal types)
- Dangling type indices (referenced but not defined)
- Dangling method indices
- Dangling field indices
- Class definitions without code-item references
- Annotations referring to non-existent items
- Source-file references to deleted source units

### Statistical model
- Reference benign distribution (per signal type) computed over 50K+ benign APKs from F-Droid + AndroZoo
- Per-signal Bayesian posterior `P(repackaging | signal-count)`
- Combined probability `P(repackaging | all signals)` via naive Bayes (with calibration)
- Threshold-tuned to FP rate < 10 % on benign corpus (HARD per PHASE_GATES.md §6)

### Per-finding output
- Byte-range
- IR-element reference
- Signal type + count
- Probability bound (with confidence interval)
- Rationale string (human-readable)

### Performance
- Single-pass analysis over BehaviorSet (no extra parses)
- Parallel forensic-signal evaluation across signals
- Throughput target: ≥ 300 APKs/sec/16-core

### Documentation
- `docs/forensics-shadow-stack.md` — design, signal taxonomy, statistical model, calibration

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Shadow Stack FP rate on benign 10K | < 10 % | < 3 % |
| Shadow Stack throughput | ≥ 300 APKs/sec/16-core | ≥ 500 APKs/sec |
| Per-pass p99 latency | ≤ 80 ms | ≤ 30 ms |
| Detection rate on Repack-2K | ≥ 80 % | ≥ 95 % |
| Each finding has typed location + rationale | 100 % | 100 % |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-l3-shadow-stack/
│       ├── Cargo.toml
│       ├── BUCK
│       └── src/
│           ├── lib.rs
│           ├── zip_signals.rs
│           ├── manifest_signals.rs
│           ├── resource_signals.rs
│           ├── dex_signals.rs
│           └── bayesian.rs
├── corpus/
│   ├── shadow-reference-benign/         # 50K F-Droid + AndroZoo APKs for distribution computation
│   └── repack-2k/                        # known repackaging pairs
├── tests/forensics/
│   └── shadow-stack-eval.rs              # FP + recall measurement
└── docs/
    └── forensics-shadow-stack.md         # NEW
```

## 10. Standalone Output

```bash
buck2 build //crates/axiom-l3-shadow-stack --release
buck2 run //tools/cli -- shadow-stack /path/to/apk.apk
# Outputs JSON list of findings with probability bounds
buck2 test //tests/forensics:shadow-stack-eval
# "FP=2.4% on benign-10K; Recall=89% on Repack-2K"
```

## 11. End-to-End Test

```bash
buck2 test //crates/axiom-l3-shadow-stack:full-eval
# - FP < 10% on benign 10K (HARD)
# - Recall ≥ 80% on Repack-2K (HARD)
# - Throughput ≥ 300 APKs/sec/16-core (HARD)
# - Each finding has byte range + rationale
```

## 12. Exit Checklist

- [ ] Shadow Stack crate compiles
- [ ] All 36+ signal types implemented
- [ ] Statistical model calibrated on benign corpus
- [ ] FP rate < 10 % on benign 10K (HARD)
- [ ] Recall ≥ 80 % on Repack-2K (HARD)
- [ ] Throughput ≥ 300 APKs/sec/16-core (HARD)
- [ ] p99 ≤ 80 ms per pass (HARD)
- [ ] Findings carry typed location + Bayesian probability
- [ ] `docs/forensics-shadow-stack.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P2.18** | Shadow-Stack findings part of E2E pipeline output |
| **Phase 4 / G7** | Findings shipped as part of `.axc` certificate |
| **External consumers** | First serious open-source single-sample repackaging detector |
