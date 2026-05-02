# P6.16 — APKAXIOM-Eval-50K Corpus Run + Dataset Release

> Run the full L0–L6 + native + dynamic + ML pipeline on the 50K APK corpus. Publish results + dataset under CC-BY-4.0. Major paper: *"The APKAXIOM Corpus: Proof-Stack Evaluation on 50K Android Packages."*

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P6.16 |
| Owner(s) | All groups |
| Duration | Weeks 12–22 |
| Critical-path | yes (gates v1.0 ship) |
| Hard prerequisites | P6.2 .. P6.15 |

## 2. Goal & Scope

The headline evaluation of v1.0: 50K APKs run end-to-end through the entire stack, results published as paper + open dataset.

### In scope
- Full eval run on 50K APKs through L0–L6 + native + ML
- Eval ≤ 72 h on 100-core cluster (HARD)
- Per-stage instrumentation + dashboards
- Per-stage success rates measured
- Dataset publication (CC-BY-4.0 + per-sample license tracking)
- Open-data paper drafted: *"The APKAXIOM Corpus: Proof-Stack Evaluation on 50K Android Packages"*
- Reproducibility Docker image
- Zenodo upload with DOI

### Out of scope
- New features
- Extension to 100K (deferred to v1.1)

## 3. Hard Dependencies on Prior Sub-Phases

All P6.2 through P6.15 must be exit-checked.

## 4. Required Tools, Libraries, and Languages

Same as Phase 5 + LaTeX for paper.

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **AWS Graviton 100-core cluster** | service | **Paid** | (existing) | Burst |
| **Zenodo** | service | **Free** | https://zenodo.org | DOI |
| **arXiv** | service | **Free** | | Pre-print |
| **AndroZoo** | corpus | **Academic-license** | https://androzoo.uni.lu | Sample source |
| **DREBIN** | corpus | **Academic-license** | https://www.sec.tu-bs.de/~danarp/drebin | Malware |
| **Hugging Face Hub** | service | **Free** + paid | https://huggingface.co/datasets | Bonus mirror |

**API keys required:** Zenodo OAuth, AndroZoo + DREBIN academic-access tokens.

## 6. System Inventory — Have vs Need

All present.

## 7. Features & Functions Delivered (Comprehensive)

### Eval-50K corpus
- Composition: 30K AndroZoo benign + 10K malware (DREBIN + recent feeds) + 5K bundles + 3K obfuscated + 2K NDK-heavy
- Per-sample license tracking
- DPO sign-off
- Manifest + SHA-256 stratification

### Eval pipeline run
- 50K through full stack on 100-core cluster
- ≤ 72 h (HARD)
- Per-stage instrumentation
- Cost dashboard

### Per-stage success rate measurement
- L0/L1: parser success rate
- L2: bundle resolution success rate
- L3: forensic FP rate
- L4: UNKNOWN rate (must be < 5 % HARD)
- L5: BSH stability + bisim TP/FP
- L6: cert emission rate

### Dataset release
- Released under CC-BY-4.0
- Eval-result JSONL (per-APK results)
- Dashboards exported as static HTML
- Zenodo DOI assigned
- Hugging Face Hub mirror

### Open-data paper
- Title: *"The APKAXIOM Corpus: Proof-Stack Evaluation on 50K Android Packages"*
- ≥ 16 pages
- Submission target: USENIX Security 2028 / VLDB / open-data track
- arXiv pre-print

### Reproducibility Docker image
- Single-image runs the eval on a sample
- ≤ 8 GB compressed
- Built reproducibly via Buck2 + Nix flake

### Documentation
- `docs/eval-50k-results.md`
- `docs/eval-50k-methodology.md`

## 8. KPIs (this sub-phase)

| KPI | HARD |
|---|---|
| 50K eval completes ≤ 72 h on 100-core cluster | yes |
| Sustained throughput on 100-core cluster | ≥ 35 APKs/sec |
| L4 UNKNOWN rate (re-confirm) | < 5 % |
| Combined forensic FP rate (re-confirm) | < 0.5 % |
| Backdoor scanner FP rate (re-confirm) | < 5 % |
| Bisim TP/FP (re-confirm) | TP ≥ 95 %, FP < 0.1 % |
| Native-lifter coverage (re-confirm) | ≥ 80 % |
| Dynamic-bridge UNKNOWN refinement (re-confirm) | ≥ 50 % |
| Reproducibility 100 % (re-confirm cross-arch) | yes |
| Dataset published with DOI | yes |
| Paper draft ≥ 16 pages | yes |
| Reproducibility Docker image ≤ 8 GB | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── corpus/
│   └── apkaxiom-eval-50k/            # released
├── results/
│   └── eval-50k/                     # NEW: JSONL + HTML dashboards
├── papers/
│   └── eval-50k-open-data/           # NEW
├── reproducibility/
│   └── eval-50k/                     # NEW: Docker
└── docs/
    ├── eval-50k-results.md           # NEW
    └── eval-50k-methodology.md       # NEW
```

## 10. Standalone Output

50K corpus + paper + dataset + DOI = the headline external-facing v1.0 artifact.

## 11. End-to-End Test

```bash
buck2 run //orch:50k-eval -- --target 100-core-cluster --timeout 72h
# Expect: ≤ 72 h, all per-stage KPIs green

# Dataset
zenodo-cli upload --title "APKAXIOM-Eval-50K" --license CC-BY-4.0 corpus/apkaxiom-eval-50k/

# Reproducibility image
docker build -t apkaxiom/eval-50k reproducibility/eval-50k
docker run apkaxiom/eval-50k --sample 1k
```

## 12. Exit Checklist

- [ ] 50K eval ≤ 72 h on 100-core (HARD)
- [ ] All re-confirm KPIs green (HARD)
- [ ] Reproducibility 100 % cross-arch (HARD)
- [ ] Dataset published + DOI assigned (HARD)
- [ ] Paper draft ≥ 16 pages (HARD)
- [ ] Docker repro image ≤ 8 GB (HARD)
- [ ] arXiv pre-print posted
- [ ] Hugging Face Hub mirror live
- [ ] Documentation `docs/eval-50k-*.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P6.17** | Eval data presented to external auditor |
| **P6.19** | Open-data paper part of v1.0 release announcement |
| **P6.20** | "50K APK eval published" item ✅ for ship gate |
