# P3.2 — AOSP Archaeology Extension: Intent-Filter Semantics Across A8–A15

> Surface every relevant change to Android's intent-resolution algorithm across A8 → A15. The semantic ground-truth that L4's Lean theorems must mirror.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §9 (Layer 4)](../../../README.md#layer-4)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P3.2 |
| Owner(s) | G2 (with G5 review) |
| Duration | Weeks 1–4 |
| Critical-path | yes — gates P3.4 / P3.5 |
| Hard prerequisites | P3.1 |

## 2. Goal & Scope

A written archaeological report enumerating every behavior change in Android's intent-resolution algorithm across A8 → A15. PackageManager state evolution, intent-filter matching subtleties, security-policy delta (e.g., implicit-intent restrictions added in A14), priority handling, signature-verification gates.

### In scope
- AOSP `frameworks/base/services/core/.../pm` archaeology across A8/A11/A12/A13/A14/A15
- AOSP `frameworks/base/core/java/android/content` (Intent + IntentFilter)
- Per-version delta enumeration (must-formalize / extension-only / irrelevant)
- Reference-implementation skeleton (call traces in real AOSP for canonical scenarios)
- Cross-version differential test plan for P3.5

### Out of scope
- Lean formalization (P3.4, P3.5)
- Symbolic execution (P3.7+)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P3.1** | AOSP A12/A13/A15 sync (continued from P2.1) |
| **P2.1** | Existing AOSP archaeology pattern |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Google `repo` tool** | latest | AOSP partial-sync extended to A15 |
| **Bazel sub-workspace** | from P1.1 | Build A15 reference |
| **`aosp-diff`** | from P2.1 | Surface semantic deltas |
| **Java decompiler / IDE (IntelliJ-like)** | optional | Source navigation aid |
| **PlantUML / Mermaid** | latest | State-diagram generation |
| **Python (for AST analysis of Java sources)** | from P1.18 | Automated semantic-change detection |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **AOSP source** | code | **Free** OSS | already provisioned | A15 is free to add when released |
| **Android Open Source Project mailing list** | community | **Free** | https://groups.google.com/g/android-platform | Clarification questions |
| **Google Issue Tracker (Android)** | issue tracker | **Free** read-only | https://issuetracker.google.com | Reference for intent-resolution-behavior issues |
| **Android Compatibility Definition Document (CDD)** | spec | **Free** | https://source.android.com/docs/compatibility/cdd | Authoritative for cross-version requirements |

**No API keys.**

## 6. System Inventory — Have vs Need

### Already present
- ✅ AOSP A8, A11, A12, A13, A14 partial syncs
- ✅ Bazel sub-workspace + Buck2 build
- ✅ aosp-diff tool (from P2.1)

### Missing
- ❌ **AOSP A15** partial sync (if released by Phase 3 start) — `repo init -b android-15.0.0_rXX`

```bash
mkdir -p external/aosp/sync-A15 && cd external/aosp/sync-A15
~/.bin/repo init -u https://android.googlesource.com/platform/manifest -b android-15.0.0_r10
cp ../sync/.repo/local_manifests/apkaxiom.xml .repo/local_manifests/
~/.bin/repo sync -j$(nproc)
```

## 7. Features & Functions Delivered (Comprehensive)

### Archaeology report (`reports/aosp-intent-resolution-A8-to-A15.md`)
- ≥ 40 pages
- Per-version delta:
  - **A8 baseline** — initial PackageManagerService intent-resolution
  - **A8 → A11** — adds priority sorting, signature-permission checks
  - **A11 → A12** — implicit-intent restrictions begin
  - **A12 → A13** — runtime-permission gates added
  - **A13 → A14** — implicit-intent restrictions tightened (only declared filters resolve)
  - **A14 → A15** — grammatical-gender, app-cloning impacts
- Per-delta classification: must-formalize / extension-only / irrelevant
- Reference-implementation skeleton for canonical scenarios:
  - Activity launch from home → manifest intent-filter resolution
  - Implicit intent: ACTION_SEND with image/jpeg → which app?
  - Cross-app intent dispatch: Activity A starts ContentProvider B
  - Signature-permission-gated intent
  - User-profile state interaction (work-profile, multi-user)
- Behavior-change call-trace excerpts (real Java source, annotated)

### Cross-version differential test plan
- Concrete test scenarios for P3.5 to mechanize
- Expected resolutions per AOSP version (table form)
- Known intent-hijack CVE references (CVE-2017-13288, CVE-2018-9367, etc.) — formalized as test inputs

### State-diagram artifacts
- PackageManager state machine (PlantUML)
- Intent-resolution flow (Mermaid)
- Per-AOSP-version state-machine deltas annotated

### Decision log
- ADR-0015 — Phase-3 intent-fragment scope (which subset of intent semantics is in / out for v1.0)

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Archaeology report length | ≥ 40 pages | ≥ 60 pages |
| AOSP versions covered | A8 + A11 + A12 + A13 + A14 (+ A15 if released) | all 6 |
| `must-formalize` deltas assigned to P3.4/P3.5 | 100 % | 100 % |
| Cross-version test scenarios drafted | ≥ 30 | ≥ 60 |
| Known intent-hijack CVEs incorporated as fixtures | ≥ 10 | ≥ 30 |
| ADR-0015 merged | yes | yes |
| Reviewer sign-off (G1, G2, G5 leads) | yes | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── reports/
│   └── aosp-intent-resolution-A8-to-A15.md  # NEW
├── docs/
│   ├── ADR-0015-phase3-intent-fragment-scope.md  # NEW
│   └── intent-resolution-archaeology-runbook.md  # NEW
├── external/aosp/
│   └── sync-A15/                              # NEW (if A15 released)
└── diagrams/
    ├── package-manager-state.puml             # NEW
    └── intent-resolution-flow.mmd             # NEW
```

## 10. Standalone Output

The archaeology report — reusable directly by P3.4 + P3.5. CVE-fixture catalog reused by P3.7 + P3.8.

## 11. End-to-End Test

```bash
# Verification
test -f reports/aosp-intent-resolution-A8-to-A15.md
wc -l reports/aosp-intent-resolution-A8-to-A15.md  # ≥ 1500 lines (40+ pages)
buck2 build //external/aosp:intent-resolver-A15-bin  # if A15 added
buck2 run //tools/aosp-diff -- --components frameworks/base/services/core/.../pm \
   --from android-8.0.0 --to android-14.0.0 --output reports/A8-to-A14-pm-deltas.md
```

## 12. Exit Checklist

- [ ] Archaeology report ≥ 40 pages (HARD)
- [ ] All AOSP versions A8–A14 covered (and A15 if released)
- [ ] All `must-formalize` deltas assigned to P3.4/P3.5
- [ ] ≥ 30 cross-version test scenarios
- [ ] ≥ 10 known intent-hijack CVE fixtures incorporated
- [ ] ADR-0015 (intent-fragment scope) merged
- [ ] State diagrams rendered + embedded in report
- [ ] Sign-off from G1, G2, G5 leads

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P3.4** | PackageManager state-machine delta enumeration |
| **P3.5** | Intent-filter resolution algorithm reference |
| **P3.7** | CVE fixtures + canonical test scenarios |
| **P3.8** | First-cut symbolic resolver evaluated against this archaeology |
