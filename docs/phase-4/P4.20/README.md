# P4.20 — Phase 4 Hard-Gate Review + Phase 5 ADR

> Walk every PHASE_GATES.md §8 line against the live dashboard. Sign-off or slip. Phase 5 scope ADR before Phase 5 starts. G9 + G10 + G11 hiring plan locked.

**Parent plan:** [../README.md](../README.md) · **PHASE_GATES.md §8:** [../../PHASE_GATES.md#phase-4](../../PHASE_GATES.md#phase-4) · **ROADMAP.md decision points:** [../ROADMAP.md#decision-points](../ROADMAP.md#decision-points)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P4.20 |
| Owner(s) | Project leadership + all Phase 4 group leads (G1–G14 representation) |
| Duration | Weeks 24–26 |
| Critical-path | yes — gates Phase 5 |
| Hard prerequisites | P4.18, P4.19 |

## 2. Goal & Scope

Every PHASE_GATES.md §8 hard gate reviewed against the live dashboard. Failed targets logged as carry-forward debt. Phase 5 scope ADR + G9 / G10 / G11 hiring plan locked.

### In scope
- Phase 4 gate review meeting (recorded + minuted)
- ADR-Phase5-Scope: which P4 target gates carry forward, scope adjustments, hiring asks (G9 native-code engineers, G10 dynamic-analysis, G11 ML-security)
- Phase 4 retrospective document
- Sign-off from G1, G2, G3, G4, G5, G6, G7, G8, G12, G13, G14 leads + leadership
- Public Phase 4 release tag (`phase-4-complete`)

### Out of scope
- Phase 5 implementation (starts next)
- v1.0 ship gate (Phase 6)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **All P4.x** | Their exit checklists must be ✅ for ≥ 7 consecutive days |
| **P4.18** | Live KPI dashboards |
| **P4.19** | Paper drafted, `.axc` spec published |

## 4. Required Tools, Libraries, and Languages

Same as P1.20 / P2.20 / P3.20.

## 5. Third-Party Software, Services, Accounts & API Keys

Same as P3.20.

**No new API keys.**

## 6. System Inventory — Have vs Need

Nothing new system-level.

## 7. Features & Functions Delivered (Comprehensive)

### Phase 4 retrospective (`docs/phase4-retrospective.md`)
- What worked: G7+G12+G14 onboarding, all 5 zk circuits shipped, verifier hit p99 gate, pilot platform ingested real findings
- What didn't: any KPIs that missed, any sub-phase that overran
- Process learnings — for Phase 5+
- Communication patterns to keep / change
- **Special section: lessons from the bug-bounty pilot** — triager UX, partner expectations, future expansion

### Phase 5 scope ADR (`docs/ADR-Phase5-Scope.md`)
- Phase 5 active groups (+ G9 + G10 + G11)
- Headcount asks (G9 native-code engineers, G10 dynamic, G11 ML-security)
- Carry-forward debt that flows to Phase 5
- Scope adjustments
- Critical-path schedule for Phase 5
- Risk register for Phase 5

### Carry-forward debt classification
- Per group: target-only KPI misses
- Per debt item: severity, Phase-5 owner, due date

### Gate review meeting
- Live dashboard walkthrough — every PHASE_GATES.md §8 hard line verified
- Minuted with decisions
- Recorded for archival

### Public release tag
- Git tag `phase-4-complete`, signed via cosign
- Release notes summarizing every G1–G14 deliverable, every Phase-4 KPI result, carry-forward debt

### G9 + G10 + G11 hiring plan
- 4 native-code engineers (G9) by Phase-5 start (M24)
- 3 dynamic-analysis engineers (G10) by Phase-5 start
- 2–3 ML-security engineers (G11) by Phase-5 start
- All hiring should already be underway during Phase 4

### Sign-offs
- All group leads + project leadership

## 8. KPIs (this sub-phase)

| KPI | HARD |
|---|---|
| All PHASE_GATES.md §8 hard gates ✅ for ≥ 7 consecutive days | yes |
| All §8 target gates either met or documented as carry-forward debt | yes |
| Phase 5 scope ADR approved | yes |
| G9 + G10 + G11 hiring plan + start dates locked | yes |
| Phase 4 retrospective complete | yes |
| Sign-off from all group leads + leadership | yes |
| Release tag `phase-4-complete` signed | yes |
| Public release notes published | yes |
| `.axc` v1 spec publicly released | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── docs/
│   ├── phase4-retrospective.md
│   ├── phase-4-carry-forward.md
│   ├── ADR-Phase5-Scope.md
│   └── phase4-pilot-postmortem.md
├── meetings/
│   └── 2026-MM-DD-phase4-gate-review.md
└── (release tag created via gh + cosign)
    phase-4-complete
```

## 10. Standalone Output

A signed Git tag `phase-4-complete` with release notes. Public `.axc` v1 spec. Reproducibility artifact (P4.19) on Zenodo with DOI.

## 11. End-to-End Test

The gate review meeting itself. Live dashboard walkthrough; every hard gate verified.

```bash
gh release create phase-4-complete \
  --title "Phase 4 Complete: Proof-Carrying APKs in Production" \
  --notes-file docs/phase4-release-notes.md \
  --target main

cosign sign-blob --yes \
  $(git rev-parse phase-4-complete) > release.sig
gh release upload phase-4-complete release.sig
```

## 12. Exit Checklist (consolidated Phase 4 ship gate)

**All hard. Every checkbox ✅ for ≥ 7 consecutive days.**

### From PHASE_GATES.md §8
- [ ] All K1–K12 hard gates met
- [ ] Verifier p99 ≤ 100 ms over 10K certs
- [ ] Cold start ≤ 500 ms
- [ ] Cert size median ≤ 100 KB, p99 ≤ 500 KB
- [ ] Cross-arch byte-identical certs 100 %
- [ ] All 3 SDKs throughput floors met
- [ ] Pilot ingestion ≥ 500 / hour
- [ ] SLSA L4 verifier round-trips
- [ ] Reproducibility 100 % across runs and architectures

### Phase 4 deliverables
- [ ] `.axc` v1 spec frozen, publicly released, with Zenodo DOI
- [ ] All 5 priority privacy invariants ship as Halo2 circuits
- [ ] Stwo (post-quantum) fallback operational
- [ ] `axiom-verify` Rust + Wasm + ARM64 mobile builds production-ready
- [ ] axiom-py + axiom-go + axiom-ts SDKs published to their registries
- [ ] SLSA L4 + reproducible-build verifier operational
- [ ] Bug-bounty pilot platform live in production
- [ ] Phase-4 paper drafted ≥ 12 pages
- [ ] Reproducibility Docker image published

### Phase 5 readiness
- [ ] Carry-forward debt logged
- [ ] Phase 5 scope ADR approved
- [ ] G9 + G10 + G11 hiring plan + start dates locked (HARD)
- [ ] Phase 5 budget approved
- [ ] Phase 5 critical-path schedule published
- [ ] Phase 4 retrospective merged
- [ ] Sign-off from all group leads + leadership
- [ ] Release tag `phase-4-complete` signed via cosign
- [ ] Public release notes + pilot post-mortem published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **Phase 5 onset** | Phase 5 scope ADR; G9 + G10 + G11 staffing plan; production-grade Phase-4 stack |
| **External stakeholders** | Public release notes; signed release tag; reproducibility artifact (P4.19) |
| **Future v1.0 ship gate (Phase 6)** | Four phases now follow this template — Phases 5 and 6 inherit |
