# P5.16 — STRIP Backdoor Scan

> Strong Intentional Perturbation: detect backdoors by predicting on perturbed-input ensembles. Low entropy under heavy perturbation indicates a backdoor.

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P5.16 |
| Owner(s) | G11 |
| Duration | Weeks 8–14 |
| Critical-path | yes |
| Hard prerequisites | P5.14 |

## 2. Goal & Scope

A STRIP scanner complementing Neural Cleanse, used as a second opinion in the ensemble. STRIP is faster than Neural Cleanse and detects different attack families (semantic / sample-specific triggers).

### In scope
- STRIP pipeline: input perturbation by linear-blend with N reference samples → predict each → measure entropy
- Quantized-model handling
- GPU acceleration
- Per-model scan ≤ 60 s HARD (≤ 15 s TARGET)
- Backdoor detection precision ≥ 85 % HARD (≥ 95 % TARGET)
- Backdoor detection recall ≥ 80 % HARD (≥ 95 % TARGET)
- Same planted-backdoor zoo from P5.15
- Cert evidence shape: scan-result digest + per-input entropy distribution

### Out of scope
- Neural Cleanse (P5.15)
- Adversarial robustness (P5.17)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P5.14** | TFLite parse |
| **P5.15** | Planted-backdoor zoo (shared) |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **TFLite runtime** | (existing) | Inference |
| **CUDA** | (existing) | GPU |
| **Rust** | 1.84+ | Implementation |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **STRIP reference impl** | code | **Free** OSS | https://github.com/garrisongys/STRIP | Gao et al. |
| **TrojanZoo** | corpus | (existing P5.15) | | |

**No new API keys.**

## 6. System Inventory — Have vs Need

All present.

## 7. Features & Functions Delivered (Comprehensive)

### Crate `axiom-strip`
- Perturbation-blend pipeline (N=100 default, configurable)
- Per-input entropy compute
- Anomaly detection on entropy distribution
- Quantized-model handling
- GPU-accelerated
- Reproducible RNG seed

### Tests
- Same planted-backdoor zoo from P5.15 — used for cross-precision/recall measurement
- Disagreements with Neural Cleanse cataloged (input to ensemble policy)

### Tools
- `axiom-strip-cli`
- `axiom-strip-bench`

### Ensemble policy
- Returns ✓ if both NC and STRIP agree clean
- Returns ✗ if either flags
- Returns UNKNOWN-with-evidence if NC + STRIP disagree, plus rationale

### Cert evidence
- Per-model scan: entropy distribution digest, perturbation seed, GPU device, NC ↔ STRIP agreement bit
- Embedded in `.axc` ML scan subtype

### Performance
- Per-model scan ≤ 60 s HARD (≤ 15 s TARGET)

### Quality
- Precision ≥ 85 % HARD (≥ 95 % TARGET)
- Recall ≥ 80 % HARD (≥ 95 % TARGET)

### Reproducibility
- 100 % across runs

### Documentation
- `docs/strip.md`

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Per-model scan time | ≤ 60 s | ≤ 15 s |
| Precision | ≥ 85 % | ≥ 95 % |
| Recall | ≥ 80 % | ≥ 95 % |
| Ensemble agreement (NC + STRIP) on benign zoo | ≥ 98 % | ≥ 99.5 % |
| Reproducibility | 100 % | 100 % |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-strip/                 # NEW
├── tools/
│   ├── axiom-strip-cli
│   └── axiom-strip-bench
└── docs/
    └── strip.md                     # NEW
```

## 10. Standalone Output

Reusable STRIP implementation.

## 11. End-to-End Test

```bash
buck2 test //crates/axiom-strip:...
buck2 run //tools:axiom-strip-bench -- --corpus planted-backdoor-zoo
# Expect: precision ≥ 85 %, recall ≥ 80 %, ≤ 60 s / model
```

## 12. Exit Checklist

- [ ] Per-model scan ≤ 60 s (HARD)
- [ ] Precision ≥ 85 % (HARD)
- [ ] Recall ≥ 80 % (HARD)
- [ ] Ensemble policy operational
- [ ] Reproducibility 100 %
- [ ] Cert evidence verifiability 100 %
- [ ] Documentation `docs/strip.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P5.18** | STRIP in E2E pipeline |
| **L6 cert** | ML scan-result subtype |
| **P5.19** | Results in paper |
