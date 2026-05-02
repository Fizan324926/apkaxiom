# P3.17 — Layer 5 Integration: BSH + Bisim + LSH Unified

> Layer 5 in production. BSH-256 + bisimulation + DiskANN LSH unified behind a single API. Coarse-to-fine pipeline: BSH filter → LSH neighborhood → bisim discharge.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §10](../../../README.md#layer-5)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P3.17 |
| Owner(s) | G6 |
| Duration | Weeks 15–18 |
| Critical-path | yes |
| Hard prerequisites | P3.14 (BSH+LSH), P3.15 (bisim engine), P3.16 (cert format) |

## 2. Goal & Scope

The unified Layer 5 API. Coarse-to-fine pipeline:
1. BSH-256 fingerprint (sub-millisecond)
2. LSH/DiskANN nearest-neighbor lookup (sub-second)
3. Bounded bisimulation on candidate pairs (per-pair seconds)

The downstream consumer (Phase 4 G7 cert emitter) gets a single API. The implementation chooses between fast hash-based equivalence and slow proof-based equivalence based on the use case.

### In scope
- `crates/axiom-l5` — unified Layer-5 façade
- `pub fn equivalence(a, b) -> EquivalenceOutcome` — full proof-based answer
- `pub fn fast_similarity(query, k) -> Vec<NeighborWithScore>` — fast LSH retrieval
- `pub fn equiv_with_witness(query, candidates) -> Vec<EquivalenceCert>` — bisim on top-k
- Per-deployment configuration (BSH-only mode, BSH+LSH mode, full bisim mode)
- E2E performance tuning

### Out of scope
- Phase 6 production-grade SLA tuning (Phase 6)
- Deeper analytical surfaces (Phase 4)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P3.14** | BSH-256 + DiskANN |
| **P3.15** | Bisim engine |
| **P3.16** | Cert format |

## 4. Required Tools, Libraries, and Languages

Same as P3.14 + P3.15 + P3.16. No new tools.

## 5. Third-Party Software, Services, Accounts & API Keys

**No new third-party.**

## 6. System Inventory — Have vs Need

Nothing new system-level.

## 7. Features & Functions Delivered (Comprehensive)

### Public Rust API (the unified surface)
- `pub fn equivalence(a: &BehaviorSet, b: &BehaviorSet, mode: Mode) -> EquivalenceOutcome`
- `pub fn fast_similarity(query: &BehaviorSet, k: usize) -> Vec<(ApkId, Distance)>`
- `pub fn equiv_with_witness(query: &BehaviorSet, candidates: &[ApkId]) -> Vec<EquivalenceCert>`
- `pub enum Mode { BshOnly, BshPlusLsh, FullBisim, Adaptive }`
- `pub enum EquivalenceOutcome { ProvenEquivalent(EquivalenceCert), Divergent(DivergencePoint), HighSimilarity(BshDistance), Unknown(Reason) }`

### Coarse-to-fine pipeline
1. **BSH-256** computed first (1–10 ms)
2. **DiskANN/LSH** lookup (10–100 ms) for candidates
3. **Bisimulation** on top-k candidates (1–2 s per pair)
4. **Equivalence cert** emitted on success

### Adaptive mode
- Tunes the pipeline per-query based on workload signals
- For "is this exactly known malware?" — BshOnly is sufficient
- For "is this similar to but provably equivalent to known malware?" — FullBisim
- For batch fleet analysis — BshPlusLsh

### Caching layer
- `(BSH-A, BSH-B) → cached_equiv_outcome` LRU cache
- Saves bisim re-runs on repeated queries

### Performance target (combined)
- Median single-pair equiv (FullBisim): ≤ 1 s
- Median batch fleet similarity (BshPlusLsh): ≤ 100 ms per query
- 1M-vector neighborhood lookup: ≤ 200 ms p99

### Documentation
- `docs/l5-unified.md`

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| L5 equivalence (FullBisim) p99 single-pair | ≤ 2 s | ≤ 500 ms |
| L5 fast_similarity (BshPlusLsh) p99 1M-index | ≤ 200 ms | ≤ 50 ms |
| L5 sustained throughput (mixed mode) | ≥ 100 ops/sec/16-core | ≥ 500 ops/sec |
| Cache hit rate on repeated queries | ≥ 50 % | ≥ 90 % |
| E2E equiv cert size median | ≤ 200 KB | ≤ 80 KB |
| 0 cert verification failures | yes | yes |
| Adaptive-mode router accuracy | ≥ 90 % correct mode selection | ≥ 99 % |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-l5/
│       ├── Cargo.toml
│       ├── BUCK
│       └── src/
│           ├── lib.rs                    # façade
│           ├── coarse_to_fine.rs
│           ├── adaptive.rs
│           └── cache.rs
└── docs/
    └── l5-unified.md                     # NEW
```

## 10. Standalone Output

```bash
buck2 build //crates/axiom-l5 --release
buck2 run //bench:l5-end-to-end -- --corpus repack-2k
# "Equiv p99: 1.4s; LSH p99: 95ms; Cache hit rate: 67%"
```

## 11. End-to-End Test

```bash
buck2 test //crates/axiom-l5:full-eval
# - L5 equiv p99 ≤ 2 s (HARD)
# - L5 LSH p99 ≤ 200 ms on 1M index (HARD)
# - Throughput ≥ 100 ops/sec/16-core (HARD)
# - 0 cert verification failures (HARD)
```

## 12. Exit Checklist

- [ ] Unified Layer-5 façade lands
- [ ] All 4 modes (BshOnly / BshPlusLsh / FullBisim / Adaptive) operational
- [ ] L5 equiv p99 ≤ 2 s (HARD)
- [ ] L5 LSH p99 ≤ 200 ms (HARD)
- [ ] Throughput ≥ 100 ops/sec/16-core (HARD)
- [ ] 0 cert verification failures (HARD)
- [ ] Adaptive-mode router ≥ 90 % accurate (HARD)
- [ ] `docs/l5-unified.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P3.18** | L5 unified surface measured in E2E |
| **Phase 4 / G7** | Single L5 API for cert emitter to consume |
| **External SDKs (Phase 4 / G14)** | `axiom-l5` is what `axiom-py` etc. wrap |
