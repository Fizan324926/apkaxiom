# P5.19 — NDSS / RAID Paper Draft + Native+Dynamic Eval Publication

> Draft *"Joint Static-Dynamic Analysis of Android Native Code"* for **NDSS 2028** or **RAID 2028**. Publish NDK-100 + planted-backdoor zoo as datasets. Reproducibility Docker image. Zero-day disclosure case study.

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P5.19 |
| Owner(s) | All groups (G1, G3, G5, G7, G9, G10, G11) lead authoring |
| Duration | Weeks 20–24 |
| Critical-path | yes |
| Hard prerequisites | P5.18 |

## 2. Goal & Scope

A paper draft + dataset release + Docker reproducibility artifact. NDSS 2028 deadline ≈ September 2027 (matches M30 calendar).

### In scope
- Paper draft ≥ 12 pages
- NDK-100 dataset release (CC-BY-4.0)
- Planted-backdoor zoo dataset release
- Reproducibility Docker image (≤ 8 GB compressed)
- Zero-day disclosure case study (coordinated with vendor)
- Zenodo upload with DOI
- Pre-print on arXiv

### Out of scope
- Phase 6 planning (P5.20)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P5.18** | Eval results |
| **P5.8** | Zero-day case study |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **LaTeX (TeX Live)** | 2024+ | Paper |
| **Overleaf** (optional) | latest | Collab editing |
| **Docker / Buildx** | latest | Reproducibility image |
| **Zenodo CLI** | latest | DOI upload |
| **arXiv** | (web) | Pre-print |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **Overleaf Pro (team)** | service | **Paid** ~$20/mo / user | https://overleaf.com | Optional |
| **Zenodo** | service | **Free** | https://zenodo.org | DOI |
| **arXiv** | service | **Free** | https://arxiv.org | Pre-print |
| **Vendor disclosure portal** (for zero-day) | varies | varies | per vendor | |

**API keys required:** Zenodo OAuth (for upload).

## 6. System Inventory — Have vs Need

All present.

## 7. Features & Functions Delivered (Comprehensive)

### Paper draft (`papers/phase-5-ndss/`)
- Title: *"Joint Static-Dynamic Analysis of Android Native Code"*
- Sections: introduction, background (Android native + dynamic + ML), AXIOM-IR-v0.4 native dialect, lifters, JNI bridge model, joint analyzer, dynamic-confirmation bridge, ML scanners, evaluation, related work, conclusion
- ≥ 12 pages excluding refs
- Anonymized version for double-blind submission

### Datasets
- NDK-100 release (100 ARM64/ARMv7 NDK shared libs, CC-BY-4.0, manifest with SHA-256)
- Planted-backdoor zoo (30 patterns × 5 carriers, CC-BY-4.0)
- Eval reproducibility data: scripts + raw results

### Reproducibility Docker image
- Single image runs the entire Phase-5 eval on a sample
- ≤ 8 GB compressed
- Builds reproducibly via Buck2 + Nix flake
- One-command: `docker run apkaxiom/phase-5-eval`

### Zero-day disclosure case study
- One full coordinated disclosure write-up with timelines + CVE
- Vendor responses captured (anonymized for paper)

### Zenodo + arXiv
- Zenodo DOI for datasets + image
- arXiv pre-print posted on submission

### Tools
- `papers-build` script

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Paper draft pages | ≥ 12 | ≥ 14 |
| NDK-100 dataset published | yes | yes |
| Planted-backdoor zoo published | yes | yes |
| Reproducibility Docker image ≤ 8 GB | yes | yes |
| Zero-day case study published | yes | yes |
| Zenodo DOI assigned | yes | yes |
| arXiv pre-print posted | yes | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── papers/
│   └── phase-5-ndss/                # NEW
│       ├── main.tex
│       ├── sections/
│       └── figures/
├── datasets/
│   ├── ndk-100/                     # NEW (released)
│   └── planted-backdoor-zoo/        # NEW (released)
├── reproducibility/
│   └── phase-5/                     # NEW: Docker
├── disclosure/
│   └── phase-5-zero-day-1.md        # NEW
└── (Zenodo + arXiv records)
```

## 10. Standalone Output

Paper + datasets + image + DOI.

## 11. End-to-End Test

```bash
cd papers/phase-5-ndss && latexmk -pdf
# Expect: ≥ 12 pages

docker build -t apkaxiom/phase-5-eval reproducibility/phase-5
docker run apkaxiom/phase-5-eval --sample
# Expect: KPIs from a sample subset reproduce
```

## 12. Exit Checklist

- [ ] Paper draft ≥ 12 pages (HARD)
- [ ] NDK-100 dataset published with DOI (HARD)
- [ ] Planted-backdoor zoo published (HARD)
- [ ] Reproducibility Docker image ≤ 8 GB built + tested (HARD)
- [ ] Zero-day case study published (HARD)
- [ ] arXiv pre-print posted
- [ ] Submission package ready for NDSS / RAID

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P5.20** | Paper artifact + datasets for gate review |
| **External community** | NDK-100 + planted-backdoor zoo |
| **Phase 6** | Reproducibility Docker pattern reused for v1.0 |
