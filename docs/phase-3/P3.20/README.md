# P3.20 — Phase 3 Hard-Gate Review + Phase 4 ADR

> Walk every PHASE_GATES.md §7 line against the live dashboard. Sign-off or slip. Phase 4 scope ADR before Phase 4 starts. G7 hiring plan locked.

**Parent plan:** [../README.md](../README.md) · **PHASE_GATES.md §7:** [../../PHASE_GATES.md#phase-3](../../PHASE_GATES.md#phase-3) · **ROADMAP.md decision points:** [../ROADMAP.md#decision-points](../ROADMAP.md#decision-points)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P3.20 |
| Owner(s) | Project leadership + all Phase 3 group leads (G1, G2, G3, G4, G5, G6, G8, G13) |
| Duration | Weeks 24–26 |
| Critical-path | yes — gates Phase 4 |
| Hard prerequisites | P3.18, P3.19 |

## 2. Goal & Scope

Every PHASE_GATES.md §7 hard gate reviewed against the live dashboard. Failed targets logged as carry-forward debt. Phase 4 scope ADR + G7 hiring plan locked.

### In scope
- Phase 3 gate review meeting (recorded + minuted)
- ADR-Phase4-Scope: which P3 target gates carry forward, scope adjustments, hiring asks (G7 cryptographers!)
- Phase 3 retrospective document
- Sign-off from G1, G2, G3, G4, G5, G6, G8, G13 leads + leadership
- Public Phase 3 release tag (`phase-3-complete`)
- Lessons-learned writeup on the BSH RFC + bisim engine + cross-APK zero-day discovery

### Out of scope
- Phase 4 implementation (starts next)
- v1.0 ship gate (Phase 6)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **All P3.x** | Their exit checklists must be ✅ for ≥ 7 consecutive days |
| **P3.18** | Live KPI dashboards |
| **P3.19** | Paper drafted |

## 4. Required Tools, Libraries, and Languages

Same as P1.20 / P2.20.

| Tool | Version | Purpose |
|---|---|---|
| **GitHub Releases / Sigstore** | bundled | Tag + sign |
| **OBS Studio** *(optional)* | latest | Record gate review |

## 5. Third-Party Software, Services, Accounts & API Keys

Same as P1.20 / P2.20.

**No new API keys.**

## 6. System Inventory — Have vs Need

Nothing new system-level.

## 7. Features & Functions Delivered (Comprehensive)

### Phase 3 retrospective (`docs/phase3-retrospective.md`)
- What worked: G5/G6 onboarding, BSH-256 freeze on schedule, bisim TP/FP gates met, cross-APK zero-day discovered
- What didn't: any KPIs that missed targets; any sub-phase that overran
- Process learnings — for Phase 4+
- Communication patterns to keep / change
- **Special section: lessons from the cross-APK zero-day** — what worked, what should be repeated in Phase 4

### Phase 4 scope ADR (`docs/ADR-Phase4-Scope.md`)
- Phase 4 active groups (+ G7 + G12 + G14)
- Headcount asks (G7 cryptographers are scarce — start hiring NOW)
- Carry-forward debt that flows to Phase 4
- Scope adjustments (if any)
- Critical-path schedule for Phase 4
- Risk register for Phase 4

### Carry-forward debt classification (`docs/phase-3-carry-forward.md`)
- Per group: target-only KPI misses
- Per debt item: severity, Phase-4 owner, due date

### Gate review meeting
- Live dashboard walkthrough — every PHASE_GATES.md §7 hard line verified
- Minuted with decisions
- Recorded for archival

### Public release tag
- Git tag `phase-3-complete`, signed via cosign
- Release notes summarizing every G1-G8 + G13 deliverable, every Phase-3 KPI result, carry-forward debt

### G7 hiring plan
- 4–5 cryptographers by Phase-4 start (M18)
- Halo2 / zk-SNARK background mandatory
- Begin sourcing immediately (lead time 3–4 months)

### Sign-offs
- All group leads + project leadership

## 8. KPIs (this sub-phase)

| KPI | HARD |
|---|---|
| All PHASE_GATES.md §7 hard gates ✅ for ≥ 7 consecutive days | yes |
| All §7 target gates either met or documented as carry-forward debt | yes |
| Phase 4 scope ADR approved | yes |
| G7 hiring plan + start dates locked | yes |
| Phase 3 retrospective complete | yes |
| Sign-off from all group leads + leadership | yes |
| Release tag `phase-3-complete` signed | yes |
| Public release notes published | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── docs/
│   ├── phase3-retrospective.md         # NEW
│   ├── phase-3-carry-forward.md        # NEW
│   ├── ADR-Phase4-Scope.md             # NEW
│   └── phase3-zero-day-postmortem.md   # NEW (cross-APK case study)
├── meetings/
│   └── 2026-MM-DD-phase3-gate-review.md
└── (release tag created via gh + cosign)
    phase-3-complete
```

## 10. Standalone Output

A signed Git tag `phase-3-complete` with release notes covering: every PHASE_GATES.md §7 hard gate, every group's deliverables, carry-forward debt, Phase-4 scope summary, G7 hiring plan.

## 11. End-to-End Test

The gate review meeting itself. Live dashboard walkthrough; every hard gate verified.

```bash
gh release create phase-3-complete \
  --title "Phase 3 Complete: Sound-and-Complete Intent Resolution + Equivalence Proofs" \
  --notes-file docs/phase3-release-notes.md \
  --target main

cosign sign-blob --yes \
  $(git rev-parse phase-3-complete) > release.sig
gh release upload phase-3-complete release.sig
```

## 12. Exit Checklist (the consolidated Phase 3 ship gate)

**All hard. Every checkbox ✅ for ≥ 7 consecutive days.**

### From PHASE_GATES.md §7
- [ ] All K1–K12 hard gates met
- [ ] L4 UNKNOWN ≤ 25 %
- [ ] BSH collision < 0.1 %, stability ≥ 90 %
- [ ] Bisim TP ≥ 85 %, FP < 1 %
- [ ] Solver timeout < 5 %
- [ ] LSH 1M-index meets size + latency gates

### Phase 3 deliverables
- [ ] BSH-256 RFC frozen and unchanged ≥ 4 weeks
- [ ] Cumulative Phase-3 Lean LOC ≥ 5,000 (PM-state + intent-resolution + bundle / Schrödinger Lean continuations)
- [ ] L4 single-APK + cross-APK + refinement loop shipped
- [ ] L5 unified surface (BSH + LSH + bisim) shipped
- [ ] DRAT cert pipeline + equiv cert format frozen
- [ ] ≥ 1 zero-day intent-hijack from cross-APK reproducible
- [ ] ≥ 100 known intent-hijacks reproduced as proofs
- [ ] Repack-2K eval published
- [ ] Phase-3 paper drafted ≥ 12 pages
- [ ] Reproducibility Docker image published

### Phase 4 readiness
- [ ] Carry-forward debt logged
- [ ] Phase 4 scope ADR approved
- [ ] G7 hiring plan + start dates locked (HARD — start NOW)
- [ ] G12 + G14 hiring underway
- [ ] Phase 4 budget approved
- [ ] Phase 4 critical-path schedule published
- [ ] Phase 3 retrospective merged
- [ ] Sign-off from G1, G2, G3, G4, G5, G6, G8, G13 leads + leadership
- [ ] Release tag `phase-3-complete` signed via cosign
- [ ] Public release notes published
- [ ] Cross-APK zero-day post-mortem published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **Phase 4 onset** | Phase 4 scope ADR; carry-forward debt; G7 + G12 + G14 staffing plan; production-grade Phase-3 stack |
| **External stakeholders** | Public release notes; signed release tag; reproducibility artifact (P3.19) |
| **Future v1.0 ship gate (Phase 6)** | Three phases now follow this template — Phases 4, 5, 6 inherit |
