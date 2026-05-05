# ADR-0027 — Lake-Buck2 deep integration deferred to research-track work

**Status:** Accepted as a deliberate, documented deferral.
**Sub-phase:** P1.9 §IV gap 15.
**Date:** 2026-05-05.

## Context

P1.1 made Buck2 the canonical hermetic build system for the
Rust + AOSP-runtime side of APKAXIOM. The Lean side uses
[Lake](https://lean-lang.org/lake/), Lean's own build system,
which has its own dependency graph (mathlib, batteries, aesop,
proofwidgets, importGraph, …).

The natural question for P1.9: can Buck2 own the **entire** build
graph, including Lean compilation, so the trust-boundary
translation-validation receipt is producible by `buck2 build`
alone (without invoking `lake` as a side process)?

## What was attempted

P1.9 §I shipped:

  - `theorems/lean-build.sh` — a Buck2 `genrule` that invokes
    `lake build Apkaxiom` and emits a sha256 manifest of every
    `.olean` produced.
  - This is intentionally **shallow integration**: Buck2 sees one
    opaque `genrule` whose output is a manifest; it cannot
    actually build any individual Lean module without going
    through Lake.

For P1.9 §IV the desire was to deepen this — let Buck2 see the
Lean module DAG and rebuild incrementally per-module.

## Why this is research-scale and not in P1.9's scope

A real Buck2-native Lean integration requires:

1. **Lake → BUCK transcription**. Each `.lean` file's transitive
   import graph would need to be projected into Buck2 rules
   (analogous to Reindeer's Cargo → BUCK transcription for Rust).
   Lake's source-of-truth is `lakefile.toml` plus `import` lines
   in every `.lean`; a transcriber would need to:
   - parse every `import` declaration in the source tree,
   - resolve the import paths against Lake's `packagesDir`,
   - emit a `lean_library` rule per file with explicit `deps`.

2. **Hermetic mathlib dependency.** Mathlib's transitive closure
   is ~5000 `.lean` files. Currently `lake build` pulls them via
   `lake-manifest.json` and the Lean Reservoir prebuilt-`.olean`
   cache. A Buck2-native build would need each mathlib `.olean`
   addressable in Buck2's RE cache, with content-hash inputs
   matching what Reservoir distributes — or to rebuild the entire
   tree under Buck2 (10+ minutes cold, even with parallelism).

3. **Custom Lean compiler driver in Buck2 toolchain.** Lake's
   `lean` invocation passes `--root`, `--setup`, `LEAN_PATH`, and
   per-target compile flags that are non-trivial to replicate via
   `genrule`. A real integration needs a `lean_library` /
   `lean_binary` Buck2 rule type backed by a custom toolchain.

4. **Cross-platform reproducibility.** Lean compiles to native
   via `leanc` which wraps `clang`/`gcc`. Bit-reproducibility
   across hosts requires pinning libc, linker, and the C toolchain
   inside the Buck2 dependency declaration — work the Rust side
   already has via `nixpkgs` but the Lean side does not.

The Lean community has prior art on this (`bazel-lean4`, the
`mathlib-lean4` Bazel rules, the lean-mwe Buck2 demos), all of
which are **explicitly research / WIP** and none of which are
production-ready. Adopting any of them requires either:

- Significant porting effort (≥ 4 person-weeks based on prior
  art's reported timelines), OR
- Building our own `lean_library` rule from scratch (≥ 8
  person-weeks).

Either path is **outside any single sub-phase's budget** and
inappropriate to land partially.

## What we ship instead

P1.9 §IV's `make tv-three-way` invokes Lake from the makefile.
The Buck2 hermeticity gate (`make p19-buck2`) builds the
Rust-side targets only:

  - `//crates/axiom-l0-zip-lfh-verified:axiom-l0-zip-lfh-verified`
  - `//crates/axiom-l0-zip-lfh-extracted:axiom-l0-zip-lfh-extracted`
  - `//tools/lfh-eval-rust:lfh-eval-rust`
  - `//tools/lfh-eval-extracted:lfh-eval-extracted`
  - `//tools/eocd-eval-rust:eocd-eval-rust`
  - `//tools/translation-validator:translation-validator`
  - `//tools/p19-perf-delta:p19-perf-delta`

The Lean side is built via Lake (`lake build lfh-eval eocd-eval`)
as a precondition. The validator's receipt records the Lake
manifest's SHA-256 (`lake-manifest.json`) so any change to the
Lean toolchain or its transitive dependencies invalidates the
committed receipt and forces re-validation.

## Decision

**Defer Buck2-native Lean integration to a dedicated
research-track sub-phase (tentatively P1.13+).** The deferral is
architecturally clean — the trust boundary (TV harness +
content-determined receipt) does not depend on which build
system runs Lake. Whether `lake build` is invoked by `make`,
`buck2 genrule`, or a future `lean_library` rule type, the
output `.olean`s feed the same evaluator binary, the same Rust
evaluator runs, the same diff fires, the same receipt gets
written.

This ADR explicitly does **not** say "we'll never integrate
Lake with Buck2." It says: doing it correctly requires its own
dedicated effort and shouldn't be rushed under P1.9's deadline.

## Re-evaluation triggers

We will reopen this ADR if any of:

  - Mathlib's transitive closure shrinks materially (unlikely)
  - The Lean community ships a production-grade Bazel/Buck2 rule
    set
  - The trust-boundary receipt's reproducibility becomes a CI
    blocker that Lake's own caching can't satisfy (e.g., we need
    cross-host `.olean` reproducibility for a release artefact)

## Author identity

`fizan ali <fizanali324926@gmail.com>` — project lead. Single-dev
sign-off per the §H-0 reframe.
