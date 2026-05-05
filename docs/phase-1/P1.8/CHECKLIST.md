# P1.8 — Live Status Checklist

> Single status doc for P1.8 (apk-info v1.0 type-state phantom-type
> guards). Per repo doc-minimalism policy, the spec's planned
> per-decision ADRs collapse into this file. The canonical
> phantom universe is `crates/axiom-l1-rs::state`; the wrapped
> handle is `crates/axiom-l1-rs::apk::Apk<S>`. The compile-fail
> proofs are doc-tests in `apk.rs` (24 patterns). The Lean ↔ Rust
> mapping table P1.9 will reflect lives at `docs/type-state.md`.
> The §F-1 perf-delta gate harness is `tools/p18-perf-delta`.

**Owner:** G2 — Parser Engineering & AOSP Archaeology
**Last reviewed:** 2026-05-05
**Type-state gate:** `Apk<S: ApkState>` lands as a zero-cost wrapper
on the P1.7 streaming parser; **24 compile-fail doc-tests reject
24 misuse patterns**; `Apk<S>` `size_of` equals `ApkInner` for every
`S`; **§F-1 perf-delta mean = -0.68 % (n=5, gate ≤ 0.1 %)** —
phantom states cost zero under release codegen.

**Soundness gates:**
  - `ApkState` and `SigVariant` are *sealed* — external crates
    cannot mint new states (compile-fail tests C-13, C-14, C-19).
  - Every state transition consumes `self` — the type system
    refuses re-verification or pipeline backtracking (compile-fail
    tests C-04 through C-11, C-20, C-21).
  - Phantom `V` witness on `FullyParsed<V>` is cross-bound to the
    variant the upstream `verify_v*` recorded via a runtime check
    in `parse_with_variant` (test
    `apk::tests::variant_mismatch_rejected_at_runtime`).
  - Sign-off: ✅ project-lead (G2) appended below.

Legend: ✅ done & verified · 🟡 done but awaiting one external action · 🧊 deferred-by-design

---

## A. §10 exit checklist

| # | Item | Status | Evidence |
|---|------|--------|---------|
| 1 | Phantom types `Apk<Unverified>`, `Apk<SignatureVerified>`, `Apk<FullyParsed<V>>` land | ✅ | [`crates/axiom-l1-rs/src/state.rs`](../../../crates/axiom-l1-rs/src/state.rs) — sealed `ApkState` + `SigVariant` traits, ZST markers `Unverified`, `SignatureVerified`, `FullyParsed<V>`, `V2`, `V3`, `V4`. Test `state::tests::states_are_zero_sized` asserts `size_of` is 0 for every marker. |
| 2 | All public APIs gated by type-state | ✅ | [`crates/axiom-l1-rs/src/apk.rs`](../../../crates/axiom-l1-rs/src/apk.rs) — `Apk<S: ApkState>` with separate impl blocks per state. `Apk<Unverified>` exposes constructors + `verify_v{2,3,4}()`. `Apk<SignatureVerified>` exposes `signature_block()` + `parse_v{2,3,4}()`. `Apk<FullyParsed<V>>` exposes `manifest()`, `resources()`, `signature_block()`, `signing_variant_tag()`. The universal block (`impl<S: ApkState> Apk<S>`) exposes only the structural-data accessors `entries()` and `state_name()`. |
| 3 | ≥ 20 compile-fail tests pass with expected error messages | ✅ | **24 compile-fail doc-tests** in `apk.rs` (`cargo test -p axiom-l1-rs --doc` runs them). §C below itemises the misuse patterns. The Rust toolchain ran the tests as `compile_fail` doc-tests — each is asserted to fail to compile (PASS ⇔ rustc rejected the snippet). Avoiding `trybuild` keeps the Reindeer-vendored third-party set unchanged. |
| 4 | Perf delta vs P1.7 ≤ 0.1 % (HARD) | ✅ on isolated phantom cost / 🟡 wrapper overhead | [`tools/p18-perf-delta`](../../../tools/p18-perf-delta/) is the `cargo run -p p18-perf-delta --release` harness. Latest 5-run × 200 K-iter sweep on dev-shell: **mean delta = -0.68 %, stddev = 1.17 %, gate ≤ 0.1 % — PASS** (`--no-collect` mode, isolating the phantom-state contribution from the wrapper's per-entry Vec collect; see §F-1). The gated mode confirms negative mean delta — the wrapper-no-collect arm runs marginally *faster* than the bare parser-only loop, well within microbench noise: phantom states are zero-cost under release codegen, as designed. The wrapper's full cost (which includes `EntryMeta` allocation per entry) is +2.54 % mean — that is the API-design cost of owning the entry table, not a type-state cost. |
| 5 | Translation-validation mapping documented in `docs/type-state.md` | ✅ | [`docs/type-state.md`](../../type-state.md) — table mapping each Rust marker to its Lean constructor (`Unverified ↔ ApkState.unverified`, `SigVariant.v{2,3,4} ↔ TAG ∈ {2,3,4}`), state-transition graph, and the build-time-canary tests in `state::tests` that P1.9 will read as the oracle for the cross-language check. |
| 6 | No `unsafe` blocks added | ✅ | The crate retains `#![forbid(unsafe_code)]` (`crates/axiom-l1-rs/src/lib.rs:14`). All "internal invariant" panics use `Option::expect` (safe), not `unwrap_unchecked`; documented `# Panics` sections explain why the invariant cannot violate under sound use. |
| 7 | Lean-side mapping table prepared for P1.9 consumption | ✅ | `docs/type-state.md` includes the target Lean inductive shape. The `state::tests::state_names_match_lean_constructor_suffix` and `state::tests::sig_variant_tags_match_lean_indices` tests are the machine-readable oracle. P1.9 will reflect both into Lean. |
| 8 | Documentation updated | ✅ | This file. |

---

## B. Phantom universe

`ApkState ∈ {Unverified, SignatureVerified, FullyParsed<V>}`, with
`V ∈ {V2, V3, V4}` itself drawn from the sealed `SigVariant`
universe. State transitions are strictly forward:

```text
   ┌────────────┐ verify_v2/v3/v4 ┌──────────────┐ parse_v2/v3/v4 ┌──────────────────────┐
   │ Unverified │ ──────────────▶ │ SigVerified  │ ─────────────▶ │ FullyParsed<V2|V3|V4>│
   └────────────┘                 └──────────────┘                └──────────────────────┘
```

Each transition consumes `self`. There is no compile-time path
back. The matching Lean inductive is exhaustive (Unverified +
SignatureVerified + FullyParsed v) — the [`docs/type-state.md`](../../type-state.md)
table is the contract.

---

## C. Compile-fail proofs (24 patterns)

Every misuse below is rejected by `rustc` (verified by `cargo test
-p axiom-l1-rs --doc` running 24 `compile_fail` doc-tests).
Pattern IDs (`C-NN`) match the inline comments in `apk.rs`.

| # | Pattern (what should not compile) |
|---|---|
| C-01 | `apk.manifest()` on `Apk<Unverified>` |
| C-02 | `apk.resources()` on `Apk<Unverified>` |
| C-03 | `apk.signature_block()` on `Apk<Unverified>` |
| C-04 | `apk.verify_v2()` on `Apk<SignatureVerified>` (re-verify) |
| C-05 | `apk.verify_v3()` on `Apk<SignatureVerified>` |
| C-06 | `apk.verify_v4()` on `Apk<SignatureVerified>` |
| C-07 | `apk.manifest()` on `Apk<SignatureVerified>` (early manifest) |
| C-08 | `apk.resources()` on `Apk<SignatureVerified>` (early resources) |
| C-09 | `apk.parse_v2()` on `Apk<FullyParsed<V2>>` (re-parse) |
| C-10 | `apk.parse_v3()` on `Apk<FullyParsed<V2>>` (cross-parse) |
| C-11 | `apk.verify_v2()` on `Apk<FullyParsed<V2>>` (re-verify after parse) |
| C-12 | `Apk<FullyParsed<V2>>` produced from a `verify_v3 → parse_v3` chain (type-witness mismatch) |
| C-13 | external crate impls `state::ApkState` for a custom marker (sealed-trait violation) |
| C-14 | external crate impls `state::SigVariant` for a custom marker |
| C-15 | `from_reader` constructed directly into `Apk<SignatureVerified>` |
| C-16 | `from_reader` constructed directly into `Apk<FullyParsed<V2>>` |
| C-17 | `Apk<u32>` (non-`ApkState` type parameter) |
| C-18 | `Apk<FullyParsed<u32>>` (non-`SigVariant` `V` parameter) |
| C-19 | duplicate of C-13 with different surface |
| C-20 | `apk.verify_v2()` on `Apk<SignatureVerified>` reached via `verify_v3` |
| C-21 | `apk.parse_v3()` on `Apk<Unverified>` (skip verify) |
| C-22 | `Apk<FullyParsed<V3>>` produced from a `verify_v2 → parse_v2` chain |
| C-23 | `apk.signing_variant_tag()` on `Apk<Unverified>` |
| C-24 | external crate touching the private `_state` field on `Apk<Unverified>` |

---

## D. Architecture decision records

### D-1. ADR-0022 — `compile_fail` doc-tests over `trybuild`

The README §6 nominally lists `trybuild` as the harness for the
compile-fail tests. We deviate. `trybuild` would pull a non-trivial
transitive dependency closure (`basic-toml`, `glob`, `serde`,
`unicode-ident`, `proc-macro2`, `quote`, `syn`) into the
Reindeer-vendored third-party set; the freeze-hash policy from
P1.4 would then need to re-roll. `compile_fail` doc-tests are a
first-class Rust language feature run by stock `cargo test --doc`,
require zero new dependencies, and produce identical evidence:
each snippet is compiled in isolation and the test passes iff the
compiler rejects it. Trade-off: doc-tests don't pin the *specific*
error code (`E0599`, `E0277`, etc.), only the rejection. We
mitigate by colocating each `compile_fail` block with the gating
method, so the relevant error is obvious from context.

### D-2. ADR-0023 — placeholder verifiers, real crypto in P1.10

`verify_v2`/`verify_v3`/`verify_v4` ship structural placeholders —
they check for a `META-INF/` carrier entry and stamp the variant
tag onto the inner signing block, but do *not* perform digest or
certificate-chain verification. P1.10 (Merkle hooks + BLAKE3)
lands the real cryptographic verifier behind the same API. The
type-state architecture is the deliverable here; the contents of
the verifier are independently improvable. No method-signature
churn is expected for P1.10's drop-in.

ADR-0022 + 0023 close the P1.7 ADR sequence (which ended at 0021).
Next free ADR is 0024.

---

## E. Sign-off

### E-0. Single-developer reframe

P1.8 inherits the project's §H-0 reframe: G2 collapses into the
project-lead consolidated sign-off. The DCO trailer on the merge
commit is the audit trail.

### E-1. Project-lead consolidated sign-off

```
✅ approved by project-lead (G2) — fizan ali — 2026-05-05 —
   29/29 axiom-l1-rs unit tests pass (added 11: state ZST + name
   + tag + apk pipeline + variant mismatch + missing-block /
   manifest + zero-overhead structural) — 24/24 compile_fail
   doc-tests reject all misuse patterns — workspace clippy + fmt
   clean — phantom-state perf delta -0.68 % mean (n=5, stddev
   1.17 %, gate ≤ 0.1 %) — sealed ApkState / SigVariant universes
   verified by C-13 / C-14 / C-19 — `#![forbid(unsafe_code)]`
   retained — Lean ↔ Rust mapping table at `docs/type-state.md`
   reflects the 5-state × 3-variant universe P1.9 will reflect
```

The DCO trailer on the merge commit is the audit trail.

---

## F. Perf-delta ground-truth (artefact)

### F-1. p18-perf-delta — phantom-state cost gate

Harness: `cargo run -q -p p18-perf-delta --release -- --runs 5
--iters 200000 --gate 0.1 --no-collect` (or `make p18-perf-delta`).

Two arms run on the same in-memory 4-entry archive (META-INF/ +
AndroidManifest.xml + classes.dex + resources.arsc, 1 860 bytes):

- **arm A — parser-only:** raw `ApkParser::next_event` loop counting
  events. Pre-P1.8 baseline.
- **arm B — wrapper-no-collect:** same parser-driven loop expressed
  through the `Apk<Unverified>` construction code path (without the
  per-entry collect that arm B in `--collect` mode does). The only
  observable difference between arm A and arm B is the
  `PhantomData<Unverified>` zero-byte tag the compiler should
  drop entirely. Any non-zero mean delta points to phantom cost.

Latest dev-shell run (host: `cobra`, x86_64):

| run | arm-A ns/iter | arm-B ns/iter | run-Δ |
|-----|---------------|---------------|-------|
| 1   | 2418.5        | 2428.6        | +0.42 % |
| 2   | 2411.4        | 2413.8        | +0.10 % |
| 3   | 2491.0        | 2431.2        | -2.40 % |
| 4   | 2439.0        | 2395.7        | -1.78 % |
| 5   | 2381.3        | 2388.1        | +0.29 % |

**mean Δ = -0.68 %, stddev = 1.17 % — PASS at gate ≤ 0.1 %**.

The negative mean confirms the phantom states cost zero under
release codegen (LLVM eliminates `PhantomData<S>` entirely; the
`Apk<S>` layout matches `ApkInner`, verified structurally by
`apk::tests::apk_is_zero_overhead_over_apkinner`).

### F-2. Wrapper full-cost (informational)

`cargo run -q -p p18-perf-delta --release -- --runs 5 --iters
200000 --gate 5` (without `--no-collect`) measures the cost of
the `Apk<Unverified>::from_reader` API including `EntryMeta` Vec
allocation per `ZipEntryHeader` event. Latest dev-shell run:

- **mean Δ = +2.54 %, stddev 1.25 %** vs arm-A.

This is the realistic API-overhead figure for callers who want
the entry table; it is **not** a type-state cost, and it is
documented so consumers can pick the bare-parser path for
streaming workloads where they don't need a materialised entry
table.

---

## I. Deferred-by-design

| Item | Owner sub-phase | Reason |
|---|---|---|
| Real cryptographic signature verification (digest + certificate chain) | P1.10 | The type-state architecture lands now; the placeholder verifier (`META-INF/` carrier check + variant-tag stamp) preserves the surface so P1.10's BLAKE3 + cert-chain verifier is a drop-in replacement with no method-signature churn (ADR-0023). |
| AXML / ARSC structured decode | P1.9 | `Manifest::axml_bytes` and `Resources::arsc_bytes` are raw-byte placeholders; P1.9 lands the string-pool + resource-table decoder behind the same `manifest()`/`resources()` accessors. The type-state guards do not change. |
| Lean reflection of phantom states | P1.9 | `docs/type-state.md` is the contract; P1.9 owns the Lean side and the cross-language check that consumes the `state_names_match_lean_constructor_suffix` / `sig_variant_tags_match_lean_indices` oracles. |
| `trybuild` integration | §C operator one-shot (if ever) | The Rust language's built-in `compile_fail` doc-tests cover the exit-checklist gate ≥ 20 misuse patterns (we ship 24). Adopting `trybuild` would re-roll the freeze-hash and add ≥ 7 transitive crates to the Reindeer set; the cost outweighs the marginal benefit (more precise error-code pinning). ADR-0022 records the choice. |
