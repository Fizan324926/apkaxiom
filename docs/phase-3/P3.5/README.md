# P3.5 — Intent-Filter Resolution Semantics in Lean

> The crown jewel of Phase 3's Lean side. Mechanize Android's intent-resolution algorithm. ~2,000 LOC. Soundness theorem against AOSP `PackageManagerService`. Cross-version (A8..A15).

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §9](../../../README.md#layer-4)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P3.5 |
| Owner(s) | G1 + G5 |
| Duration | Weeks 5–11 |
| Critical-path | yes — gates the symbolic resolver |
| Hard prerequisites | P3.4 (PM state) |

## 2. Goal & Scope

The complete formalization of Android's intent-filter resolution algorithm in Lean. Given `(deviceState, intent, callerPackage, userId)` produce `Set ResolvedComponent` — exactly what `PackageManagerService.queryIntentActivities` (and friends) returns. Soundness theorem: Lean output equals AOSP output on the same input across A8 → A15.

### In scope
- `theorems/Apkaxiom/IntentResolution/Filter.lean` — intent-filter matching
- `theorems/Apkaxiom/IntentResolution/Algorithm.lean` — resolution algorithm
- `theorems/Apkaxiom/IntentResolution/Priority.lean` — priority sorting + tie-breaking
- `theorems/Apkaxiom/IntentResolution/Soundness.lean` — versus AOSP
- Differential corpus + adversarial cases
- All ≥ 30 cross-version test scenarios from P3.2 mechanized

### Out of scope
- Symbolic execution (P3.7)
- CHC encoding (P3.7)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P3.2** | AOSP archaeology — exact algorithm delta per version |
| **P3.4** | DeviceState type |
| **P2.9** | AXIOM-IR-v0.2 — manifest dialect for IntentFilter representation |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Lean 4 + mathlib4** | pinned | Theorem prover |
| **AOSP `PackageManagerService.queryIntentActivities` (etc.)** | pinned per version | Reference implementation |
| **Cuttlefish A8/A11/A12/A13/A14/A15** | from prior sub-phases | Differential reference (run actual `pm queryintentactivities`) |
| **adb** | from P2.13 | Drive Cuttlefish queries |
| **Hypothesis** | property-based fixtures | Generate intent + state sequences |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **AOSP source** | reference | **Free** OSS | already provisioned | |
| **Cuttlefish images** | reference runtime | **Free** OSS | already on KVM nodes | |
| **adb** | client | **Free** | already installed | |
| **Android Test Vectors** *(if released by Google)* | reference test inputs | **Free** | https://source.android.com | Optional |

**No new API keys.**

## 6. System Inventory — Have vs Need

### Already present
- ✅ Lean / Lake / mathlib4
- ✅ Cuttlefish + adb (from P2.13)
- ✅ AOSP partial syncs

### Missing
- Nothing new system-level.

## 7. Features & Functions Delivered (Comprehensive)

### Lean theorems

#### Intent-filter matching
- `intentFilter_matchesAction : IntentFilter → Action → Bool`
- `intentFilter_matchesCategory : IntentFilter → Set Category → Bool`
- `intentFilter_matchesData : IntentFilter → Uri → Option MimeType → Bool`
- Combined: `filter_matches : IntentFilter → Intent → Bool`

#### Resolution algorithm
- `resolveActivities : DeviceState → Intent → CallerPackage → UserId → Vec ResolvedComponent`
- `resolveServices : ...`
- `resolveReceivers : ...`
- `resolveContentProvider : ...`
- Each implements the AOSP-version-specific algorithm — e.g., A14's implicit-intent restrictions
- `resolution_well_typed : ∀ args, resolution args |>.allComponentsHaveValidMatchingFilters`

#### Priority handling
- `sortByPriority : Vec ResolvedComponent → Vec ResolvedComponent` (with stable tie-breaking by package install order)
- `priority_sort_correct : ∀ s I c u, sortByPriority (resolveActivities s I c u) = AOSP_v.sort`

#### Multi-user resolution
- `crossProfileResolution : DeviceState → Intent → UserId → Set ResolvedComponent` — work-profile / restricted-user semantics

#### Soundness theorem
- `intent_resolution_sound : ∀ v ∈ {A8..A15}, ∀ s I c u, resolveActivities (s, v) I c u = AOSP_v.queryIntentActivities (s, I, c, u)` — version-stratified soundness

#### Adversarial fixtures
- All ≥ 10 known intent-hijack CVEs from P3.2 mechanized as `(initialState, attackInstall, intent, expectedHijackOutcome)` tuples
- Lean predicts the same outcome AOSP does on Cuttlefish

### Differential test driver
- `tests/intent-resolution-vs-aosp/` — drives Cuttlefish for each (state, intent) tuple, captures `pm queryintentactivities` output, diffs against Lean reference
- Per-AOSP-version pass/fail report
- ≥ 5,000 randomly-generated (Hypothesis) + 500 adversarial test cases

### Documentation
- `docs/lean-intent-resolution.md` — algorithm walk-through, priority semantics, multi-user, AOSP cross-reference

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Cumulative Lean LOC (intent-resolution) | ≥ 2,000 | ≥ 3,000 |
| Theorem re-verify on CI | ≤ 60 min | ≤ 35 min |
| Differential corpus | ≥ 5,000 + 500 adversarial | ≥ 15K + 2K |
| Lean ↔ AOSP agreement on benign | 100 % | 100 % |
| Cross-version (A8..A15) agreement on benign | 100 % | 100 % |
| Known intent-hijack CVEs reproduced as fixtures | ≥ 10 | ≥ 30 |
| Multi-user / work-profile fixtures | ≥ 50 | ≥ 100 |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── theorems/Apkaxiom/IntentResolution/
│   ├── Filter.lean                      # NEW — ~400 LOC
│   ├── Algorithm.lean                   # NEW — ~700 LOC
│   ├── Priority.lean                    # NEW — ~300 LOC
│   ├── MultiUser.lean                   # NEW — ~300 LOC
│   └── Soundness.lean                   # NEW — ~300 LOC
├── corpus/intent-resolution/
│   ├── benign/                          # 5K Hypothesis-generated
│   └── adversarial/                     # 500 (CVE-derived + hand-crafted)
├── tests/intent-resolution-vs-aosp/     # NEW — Cuttlefish differential
│   └── src/main.rs
└── docs/
    └── lean-intent-resolution.md        # NEW
```

## 10. Standalone Output

```bash
nix develop
buck2 build //theorems:intent-resolution-all
buck2 test //tests/intent-resolution-vs-aosp
# "5500/5500 (state, intent) tuples Lean ↔ AOSP agree across A8..A15"
```

## 11. End-to-End Test

Per-AOSP-version differential against Cuttlefish-running `pm queryintentactivities`. All known intent-hijack CVEs reproduced and confirmed.

```bash
for v in A8 A11 A12 A13 A14 A15; do
  buck2 test //tests/intent-resolution-vs-aosp:$v
done
# All HARD: 100% agreement
```

## 12. Exit Checklist

- [ ] All 5 intent-resolution Lean modules land
- [ ] Cumulative LOC ≥ 2,000 (HARD)
- [ ] Theorem re-verify ≤ 60 min (HARD)
- [ ] ≥ 5,000 + 500 adversarial fixtures
- [ ] 100 % Lean ↔ AOSP agreement on benign + adversarial (HARD)
- [ ] 100 % cross-version (A8..A15) agreement (HARD)
- [ ] ≥ 10 intent-hijack CVEs mechanized
- [ ] ≥ 50 multi-user / work-profile fixtures
- [ ] Documentation published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P3.7** | Resolution algorithm encoded as CHC for Spacer |
| **P3.8** | Lean reference for symbolic resolver soundness |
| **P3.9** | Cross-APK extends to "set of installed APKs" |
| **P3.19** | The Phase-3 paper builds on this formalization |
