# P2.19 — USENIX/NDSS Paper Draft + AndroZoo Bundle Benchmark Publication

> Phase-2 paper: *"Rethinking the Unit of Analysis for Android Security in the App Bundle Era."* AndroZoo bundle eval published. Reproducibility artifact + Zenodo DOI.

**Parent plan:** [../README.md](../README.md) · **ROADMAP.md Phase 2 publication target:** [../ROADMAP.md#phase-2](../ROADMAP.md#phase-2)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P2.19 |
| Owner(s) | All Phase 2 groups + lead author from G1 or G3 |
| Duration | Weeks 20–24 |
| Critical-path | yes |
| Hard prerequisites | P2.18 (KPIs measured), P2.10 (formalization paper-citable) |

## 2. Goal & Scope

A ≥ 12-page paper drafted for **USENIX Security 2027** or **NDSS 2027** under the working title *"Rethinking the Unit of Analysis for Android Security in the App Bundle Era."* Plus a Bundles-5K AndroZoo benchmark publication and reviewer-runnable reproducibility artifact.

### In scope
- AndroZoo Bundles-5K eval published with full numbers
- Comparison: APKAXIOM-Phase2 vs Androguard vs MobSF vs apkInspector on bundle handling
- Paper draft ≥ 12 pages with full bibliography
- Reproducibility appendix
- Reproducibility Docker image (artifact for reviewers)
- Internal demo: end-to-end pipeline on representative AABs (including bundle-era malware)

### Out of scope
- Submission timeline (handled by lead author after sub-phase end)
- Acceptance (independent timeline)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P2.18** | All KPI measurements + dashboards |
| **P2.10** | Schrödinger formalization paper-citable |
| **P2.13** | Bundle differential agreement numbers |
| **P2.14, P2.15, P2.16** | Forensic-pass results |
| **P2.17** | Zero-day count + classification breakdown |
| **P1.3** | AndroZoo academic access |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **LaTeX (TeX Live full)** | from P1.19 | Paper typesetting |
| **biber / biblatex** | from P1.19 | Bibliography |
| **TikZ + pgfplots** | latest | Diagrams + plots |
| **Python (matplotlib, seaborn, pandas, duckdb)** | from P1.19 | Performance plots |
| **DuckDB + Arrow** | already in stack | Corpus analytics |
| **Reproducibility Docker image** | OCI | Reviewer-reproducible artifact |

## 5. Third-Party Software, Services, Accounts & API Keys

Same as P1.19 — paper-publication tooling.

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **AndroZoo** | corpus | **Free academic** | already provisioned | Bundles-5K source |
| **arXiv** | preprint | **Free** | https://arxiv.org/user/ | Endorsement from Phase 1 still active |
| **USENIX Security HotCRP** | conference | **Free** | https://www.usenix.org/conference/usenixsecurity27 | Submission account |
| **NDSS HotCRP** | conference | **Free** | https://www.ndss-symposium.org/ndss2027/ | Alt venue |
| **Zenodo** | DOI | **Free** | https://zenodo.org | Permanent DOI for artifact |
| **Software Heritage** | source archival | **Free** | https://www.softwareheritage.org | Permanent archive |
| **Overleaf** *(optional)* | collaborative LaTeX | Free tier; **paid** $$$ | https://www.overleaf.com | Optional |
| **GHCR** | OCI registry | **Free** for our org | https://ghcr.io | Hosts repro Docker image |

**No new API keys** beyond those provisioned in P1.19.

## 6. System Inventory — Have vs Need

### Already present
- ✅ Full TeX Live + biber (from P1.19)
- ✅ Python plotting libs
- ✅ Phase-2 software stack

### Missing
- Phase-2-specific paper template / venue style files

```bash
# Pull USENIX Security style
git clone https://github.com/USENIX/usenix-templates papers/styles/usenix
# Pull NDSS style
git clone https://gitlab.com/ndss/ndss-templates papers/styles/ndss
```

## 7. Features & Functions Delivered (Comprehensive)

### Paper draft (`papers/phase2-usenix/main.tex`)
- ≥ 12 pages
- Sections: Abstract, Introduction, Motivation (bundle-era blind spots), Background (App Bundles, AOSP install), Approach (Schrödinger semantics + ⊕ operator), Implementation (Lean + Rust + bundle resolver + forensics), Evaluation (Bundles-5K, comparison vs other tools, KPI breakdown), Related Work, Discussion, Conclusion
- Full bibliography (≥ 60 references)
- Reproducibility appendix

### AndroZoo Bundles-5K eval (`reports/androzoo-bundles-5k.md`)
- Coverage breakdown
- Bundle-era malware findings
- Forensic-pass FP/recall numbers
- Comparison vs Androguard / MobSF / apkInspector / bundletool baseline
- Per-AOSP-version cross-section

### Comparison study
- Same 1,000 bundles run through APKAXIOM-Phase2 + each baseline tool
- Per-tool: parses successfully (yes/no/partial), bundle-aware (yes/no), dynamic-feature handling (yes/no/partial), repackaging detection (yes/no), reproducibility (yes/no)

### Reproducibility Docker image
- `ghcr.io/Fizan324926/apkaxiom-phase2-eval:1.0`
- Reviewer runs `docker run apkaxiom-phase2-eval /run-eval.sh /corpus/bundles-5k`
- Reproduces all paper numbers within 5 %

### Public release artifact
- Zenodo deposit with DOI
- Software Heritage archive
- Tagged as `phase-2-paper-artifact-v1`

### Internal demo
- End-to-end pipeline run on 100 representative AABs
- Includes bundle-era malware samples (showing cross-config detection)
- Recorded screencast for paper supplementary

### Documentation
- `docs/phase2-eval.md` updated with paper numbers
- `papers/phase2-usenix/repro/README.md`

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Bundles-5K eval coverage | ≥ 99 % | ≥ 99.5 % |
| Comparison vs Androguard / MobSF / apkInspector / bundletool: bundle-handling categorical advantage demonstrated | yes | with quantitative numbers |
| Paper draft length | ≥ 12 pages | ≥ 14 pages |
| Reproducibility Docker reproduces paper numbers | within 5 % | within 1 % |
| Zenodo DOI minted | yes | yes |
| Internal demo run cleanly | yes | yes |
| Public benchmark dashboard live | yes | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── papers/
│   ├── phase2-usenix/
│   │   ├── main.tex
│   │   ├── bibliography.bib
│   │   ├── figures/
│   │   ├── tables/
│   │   └── repro/
│   │       ├── README.md
│   │       └── Dockerfile
│   └── phase2-ndss/                     # alt venue prep
├── reports/
│   ├── androzoo-bundles-5k.md
│   └── tool-comparison.md
└── docs/
    └── phase2-eval.md
```

## 10. Standalone Output

```bash
# Paper builds reproducibly
make paper PAPER=phase2-usenix
sha256sum papers/phase2-usenix/main.pdf > /tmp/paper-hash
diff /tmp/paper-hash papers/phase2-usenix/REFERENCE.sha256

# Reproducibility image
docker run --rm -v $PWD/corpus:/corpus ghcr.io/Fizan324926/apkaxiom-phase2-eval:1.0 \
  /run-eval.sh /corpus/bundles-5k > /tmp/repro-numbers.json
```

## 11. End-to-End Test

```bash
buck2 test //papers/phase2-usenix:repro
# - Numbers within 5% of paper claims (HARD)
# - Docker image runs end-to-end
```

## 12. Exit Checklist

- [ ] AndroZoo Bundles-5K eval complete; coverage ≥ 99 % (HARD)
- [ ] Comparison vs Androguard / MobSF / apkInspector / bundletool published (HARD)
- [ ] Paper draft ≥ 12 pages (HARD)
- [ ] Reproducibility Docker image published to GHCR
- [ ] Internal demo screencast published
- [ ] Public benchmark dashboard live
- [ ] Zenodo DOI minted
- [ ] Software Heritage archive deposited
- [ ] HotCRP submission account confirmed for target venue
- [ ] Lead author has clear submission timeline

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P2.20** | Paper + benchmark numbers cited at gate review |
| **External community** | Public artifact + DOI for citation |
| **Phase 3** | Phase-2 paper baseline cited from Phase-3's intent-resolution paper |
