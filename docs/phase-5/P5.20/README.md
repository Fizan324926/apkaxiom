# P5.20 — Phase 5 Hard-Gate Review + Phase 6 ADR

> Walk every PHASE_GATES.md §9 line against the live dashboard. Sign-off or slip. Phase 6 stabilization-mode ADR before Phase 6 starts. External-audit RFP issued. APKAXIOM-Eval-50K corpus locked.

**Parent plan:** [../README.md](../README.md) · **PHASE_GATES.md §9:** [../../PHASE_GATES.md#phase-5](../../PHASE_GATES.md#phase-5) · **ROADMAP.md decision points:** [../ROADMAP.md#decision-points](../ROADMAP.md#decision-points)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P5.20 |
| Owner(s) | Project leadership + all Phase-5 group leads (G1–G14) |
| Duration | Weeks 24–26 |
| Critical-path | yes — gates Phase 6 |
| Hard prerequisites | P5.18, P5.19 |

## 2. Goal & Scope

Every PHASE_GATES.md §9 hard gate reviewed against the live dashboard. Failed targets logged as carry-forward debt. Phase 6 scope ADR (no new groups; stabilization mode) + external-audit RFP issued + APKAXIOM-Eval-50K corpus locked.

### In scope
- Phase 5 gate review meeting (recorded + minuted)
- ADR-Phase6-Scope: stabilization plan per group, KPI ratchet for v1.0
- ADR-Phase6-Audit: RFP issued to Trail of Bits, NCC Group, Aleph Research (or equivalent)
- ADR-Phase6-Eval50K: corpus composition + license + governance for the v1.0 dataset
- Phase 5 retrospective document
- Sign-off from G1–G14 leads + leadership
- Public Phase 5 release tag (`phase-5-complete`)

### Out of scope
- Phase 6 implementation (starts next)
- v1.0 ship gate (P6.20)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **All P5.x** | Their exit checklists must be ✅ for ≥ 7 consecutive days |
| **P5.18** | Live KPI dashboards |
| **P5.19** | Paper drafted, datasets published |

## 4. Required Tools, Libraries, and Languages

Same as P4.20.

## 5. Third-Party Software, Services, Accounts & API Keys

Same as P4.20.

**No new API keys** beyond audit-firm contact + RFP portals.

## 6. System Inventory — Have vs Need

Nothing new system-level.

## 7. Features & Functions Delivered (Comprehensive)

### Phase 5 retrospective (`docs/phase5-retrospective.md`)
- What worked: G9 + G10 + G11 onboarding, native lifters at scale, dynamic-confirmation bridge, ML scanners, joint analyzer cross-language zero-day
- What didn't: any KPIs that missed, any sub-phase that overran
- Process learnings — for Phase 6 (stabilization)
- Communication patterns to keep / change

### Phase 6 scope ADR (`docs/ADR-Phase6-Scope.md`)
- No new groups; all groups in stabilization mode
- Per-group stabilization plan
- KPI ratchet for v1.0
- Critical-path schedule for Phase 6
- Risk register for Phase 6
- v1.0 ship-gate definition (mirroring ROADMAP §15)

### External-audit RFP (`docs/ADR-Phase6-Audit.md`)
- Engagement scope (~10 weeks starting M31)
- Candidate firms: Trail of Bits, NCC Group, Aleph Research, Atredis
- Procurement timeline
- NDA templates
- Scoping assumptions

### APKAXIOM-Eval-50K corpus ADR (`docs/ADR-Phase6-Eval50K.md`)
- Composition: 30K AndroZoo benign + 10K malware (DREBIN + AndroZoo + recent feeds) + 5K bundles + 3K obfuscated + 2K NDK-heavy
- License governance: each sample categorized for redistribution
- Governance: data-protection officer review
- Manifest with SHA-256 stratification

### Carry-forward debt classification
- Per group: target-only KPI misses
- Per debt item: severity, Phase-6 owner, due date

### Gate review meeting
- Live dashboard walkthrough — every PHASE_GATES.md §9 hard line verified
- Minuted with decisions
- Recorded for archival

### Public release tag
- Git tag `phase-5-complete`, signed via cosign
- Release notes summarizing every G1–G14 deliverable, every Phase-5 KPI result, carry-forward debt

### Sign-offs
- All group leads + project leadership

## 8. KPIs (this sub-phase)

| KPI | HARD |
|---|---|
| All PHASE_GATES.md §9 hard gates ✅ for ≥ 7 consecutive days | yes |
| All §9 target gates either met or documented as carry-forward debt | yes |
| Phase 6 scope ADR approved | yes |
| External-audit RFP issued | yes |
| APKAXIOM-Eval-50K corpus locked | yes |
| Phase 5 retrospective complete | yes |
| Sign-off from all group leads + leadership | yes |
| Release tag `phase-5-complete` signed | yes |
| Public release notes published | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── docs/
│   ├── phase5-retrospective.md
│   ├── phase-5-carry-forward.md
│   ├── ADR-Phase6-Scope.md
│   ├── ADR-Phase6-Audit.md
│   └── ADR-Phase6-Eval50K.md
├── meetings/
│   └── 2026-MM-DD-phase5-gate-review.md
└── (release tag created via gh + cosign)
    phase-5-complete
```

## 10. Standalone Output

A signed Git tag `phase-5-complete` with release notes. External-audit RFP package. Eval-50K corpus manifest published.

## 11. End-to-End Test

The gate review meeting itself. Live dashboard walkthrough; every hard gate verified.

```bash
gh release create phase-5-complete \
  --title "Phase 5 Complete: Joint Static-Dynamic Android Analysis" \
  --notes-file docs/phase5-release-notes.md \
  --target main

cosign sign-blob --yes \
  $(git rev-parse phase-5-complete) > release.sig
gh release upload phase-5-complete release.sig
```

## 12. Exit Checklist (consolidated Phase 5 ship gate)

**All hard. Every checkbox ✅ for ≥ 7 consecutive days.**

### From PHASE_GATES.md §9
- [ ] All K1–K12 hard gates met
- [ ] Native lift ≥ 50 MB/s DEX, ≥ 25 MB/s ELF, ≥ 60 % NDK function coverage
- [ ] Dynamic bridge resolves ≥ 30 % UNKNOWNs
- [ ] ML scanner ≥ 90 % precision, ≥ 80 % recall
- [ ] Full pipeline ≥ 7 APKs/sec on 16-core
- [ ] p99 ≤ 30 s static, ≤ 120 s dynamic
- [ ] Cross-arch byte-identical certs 100 %
- [ ] Reproducibility 100 % across runs and architectures

### Phase 5 deliverables
- [ ] AXIOM-IR-v0.4 native dialect frozen + published
- [ ] DEX + ARM64 + ARMv7 lifters production-grade
- [ ] JNI bridge model
- [ ] Native common-library catalog
- [ ] Joint Java + native analyzer + ≥ 1 zero-day disclosed
- [ ] Lean theorems for native lifter soundness machine-checked
- [ ] Emulator pool live + chaos-drilled
- [ ] Frida + eBPF script libraries
- [ ] Dynamic-confirmation bridge operational
- [ ] TFLite parse + Neural Cleanse + STRIP + adversarial robustness
- [ ] Phase-5 paper drafted ≥ 12 pages
- [ ] NDK-100 + planted-backdoor zoo datasets released
- [ ] Reproducibility Docker image published

### Phase 6 readiness
- [ ] Carry-forward debt logged
- [ ] Phase 6 stabilization scope ADR approved
- [ ] External-audit RFP issued (Trail of Bits / NCC / Aleph / Atredis)
- [ ] APKAXIOM-Eval-50K corpus locked
- [ ] Phase 6 budget approved
- [ ] Phase 6 critical-path schedule published
- [ ] Phase 5 retrospective merged
- [ ] Sign-off from all group leads + leadership
- [ ] Release tag `phase-5-complete` signed via cosign
- [ ] Public release notes published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **Phase 6 onset** | Phase 6 stabilization ADR; external-audit RFP issued; Eval-50K corpus locked |
| **External stakeholders** | Public release notes; signed release tag; reproducibility artifact (P5.19) |
| **Future v1.0 ship gate (P6.20)** | Five phases now follow this template — Phase 6 stabilizes to ship |
