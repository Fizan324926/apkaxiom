# ADR-0002 — Buck2 as the primary build system; Reindeer for Rust third-party

**Status:** Accepted (P1.1)
**Date:** 2026-05-02
**Owner:** G13 — Platform Infrastructure
**Supersedes:** none
**Related:** ADR-0004 (Nix flake)

---

## Context

APKAXIOM is a multi-language project (Rust core, Lean kernel, eventual Java/JNI/native blobs from AOSP, Python tooling, Bazel-built AOSP harnesses). Phase 1 needs a build system with five non-negotiable properties:

1. **Hermeticity.** Two clean builds on the same host must produce byte-identical outputs. A clean build on a *different* host of the same platform must produce identical outputs.
2. **Polyglot.** Rust, Java, native (C/C++), Python, Erlang/Haskell/Go for AOSP harness leaks — all in one graph.
3. **Remote build execution (RBE) ready.** We will exceed a single-machine build budget by P1.18.
4. **Bytecode-stable Cargo bridge.** Rust ecosystem expects `Cargo.toml`; we cannot rewrite every third-party dep.
5. **Tractable to operate.** Open-source tooling, no proprietary cloud lock-in.

## Decision

We adopt **Buck2** (Meta's open-source successor to Buck1) as the **primary** build system for the entire repository, with **Reindeer** as the Cargo→Buck2 bridge for third-party Rust crates.

**Bazel** is confined to the `external/aosp/` sub-workspace because AOSP's Soong build system natively emits Bazel rules; rewriting them would be a perpetual chore (see ADR-future for the full AOSP-on-Bazel rationale).

### Concretely

- The repo root is a Buck2 cell; every first-party crate (`crates/axiom-l0`, etc.) has a hand-written `BUCK` file.
- The Buck2 *prelude* is consumed via the **bundled** mechanism (`external_cells.prelude = bundled` in `.buckconfig`). The prelude is therefore pinned transitively by the Buck2 binary, which is in turn pinned by `flake.nix` / `flake.lock`. No git submodule needed.
- Toolchains are registered in `toolchains/BUCK` using `system_*_toolchain` macros from the bundled prelude. We register only what we use (rust, cxx, genrule, python_bootstrap, test); we do not call `system_demo_toolchains()` because it pulls in Java/Erlang/Haskell/OCaml/Go/Android toolchains that would clutter `buck2 audit`.
- Reindeer manages the third-party Rust graph at `third-party/rust/`: `Cargo.toml` declares deps, `vendor/` holds checked-in crate sources for hermeticity, `BUCK` is `@generated`. Regenerating is `make third-party`.
- The Buck2 entry point is `//:all` (an explicit alias). We do **not** rely on `buck2 build //...` because directly building proc-macro buildscript binaries trips a known prelude corner case in some Buck2 versions; the alias gives us an explicit, version-stable entry.

## Why Buck2 over Bazel

| Property | Buck2 | Bazel |
|---|---|---|
| Hermeticity by default | Yes (sandboxed-by-default; disable explicitly) | Yes (with care) |
| Rust integration | First-class via `prelude//rust`; Reindeer is a known-good bridge | `rules_rust` works but Cargo bridge is bumpier |
| Configuration language | Starlark, same as Bazel | Starlark |
| Performance | Faster cold + warm graphs in our benchmarks (Meta's data + our pilot) | Strong but slower on graph re-evaluation |
| RBE | Native (BuildBuddy etc.) | Native (Buildfarm etc.) |
| Maturity | OSS since 2023; actively developed at Meta | OSS since 2015; very mature |
| Operational footprint | Smaller daemon; simpler config | More moving parts |

The maturity argument cuts the other way (Bazel is more mature). We accept that risk because:

1. Phase 1's Rust + small-Python footprint is well within Buck2's strong-suit. Lean integration arrives in P1.2 and Buck2 supports it via custom rules — Bazel's Lean support is approximately equivalent (both via custom rules).
2. AOSP forces us to run Bazel in the sub-workspace anyway. Running Bazel as the *primary* would mean fighting `rules_rust` at scale; running Buck2 as primary lets us treat AOSP as a self-contained sub-tree.
3. Meta runs Buck2 at a larger scale than we will; it is unlikely to be the long-pole performance constraint.

## Why Reindeer over alternatives

Alternatives considered:
- **`rules_rust`'s `crate_universe`** — Bazel-only.
- **`cargo-raze`** — deprecated.
- **Hand-written Buck targets per crate** — unmaintainable beyond a dozen deps.

Reindeer is the only mature Cargo→Buck2 bridge today, is maintained by Meta (who use it internally), and produces vendored, hermetic output.

The Reindeer-generated `third-party/rust/BUCK` references vendored sources at `third-party/rust/vendor/<crate>-<version>/`. The vendor tree is **committed to git** so the build does not depend on `crates.io` availability — a prerequisite for the "hermetic" property.

## Trade-offs and known limitations

- **Buck2 daemon state can desync.** A failing `buck2 build //...` is sometimes recovered by `buck2 kill`. We mitigate by using explicit aliases (`//:all`) for entry points.
- **Reindeer fixups are manual.** Crates with build scripts need a `fixups/<crate>/fixups.toml`. In Phase 1 we set `buildscript.run = true` for `thiserror`, `proc-macro2`, `quote`. As the dep graph grows we will need more nuanced fixups (some deps require `omit_targets`, `extra_rustc_flags`, etc.).
- **Buck2's bundled prelude can shift API across versions.** `rust_test.bzl` was once load-able; in current versions `rust_test` is a built-in. We pin Buck2 by version to insulate against this.
- **Cross-arch byte identity is impossible for compiled artifacts.** P1.1's exit checklist asked for `diff hashes-ubuntu-24.04.txt hashes-ubuntu-24.04-arm.txt` — that diff cannot succeed for any compiled output. We verify *per-platform* identity instead (`docs/reproducibility-hashes.<platform>.txt`); see ADR-0004.

## Consequences

- The build graph is the source of truth. `Cargo.toml` exists for IDE / fast-iteration, but `buck2 build //:all` is the canonical "did the build pass" signal.
- New first-party crates require a hand-written `BUCK` file. We accept this as a forcing function: every crate gets a deliberate review of its dependency graph.
- New third-party deps require a `make third-party` round-trip plus a fixup file if the crate has a build script.
- Buck2 daemon version skew within a CI run is impossible because `nix develop` resolves Buck2 from the flake.

## References

- Buck2 docs: https://buck2.build/docs/
- Reindeer: https://github.com/facebookincubator/reindeer
- Bundled prelude design: https://buck2.build/docs/concepts/cell/
- AOSP / Bazel: ADR-future (P1.13)
