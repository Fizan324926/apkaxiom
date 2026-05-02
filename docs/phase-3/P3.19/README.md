# P3.19 — IEEE S&P / NDSS Paper Draft + Repack-2K Eval Publication

> Phase-3 paper: *"Sound and Complete Intent Resolution for Android."* Repack-2K bisim eval published. Reproducibility artifact + Zenodo DOI. ≥ 12 pages.

**Parent plan:** [../README.md](../README.md) · **ROADMAP.md Phase 3 publication target:** [../ROADMAP.md#phase-3](../ROADMAP.md#phase-3)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P3.19 |
| Owner(s) | All Phase 3 groups + lead author from G5 |
| Duration | Weeks 20–24 |
| Critical-path | yes |
| Hard prerequisites | P3.18 (KPIs), P3.5 (formalization paper-citable), P3.9 (zero-day) |

## 2. Goal & Scope

A ≥ 12-page paper drafted for **IEEE S&P 2027** or **NDSS 2028** under the working title *"Sound and Complete Intent Resolution for Android."* Plus Repack-2K bisim eval publication and reviewer-runnable reproducibility artifact.

### In scope
- Repack-2K bisim eval published with TP/FP numbers
- Snapshot-1K cross-APK eval published
- Comparison: APKAXIOM-Phase3 vs IC3 / COVERT / FlowDroid / IntentScope on intent resolution + repackaging detection
- Paper draft ≥ 12 pages
- Reproducibility appendix
- Reproducibility Docker image (artifact)
- Internal demo: end-to-end pipeline on representative scenarios (cross-APK zero-day demonstration)

### Out of scope
- Submission timeline (lead author after sub-phase end)
- Acceptance (independent timeline)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P3.18** | KPI measurements + dashboards |
| **P3.5** | Lean formalization paper-citable |
| **P3.9** | Zero-day cross-APK finding |
| **P3.13** | BSH-256 RFC published |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **LaTeX (TeX Live full)** | from P1.19 | Paper |
| **biber / biblatex** | latest | Bibliography |
| **TikZ + pgfplots** | latest | Plots |
| **Python (matplotlib, seaborn, pandas, duckdb)** | from P1.19 | Performance plots |
| **DuckDB + Arrow** | already in stack | Corpus analytics |
| **Reproducibility Docker image** | OCI | Reviewer-reproducible |

## 5. Third-Party Software, Services, Accounts & API Keys

Same as P2.19.

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **AndroZoo / Repack-2K / DREBIN** | corpora | **Free** | already provisioned | |
| **arXiv** | preprint | **Free** | already endorsed | |
| **IEEE S&P HotCRP** | conference | **Free** | https://www.ieee-security.org/TC/SP2027 | |
| **NDSS HotCRP** | conference | **Free** | https://www.ndss-symposium.org/ndss2028/ | Alt venue |
| **Zenodo** | DOI | **Free** | https://zenodo.org | |
| **Software Heritage** | source archival | **Free** | already provisioned | |
| **GHCR** | OCI registry | **Free** | already provisioned | |

**No new API keys.**

## 6. System Inventory — Have vs Need

Same as P2.19. Just paper-template prep:

```bash
git clone https://github.com/USENIX/usenix-templates papers/styles/usenix
git clone https://gitlab.com/ieee-sp/sp-templates papers/styles/sp
```

## 7. Features & Functions Delivered (Comprehensive)

### Paper draft (`papers/phase3-sp/main.tex`)
- ≥ 12 pages
- Sections: Abstract, Introduction, Motivation (intent-hijack history; over-approximation problem), Background (Android intent resolution, prior work IC3 / COVERT / IntentScope / FlowDroid), Approach (Lean formalization + CHC encoding + abstraction-refinement), Implementation (cvc5 / Spacer / abstract domains / DRAT certs), Evaluation (Bench-10K, Repack-2K, Snapshot-1K, comparison vs prior work), Related Work, Discussion (zero-day case study), Conclusion
- Full bibliography (≥ 80 references)
- Reproducibility appendix
- Threat-to-validity discussion

### Repack-2K bisim eval (`reports/phase3-repack-2k-eval.md`)
- TP/FP per obfuscator (ProGuard / R8 / DexGuard)
- Per-cert verification rate
- Comparison vs ssdeep / TLSH / Dexofuzzy

### Snapshot-1K cross-APK eval (`reports/phase3-snapshots-eval.md`)
- Zero-day reproduction
- UNKNOWN-rate analysis
- Snapshot-budget tuning curves

### Tool comparison (`reports/phase3-tool-comparison.md`)
- Same 100 intent-resolution scenarios run through APKAXIOM + IC3 + COVERT + IntentScope + FlowDroid
- Per-tool: sound (yes/no), complete (yes/no), produces certs (yes/no), supports cross-APK (yes/no), latency
- APKAXIOM is the only sound + complete + cert-producing tool

### Reproducibility Docker image
- `ghcr.io/Fizan324926/apkaxiom-phase3-eval:1.0`
- Reviewer runs `docker run apkaxiom-phase3-eval /run-eval.sh /corpus`
- Reproduces all paper numbers within 5 %

### Public release artifact
- Zenodo deposit with DOI
- Software Heritage archive
- Tagged `phase-3-paper-artifact-v1`

### Internal demo
- Recorded end-to-end pipeline run on 50 scenarios (including the cross-APK zero-day)
- Screencast for paper supplementary

### Documentation
- `docs/phase3-eval.md` updated with paper numbers
- `papers/phase3-sp/repro/README.md`

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Repack-2K eval coverage | ≥ 99 % | ≥ 99.5 % |
| Comparison vs IC3 / COVERT / IntentScope / FlowDroid | yes (categorical advantage) | + quantitative numbers |
| Paper draft length | ≥ 12 pages | ≥ 14 pages |
| Reproducibility Docker reproduces paper numbers | within 5 % | within 1 % |
| Zenodo DOI minted | yes | yes |
| Internal demo run cleanly | yes | yes |
| Public benchmark dashboard live | yes | yes |
| Zero-day case study formally written up | yes | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── papers/
│   └── phase3-sp/
│       ├── main.tex
│       ├── bibliography.bib
│       ├── figures/
│       ├── tables/
│       ├── repro/
│       │   ├── README.md
│       │   └── Dockerfile
│       └── styles/
├── reports/
│   ├── phase3-repack-2k-eval.md
│   ├── phase3-snapshots-eval.md
│   └── phase3-tool-comparison.md
└── docs/
    └── phase3-eval.md
```

## 10. Standalone Output

```bash
make paper PAPER=phase3-sp
sha256sum papers/phase3-sp/main.pdf > /tmp/paper-hash
diff /tmp/paper-hash papers/phase3-sp/REFERENCE.sha256

docker run --rm -v $PWD/corpus:/corpus ghcr.io/Fizan324926/apkaxiom-phase3-eval:1.0 \
  /run-eval.sh /corpus > /tmp/repro-numbers.json
```

## 11. End-to-End Test

```bash
buck2 test //papers/phase3-sp:repro
# - Numbers within 5% of paper claims (HARD)
# - Docker image runs end-to-end
```

## 12. Exit Checklist

- [ ] Repack-2K eval coverage ≥ 99 % (HARD)
- [ ] Comparison vs prior work published (HARD)
- [ ] Paper draft ≥ 12 pages (HARD)
- [ ] Reproducibility Docker published to GHCR (HARD)
- [ ] Internal demo screencast published
- [ ] Public benchmark dashboard live
- [ ] Zenodo DOI minted (HARD)
- [ ] Software Heritage deposited
- [ ] HotCRP submission account confirmed
- [ ] Zero-day case study formally written
- [ ] Lead author has submission timeline

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P3.20** | Paper + benchmark numbers cited at gate review |
| **External community** | Public artifact + DOI for citation |
| **Phase 4** | Phase-3 paper baseline cited from Phase-4's PCC paper |
