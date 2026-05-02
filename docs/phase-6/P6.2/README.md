# P6.2 — G1 Stabilization: Re-Prove All Theorems Against Final Lean Toolchain

> Pin Lean 4 to the v1.0 toolchain version. Re-verify every theorem from Phase 1 through Phase 5 in clean tree. Audit the proof spine for soundness regressions. No new theorems unless safety-critical.

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P6.2 |
| Owner(s) | G1 |
| Duration | Weeks 1–14 |
| Critical-path | yes (gates v1.0 soundness claim) |
| Hard prerequisites | P6.1 |

## 2. Goal & Scope

A frozen Lean 4 toolchain version, every previously proven theorem re-verified on clean tree, audit log of any proof drift fixed.

### In scope
- Pin Lean 4 toolchain version (`lean-toolchain` file) — final v1.0 version locked in W4
- Re-verify all theorems: Phase 1 (ZIP, signing, AXML, ARSC), Phase 2 (bundle compose), Phase 3 (PM state, intent-filter resolution, abstract domains, BSH soundness, bisim), Phase 5 (DEX SSA, JNI boundary, catalog summaries, provenance, ARM64 AAPCS64 subset)
- Audit log: every theorem's proof object hash + size + verify time
- Fix proof drift if any (unlikely if CI gates have held)
- mathlib4 pinned compatible version
- 90-day soundness-zero window opens

### Out of scope
- New theorems (unless safety-critical, requires leadership ADR)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P6.1** | Stabilization-mode policy + punch-list |
| **All Phase 1–5 theorem files** | re-verify target |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Lean 4** | v1.0-pinned | Theorem prover |
| **mathlib4** | matching | Math lib |
| **elan** | latest | Lean toolchain manager |

## 5. Third-Party Software, Services, Accounts & API Keys

All free OSS, pinned via Nix flake.

**No new API keys.**

## 6. System Inventory — Have vs Need

All present.

## 7. Features & Functions Delivered (Comprehensive)

### Lean 4 toolchain pin
- `lean-toolchain` file pinned to v1.0 release
- Nix flake updated
- All CI runners migrated

### Re-verification
- Every theorem in `theorems/` re-verified
- Proof object hashes recorded
- Verify times benchmarked (CI gate: < 30 min total, target < 15 min)

### Audit log
- Per theorem: SHA-256 of proof object, size, verify time, prover memory peak
- Audit log signed via cosign
- Published with v1.0 release

### Soundness regression suite
- Continuous re-verify on every L1/L4/L5/L6 PR (already in place from Phase 1)
- 90-day zero-regression window opens (target: green by v1.0 ship)

### Documentation freshening
- Per-theorem comment block with statement + scope
- `docs/lean-soundness.md` — index of all v1.0 theorems

### Performance
- Total re-verify time on CI ≤ 30 min HARD (≤ 15 min TARGET)
- Per-PR L1 incremental re-verify ≤ 5 min HARD

### Reproducibility
- Bytewise-identical Lean compiled artifacts across runs / arches (re-confirmed)

## 8. KPIs (this sub-phase)

| KPI | HARD |
|---|---|
| Lean 4 toolchain pinned to v1.0 | yes |
| All Phase 1–5 theorems re-verified clean | yes |
| Proof-object audit log generated + signed | yes |
| Total re-verify time ≤ 30 min on CI | yes |
| Per-PR incremental re-verify ≤ 5 min | yes |
| 90-day rolling soundness-zero window: 0 incidents (continuous) | yes |
| Bytewise-identical Lean artifacts across runs / arches | yes |
| `docs/lean-soundness.md` index published | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── lean-toolchain                    # PINNED to v1.0
├── theorems/                         # all re-verified
├── docs/
│   ├── lean-soundness.md             # NEW: master index
│   └── lean-toolchain-pin-rationale.md
└── audit/
    └── proof-object-log.jsonl        # NEW: signed via cosign
```

## 10. Standalone Output

The pinned Lean 4 toolchain + audit log makes the soundness claim externally checkable.

## 11. End-to-End Test

```bash
elan default $(cat lean-toolchain)
buck2 build //theorems:...
buck2 test //theorems/...:re-verify
# Expect: all green, total ≤ 30 min

# Audit log
cosign verify-blob --signature audit/proof-object-log.sig audit/proof-object-log.jsonl
```

## 12. Exit Checklist

- [ ] Lean 4 toolchain pinned (HARD)
- [ ] All theorems re-verified clean (HARD)
- [ ] Audit log signed + published (HARD)
- [ ] Total re-verify time ≤ 30 min on CI
- [ ] Per-PR incremental re-verify ≤ 5 min
- [ ] 90-day soundness-zero-incident window in progress (continuous)
- [ ] Bytewise-identical Lean artifacts 100 %
- [ ] `docs/lean-soundness.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P6.16** | Theorems re-verifiable in 50K eval |
| **P6.17** | Audit log presented to external auditor |
| **P6.20** | "Lean theorems re-verified" item ✅ for ship gate |
