# P4.19 — CCS / S&P Paper Draft + `.axc` Spec Publication

> Phase-4 paper: *"Proof-Carrying APKs: A New Architecture for Mobile App Distribution."* `.axc` v1 RFC published. Reproducibility artifact + Zenodo DOI.

**Parent plan:** [../README.md](../README.md) · **ROADMAP.md Phase 4 publication target:** [../ROADMAP.md#phase-4](../ROADMAP.md#phase-4)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P4.19 |
| Owner(s) | All Phase 4 groups + lead author from G7 |
| Duration | Weeks 20–24 |
| Critical-path | yes |
| Hard prerequisites | P4.18 (KPIs), P4.2 (`.axc` RFC frozen) |

## 2. Goal & Scope

A ≥ 12-page paper drafted for **CCS 2027** or **IEEE S&P 2028** under the working title *"Proof-Carrying APKs: A New Architecture for Mobile App Distribution."* Plus public `.axc` v1 spec publication and reviewer-runnable reproducibility artifact.

### In scope
- `.axc` v1 spec publicly published (RFC + Cap'n Proto schema)
- Bench-10K cert-emission + verifier eval published
- Comparison: APKAXIOM-Phase4 vs no-cert baseline (in terms of triager confidence + verification time)
- Paper draft ≥ 12 pages
- Reproducibility appendix
- Reproducibility Docker image (artifact)
- Internal demo: end-to-end flow from APK → cert → triager-facing verdict

### Out of scope
- Submission timeline (lead author after sub-phase end)
- Acceptance (independent timeline)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P4.18** | KPI measurements + dashboards |
| **P4.2** | `.axc` RFC frozen and ready for public release |
| **P4.5–P4.10** | Privacy-invariant + STARK examples |
| **P4.11/P4.12** | Verifier perf numbers |
| **P4.17** | Pilot results |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **LaTeX (TeX Live full)** | from P1.19 | Paper |
| **biber / biblatex** | latest | Bibliography |
| **TikZ + pgfplots** | latest | Plots |
| **Python plotting** | from P1.19 | Performance plots |
| **DuckDB + Arrow** | already in stack | Corpus analytics |
| **Reproducibility Docker image** | OCI | Reviewer-reproducible |

## 5. Third-Party Software, Services, Accounts & API Keys

Same as P2.19 / P3.19.

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **AndroZoo + Bench-10K** | corpora | **Free** | already provisioned | |
| **arXiv** | preprint | **Free** | already endorsed | |
| **CCS submission system** | conference | **Free** | https://www.sigsac.org/ccs/CCS2027 | |
| **IEEE S&P submission system** | conference | **Free** | https://www.ieee-security.org/TC/SP2028 | |
| **Zenodo** | DOI | **Free** | https://zenodo.org | |
| **Software Heritage** | source archival | **Free** | already provisioned | |
| **GHCR** | OCI registry | **Free** | already provisioned | |

**No new API keys.**

## 6. System Inventory — Have vs Need

Same as P2.19 / P3.19. New: CCS / S&P style files.

```bash
git clone https://github.com/sigsac/ccs-templates papers/styles/ccs
```

## 7. Features & Functions Delivered (Comprehensive)

### Paper draft (`papers/phase4-ccs/main.tex`)
- ≥ 12 pages
- Sections: Abstract, Introduction (the heuristic-stack vs proof-stack thesis), Background (zk-SNARKs, Halo2, app-store provenance), Approach (`.axc` format + Halo2 circuits + verifier architecture), Implementation, Evaluation (Bench-10K cert emission + verifier perf + pilot results), Related Work (proof-carrying code lineage, App Attest, Play Protect), Discussion (cross-platform deployment, mobile UX, triager experience), Conclusion
- Full bibliography (≥ 80 references)
- Reproducibility appendix
- Cross-platform results table

### `.axc` v1 spec public release
- `docs/AXC-v1.md` (frozen) deposited to:
  - GitHub release `axc-spec-v1.0`
  - Zenodo with DOI
  - Software Heritage

### Bench-10K eval (`reports/phase4-bench-10k-cert-emission.md`)
- Per-APK: emission time (per-claim breakdown), cert size, verification time
- Distribution plots
- Comparison Halo2 vs Stwo per claim

### Tool comparison (`reports/phase4-tool-comparison.md`)
- APKAXIOM `.axc` certs vs no-certs baseline
- Triager survey responses (from P4.17 pilot)
- Time-to-verdict metrics

### Reproducibility Docker image
- `ghcr.io/Fizan324926/apkaxiom-phase4-eval:1.0`
- Reviewer runs `docker run apkaxiom-phase4-eval /run-eval.sh`
- Reproduces all paper numbers within 5 %

### Internal demo
- Recorded end-to-end pipeline screencast
- Shows APK → analysis → cert generation → verifier UI → ✅ verdict

### Documentation
- `docs/phase4-eval.md` updated with paper numbers

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| `.axc` v1 spec publicly published with Zenodo DOI | yes | yes |
| Bench-10K eval coverage | ≥ 99 % | ≥ 99.5 % |
| Comparison vs no-cert baseline | yes (with quantitative metrics) | with triager survey |
| Paper draft length | ≥ 12 pages | ≥ 14 pages |
| Reproducibility Docker reproduces paper numbers | within 5 % | within 1 % |
| Internal demo run cleanly | yes | yes |
| Public benchmark dashboard live | yes | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── papers/
│   └── phase4-ccs/
│       ├── main.tex
│       ├── bibliography.bib
│       ├── figures/
│       ├── tables/
│       ├── repro/
│       └── styles/
├── reports/
│   ├── phase4-bench-10k-cert-emission.md
│   ├── phase4-tool-comparison.md
│   └── phase4-pilot-survey.md
└── docs/
    └── phase4-eval.md
```

## 10. Standalone Output

```bash
make paper PAPER=phase4-ccs
sha256sum papers/phase4-ccs/main.pdf > /tmp/paper-hash
diff /tmp/paper-hash papers/phase4-ccs/REFERENCE.sha256

docker run --rm -v $PWD/corpus:/corpus ghcr.io/Fizan324926/apkaxiom-phase4-eval:1.0 \
  /run-eval.sh /corpus > /tmp/repro-numbers.json
```

## 11. End-to-End Test

```bash
buck2 test //papers/phase4-ccs:repro
# - Numbers within 5% of paper claims (HARD)
# - Docker image runs end-to-end
```

## 12. Exit Checklist

- [ ] `.axc` v1 spec publicly released with DOI (HARD)
- [ ] Bench-10K eval ≥ 99 % coverage (HARD)
- [ ] Comparison vs no-cert baseline (HARD)
- [ ] Paper draft ≥ 12 pages (HARD)
- [ ] Reproducibility Docker published (HARD)
- [ ] Internal demo screencast published
- [ ] Public benchmark dashboard live
- [ ] Zenodo DOI minted
- [ ] HotCRP submission account confirmed
- [ ] Lead author has submission timeline

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P4.20** | Paper + benchmark numbers cited at gate review |
| **External community** | Public `.axc` spec + paper artifact |
| **Phase 5** | Phase-4 paper baseline for native-code paper |
