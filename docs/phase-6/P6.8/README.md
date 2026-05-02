# P6.8 — G7 Stabilization: Circuit Gas Optimization + Cert Size Reduction

> Drive Halo2 / Plonky3 / Binius / Stwo circuit gas down. Cert size median ≤ 50 KB, p99 ≤ 200 KB. Verifier-side cost reduction. No new circuits.

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P6.8 |
| Owner(s) | G7 |
| Duration | Weeks 1–14 |
| Critical-path | yes |
| Hard prerequisites | P6.1 |

## 2. Goal & Scope

Existing circuits hardened: gas optimized, proof sizes shrunk, verify times shrunk. Circuit-of-day for the 5 priority privacy invariants pinned to the optimal scheme per workload.

### In scope
- Halo2 circuit gas optimization (constraint-count reduction)
- Plonky3 alternate compilation for hash-heavy invariants
- Binius for binary-field-friendly invariants
- Stwo for post-quantum / regulated workloads
- Per-invariant scheme selection table
- GPU-acceleration tuning (sppark / icicle kernel parameters)
- Cert size budgets enforced by CI
- Verifier-side cost reduction: precomputed verifying-key tables, batch verification

### Out of scope
- New circuits (deferred to v1.1)
- New schemes

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P6.1** | Stabilization punch-list |
| **All Phase 4 G7 deliverables** | Continued |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Halo2 / Plonky3 / Binius / Stwo** | (existing, pinned) | |
| **sppark / icicle** | (existing) | GPU |

## 5. Third-Party Software, Services, Accounts & API Keys

All free OSS.

**No new API keys.**

## 6. System Inventory — Have vs Need

All present.

## 7. Features & Functions Delivered (Comprehensive)

### Halo2 circuit gas optimization
- Per-circuit constraint-count tracked
- Custom gates introduced where constraint reduction ≥ 30 %
- Range-check optimizations
- Reuse of common gates across the 5 invariants

### Plonky3 / Binius / Stwo benchmarks
- Per-invariant head-to-head: prove time, verify time, proof size
- Per-invariant scheme selection table (`tables/circuit-of-day.toml`)

### GPU tuning
- sppark NTT parameters tuned per circuit shape
- icicle MSM batch size tuned
- 10× CPU baseline maintained, 30× achieved on H100/L40S target

### Cert size reduction
- L6 cert format compression: domain-specific Brotli dictionary trained over 10K certs
- Witness deflation: drop redundant abstraction-domain certs
- Median ≤ 50 KB, p99 ≤ 200 KB

### Verifier-side cost reduction
- Precomputed verifying-key tables baked into `axiom-verify` binary
- Batch verification: amortize ECC ops across N certs
- p99 verifier ≤ 100 ms maintained, p50 ≤ 15 ms target

### Reproducibility
- Per-circuit prove output bytewise-deterministic given same witness + same RNG seed

### Documentation
- `docs/g7-stabilization.md`
- `docs/circuit-of-day.md` (per-invariant scheme selection)

## 8. KPIs (this sub-phase)

| KPI | HARD |
|---|---|
| Cert size median | ≤ 50 KB |
| Cert size p99 | ≤ 200 KB |
| Halo2 prove p99 (per circuit) | ≤ 1.5 s |
| Halo2 verify p99 (per proof) | ≤ 5 ms |
| GPU vs CPU speedup | ≥ 30× |
| Per-invariant scheme selection table published | yes |
| Verifier batch verification deployed | yes |
| Reproducibility (proof bytewise-identical given witness + seed) | 100 % |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── circuits/
│   └── halo2/                        # tuned
├── tables/
│   └── circuit-of-day.toml           # NEW
├── crates/
│   └── axiom-verify/                 # batch verify
└── docs/
    ├── g7-stabilization.md           # NEW
    └── circuit-of-day.md             # NEW
```

## 10. Standalone Output

Optimized circuits + scheme-selection table citable in paper.

## 11. End-to-End Test

```bash
buck2 run //tools:axiom-circuit-bench -- --schemes halo2,plonky3,binius,stwo
# Expect: per-invariant table populated

buck2 run //tools:axiom-verify-bench -- --corpus 10k-certs --batch
# Expect: p99 ≤ 100 ms, batch speedup measurable
```

## 12. Exit Checklist

- [ ] Cert size median ≤ 50 KB (HARD)
- [ ] Cert size p99 ≤ 200 KB (HARD)
- [ ] Halo2 prove p99 ≤ 1.5 s (HARD)
- [ ] Halo2 verify p99 ≤ 5 ms (HARD)
- [ ] GPU 30× speedup
- [ ] Scheme-of-day table published
- [ ] Batch verification deployed
- [ ] Documentation `docs/g7-stabilization.md` + `docs/circuit-of-day.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P6.16** | Optimized circuits used in 50K eval |
| **P6.17** | Crypto path explained to auditor |
| **P6.19** | Optimized verifier deployed to production |
| **P6.20** | "Halo2 circuits ship for 5 priority invariants" item ✅ for ship gate |
