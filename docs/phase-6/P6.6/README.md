# P6.6 — G5 Stabilization: Solver Tuning, UNKNOWN Rate < 5 %

> Drive symbolic resolver UNKNOWN rate below 5 % on the 50K corpus via solver tuning + abstraction-domain refinement + deeper integration with dynamic confirmation. No new resolvers.

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P6.6 |
| Owner(s) | G5 |
| Duration | Weeks 1–16 |
| Critical-path | yes |
| Hard prerequisites | P6.1 |

## 2. Goal & Scope

UNKNOWN rate driven below 5 % on the benign 50K subset (HARD). Solver portfolio tuned. Abstraction-refinement loop tuned. Dynamic-confirmation bridge wired into UNKNOWN flow as default for high-priority claims.

### In scope
- Solver portfolio tuning: per-query-class solver selection (cvc5 / Bitwuzla / Yices2 / Spacer / Eldarica / Pono)
- Abstract-domain refinement: more aggressive widening + targeted unrolling
- Per-Android-version intent-filter table optimization
- Dynamic-bridge fallback wired in (UNKNOWN → dynamic refinement) where consent-gated
- Solver timeout pruning: 5 s prod / 60 s research / 300 s eval
- 50K-eval UNKNOWN dashboard

### Out of scope
- New resolvers
- New abstract domains (deferred to v1.1)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P6.1** | Stabilization punch-list |
| **All Phase 3 G5 deliverables** | Continued |
| **P5.13** | Dynamic-confirmation bridge (used as fallback) |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **cvc5 / Bitwuzla / Yices2 / Spacer / Eldarica / Pono** | (existing, pinned) | Continued |

## 5. Third-Party Software, Services, Accounts & API Keys

All free OSS.

**No new API keys.**

## 6. System Inventory — Have vs Need

All present.

## 7. Features & Functions Delivered (Comprehensive)

### Solver portfolio tuning
- Per-query-class learned selection (cvc5 default; Bitwuzla for QF_BV-heavy; Yices2 for linear arithmetic shortcut; Spacer for CHC; Eldarica fallback; Pono if Spacer fails to terminate)
- Empirical timing tables across 50K subset

### Abstraction-domain refinement
- More aggressive widening on numeric domain
- Targeted unrolling on string-domain
- Per-domain UNKNOWN classifier (which domain is the bottleneck)

### Per-Android-version intent-filter optimization
- Per-version intent-filter table compiled to fast resolver lookup
- A8 / A10 / A12 / A14 / A15 covered

### Dynamic-bridge fallback
- For claim severity ≥ HIGH, UNKNOWN → dynamic refinement
- Consent-gated: dynamic only runs when policy allows

### Timeouts
- 5 s production
- 60 s research / pilot
- 300 s eval-only

### 50K-eval UNKNOWN dashboard
- Per-version, per-domain, per-claim breakdown
- Guides the next iteration of tuning

### Documentation
- `docs/g5-stabilization.md`

## 8. KPIs (this sub-phase)

| KPI | HARD |
|---|---|
| L4 UNKNOWN rate on benign 50K | < 5 % |
| L4 UNSAT correctness on Malware-1K (re-confirm) | 100 % |
| Solver timeout rate (production) | < 1 % |
| Per-query p99 (production timeout) | ≤ 500 ms |
| Per-query p99 (research) | ≤ 5 s |
| Dynamic-bridge fallback wired in | yes |
| Per-version intent-filter table optimized | A8 / A10 / A12 / A14 / A15 |
| 50K-eval UNKNOWN dashboard live | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-l4/                     # tuned
├── tables/
│   └── intent-filter-per-version/    # NEW
├── dashboards/
│   └── unknown-50k.json
└── docs/
    └── g5-stabilization.md           # NEW
```

## 10. Standalone Output

Tuned solver portfolio + tables citable in Phase-6 paper.

## 11. End-to-End Test

```bash
buck2 run //tools:axiom-l4-bench -- --corpus eval-50k --benign --report unknown-rate
# Expect: < 5 %

buck2 run //tools:axiom-l4-bench -- --corpus malware-1k --report unsat-correctness
# Expect: 100 %
```

## 12. Exit Checklist

- [ ] UNKNOWN rate < 5 % on benign 50K (HARD)
- [ ] UNSAT correctness 100 % (HARD)
- [ ] Production timeout p99 ≤ 500 ms
- [ ] Solver-timeout rate < 1 %
- [ ] Dynamic-bridge fallback wired in
- [ ] Per-version intent-filter tables published
- [ ] 50K-eval UNKNOWN dashboard live
- [ ] Documentation `docs/g5-stabilization.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P6.16** | UNKNOWN rate evidence in 50K eval |
| **P6.17** | Solver portfolio explained to auditor |
| **P6.20** | "UNKNOWN rate < 5 %" item ✅ for ship gate |
