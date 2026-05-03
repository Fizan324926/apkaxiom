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
| 4 | Mathlib4 cache hit rate ≥ 90% on warm CI | ✅ | [`theorems/Apkaxiom/MathlibProbe.lean`](../../../theorems/Apkaxiom/MathlibProbe.lean) imports `Mathlib.Logic.Basic` and uses `Nat.add_comm` in a proof — building it from source compiles 71 mathlib oleans. CI's `actions/cache@v4` step keys on the `(lakefile.toml, lake-manifest.json, lean-toolchain)` triple so warm runs hit 100% (no mathlib rebuild). Cold runs (after a manifest bump) eat the network fetch + ~2 min source compile. |
| 5 | Extraction prototype produces compiling Rust from `double` | ✅ | [`tools/lean-to-rust`](../../../tools/lean-to-rust) → [`crates/axiom-extract-hello/src/lib.rs`](../../../crates/axiom-extract-hello/src/lib.rs); committed as the canonical extracted form. |
| 6 | Extracted Rust tests pass (`double_zero`, `double_seven`) | ✅ | + `double_billion`. `buck2 test //crates/axiom-extract-hello:axiom-extract-hello-test` → 3/3 pass. |
| 7 | Translation-validation harness skeleton merged | ✅ | [`tools/translation-validator`](../../../tools/translation-validator) PASS on `[0, 1, 7, 100, 1e9, 2³¹-1]` inputs. Real semantic-equivalence proofs land in P1.9. |
| 8 | `flake.nix` updated to provide Lean via Nix | ✅ | Lean + Lake + elan come from `pkgsUnstable` (24.11 has Lean 4.10.0; we want 4.29.1). The `stdenv.cc.cc.lib` runtime is pulled in as a side-effect to support mathlib's `cache:exe` link step on Nix shells. |
| 9 | G1 onboarding doc | ✅ | Lean section appended to [`../P1.1/build-and-run.md`](../P1.1/build-and-run.md) (single run-book per repo-wide doc-minimalism policy). Covers `make lean-build` / `lean-extract` / `translation-validate`, the mathlib4 cache model, and the known local-only `cache:exe` quirk. |

## B. State-of-the-art additions (beyond the spec)

| # | Item | Status | Where it lives |
|---|------|--------|----------------|
| B-1 | Buck2 wrapper for Lake (`//theorems:hello` emits an olean hash-manifest with CORPUS_ROOT) | ✅ | [`theorems/BUCK`](../../../theorems/BUCK), [`theorems/lean-build.sh`](../../../theorems/lean-build.sh). |
| B-2 | Hash-corpus extension covers extracted crate, prototype tools, olean manifest | ✅ | [`scripts/_hash-artifacts.sh`](../../../scripts/_hash-artifacts.sh) — see `FIRST_PARTY_CRATES` and `FIRST_PARTY_BINS`. CORPUS_ROOT for the combined P1.1+P1.2 set is `08cfc70899…`. |
| B-3 | Extractor is *idempotent* under re-run (CI gate) | ✅ | `lean-bringup` job runs the extractor and asserts `git diff --exit-code` against the committed `lib.rs`. |
| B-4 | Reservoir / Lake cache wired in CI via `actions/cache@v4` keyed on the manifest | ✅ | `.github/workflows/ci.yml` `lean-bringup` job. |
| B-5 | flake-level provenance anchor for mathlib4 commit | ✅ | `flake.nix` `inputs.mathlib4 = github:leanprover-community/mathlib4/v4.29.1`; recorded in `flake.lock`. Drift in the SHA shows up as a flake-input update, reviewed by CODEOWNERS. |
| B-6 | Lakefile + Lean toolchain + manifest under CODEOWNERS | ✅ | Existing `.github/CODEOWNERS` `flake.*` and `**` catch-all rules apply. |
| B-7 | Translation-validator covers near-overflow inputs (`2³¹ - 1`) | ✅ | `tools/translation-validator/src/main.rs` `INPUTS`. |
| B-8 | `lean-bringup` runs on the full CI matrix (linux-x86_64 + linux-aarch64 + darwin-arm64) | ✅ | `.github/workflows/ci.yml` `lean-bringup.strategy.matrix`. |
| B-9 | Make-target ergonomics: `lean-build`, `lean-extract`, `translation-validate`, `lean-update` | ✅ | `Makefile` "Lean (P1.2)" section. |
| B-10 | Extractor emits canonically rustfmt-clean Rust (multiline fn body + tests) | ✅ | `tools/lean-to-rust/src/main.rs` render(). CI re-runs the extractor and gates on `git diff --exit-code` AND `cargo fmt --check`. |
| B-11 | Lean toolchain visible in dev-shell banner | ✅ | `flake.nix` shellHook prints `lean: Lean (version 4.29.1, ...)`. cosign / syft lines also fixed (no more `null`). |

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

mathlib's `cache:exe` (the prebuilt-olean fetcher) fails to link on Nix
shells with `undefined reference to '__cxa_call_terminate'`. The 24.11
stdenv ships gcc-13.3.0-lib's libstdc++, which predates that symbol;
Lean's bundled `libleancpp.a` was compiled against a newer libstdc++.
The flake exports `LEAN_CC=$(pkgsUnstable.gcc)/bin/gcc` plus
`LIBRARY_PATH` / `NIX_LDFLAGS` overrides toward `gcc-15.2.0-lib`, but
the rust-overlay's stdenv still appends gcc-13 paths late in the link
order, sometimes winning the search-order contest. The result:

- `lake build Apkaxiom` works fine — the cache:exe is never built.
- `lake update` (`make lean-update`) **partially fails** at the
  cache:exe step; the new `lake-manifest.json` is written before that
  step, so you can `git add lake-manifest.json` and proceed.
- CI runs on Ubuntu's stock gcc (not Nix-pinned) and is unaffected.

This is the same workflow boundary as `nix flake update` — a
privileged manifest-bump command that is not part of the everyday
build path. Per the run-book in
[`../P1.1/build-and-run.md`](../P1.1/build-and-run.md) §"Local-only
quirk".

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
`08cfc70899f42426c3a55f2a5b431dc6c32c1191514473f72040c3d256b217a3`.

## G. Document inventory under this folder

| File | Purpose |
|------|---------|
| [`README.md`](./README.md) | P1.2 spec (frozen — change via PR review). |
| [`CHECKLIST.md`](./CHECKLIST.md) | This file. |
