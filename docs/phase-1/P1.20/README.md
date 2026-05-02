# P1.20 — Phase 1 Hard-Gate Review + Phase 2 ADR

> Walk every PHASE_GATES.md §5 line against the live dashboard. Sign-off or slip. Phase 2 scope ADR before the next phase starts.

**Parent plan:** [../README.md](../README.md) · **PHASE_GATES.md §5:** [../../PHASE_GATES.md#phase-1](../../PHASE_GATES.md#phase-1) · **ROADMAP.md §12 decision points:** [../../ROADMAP.md#decision-points](../../ROADMAP.md#decision-points)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P1.20 |
| Owner(s) | Project leadership + all Phase 1 group leads (G1, G2, G3, G8, G13) |
| Duration | Weeks 24–26 |
| Critical-path | yes — gates Phase 2 |
| Hard prerequisites | P1.18 (KPIs measured), P1.19 (paper drafted) |

## 2. Goal & Scope

Every PHASE_GATES.md §5 hard gate is reviewed against the live dashboard. Failed targets (not hards) are logged as carry-forward debt. The Phase 2 scope ADR is written, reviewed, and approved.

### In scope
- Phase 1 gate review meeting (recorded + minuted)
- ADR-Phase2-Scope: which P1 target gates carry forward, scope adjustments, Phase-2 hiring asks
- Phase 1 retrospective document
- Sign-off from G1, G2, G3, G8, G13 leads + leadership
- Public Phase 1 release tag (`phase-1-complete`)

### Out of scope
- Phase 2 implementation (starts next)
- v1.0 ship gate (Phase 6)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **All P1.x** | Their exit checklists must be ✅ for ≥ 7 consecutive days |
| **P1.18** | Live KPI dashboards |
| **P1.19** | Paper drafted |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **GitHub Releases** | bundled with GH | Tag `phase-1-complete` |
| **Sigstore (cosign)** | from P1.1 | Sign release artifacts |
| **OBS Studio** *(optional)* | latest | Record gate review meeting |
| **Markdown linting** | from P1.3 | ADR + retrospective format |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **GitHub Releases** | release publishing | **Free** | bundled | Standard release artifact host |
| **Sigstore (cosign)** | signing | **Free** OSS | https://www.sigstore.dev | Sign tag + artifacts |
| **Slack / Mattermost / Discord** | meeting comms | Free tiers; **paid** $$$ for advanced | self-hosted Mattermost recommended | Internal-only |
| **OBS Studio** | meeting recording | **Free** OSS | https://obsproject.com | Optional recording of the gate review |
| **Notion / Coda / Outline** *(optional)* | retrospective doc | Notion/Coda paid; Outline free OSS | https://www.getoutline.com | We can also use Markdown in-repo |

**No new API keys.** Reuses GitHub + signing infrastructure from P1.1.

## 6. System Inventory — Have vs Need

### Already present
- ✅ Everything from prior sub-phases
- ✅ git, gh, sigstore, markdown tools

### Missing
- Nothing system-level. This sub-phase is a coordination + documentation activity.

## 7. Working Directory & Files Produced

```
apkaxiom/
├── docs/
│   ├── phase1-retrospective.md          # NEW — what worked, what didn't
│   └── ADR-Phase2-Scope.md              # NEW — scope adjustments + hiring
├── meetings/
│   └── 2026-MM-DD-phase1-gate-review.md # NEW — minutes + decisions
└── (release tag created via gh)
    phase-1-complete                     # Git tag, signed via cosign
```

## 8. Standalone Output

A signed Git tag `phase-1-complete` whose attached release notes summarize:
- Every PHASE_GATES.md §5 hard gate result.
- Every group's deliverables.
- Carry-forward debt list.
- Phase 2 scope.
- Acknowledgments.

## 9. End-to-End Test

The gate review meeting itself is the test. The dashboard is walked line-by-line; every hard gate is verified against live numbers. The ADR is reviewed and merged in the same window.

```bash
# After meeting:
gh release create phase-1-complete \
  --title "Phase 1 Complete: Verified Parser Foundation" \
  --notes-file docs/phase1-release-notes.md \
  --target main

# Sign the tag with sigstore
cosign sign-blob --yes \
  $(git rev-parse phase-1-complete) > release.sig
gh release upload phase-1-complete release.sig
```

## 10. Exit Checklist (this is the Phase 1 ship gate consolidated)

**All hard. Every checkbox ✅ for ≥ 7 consecutive days for Phase 1 to close.**

### From PHASE_GATES.md §5 (full Phase 1 KPI set)
- [ ] K1 throughput hard gates met
- [ ] K2 latency hard gates met
- [ ] K3 memory hard gates met
- [ ] K4 CPU efficiency hard gates met
- [ ] K5 scalability hard gates met
- [ ] K6 real-time hard gates met
- [ ] K7 stability hard gates met (zero soundness regressions; <10 crashes/1M)
- [ ] K8 stress/burst hard gates met
- [ ] K9 cross-platform parity met
- [ ] K10 reproducibility 100%
- [ ] K11 soundness regressions = 0
- [ ] K12 fuzzer ≥ 10 disagreements/week classified, ≥ 99% uptime

### From the Phase 1 plan
- [ ] AXIOM-IR-v0.1 spec frozen and unchanged ≥ 4 weeks
- [ ] apk-info v1.0 (`axiom-l1-rs`) released, no perf regression vs v0.x
- [ ] Bench-1K E2E smoke green; Bench-10K perf eval published
- [ ] AndroZoo 10K eval published
- [ ] Phase-1 paper drafted, ready for submission
- [ ] Phase 2 scope ADR approved
- [ ] Phase 1 retrospective merged
- [ ] Sign-off from G1, G2, G3, G8, G13 leads + leadership
- [ ] Release tag `phase-1-complete` signed via cosign
- [ ] Public release notes published

### Carry-forward and Phase 2 readiness
- [ ] All target-only gate failures logged as carry-forward debt
- [ ] Phase 2 hiring asks documented (G4 staffing in particular)
- [ ] Phase 2 budget approved
- [ ] Phase 2 critical-path schedule published

## 11. Hand-Off

| Consumed by | What they need |
|---|---|
| **Phase 2 onset** | Phase 2 scope ADR; carry-forward debt; staffing plan; production-grade Phase-1 stack |
| **External stakeholders** | Public release notes; signed release tag; reproducibility artifact (P1.19) |
| **Future v1.0 ship gate (Phase 6)** | This phase establishes the model — every later phase's gate review follows the same template |
