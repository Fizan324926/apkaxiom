# P5.17 — Adversarial Robustness Scoring

> Run a battery of adversarial-attack benchmarks (FGSM, PGD, C&W, DeepFool, AutoAttack) against TFLite models. Produce a single robustness score + per-attack detail. Lower bound on the model's confidence under perturbation.

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P5.17 |
| Owner(s) | G11 |
| Duration | Weeks 10–16 |
| Critical-path | yes |
| Hard prerequisites | P5.14 |

## 2. Goal & Scope

A robustness score for TFLite models surfacing models with brittle behavior under standard adversarial attacks. Independent value beyond backdoor detection — captures supply-chain quality.

### In scope
- Attack battery: FGSM, PGD, C&W (L2 + L∞), DeepFool, AutoAttack (auto-PGD-ce + auto-PGD-dlr)
- TFLite quantized + float models
- Per-model scan ≤ 300 s HARD (≤ 60 s TARGET)
- Reproducible attack RNG
- Score in [0, 1] interpretable: 1.0 = robust at all ε levels tested, 0 = trivial to break
- Cert evidence: per-attack success rate + score + RNG seed

### Out of scope
- Backdoor scans (P5.15 / P5.16)
- Production-grade defenses (research-grade only)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P5.14** | TFLite parse |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Adversarial Robustness Toolbox (IBM ART)** | latest | Reference attacks |
| **AutoAttack** | latest | Reference auto-PGD |
| **PyTorch** (eval only) | latest | Reference impl |
| **TFLite runtime** | (existing) | Target |
| **CUDA** | (existing) | GPU |
| **Rust** | 1.84+ | Implementation |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **IBM ART** | lib | **Free** OSS (MIT) | https://github.com/Trusted-AI/adversarial-robustness-toolbox | |
| **AutoAttack** | lib | **Free** OSS | https://github.com/fra31/auto-attack | Croce & Hein |

**No new API keys.**

## 6. System Inventory — Have vs Need

All present from P5.1.

## 7. Features & Functions Delivered (Comprehensive)

### Crate `axiom-adv-robust`
- Attack pipelines: FGSM, PGD (L2 + L∞), C&W (L2 + L∞), DeepFool, AutoAttack (APGD-ce + APGD-dlr)
- Per-attack success-rate curve over ε
- Aggregated score in [0, 1]
- TFLite-quantized handling: estimate gradients via finite differences when symbolic gradients unavailable
- GPU acceleration
- Reproducible RNG seed

### Tests
- Reference dataset (CIFAR-10, ImageNet-mini, Android-mobile-class custom test set)
- Robustness fixtures for known-robust + known-brittle baselines

### Tools
- `axiom-adv-robust-cli`
- `axiom-adv-robust-bench`

### Cert evidence
- Per-attack success rate + score + seed + GPU device
- Embedded in `.axc` ML scan subtype

### Performance
- Per-model scan ≤ 300 s HARD (≤ 60 s TARGET) on H100/L40S

### Reproducibility
- 100 % across runs (pinned seed + cudnn-deterministic)

### Documentation
- `docs/adv-robust.md`

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Per-model scan time | ≤ 300 s | ≤ 60 s |
| Reproducibility | 100 % | 100 % |
| Score sensitivity (robust vs brittle baseline gap) | ≥ 0.3 | ≥ 0.5 |
| Attack-implementation diff vs ART (success rate) | within 5 pp | within 1 pp |
| Cert evidence verifiability | 100 % | 100 % |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-adv-robust/            # NEW
├── tools/
│   ├── axiom-adv-robust-cli
│   └── axiom-adv-robust-bench
└── docs/
    └── adv-robust.md                # NEW
```

## 10. Standalone Output

Reusable adversarial-robustness scorer.

## 11. End-to-End Test

```bash
buck2 test //crates/axiom-adv-robust:...
buck2 run //tools:axiom-adv-robust-bench -- --corpus tflite-100
# Expect: ≤ 300 s / model, score-sensitivity gap ≥ 0.3
```

## 12. Exit Checklist

- [ ] Per-model scan ≤ 300 s (HARD)
- [ ] Reproducibility 100 %
- [ ] Score-sensitivity gap ≥ 0.3
- [ ] Diff vs ART within 5 pp
- [ ] Cert evidence verifiability 100 %
- [ ] Documentation `docs/adv-robust.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P5.18** | Robustness score in E2E pipeline |
| **L6 cert** | ML scan-result subtype |
| **P5.19** | Score distribution in paper |
