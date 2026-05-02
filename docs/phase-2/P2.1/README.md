# P2.1 — Phase 2 Onboarding: G4 Staffing + Carry-Forward Debt + AOSP A12/A13 Archaeology Kickoff

> Land G4 (Structural Forensics group). Resolve every Phase-1 carry-forward item. Run the AOSP archaeology sprint that surfaces all A12/A13 semantic deltas Lean must absorb.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md](../../../README.md) · [../../TECH_STACK.md](../../TECH_STACK.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P2.1 |
| Owner(s) | Project leadership + all Phase-1 groups + new G4 |
| Duration | Weeks 1–2 |
| Critical-path | **yes** — gates every other Phase-2 sub-phase |
| Hard prerequisites | P1.20 (Phase 1 closed) |

## 2. Goal & Scope

A clean Phase-2 start: G4 fully onboarded, all Phase-1 carry-forward debt resolved or re-classified to Phase 3, and an AOSP archaeology sprint that surfaces every relevant semantic delta from A11 → A12 → A13 that Lean parsers must accommodate.

### In scope
- G4 onboarding (4 engineers): forensics + statistical analysis backgrounds
- Carry-forward debt review meeting (one per group)
- AOSP A12 + A13 archaeology sprint produces a written report
- Phase-2 kickoff meeting + decision log
- Phase-2 budget approval and infrastructure ramp planning

### Out of scope
- Implementing any P1-debt fixes (those flow to the right Phase-2 sub-phase)
- Extending Lean theorems to A12/A13 (P2.3, P2.4, P2.7 do this)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P1.20** | Phase 1 closed; carry-forward debt list; Phase 2 ADR approved |
| **P1.3** | apk-info v0.x audit and AOSP archaeology runbook |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Google `repo` tool** | latest | AOSP partial-sync for A12 and A13 |
| **Bazel** | from P1.1 | Build A12/A13 reference binaries |
| **`aosp-diff`** | from G2's archaeology toolkit | Surface semantic deltas across versions |
| **Lean 4** | pinned | For documenting theorem-statement deltas |
| **Markdown / Mermaid / PlantUML** | latest | Onboarding docs + delta diagrams |
| **Notion / Outline / Mattermost** | as chosen | Onboarding handbook host |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL / Account | Notes |
|---|---|---|---|---|
| **AOSP source repository** | code | **Free** OSS (Apache 2.0) | https://source.android.com | Already used since P1.5 |
| **Android Open Source Project mailing list** | community | **Free** | https://groups.google.com/g/android-platform | For semantic-delta clarification |
| **GitHub team / org** | account | **Paid** ($4–21/user/mo Team/Enterprise) | already provisioned | Add G4 members |
| **Buildkite agent allocation** | CI runtime | **Paid** (existing) | already provisioned | Allocate G4 share |
| **HR/payroll for G4 hires** | service | **Paid** | (org-level decision) | Outside engineering scope |
| **Background-check service for security clearance** | service | **Paid** ~$40–200/check | https://www.checkr.com or equivalent | Optional but recommended for forensics-cleared work |

**No new API keys at this sub-phase** beyond GitHub OAuth tokens (issued per new G4 hire).

## 6. System Inventory — Have vs Need

### Already present
- ✅ AOSP partial sync at A8, A11, A14 (from P1.5/P1.13/P1.14)
- ✅ All Phase-1 toolchains
- ✅ Cuttlefish A14 + Nyx infrastructure

### Missing — must add
- ❌ **AOSP A12** partial sync — `repo init -b android-12.0.0_r34`
- ❌ **AOSP A13** partial sync — `repo init -b android-13.0.0_r83`
- ❌ G4 development workstations provisioned

### Install commands

```bash
# Add A12 sync
mkdir -p external/aosp/sync-A12 && cd external/aosp/sync-A12
~/.bin/repo init -u https://android.googlesource.com/platform/manifest -b android-12.0.0_r34
cp ../sync/.repo/local_manifests/apkaxiom.xml .repo/local_manifests/
~/.bin/repo sync -j$(nproc)
cd ../../..

# Add A13 sync
mkdir -p external/aosp/sync-A13 && cd external/aosp/sync-A13
~/.bin/repo init -u https://android.googlesource.com/platform/manifest -b android-13.0.0_r83
cp ../sync/.repo/local_manifests/apkaxiom.xml .repo/local_manifests/
~/.bin/repo sync -j$(nproc)

# Run archaeology tool to extract semantic deltas
buck2 run //tools/aosp-diff -- --from android-11.0.0_r48 --to android-13.0.0_r83 \
  --components system/core/libziparchive,frameworks/base/core/java/android/content/pm/PackageParser \
  --output reports/aosp-deltas-A11-to-A13.md
```

Disk: ~ 8 GB additional for A12 + A13 partial sync.

## 7. Features & Functions Delivered (Comprehensive)

### Onboarding deliverables
- **G4 onboarding handbook** (`docs/g4-onboarding.md`) — covers AXIOM-IR, Lean toolchain, Buck2, Bazel-for-AOSP, Pyroscope, Grafana, BLAKE3 / HACL\* invariants, code-review norms.
- **First-week tasks per G4 hire** — paired-programming sessions, mathlib4 mini-tutorial, documented PR mentor.
- **G4 group charter** — mission, layer ownership (L3.1, L3.2, L3.3), interfaces with G1/G2/G3, headcount plan to v1.0.

### Carry-forward debt resolution
- **Debt review meeting per Phase-1 group** — minuted, action items assigned to specific Phase-2 sub-phases.
- **Debt rollup document** (`docs/phase-1-carry-forward.md`) — every target-only KPI miss from PHASE_GATES.md §5, with owner + Phase-2 due date.
- **Re-classification ADR** for any debt that legitimately defers to Phase 3.

### AOSP archaeology
- **`reports/aosp-deltas-A11-to-A13.md`** — full enumeration of:
  - libziparchive parser changes (new fields, new validation logic)
  - PackageParser changes (manifest schema additions, validation order changes)
  - APK Signing Block changes (v3.1 propagation, scheme variations)
  - Resource format changes (sparse encoding, new config qualifiers)
  - DEX format changes (new opcodes, alignment rules)
- **Per-delta classification:** `must-formalize` (changes Lean theorems), `extension-only` (new fields, no change to existing proofs), `irrelevant` (out of scope for our coverage).
- **Decision log** for each `must-formalize` delta — assigned to the right Phase-2 sub-phase.

### Phase-2 kickoff artifacts
- **Kickoff meeting minutes** with decisions on: scope adjustments, hiring asks for the rest of Phase 2, infrastructure ramp (more KVM nodes for P2.17), corpus expansion (Bundles-5K from AndroZoo).
- **Phase-2 communication plan** — internal stand-up cadence, weekly all-hands, paper-writing schedule.

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET | Source |
|---|---|---|---|
| G4 onboarded engineers | ≥ 3 (of 4) | 4 of 4 | Phase-2 plan |
| Carry-forward debt items closed or re-classified | 100 % | 100 % | Phase-1 review |
| AOSP archaeology delta report length | ≥ 20 pages | ≥ 30 pages | Quality bar |
| `must-formalize` deltas assigned to Phase-2 sub-phases | 100 % | 100 % | Decision log |
| Phase-2 kickoff sign-off | by leadership + all group leads | same | Process |
| ADR-Phase2-Kickoff merged | yes | yes | Operational |
| AOSP A12, A13 partial sync compiles libziparchive reproducibly | yes | yes | Hermetic build |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── docs/
│   ├── g4-onboarding.md                  # NEW
│   ├── g4-charter.md                     # NEW
│   ├── phase-1-carry-forward.md          # NEW
│   └── ADR-Phase2-Kickoff.md             # NEW
├── reports/
│   └── aosp-deltas-A11-to-A13.md         # NEW
├── external/aosp/
│   ├── sync-A12/                         # NEW
│   └── sync-A13/                         # NEW
└── meetings/
    ├── 2026-MM-DD-phase2-kickoff.md      # NEW
    └── 2026-MM-DD-debt-review-G{1..13}.md
```

## 10. Standalone Output

The archaeology delta report (`reports/aosp-deltas-A11-to-A13.md`) is reusable — Phase 2's Lean sub-phases (P2.3, P2.4, P2.7) will reference it directly. The G4 onboarding handbook is reusable for any future hires onto the forensics team.

## 11. End-to-End Test

This sub-phase is coordination-heavy; "test" = sign-off:

```bash
# Verification
test -f docs/ADR-Phase2-Kickoff.md
grep -c "^✅ approved by" docs/ADR-Phase2-Kickoff.md  # ≥ 5 leads
test -f reports/aosp-deltas-A11-to-A13.md
wc -l reports/aosp-deltas-A11-to-A13.md  # ≥ 800 lines
buck2 build //external/aosp:libziparchive-A12-bin //external/aosp:libziparchive-A13-bin
# both must build hermetically
```

## 12. Exit Checklist

- [ ] G4 staffed: ≥ 3 of 4 engineers onboarded
- [ ] G4 onboarding handbook published
- [ ] G4 charter approved
- [ ] Phase-1 carry-forward debt 100 % closed or re-classified to Phase 3
- [ ] AOSP A12 + A13 partial sync working
- [ ] `aosp-diff` tool produces delta report A11 → A13 (≥ 20 pages)
- [ ] Every `must-formalize` delta assigned to a Phase-2 sub-phase owner
- [ ] Phase-2 kickoff meeting minuted and signed
- [ ] ADR-Phase2-Kickoff merged

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P2.3, P2.4, P2.7** | AOSP A12/A13 deltas inform Lean theorems |
| **P2.14, P2.15, P2.16** | G4 staffed and oriented to start forensic passes |
| **P2.17** | A12/A13 sync ready for fuzz harness wrap |
| **All P2.x** | Carry-forward debt plan; clean Phase-2 start |
