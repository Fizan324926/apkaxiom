# P3.4 — PackageManager State Model in Lean

> Formalize what "device state" means in Lean. Installed package set, per-package signatures, per-component enabled state, user-profile state, default-app preferences. The substrate L4 reasons over.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §9 (Layer 4)](../../../README.md#layer-4)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P3.4 |
| Owner(s) | G1 + G5 |
| Duration | Weeks 3–9 |
| Critical-path | yes |
| Hard prerequisites | P3.2 (AOSP archaeology) |

## 2. Goal & Scope

A Lean formalization of the device state PackageManager reasons over. ~1,500 LOC. Mechanizes: installed APK set, per-package signature, per-component (Activity/Service/Receiver/Provider) enabled state, user-profile state (primary, work, secondary user), default-app preferences, runtime-permission grants.

### In scope
- `theorems/Apkaxiom/Pm/State.lean` — typed device-state record
- `theorems/Apkaxiom/Pm/InstallOps.lean` — install / uninstall / enable / disable / set-default operations
- `theorems/Apkaxiom/Pm/Signatures.lean` — signature equivalence classes
- `theorems/Apkaxiom/Pm/UserProfile.lean` — multi-user / work-profile state
- `theorems/Apkaxiom/Pm/Permissions.lean` — runtime-permission state
- Adversarial corpus: hand-crafted state sequences leading to known intent-hijack scenarios

### Out of scope
- Intent-filter resolution itself (P3.5)
- Symbolic execution (P3.7+)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P3.2** | AOSP archaeology — PackageManager state-machine delta enumeration |
| **P2.9** | AXIOM-IR-v0.2 — manifest dialect for component/permission references |
| **P3.3** | AXIOM-IR-symbolic preview — for SymVal references in state |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Lean 4 + mathlib4** | pinned | Theorem prover |
| **mathlib4 finite-set + Map machinery** | from mathlib | Device-state representation |
| **AOSP `frameworks/base/services/core/.../pm/PackageManagerService` source** | pinned per Android version | Reference |
| **Hypothesis** (Python) | for corpus | Property-based state-sequence generation |
| **Z3 / cvc5** | from P3.1 | Cross-check certain invariants |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **AOSP source** | reference | **Free** OSS | already provisioned | |
| **Android Compatibility Definition Document** | spec | **Free** | https://source.android.com/docs/compatibility/cdd | Authoritative for state-machine requirements |
| **mathlib4 PR review** | community | **Free** | leanprover-community/mathlib4 | If we contribute upstream-useful state primitives |

**No new API keys.**

## 6. System Inventory — Have vs Need

### Already present
- ✅ Lean / Lake / mathlib4
- ✅ AOSP partial syncs
- ✅ Hypothesis

### Missing
- Nothing new system-level.

## 7. Features & Functions Delivered (Comprehensive)

### Lean theorems

#### Device-state record
- `structure DeviceState where`
  - `installedApks : Finset ApkId`
  - `apkSignatures : ApkId → Option SignatureSet`
  - `componentStates : ApkId → ComponentName → ComponentEnabled`
  - `userProfiles : Finset UserId × UserProfile`
  - `defaultApps : Map (Action × UserId) ApkId`
  - `permissionGrants : ApkId → UserId → Finset Permission`
  - `apiLevel : AndroidVersion`

#### State-mutation operations
- `installApk : DeviceState → Apk → SignatureSet → Except InstallError DeviceState`
- `uninstallApk : DeviceState → ApkId → DeviceState`
- `enableComponent / disableComponent : DeviceState → ApkId → ComponentName → DeviceState`
- `grantPermission / revokePermission : DeviceState → ApkId → UserId → Permission → DeviceState`
- `setDefault : DeviceState → Action → UserId → ApkId → DeviceState`
- `switchUser : DeviceState → UserId → DeviceState`

#### Invariants
- `device_state_invariant : DeviceState → Prop` — every installed APK has a signature, every default-app reference resolves, etc.
- `install_preserves_invariant : ∀ s a sig, install s a sig = ok s' → invariant s → invariant s'`
- `permission_grant_preserves_invariant : ...`

#### Signature equivalence
- `SignatureSet := Finset Certificate`
- `signaturesEquivalent : SignatureSet → SignatureSet → Bool` — formalizes Android's signing-equivalence check
- `signaturePermissionGate : Permission → DeviceState → ApkId → ApkId → Bool` — signature-permission semantics

#### User-profile model
- `UserProfile := { primary, work, secondary, restricted }`
- `crossProfileAccess : UserProfile → UserProfile → ResourceClass → Bool`

#### Adversarial corpus
- ≥ 200 hand-crafted state sequences leading to known intent-hijack CVEs (CVE-2017-13288, CVE-2018-9367, CVE-2020-0103, CVE-2022-20413, ...)
- Each fixture: install order + permissions + components + expected resolution outcome

### Documentation
- `docs/lean-pm-state.md` — design, invariants, AOSP-reference call-trace mapping

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Cumulative Lean LOC (PM state) | ≥ 1,500 | ≥ 2,500 |
| Theorem re-verify on CI | ≤ 35 min | ≤ 20 min |
| All state-mutation ops formalized | yes | yes |
| Signature-permission semantics formalized | yes | yes |
| Multi-user / work-profile semantics formalized | yes | yes |
| Adversarial state-sequence corpus | ≥ 200 fixtures | ≥ 500 |
| Cross-version A8..A15 deltas captured | 100 % | 100 % |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── theorems/Apkaxiom/Pm/
│   ├── State.lean                       # NEW — ~400 LOC
│   ├── InstallOps.lean                  # NEW — ~400 LOC
│   ├── Signatures.lean                  # NEW — ~300 LOC
│   ├── UserProfile.lean                 # NEW — ~200 LOC
│   ├── Permissions.lean                 # NEW — ~200 LOC
│   └── Invariants.lean                  # NEW — ~200 LOC
├── corpus/
│   └── pm-state-fixtures/                # 200+ adversarial state sequences
└── docs/
    └── lean-pm-state.md                  # NEW
```

## 10. Standalone Output

```bash
nix develop
buck2 build //theorems:pm-state-all
buck2 test //tests/pm-state-fixtures
# All 200+ fixtures evaluate to expected outcome
```

## 11. End-to-End Test

```bash
buck2 test //tests/pm-state-fixtures
# - All 200+ fixtures match AOSP-reference outcomes (HARD)
# - Theorems re-verify in ≤ 35 min (HARD)
# - Adversarial CVE fixtures reach expected vulnerable state (sanity check)
```

## 12. Exit Checklist

- [ ] All 6 PM-state Lean modules land
- [ ] Cumulative LOC ≥ 1,500 (HARD)
- [ ] All state-mutation ops formalized
- [ ] Signature-permission semantics formalized (incl. cross-user)
- [ ] Multi-user / work-profile state formalized
- [ ] ≥ 200 adversarial fixtures
- [ ] Theorem re-verify ≤ 35 min on CI (HARD)
- [ ] All known intent-hijack CVEs reachable as state sequences
- [ ] `docs/lean-pm-state.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P3.5** | Device-state record as input to intent-resolution algorithm |
| **P3.7** | PM state model encoded as CHC variables |
| **P3.8** | Symbolic resolver operates over `Approx<DeviceState>` |
| **P3.9** | Cross-APK device-snapshot prototype is precisely a `DeviceState` |
