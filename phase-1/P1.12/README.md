# P1.12 — Rust Extraction of Full ZIP Layer

> Replace hand-written Rust ZIP parser with Lean-extracted code. Translation validator green on Bench-10K. Perf within 15% of hand-written.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../README.md §6](../../README.md#layer-1)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P1.12 |
| Owner(s) | G1 + G2 |
| Duration | Weeks 11–14 |
| Critical-path | yes |
| Hard prerequisites | P1.6 (full ZIP Lean), P1.9 (extraction pipeline) |

## 2. Goal & Scope

The full Lean ZIP layer (LFH + CDR + EOCD + cross-record consistency) is extracted to Rust and replaces the hand-written ZIP parser inside `axiom-l0`. The translation validator passes on Bench-10K. Performance is within 15% of the hand-written reference.

### In scope
- Extracted crate `axiom-l0-zip-verified`
- `axiom-l0` defaults to verified path; feature flag retains hand-written fallback (deleted in Phase 2)
- Translation validator runs nightly on Bench-10K
- Performance regression gate: verified vs hand-written ≤ 15%

### Out of scope
- Signing block extraction (P1.16).
- Production-grade SIMD/AVX-512 hand-tuning of extracted code (Phase 2).

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P1.6** | Full Lean ZIP modules (LFH + CDR + EOCD + Consistency) |
| **P1.9** | Extraction pipeline + translation validator |

## 4. Required Tools, Libraries, and Languages

Same as P1.9 + P1.6. New: a robust set of perf benchmarks comparing verified vs hand-written.

| Tool | Version | Purpose |
|---|---|---|
| **criterion** | 0.5+ | Microbenchmarks |
| **cargo-flamegraph** | from P1.3 | Profile divergences |
| **perf** | HAVE | Hardware counters |
| **Bench-10K corpus** | from P1.18 prep | Real-world APK distribution |

## 5. Third-Party Software, Services, Accounts & API Keys

Same dependencies as P1.6 and P1.9. **No new third-party services or API keys.**

## 6. System Inventory — Have vs Need

### Already present (after prior sub-phases)
- ✅ Lean / Lake / extraction pipeline / ZIP theorems
- ✅ Hand-written Rust ZIP layer in `axiom-l0` (built incrementally during P1.7+)
- ✅ Bench-1K (in progress) and start of Bench-10K curation

### Missing
- Bench-10K curation must complete by sub-phase end (depends on AndroZoo access from P1.3)

## 7. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   ├── axiom-l0-zip-verified/           # NEW — auto-generated from Lean
│   │   ├── Cargo.toml
│   │   └── src/lib.rs                    # generated
│   └── axiom-l0/
│       ├── Cargo.toml                    # adds dep on axiom-l0-zip-verified
│       ├── src/
│       │   ├── lib.rs                    # switches to verified by default
│       │   └── legacy.rs                 # hand-written fallback (feature-flagged)
├── bench/
│   └── verified-vs-handwritten.rs        # NEW — Criterion comparison
├── tests/
│   └── translation-validation/
│       └── full-zip.rs                   # NEW — Bench-10K e2e
└── docs/
    └── verified-l0.md                    # NEW
```

## 8. Standalone Output

```bash
nix develop
make extract-zip                  # full ZIP layer Lean → Rust
buck2 test //tests/translation-validation:full-zip
# "10000/10000 APKs verified-Rust ↔ Lean reference agreed"
buck2 run //bench:verified-vs-handwritten
# Reports per-APK perf delta; aggregate ≤15%
```

## 9. End-to-End Test

```bash
buck2 test //axiom-l0:integration-bench-10k
# Runs Bench-10K through verified axiom-l0:
#   - sustained throughput ≥250 APKs/sec/16-core (HARD)
#   - p99 latency ≤80 ms (HARD)
#   - verified vs hand-written perf delta ≤15% (HARD)
#   - reproducibility: 100% bit-identical commit chains across 2 runs
```

## 10. Exit Checklist

- [ ] Full ZIP layer extracted (LFH + CDR + EOCD + Consistency)
- [ ] `axiom-l0` defaults to verified path
- [ ] Translation validator green on Bench-10K (HARD)
- [ ] Verified perf within 15% of hand-written (HARD)
- [ ] Throughput ≥ 250 APKs/sec on 16-core (HARD per PHASE_GATES.md §5)
- [ ] L0 p99 ≤ 80 ms (HARD)
- [ ] Bench-1K commit-chain reproducibility 100%
- [ ] `docs/verified-l0.md` published
- [ ] Hand-written fallback flagged for removal in Phase 2

## 11. Hand-Off

| Consumed by | What they need |
|---|---|
| **P1.15** | Verified `axiom-l0` produces commit-chain inputs for IR emission |
| **P1.16** | Extraction pattern reused for signing block |
| **P1.17** | Soundness regression suite includes verified ZIP |
| **P1.18** | Verified `axiom-l0` is the L0 of the E2E pipeline measured |
