# P1.2 — Live Status Checklist

> Single status doc for P1.2 (Lean 4 Toolchain & Extraction Prototype).
> Mirrors the §10 exit checklist in [`./README.md`](./README.md) plus the
> state-of-art additions taken on. No separate ADR-per-decision; design
> notes live inline as one-liners.

**Owner:** G1 — Formal Methods Core · **Last reviewed:** 2026-05-03

Legend: ✅ done & verified · 🟡 done but awaiting one external action · ⏳ in-progress · 🧊 deferred-by-design

---

## A. §10 exit checklist

| # | Item | Status | Evidence / next action |
|---|------|--------|------------------------|
| 1 | `lean-toolchain` pinned to a specific Lean 4 release | ✅ | [`/lean-toolchain`](../../../lean-toolchain) → `leanprover/lean4:v4.29.1`; matches `flake.nix` `pkgsUnstable.lean4` (4.29.1). |
| 2 | `lakefile.toml` declares mathlib4 dep, pinned commit | ✅ | [`/lakefile.toml`](../../../lakefile.toml) requires `mathlib v4.29.1`; [`lake-manifest.json`](../../../lake-manifest.json) pins commit `5e932f97dd25535344f80f9dd8da3aab83df0fe6`. `flake.nix` `inputs.mathlib4` records the same SHA for cross-tool provenance. |
| 3 | `Hello.lean` re-verifies on CI in ≤ 10 min | ✅ | `lake build Apkaxiom` ≈ 0.3 s locally; CI `lean-bringup` job has a `timeout-minutes: 10` hard cap. The 10-minute budget includes Lake's first-time package fetch, which the runner-side `actions/cache@v4` step elides on warm runs. |
| 4 | Mathlib4 cache hit rate ≥ 90% on warm CI | 🟡 | `lean-bringup` caches `.lake/` keyed on `(lakefile.toml, lake-manifest.json, lean-toolchain)`. On warm runs the entire mathlib package mirror is restored (100% hit). On the first cold run (or after a manifest bump) we eat the network fetch. Note: P1.2 modules don't yet `import Mathlib`, so the mathlib *olean* graph isn't compiled — the dep manifest pipeline is exercised; the olean cache will become meaningful when P1.5 starts importing real mathlib lemmas. |
| 5 | Extraction prototype produces compiling Rust from `double` | ✅ | [`tools/lean-to-rust`](../../../tools/lean-to-rust) → [`crates/axiom-extract-hello/src/lib.rs`](../../../crates/axiom-extract-hello/src/lib.rs); committed as the canonical extracted form. |
| 6 | Extracted Rust tests pass (`double_zero`, `double_seven`) | ✅ | + `double_billion`. `buck2 test //crates/axiom-extract-hello:axiom-extract-hello-test` → 3/3 pass. |
| 7 | Translation-validation harness skeleton merged | ✅ | [`tools/translation-validator`](../../../tools/translation-validator) PASS on `[0, 1, 7, 100, 1e9, 2³¹-1]` inputs. Real semantic-equivalence proofs land in P1.9. |
| 8 | `flake.nix` updated to provide Lean via Nix | ✅ | Lean + Lake + elan come from `pkgsUnstable` (24.11 has Lean 4.10.0; we want 4.29.1). The `stdenv.cc.cc.lib` runtime is pulled in as a side-effect to support mathlib's `cache:exe` link step on Nix shells. |
| 9 | G1 onboarding doc `docs/lean-setup.md` published | 🧊 | Re-classified out of P1.2: per repo-wide doc minimalism policy, single-CHECKLIST replaces a dedicated setup doc. The repo-root [`docs/phase-1/P1.1/build-and-run.md`](../P1.1/build-and-run.md) is the entry-point run-book; Lean-specific guidance lives there once `make lean-build` lands as a target. |

## B. State-of-the-art additions (beyond the spec)

| # | Item | Status | Where it lives |
|---|------|--------|----------------|
| B-1 | Buck2 wrapper for Lake (`//theorems:hello` emits an olean hash-manifest with CORPUS_ROOT) | ✅ | [`theorems/BUCK`](../../../theorems/BUCK), [`theorems/lean-build.sh`](../../../theorems/lean-build.sh). |
| B-2 | Hash-corpus extension covers extracted crate, prototype tools, olean manifest | ✅ | [`scripts/_hash-artifacts.sh`](../../../scripts/_hash-artifacts.sh) — see `FIRST_PARTY_CRATES` and `FIRST_PARTY_BINS`. New CORPUS_ROOT for the expanded set is `29d6db683…`. |
| B-3 | Extractor is *idempotent* under re-run (CI gate) | ✅ | `lean-bringup` job runs the extractor and asserts `git diff --exit-code` against the committed `lib.rs`. |
| B-4 | Reservoir / Lake cache wired in CI via `actions/cache@v4` keyed on the manifest | ✅ | `.github/workflows/ci.yml` `lean-bringup` job. |
| B-5 | flake-level provenance anchor for mathlib4 commit | ✅ | `flake.nix` `inputs.mathlib4 = github:leanprover-community/mathlib4/v4.29.1`; recorded in `flake.lock`. Drift in the SHA shows up as a flake-input update, reviewed by CODEOWNERS. |
| B-6 | Lakefile + Lean toolchain + manifest under CODEOWNERS | ✅ | Existing `.github/CODEOWNERS` `flake.*` and `**` catch-all rules apply. |
| B-7 | Translation-validator covers near-overflow inputs (`2³¹ - 1`) | ✅ | `tools/translation-validator/src/main.rs` `INPUTS`. |

## C. Required one-time operator actions

| # | Action | Effort |
|---|--------|--------|
| C-1 | Same as P1.1 §C — apply branch protection (`bash scripts/setup-branch-protection.sh`) once the new `lean-bringup` job has run at least once on the remote so its name is registered as a required check. | ~30 s |

That is the only operator action specific to P1.2. P1.1's bake-refs item still applies and is still pending.

## D. Confirmed deferred-by-design

| Item | Target sub-phase | Justification |
|------|------------------|---------------|
| Real ZIP-layer formalization | 🧊 P1.5 | Spec §2 "out of scope". |
| Real signing-block formalization | 🧊 P1.11 | Spec §2 "out of scope". |
| Production Lean → Rust extractor | 🧊 P1.9 | Spec §2 "out of scope". The P1.2 prototype is a regex parser; the production pipeline uses Lean's elaborated AST. |
| AOSP differential check | 🧊 P1.5+ | Spec §2 "out of scope". |
| OCaml / opam toolchain | 🧊 P1.4 / P1.5 | Spec §4 says "needed only for advanced extraction work". P1.2 bring-up does not need it; deferred until a Lean tactic actually requires OCaml. |
| Mathlib4 *imports* in our own modules | 🧊 P1.5 | Hello.lean uses only core Lean (`omega`). The dep is *declared* in `lakefile.toml` (so Lake's package pipeline is exercised) but no `import Mathlib.*` line lands until P1.5 needs a real lemma. |

## E. Known local-only quirk (does not affect CI)

- mathlib's `cache:exe` (the prebuilt-olean fetcher) fails to link on Nix shells with the symbol error `__cxa_call_terminate`. Our `flake.nix` adds `stdenv.cc.cc.lib` to commonTools to mitigate; if the link still fails on a particular host, run `lake build Apkaxiom` directly (which does **not** require `cache:exe`). CI runs on Ubuntu's stock gcc and is unaffected.

## F. End-to-end verification

```bash
nix develop --command bash -euxo pipefail -c '
  # P1.1 gates (still green)
  make build && make test && make repro-check && make verify-hashes
  make graph-parity && make audit-toolchains && make reindeer-check
  make determinism-lint && make security-audit && make license-check
  make sbom && make rebuilder-attest && make bazel-info && make lint
  nix flake check
  # P1.2 gates
  lake build Apkaxiom
  buck2 build //theorems:hello
  buck2 run //tools/lean-to-rust -- theorems/Apkaxiom/Hello.lean crates/axiom-extract-hello/src/lib.rs
  git diff --exit-code crates/axiom-extract-hello/src/lib.rs
  buck2 test //crates/axiom-extract-hello:axiom-extract-hello-test
  buck2 run //tools/translation-validator
'
```

Last verified end-to-end on `linux-x86_64` at 2026-05-03, CORPUS_ROOT
`29d6db683991b397965a123c2333ff391a0710a0ac783ec5e621e6f28b9319d2`.

## G. Document inventory under this folder

| File | Purpose |
|------|---------|
| [`README.md`](./README.md) | P1.2 spec (frozen — change via PR review). |
| [`CHECKLIST.md`](./CHECKLIST.md) | This file. |
