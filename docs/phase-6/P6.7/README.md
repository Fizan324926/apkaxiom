# P6.7 — G6 Stabilization: Bisim k-Bound Tuning Per Workload

> Tune bisimulation k-step bound + abstract-domain composition per workload class. Drive bisim TP ≥ 95 %, FP < 0.1 % on Repack-2K + 1000 known repackaging pairs in 50K subset.

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P6.7 |
| Owner(s) | G6 |
| Duration | Weeks 1–14 |
| Critical-path | yes |
| Hard prerequisites | P6.1 |

## 2. Goal & Scope

Workload-aware bisim tuning so v1.0 ships with high precision + recall on equivalence claims. Witness production made deterministic + size-bounded.

### In scope
- k-step bound tuning per workload class (small UI app / large fintech / NDK-heavy / messenger / game)
- Abstract-domain composition tuning per class
- Witness size budget: ≤ 50 KB median, ≤ 200 KB p99
- BSH-256 stability re-measured on 50K subset across ProGuard / R8 / DexGuard
- LSH index tuning for 1M+ scale (DiskANN parameter tuning)

### Out of scope
- New abstract domains
- New equivalence semantics

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P6.1** | Stabilization punch-list |
| **All Phase 3 G6 deliverables** | Continued |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **All Phase 3 G6 stack** | (existing) | Continued |

## 5. Third-Party Software, Services, Accounts & API Keys

All free OSS.

**No new API keys.**

## 6. System Inventory — Have vs Need

All present.

## 7. Features & Functions Delivered (Comprehensive)

### Per-workload k-step tuning
- Workload classifier: small UI / large fintech / NDK-heavy / messenger / game
- Per-class k-bound table
- Per-class abstract-domain composition

### Witness size budget
- Median ≤ 50 KB, p99 ≤ 200 KB
- Witness deflation rules (drop redundant abstraction-domain certificates)

### BSH-256 stability re-measure
- Across ProGuard / R8 / DexGuard / Bangcle / Tencent on 50K-subset repacks
- Stability ≥ 95 % HARD

### LSH index tuning
- DiskANN: M, L, alpha tuned for 1M-scale
- Index size ≤ 8 GB, query p99 ≤ 200 ms

### Tools
- `axiom-l5-bench` updated for 50K
- Workload-classifier reproducibility test

### Documentation
- `docs/g6-stabilization.md`

## 8. KPIs (this sub-phase)

| KPI | HARD |
|---|---|
| Bisim TP on Repack-2K | ≥ 95 % |
| Bisim FP on benign pairs | < 0.1 % |
| BSH stability across ProGuard / R8 / DexGuard | ≥ 95 % |
| Witness size median | ≤ 50 KB |
| Witness size p99 | ≤ 200 KB |
| LSH index size for 1M | ≤ 8 GB |
| LSH lookup p99 | ≤ 200 ms |
| Workload classifier reproducible | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-l5-*/                   # tuned
├── tables/
│   └── bisim-k-per-workload/         # NEW
└── docs/
    └── g6-stabilization.md           # NEW
```

## 10. Standalone Output

Tuned bisim engine citable in Phase-6 paper.

## 11. End-to-End Test

```bash
buck2 run //tools:axiom-l5-bench -- --corpus repack-2k --report tp-fp
# Expect: TP ≥ 95 %, FP < 0.1 %

buck2 run //tools:axiom-l5-bench -- --corpus eval-50k-repack-subset --report bsh-stability
# Expect: ≥ 95 %
```

## 12. Exit Checklist

- [ ] Bisim TP ≥ 95 % (HARD)
- [ ] Bisim FP < 0.1 % (HARD)
- [ ] BSH stability ≥ 95 % (HARD)
- [ ] Witness size median ≤ 50 KB
- [ ] LSH index ≤ 8 GB
- [ ] LSH p99 ≤ 200 ms
- [ ] Workload classifier reproducible
- [ ] Documentation `docs/g6-stabilization.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P6.16** | Tuned bisim for 50K eval |
| **P6.17** | Equivalence claims explained to auditor |
| **P6.20** | "Bisim engine produces witnesses for repackaging corpus" item ✅ for ship gate |
