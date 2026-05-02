# P6.12 — G11 Stabilization: ML Model Corpus Expansion + Production Scanning

> Expand TFLite model corpus + planted-backdoor zoo. Drive backdoor detection FP rate < 5 % on benign-only model corpus. Production-scanning pipeline ready.

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P6.12 |
| Owner(s) | G11 |
| Duration | Weeks 1–14 |
| Critical-path | yes |
| Hard prerequisites | P6.1 |

## 2. Goal & Scope

ML scanning hardened: more reference models, more backdoor patterns, FP rate driven below 5 %, production-scanning pipeline ready for the public verifier service.

### In scope
- TFLite reference corpus: 100 → 500 models (TF-Hub + Kaggle + MLPerf + NLP/CV/audio mix)
- Planted-backdoor zoo: 30 → 50 patterns × 5 carriers = 250 samples
- FP rate < 5 % HARD on benign-only model corpus
- Production-scanning pipeline (chunked + GPU-pool)
- ONNX bonus support (best-effort; flag, not gate)

### Out of scope
- New scanners (Neural Cleanse + STRIP + adv-robust are the v1.0 set)
- ONNX as HARD requirement (v1.1)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P6.1** | Stabilization punch-list |
| **All Phase 5 G11 deliverables** | Continued |

## 4. Required Tools, Libraries, and Languages

Same as Phase 5.

## 5. Third-Party Software, Services, Accounts & API Keys

All free / existing.

**No new API keys.**

## 6. System Inventory — Have vs Need

All present.

## 7. Features & Functions Delivered (Comprehensive)

### TFLite reference corpus expansion
- 500 models across CV / NLP / audio / multimodal
- Per-model fixture + license tracking
- Cross-domain fairness sample for bias-detection bonus

### Planted-backdoor zoo expansion
- 50 patterns × 5 carriers = 250 samples
- New patterns: frequency-domain triggers, semantic triggers, sample-specific triggers, label-flip triggers

### FP rate driver
- Threshold tuning per scanner on benign-only model corpus
- Ensemble policy refined: ✓ if NC + STRIP both clean and adv-robust ≥ 0.6
- Combined FP rate < 5 % HARD

### Production scanning pipeline
- Chunked scanning for large models
- GPU-pool scheduling (sppark-style queue)
- Per-model SLA: ≤ 60 s typical, ≤ 300 s worst case
- Reproducibility: deterministic verdicts (pinned seed + cudnn-deterministic)

### Documentation
- `docs/g11-stabilization.md`

## 8. KPIs (this sub-phase)

| KPI | HARD |
|---|---|
| TFLite reference corpus | ≥ 500 models |
| Planted-backdoor zoo | ≥ 50 × 5 = 250 samples |
| Backdoor FP rate on benign | < 5 % |
| Backdoor precision (re-confirm) | ≥ 90 % |
| Backdoor recall (re-confirm) | ≥ 80 % |
| Production-scan SLA: typical model | ≤ 60 s |
| Production-scan SLA: worst case | ≤ 300 s |
| Reproducibility | 100 % |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── corpora/
│   ├── tflite-500/                   # NEW: expanded
│   └── planted-backdoor-zoo-250/     # NEW
├── crates/
│   └── axiom-ml-scan/                # production pipeline
└── docs/
    └── g11-stabilization.md          # NEW
```

## 10. Standalone Output

500-model corpus + 250-backdoor zoo released as Phase-6 dataset (CC-BY-4.0).

## 11. End-to-End Test

```bash
buck2 run //tools:axiom-ml-bench -- --corpus tflite-500 --report fp-rate
# Expect: < 5 %

buck2 run //tools:axiom-ml-bench -- --corpus planted-backdoor-zoo-250 --report precision-recall
# Expect: ≥ 90 % / ≥ 80 %
```

## 12. Exit Checklist

- [ ] TFLite corpus ≥ 500 (HARD)
- [ ] Backdoor zoo ≥ 250 (HARD)
- [ ] Combined FP rate < 5 % (HARD)
- [ ] Precision ≥ 90 % (HARD)
- [ ] Recall ≥ 80 % (HARD)
- [ ] Production-scan SLA met
- [ ] Reproducibility 100 %
- [ ] Documentation `docs/g11-stabilization.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P6.16** | Stabilized scanner for 50K eval |
| **P6.17** | Scanner explained to auditor |
| **P6.19** | Production scanner deployed |
| **P6.20** | "TFLite scanner FP rate < 5 %" item ✅ for ship gate |
