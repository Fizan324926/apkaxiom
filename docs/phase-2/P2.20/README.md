# P2.20 — Phase 2 Hard-Gate Review + Phase 3 ADR + Carry-Forward Debt Rollup

> Walk every PHASE_GATES.md §6 line against the live dashboard. Sign-off or slip. Phase 3 scope ADR before Phase 3 starts. Carry-forward debt classified.

**Parent plan:** [../README.md](../README.md) · **PHASE_GATES.md §6:** [../../PHASE_GATES.md#phase-2](../../PHASE_GATES.md#phase-2) · **ROADMAP.md decision points:** [../ROADMAP.md#decision-points](../ROADMAP.md#decision-points)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P2.20 |
| Owner(s) | Project leadership + all Phase 2 group leads (G1, G2, G3, G4, G8, G13) |
| Duration | Weeks 24–26 |
| Critical-path | yes — gates Phase 3 |
| Hard prerequisites | P2.18 (KPIs measured), P2.19 (paper drafted) |

## 2. Goal & Scope

Every PHASE_GATES.md §6 hard gate reviewed against the live dashboard. Failed targets logged as carry-forward debt. Phase 3 scope ADR written, reviewed, approved.

### In scope
- Phase 2 gate review meeting (recorded + minuted)
- ADR-Phase3-Scope: which P2 target gates carry forward, scope adjustments, hiring asks
- Phase 2 retrospective document
- Sign-off from G1, G2, G3, G4, G8, G13 leads + leadership
- Public Phase 2 release tag (`phase-2-complete`)
- Bundle-era findings published (combination of fuzzer findings, forensic patterns, bundle differential gaps)

### Out of scope
- Phase 3 implementation (starts next)
- v1.0 ship gate (Phase 6)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **All P2.x** | Their exit checklists must be ✅ for ≥ 7 consecutive days |
| **P2.18** | Live KPI dashboards |
| **P2.19** | Paper drafted |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **GitHub Releases** | bundled | Tag `phase-2-complete` |
| **Sigstore (cosign)** | from P1.20 | Sign release artifacts |
| **OBS Studio** *(optional)* | latest | Record gate review |

## 5. Third-Party Software, Services, Accounts & API Keys

Same as P1.20 — coordination tooling.

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **GitHub Releases / Sigstore** | release publishing | **Free** | already provisioned | |
| **Slack / Mattermost** | meeting comms | as configured | already provisioned | |
| **OBS Studio** | recording | **Free** | https://obsproject.com | Optional |
| **Notion / Coda / Outline** *(optional)* | retro doc | as chosen | already provisioned | We can use Markdown in-repo |

**No new API keys.**

## 6. System Inventory — Have vs Need

### Already present
- ✅ Everything from prior sub-phases
- ✅ git, gh, sigstore, markdown tools

### Missing
- Nothing system-level. This sub-phase is coordination + documentation.

## 7. Features & Functions Delivered (Comprehensive)

### Phase 2 retrospective (`docs/phase2-retrospective.md`)
- What worked: G4 onboarding, AXIOM-IR-v0.2 freeze on schedule, bundle resolver throughput target hit, 5-harness fuzzer scaling
- What didn't: any KPIs that missed target gates; any sub-phase that overran
- Process learnings — for Phase 3+ team
- Communication patterns to keep / change

### Phase 3 scope ADR (`docs/ADR-Phase3-Scope.md`)
- Phase 3 active groups (+ G5 + G6)
- Headcount asks
- Carry-forward debt that flows to Phase 3
- Scope adjustments (if any) due to Phase 2 misses
- Critical-path schedule for Phase 3
- Risk register for Phase 3

### Carry-forward debt classification (`docs/phase-2-carry-forward.md`)
- Per group: target-only KPI misses
- Per debt item: severity (P0 / P1 / P2), Phase-3 owner, due date

### Gate review meeting
- Live dashboard walkthrough — every PHASE_GATES.md §6 hard line verified
- Minuted with decisions
- Recorded for archival

### Public release tag
- Git tag `phase-2-complete`
- Signed via cosign
- Release notes summarizing every G1–G8 + G13 deliverable, every Phase-2 KPI result, carry-forward debt

### Phase-2 community announcement
- Blog post or technical writeup (internal-only or public depending on disclosure timing)
- Cite Phase-2 paper draft

### Sign-offs
- G1 lead, G2 lead, G3 lead, G4 lead, G8 lead, G13 lead, project leadership
- Each signs on the live dashboard, captured in the meeting minutes

## 8. KPIs (this sub-phase)

| KPI | HARD |
|---|---|
| All PHASE_GATES.md §6 hard gates ✅ for ≥ 7 consecutive days | yes |
| All §6 target gates either met or documented as carry-forward debt | yes |
| Phase 3 scope ADR approved | yes |
| Phase 2 retrospective complete | yes |
| Sign-off from all group leads + leadership | yes |
| Release tag `phase-2-complete` signed | yes |
| Public release notes published | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── docs/
│   ├── phase2-retrospective.md         # NEW
│   ├── phase-2-carry-forward.md        # NEW
│   └── ADR-Phase3-Scope.md             # NEW
├── meetings/
│   └── 2026-MM-DD-phase2-gate-review.md
└── (release tag created via gh + cosign)
    phase-2-complete
```

## 10. Standalone Output

A signed Git tag `phase-2-complete` whose attached release notes summarize:
- Every PHASE_GATES.md §6 hard gate result
- Every group's deliverables across the 6 months
- Carry-forward debt list to Phase 3
- Phase 3 scope summary
- Acknowledgments

## 11. End-to-End Test

The gate review meeting itself is the test. Live dashboard walked line-by-line; every hard gate verified against live numbers. ADR reviewed and merged in the same window.

```bash
# After meeting
gh release create phase-2-complete \
  --title "Phase 2 Complete: Bundle-Era Analysis Platform" \
  --notes-file docs/phase2-release-notes.md \
  --target main

cosign sign-blob --yes \
  $(git rev-parse phase-2-complete) > release.sig
gh release upload phase-2-complete release.sig
```

## 12. Exit Checklist (the consolidated Phase 2 ship gate)

**All hard. Every checkbox ✅ for ≥ 7 consecutive days.**

### From PHASE_GATES.md §6 (full Phase 2 KPI set)
- [ ] K1 throughput hard gates met
- [ ] K2 latency hard gates met
- [ ] K3 memory hard gates met
- [ ] K4 CPU efficiency hard gates met
- [ ] K5 scalability hard gates met
- [ ] K6 real-time hard gates met
- [ ] K7 stability hard gates met
- [ ] K8 stress/burst hard gates met
- [ ] K9 cross-platform parity met
- [ ] K10 reproducibility 100 %
- [ ] K11 soundness regressions = 0
- [ ] K12 fuzzer ≥ 30 disagreements/week classified, 5 harnesses ≥ 99 % uptime
- [ ] Bundle correctness ≥ 99.9 % vs AOSP installer
- [ ] Combined forensic FP < 12 % on benign

### Phase 2 deliverables
- [ ] AXIOM-IR-v0.2 spec frozen and unchanged ≥ 4 weeks
- [ ] Cumulative Phase-2 Lean LOC ≥ 7,000
- [ ] All extracted Rust crates (AXML, ARSC, DEX, signing) production
- [ ] Schrödinger semantics theorems proved
- [ ] Bundle resolver shipped, dynamic features ≥ 95 % discovery
- [ ] All 3 forensic passes operational with FP gates met
- [ ] ≥ 1 zero-day CVE candidate filed
- [ ] AndroZoo Bundles-5K eval published
- [ ] Phase-2 paper drafted, ≥ 12 pages, ready for submission
- [ ] Reproducibility Docker image published

### Phase 3 readiness
- [ ] Carry-forward debt logged
- [ ] Phase 3 scope ADR approved
- [ ] G5 + G6 hiring plan + start dates
- [ ] Phase 3 budget approved
- [ ] Phase 3 critical-path schedule published
- [ ] Phase 2 retrospective merged
- [ ] Sign-off from G1, G2, G3, G4, G8, G13 leads + leadership
- [ ] Release tag `phase-2-complete` signed via cosign
- [ ] Public release notes published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **Phase 3 onset** | Phase 3 scope ADR; carry-forward debt; G5 + G6 staffing plan; production-grade Phase-2 stack |
| **External stakeholders** | Public release notes; signed release tag; reproducibility artifact (P2.19) |
| **Future v1.0 ship gate (Phase 6)** | Two phases now follow this template — Phases 3, 4, 5, 6 will inherit |
