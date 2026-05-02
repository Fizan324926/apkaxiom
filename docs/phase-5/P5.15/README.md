# P5.15 — Neural Cleanse Backdoor Scan

> Wang et al.'s Neural Cleanse: reverse-engineer trigger patterns by searching the input space per output class. Detect backdoors planted in TFLite models. Precision ≥ 90 % on planted-backdoor zoo.

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P5.15 |
| Owner(s) | G11 |
| Duration | Weeks 6–14 |
| Critical-path | yes |
| Hard prerequisites | P5.14 |

## 2. Goal & Scope

A Neural Cleanse implementation specialized to TFLite + Android constraints (mobile-class models, often quantized): per output class, optimize a minimal trigger that flips classification → if any class needs an anomalously small trigger, flag.

### In scope
- Neural Cleanse pipeline: per-class trigger optimization + MAD anomaly detection
- TFLite quantized-model handling (int8 inputs / outputs)
- GPU acceleration via CUDA / CUDA-OpenCL fallback
- Per-model scan ≤ 120 s HARD (≤ 30 s TARGET)
- Backdoor detection precision ≥ 90 % HARD (≥ 98 % TARGET)
- Backdoor detection recall ≥ 80 % HARD (≥ 95 % TARGET)
- Planted-backdoor zoo: ≥ 30 backdoor patterns × 5 carrier models = 150-sample test corpus
- Cert evidence shape: scan-result digest + per-class anomaly score

### Out of scope
- STRIP scan (P5.16)
- Adversarial robustness (P5.17)
- Custom backdoor-attack engineering — borrowed from BadNets / TrojanNN literature

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P5.14** | TFLite parse + canonical hash |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **TFLite C/C++ runtime** | (existing) | Inference |
| **PyTorch** (eval only) | latest | Reference impl |
| **CUDA + cuDNN** | (existing) | GPU |
| **OpenCL** | latest | Fallback |
| **Rust** | 1.84+ | Implementation |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **Neural Cleanse reference impl** | code | **Free** OSS | https://github.com/bolunwang/backdoor | Wang et al. |
| **TrojanZoo** | corpus | **Free** OSS | https://github.com/ain-soph/trojanzoo | Backdoor benchmark |
| **BadNets** | reference | **Free** | https://arxiv.org/abs/1708.06733 | Reference attack |
| **TrojanNN** | reference | **Free** | https://github.com/PurduePAML/TrojanNN | Reference attack |

**No new API keys.**

## 6. System Inventory — Have vs Need

All present from P5.1.

## 7. Features & Functions Delivered (Comprehensive)

### Crate `axiom-neural-cleanse`
- Trigger optimization with Adam + clipped-gradient (TFLite-quantized-aware)
- Per-class loop, parallelized on GPU
- MAD (Median Absolute Deviation) anomaly detection on trigger size
- Anomaly threshold tunable (default 2× MAD over median)
- Streaming GPU memory (large-class-count models)
- Reproducible RNG seed pinned per scan

### Planted-backdoor zoo
- 30 backdoor patterns (square pixel, watermark, single-pixel, semantic, blended, sinusoidal, frequency-domain, etc.)
- 5 carrier models (MobileNetV2, EfficientNet-Lite, MNASNet, ResNet-Lite, custom-Android benchmark)
- Tests cover both clean and poisoned versions
- Fixtures: every test result is reproducible

### Tools
- `axiom-neural-cleanse-cli`
- `axiom-neural-cleanse-bench`

### Cert evidence
- Per-model scan result: per-class anomaly score, flagged classes, trigger digest, RNG seed, GPU device id
- Round-trippable: same model + seed → same result
- Embedded in `.axc` cert as ML claim subtype

### Performance
- Per-model scan ≤ 120 s HARD (≤ 30 s TARGET) on H100/L40S
- Multi-GPU parallel scan supported

### Quality
- Precision ≥ 90 % HARD on planted-backdoor zoo (≥ 98 % TARGET)
- Recall ≥ 80 % HARD (≥ 95 % TARGET)

### Reproducibility
- Pinned RNG seed
- Deterministic GPU kernel choice (cudnn-deterministic)
- Bytewise-identical scan output across runs

### Documentation
- `docs/neural-cleanse.md`

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Per-model scan time (typical) | ≤ 120 s | ≤ 30 s |
| Precision on planted-backdoor zoo | ≥ 90 % | ≥ 98 % |
| Recall on planted-backdoor zoo | ≥ 80 % | ≥ 95 % |
| Reproducibility (pinned RNG) | 100 % | 100 % |
| Cert evidence verifiability | 100 % | 100 % |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-neural-cleanse/        # NEW
├── tools/
│   ├── axiom-neural-cleanse-cli
│   └── axiom-neural-cleanse-bench
├── corpora/
│   └── planted-backdoor-zoo/        # NEW: 30 × 5 = 150 samples
└── docs/
    └── neural-cleanse.md            # NEW
```

## 10. Standalone Output

Reusable Neural Cleanse implementation (open-source, AGPL+commercial).

## 11. End-to-End Test

```bash
buck2 test //crates/axiom-neural-cleanse:...
buck2 run //tools:axiom-neural-cleanse-bench -- --corpus planted-backdoor-zoo
# Expect: precision ≥ 90 %, recall ≥ 80 %, ≤ 120 s / model
```

## 12. Exit Checklist

- [ ] Per-model scan ≤ 120 s (HARD)
- [ ] Precision ≥ 90 % (HARD)
- [ ] Recall ≥ 80 % (HARD)
- [ ] Reproducibility 100 %
- [ ] Cert evidence verifiability 100 %
- [ ] Multi-GPU parallel scan operational
- [ ] Documentation `docs/neural-cleanse.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P5.16** | Ensemble pairing with STRIP |
| **P5.18** | Neural Cleanse in E2E pipeline |
| **L6 cert** | ML scan-result subtype |
| **P5.19** | Results in paper |
