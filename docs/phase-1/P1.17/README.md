# P1.17 — Soundness Regression Suite as Fail-Closed CI Gate

> The CI gate nobody can override. Theorem re-verify + translation-validation on every L1 PR. Deliberate-break test confirms it's real.

**Parent plan:** [../README.md](../README.md) · **PHASE_GATES.md K11 Soundness:** [../../PHASE_GATES.md §5](../../PHASE_GATES.md#phase-1)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P1.17 |
| Owner(s) | G1 + G13 |
| Duration | Weeks 12–20 |
| Critical-path | yes — gates the trust we extend over the rest of the proof stack |
| Hard prerequisites | P1.9 (extraction pipeline), P1.16 (signing extracted) |

## 2. Goal & Scope

Every PR that touches Lean theorems or extracted Rust **must** pass the soundness regression suite — re-verify of all theorems plus translation-validation on the full Bench-1K corpus. The gate is **fail-closed**: a red gate blocks merge with no override. A deliberate-break test confirms the gate is real, not theatrical.

### In scope
- CI workflow `.github/workflows/soundness.yml`
- `make soundness` runs full suite locally
- 30+ PRs land with green gates (proves operational reality)
- Quarterly mathlib4 upgrade dry-run runbook
- Deliberate-break test in a sandbox branch

### Out of scope
- Performance regression (separate gate)
- Reproducibility regression (separate gate from P1.1)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P1.1** | CI substrate (Buck2 + GH Actions / Buildkite) |
| **P1.2** | Lean toolchain in CI |
| **P1.9** | Extraction pipeline + translation validator |
| **P1.16** | Signing extraction (covered by the gate) |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **GitHub Actions / Buildkite** | from P1.1 | CI runtime |
| **Lean / Lake** | pinned | Re-verify |
| **Buck2** | from P1.1 | Build orchestration |
| **Cachix / mathlib reservoir** | from P1.2 | Speed re-verify |
| **OpenTelemetry** | from P1.7 | CI run timing |
| **Pyroscope** | from P1.7 | CI flamegraphs of the soundness run |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **GitHub Actions** | CI | Free public; **paid** $0.008/min Linux for private | bundled | For PR-gate runs |
| **Buildkite** | self-hosted-agent CI | **Paid** ($15/user/mo + agent infra) | https://buildkite.com | For long-running soundness runs |
| **mathlib reservoir cache** | Lean cache | **Free** | https://reservoir.lean-lang.org | Critical for ≤ 90 min wall time |

**No new API keys.** Reuses tokens issued by GitHub / Buildkite.

## 6. System Inventory — Have vs Need

### Already present
- ✅ All from P1.1, P1.2, P1.9, P1.16

### Missing
- Nothing system-level. CI workflow file is the deliverable.

## 7. Working Directory & Files Produced

```
apkaxiom/
├── .github/
│   └── workflows/
│       └── soundness.yml                # NEW — fail-closed gate
├── ci/
│   ├── soundness/
│   │   ├── run.sh                       # NEW — make soundness entrypoint
│   │   └── changed-modules.sh           # NEW — only re-run affected theorems
│   └── deliberate-break-test/
│       └── README.md                    # NEW — runbook for the test
├── Makefile                             # adds `make soundness` target
└── docs/
    ├── soundness-regression.md          # NEW — operational doc
    └── mathlib-upgrade-runbook.md       # NEW — quarterly procedure
```

## 8. Standalone Output

A CI workflow that any external observer can read in `.github/workflows/soundness.yml`. The gate is publicly visible on every PR.

## 9. End-to-End Test

The "deliberate-break test" is itself the E2E test of this sub-phase:

```bash
# In a sandbox branch:
# 1) Edit theorems/Apkaxiom/Zip/LocalHeader.lean to break a theorem
sed -i 's/by rfl/by sorry/' theorems/Apkaxiom/Zip/LocalHeader.lean

# 2) Push and observe the gate fail with the expected error
git push origin sandbox/break-test
gh pr create --base main --head sandbox/break-test
# Expected: PR shows red soundness gate with "theorem zip_local_header_size used `sorry`"

# 3) Revert and confirm gate goes green
git revert HEAD && git push
# Expected: PR turns green, becomes mergeable
```

The runbook is checked into `ci/deliberate-break-test/README.md` and re-executed quarterly.

## 10. Exit Checklist

- [ ] Soundness CI workflow live (`.github/workflows/soundness.yml`)
- [ ] `make soundness` documented and works locally
- [ ] 30 consecutive PRs land with green soundness gate (HARD)
- [ ] Deliberate-break test confirms fail-closed (HARD)
- [ ] Soundness regression CI wall-time ≤ 90 min p99 (HARD per PHASE_GATES.md §5)
- [ ] Quarterly mathlib4 upgrade runbook documented
- [ ] Pyroscope captures profile of every soundness run
- [ ] No PR with `sorry` in Lean ever merges (proven by gate + repo audit)

## 11. Hand-Off

| Consumed by | What they need |
|---|---|
| **P1.18** | Soundness gate must be green on all changes leading into Bench-10K eval |
| **P1.20** | Phase 1 ship gate cites soundness regressions = 0 |
| **All later phases** | This gate is permanent infrastructure |
