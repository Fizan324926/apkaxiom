# P6.3 — G2 Stabilization: Perf-Tune, Memory Budgets, No New Features

> Drive apk-info v2.0 toward production: fixed memory budgets per layer, per-PR perf gate green for 90 days, no new features unless safety-critical. Migration guide for downstream apk-info v0.x users.

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P6.3 |
| Owner(s) | G2 |
| Duration | Weeks 1–14 |
| Critical-path | yes |
| Hard prerequisites | P6.1 |

## 2. Goal & Scope

apk-info v2.0 hardened to v1.0-quality: per-layer memory budgets enforced as CI gates, every per-PR perf check within 5 % of baseline, no new features.

### In scope
- Per-layer memory budgets enforced via CI gate
- Sustained-load perf tuning to reach 50K eval throughput
- AOSP archaeology continued (track A8–A15)
- apk-info v0.x → v2.0 migration guide
- API stabilization: every public function tagged stable / deprecated
- Reproducibility audit (re-confirm 100 %)

### Out of scope
- A16+ formalization (deferred to v1.1)
- New parser features

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P6.1** | Stabilization punch-list |
| **All Phase 1–5 G2 deliverables** | continued |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **All Phase 1 G2 stack** | (existing, pinned) | apk-info v2.0 |
| **Pyroscope / `cargo flamegraph` / `perf`** | (existing) | Perf tuning |
| **mimalloc / jemalloc** | (existing) | Memory tuning |

## 5. Third-Party Software, Services, Accounts & API Keys

All free OSS.

**No new API keys.**

## 6. System Inventory — Have vs Need

All present.

## 7. Features & Functions Delivered (Comprehensive)

### Per-layer memory budget CI
- L0 RSS ≤ 80 MB peak (HARD)
- L1 RSS ≤ 150 MB peak (HARD)
- L2 RSS ≤ 200 MB peak
- Combined L0–L3 ≤ 300 MB peak
- Per-PR memory gate: within 10 % of baseline

### Perf tuning
- SIMD on hot paths
- Per-thread arena allocator
- Memory pool for SSA value allocation (carries over from Phase 5)
- Reduce per-APK alloc count: ≤ 80K HARD

### apk-info v0.x → v2.0 migration guide
- `docs/apk-info-migration.md`
- Per-API mapping
- Behavior diffs
- Codemod scripts (`tools/apk-info-codemod`)

### API stabilization
- Public API surface auditing
- `#[stable]` / `#[deprecated]` attributes
- Deprecation warnings on v0.x compat shims
- Compat shims removed in v1.0

### AOSP archaeology
- Continued upstream commit tracking
- Re-formalize relevant changes (no new theorems unless safety-critical)
- Quarterly summary report

### Reproducibility re-confirm
- Bytewise-identical parser output 100 % (re-confirmed)

### Documentation
- `docs/apk-info-v2.0-stabilization.md`

## 8. KPIs (this sub-phase)

| KPI | HARD |
|---|---|
| L0 RSS ≤ 80 MB | yes |
| L1 RSS ≤ 150 MB | yes |
| Per-PR perf within 5 % baseline | yes (continuous) |
| Per-PR memory within 10 % baseline | yes (continuous) |
| Per-APK alloc count ≤ 80K | yes |
| 50K-eval throughput projection sustained | yes |
| apk-info migration guide published | yes |
| Public API stabilized (`#[stable]` everywhere) | yes |
| Deprecation shims removed | yes |
| Reproducibility 100 % | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── apk-info/                     # API stabilized
├── docs/
│   ├── apk-info-migration.md         # NEW
│   └── apk-info-v2.0-stabilization.md # NEW
└── tools/
    └── apk-info-codemod              # NEW
```

## 10. Standalone Output

apk-info v2.0 is releasable independently of APKAXIOM as a stable parser library.

## 11. End-to-End Test

```bash
buck2 test //crates/apk-info:...
buck2 run //tools:perf-bench -- --corpus bench-10k --budgets enforce
# Expect: all memory budgets ≤ HARD

# Migration codemod
buck2 run //tools:apk-info-codemod -- --check sample-v0-project/
```

## 12. Exit Checklist

- [ ] L0 + L1 memory budgets HARD ✅
- [ ] Per-PR perf within 5 % continuous (HARD)
- [ ] Per-PR memory within 10 % continuous (HARD)
- [ ] apk-info migration guide published
- [ ] Public API `#[stable]` audit complete
- [ ] Deprecation shims removed
- [ ] Reproducibility 100 % re-confirmed
- [ ] AOSP archaeology summary published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P6.16** | Stable apk-info for 50K eval |
| **P6.17** | Migration guide for auditor reading |
| **P6.19** | Stable API for production verifier integration |
