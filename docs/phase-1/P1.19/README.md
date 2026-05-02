# P1.19 — Public AndroZoo Benchmark + Phase-1 Paper Draft

> 10K AndroZoo subset evaluated. Numbers vs apk-info v0.x and Androguard published. Paper draft ready for CAV/OOPSLA submission.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../ROADMAP.md §5 Phase 1 publication target](../../ROADMAP.md#phase-1)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P1.19 |
| Owner(s) | All Phase 1 groups + lead author from G1 |
| Duration | Weeks 20–24 |
| Critical-path | yes — paper submission is a Phase-1 deliverable |
| Hard prerequisites | P1.18 (perf numbers + dashboards) |

## 2. Goal & Scope

APKAXIOM-Phase1 is evaluated on a 10K AndroZoo subset. Numbers are published. A ~10–12 page paper is drafted, ready for submission to **CAV 2026** or **OOPSLA 2026** under the working title *"Verified Parsing for the Android Package Format."*

### In scope
- AndroZoo 10K subset run, results dashboarded
- Comparison: APKAXIOM-Phase1 vs `apk-info` v0.x vs Androguard
- Paper draft ≥ 10 pages with full bibliography
- Reproducibility appendix (how reviewers reproduce numbers)
- Internal demo: 1,000 known-good APKs parsed across A8/A11/A14 with Lean proof check passing

### Out of scope
- Submission itself (handled by lead author at sub-phase end)
- Acceptance (independent timeline)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P1.18** | All KPI measurements; reproducible benchmarks |
| **P1.13/P1.14** | Fuzzer findings (cited as motivation) |
| **P1.3** | AndroZoo academic access |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **LaTeX** | TeX Live 2024+ | Paper typesetting |
| **pdflatex** (HAVE) | from texlive-base | PDF output |
| **biber / biblatex** | latest | Bibliography |
| **TikZ + pgfplots** | latest | Diagrams + plots |
| **R / Python (matplotlib + seaborn)** | latest | Performance plots |
| **DuckDB + Arrow** (from TECH_STACK §11) | latest | Corpus analytics |
| **Reproducibility Docker image** | OCI | Reviewer-reproducible artifact |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL / Account | Notes |
|---|---|---|---|---|
| **AndroZoo** | corpus | **Free academic** | https://androzoo.uni.lu | API key from P1.3 |
| **arXiv account** | preprint | **Free** | https://arxiv.org/user/ | Endorsement may be required for first-time submitter (1–2 weeks) |
| **CAV submission system (HotCRP)** | conference | **Free** | https://easychair.org/conferences/?conf=cav2026 | Submission account |
| **OOPSLA submission system (HotCRP)** | conference | **Free** | https://oopsla26.hotcrp.com | Alt venue |
| **Overleaf** *(optional)* | collaborative LaTeX | Free tier; **paid** $$$ for larger projects | https://www.overleaf.com | Some teams prefer; we can also use plain LaTeX in-repo |
| **Zenodo** | DOI for artifact | **Free** | https://zenodo.org | DOI for the artifact reproducibility package |
| **Software Heritage** | source archival | **Free** | https://www.softwareheritage.org | Permanent archive of paper-cited source |

**Account-level requirements:**
- arXiv endorsement — request by Week 20 to avoid blocking submission.
- HotCRP / EasyChair accounts — free, instant.
- Zenodo account — free, instant.

## 6. System Inventory — Have vs Need

### Already present
- ✅ pdflatex, dot (graphviz)
- ✅ Python 3.12, pip
- ✅ Full Phase-1 stack

### Missing — must install
- ❌ **biber** (`sudo apt-get install -y biber`)
- ❌ **TeX Live full** (`sudo apt-get install -y texlive-full` — large, ~6 GB)
- ❌ **Python plotting libs** (`pip install matplotlib seaborn pandas duckdb pyarrow`)

### Install commands

```bash
sudo apt-get install -y texlive-full biber
pip3 install matplotlib seaborn pandas duckdb pyarrow

# Reproducibility Docker image
buck2 run //tools/repro-docker -- --tag apkaxiom-phase1-eval:1.0
docker push ghcr.io/Fizan324926/apkaxiom-phase1-eval:1.0
```

## 7. Working Directory & Files Produced

```
apkaxiom/
├── papers/
│   └── phase1-cav/
│       ├── main.tex                     # NEW — paper source
│       ├── bibliography.bib             # NEW
│       ├── figures/
│       │   ├── architecture.tex
│       │   ├── perf-comparison.pdf      # generated from corpus eval
│       │   └── lean-loc-growth.pdf
│       ├── tables/
│       │   └── kpi-summary.tex          # auto-generated from PHASE_GATES.md §5
│       └── repro/
│           ├── README.md                # how to reproduce numbers
│           └── Dockerfile               # reviewer-runnable image
├── reports/
│   └── androzoo-10k-eval.md            # NEW — public benchmark report
└── docs/
    └── phase1-eval.md                  # updated with AndroZoo numbers
```

## 8. Standalone Output

Two artifacts:
1. **The paper PDF** — `papers/phase1-cav/main.pdf`, ≥ 10 pages, fully cited.
2. **The reproducibility image** — `ghcr.io/Fizan324926/apkaxiom-phase1-eval:1.0`. Reviewers run it and get our numbers within 5%.

## 9. End-to-End Test

```bash
# Paper builds reproducibly
make paper PAPER=phase1-cav
# diff against ref hash
sha256sum papers/phase1-cav/main.pdf > /tmp/paper-hash
diff /tmp/paper-hash papers/phase1-cav/REFERENCE.sha256

# Reproducibility image runs end-to-end
docker run --rm -v $PWD/corpus:/corpus ghcr.io/Fizan324926/apkaxiom-phase1-eval:1.0 \
  /run-eval.sh /corpus/bench-10k > /tmp/repro-numbers.json
# diff vs published numbers, within 5%
```

## 10. Exit Checklist

- [ ] AndroZoo 10K eval complete; ≥ 99% coverage (HARD per PHASE_GATES.md §5)
- [ ] Comparison vs apk-info v0.x: no regression (HARD)
- [ ] Comparison vs Androguard: ≥ 10× faster (HARD)
- [ ] Paper draft ≥ 10 pages
- [ ] Reproducibility Docker image published to GHCR
- [ ] Internal demo run cleanly: 1K APKs across A8/A11/A14 with Lean proof check
- [ ] Public benchmark dashboard live
- [ ] arXiv endorsement obtained (or already endorsed)
- [ ] Zenodo DOI minted for artifact
- [ ] HotCRP/EasyChair submission account confirmed for target venue
- [ ] Lead author has clear submission timeline

## 11. Hand-Off

| Consumed by | What they need |
|---|---|
| **P1.20** | Paper + benchmark numbers cited at the gate review |
| **External community** | Public artifact + DOI for citation |
| **Phase 2 / G3 + G4** | Bench-10K corpus + dashboards reused for Phase 2 KPIs |
