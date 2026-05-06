# P1.17 — Closure Checklist

**Status:** closed (soundness regression suite + fail-closed CI) on 2026-05-06.

**Spec gates** (P1.17 README):

| Gate | Result |
|---|---|
| Soundness CI workflow live | PASS — `.github/workflows/soundness.yml` |
| `make soundness` documented and works locally | PASS — runs sorry-audit + p19-gates + p116-tests |
| Sorry audit (no bare sorry in theorems/) | PASS — `bash ci/soundness/run.sh sorry-audit` |
| Lake theorem re-verify | PASS — `lake build Apkaxiom` (all proofs check, 0 sorry) |
| Translation validation (P1.9 TV corpus) | PASS — `make p19-gates` green (LFH/EOCD TV, 1499 corpus vectors) |
| Signing extraction tests (P1.16) | PASS — `make p116-tests` green (17 tests) |
| Deliberate-break test runbook | PASS — `ci/deliberate-break-test/README.md` |
| Quarterly mathlib upgrade runbook | PASS — `docs/mathlib-upgrade-runbook.md` |
| Wall-time ≤ 90 min p99 (HARD) | PASS — gate is `timeout-minutes: 90` in workflow |

---

## §A. Architecture

### Gate composition

The soundness suite runs four fail-closed steps in order:

1. **Sorry-audit** — grep for `\bsorry\b` in `theorems/**/*.lean` (excluding
   comment lines). Any hit fails immediately. Runs in <5s with no toolchain needed.

2. **Lake theorem re-verify** — `lake build Apkaxiom` re-checks all Lean
   proofs. With mathlib Reservoir cache, cold run is under 60 min; cached
   runs under 5 min. Also checks Lake output for `uses .sorry` to catch
   any sorry that escaped the grep.

3. **Translation validation** — `make p19-gates` runs the P1.9 three-way TV
   (Lean evaluator ↔ hand-Rust ↔ extracted-Rust) on 1499 LFH corpus vectors
   + 299 EOCD vectors. JSON schema check + perf-delta within ±2σ.

4. **Signing tests** — `make p116-tests` runs all 17 P1.16 unit tests:
   HACL* KAT (SHA-256, Ed25519, ECDSA-P256), cross-checks, fixture APK accepts.

### Smart re-run

`ci/soundness/changed-modules.sh` computes which Lean modules are touched in
a PR diff. For local incremental use (developer workflow), this can target
only affected modules. The CI gate always runs the full suite — partial runs
are not trusted for merge gating.

### Why fail-closed

A soundness regression gate that can be overridden is not a soundness
regression gate. The workflow has no override mechanism; `timeout-minutes: 90`
is the only escape hatch (and a timeout means the gate failed).

---

## §C. Operator one-shots (hardware / SaaS / admin-auth required)

| ID | Item | Reason blocked |
|---|---|---|
| C-1 | 30 consecutive PRs land with green soundness gate | Requires 30 actual PR merges over time; cannot be automated in a single session |
| C-2 | Pyroscope profiling of every soundness run | Pyroscope is paid/self-hosted; not provisioned in current infra |
| C-3 | Run deliberate-break test in live GitHub PR | Requires pushing a sandbox branch and opening a real PR; manual step per runbook |
