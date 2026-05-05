# Type-State Phantom Mapping (Rust ↔ Lean)

> P1.8 deliverable. The `Apk<S>` (sync) and `ApkAsync<S>` (async)
> phantom universes map 1-to-1 to the Lean inductive that P1.9's
> translation-validation pass will reflect. Adding a new state on
> either side without updating the other breaks P1.9's build.
>
> P1.8 §I revision (2026-05-05): the Rust side now carries a
> per-state `Data` payload via the `ApkState::Data` associated
> type. The Lean side will mirror this via constructor-indexed
> records when P1.9 reflects the inductive — see "Per-state
> payload" below.

The Rust side is `crates/axiom-l1-rs/src/state.rs`. The Lean side
will live in `lean/Axiom/L1/State.lean` (P1.9 owns the file). This
document is the contract between the two; **both** sides must
match the table below.

---

## State markers — `ApkState`

| Rust marker | Lean constructor | Phantom params | Reachable via | Methods unlocked |
|---|---|---|---|---|
| `Unverified` | `ApkState.unverified` | none | `Apk::<Unverified>::from_reader(_)` | `entries()`, `verify_v2()`, `verify_v3()`, `verify_v4()`, `state_name()` |
| `SignatureVerified` | `ApkState.sigVerified` | none | `Apk<Unverified>::verify_v{2,3,4}()` | `entries()`, `signature_block()`, `parse_v{2,3,4}()`, `state_name()` |
| `FullyParsed<V2>` | `ApkState.fullyParsed SigVariant.v2` | `V = V2` | `Apk<SignatureVerified>::parse_v2()` | `entries()`, `manifest()`, `resources()`, `signature_block()`, `signing_variant_tag()`, `state_name()` |
| `FullyParsed<V3>` | `ApkState.fullyParsed SigVariant.v3` | `V = V3` | `Apk<SignatureVerified>::parse_v3()` | identical to `FullyParsed<V2>` modulo the `V` witness |
| `FullyParsed<V4>` | `ApkState.fullyParsed SigVariant.v4` | `V = V4` | `Apk<SignatureVerified>::parse_v4()` | identical to `FullyParsed<V2>` modulo the `V` witness |

`ApkState` is **sealed**: external crates cannot add new state
markers (verified by compile-fail tests C-13/C-19 in `apk.rs`). The
universe is closed; Lean's inductive is exhaustive over the same
five constructors.

---

## Signing-block variants — `SigVariant`

| Rust marker | Lean constructor | `TAG` | Lean tag |
|---|---|---|---|
| `V2` | `SigVariant.v2` | 2 | 2 |
| `V3` | `SigVariant.v3` | 3 | 3 |
| `V4` | `SigVariant.v4` | 4 | 4 |

`SigVariant` is sealed — compile-fail test C-14 verifies external
crates cannot add a `V99`. The numeric `TAG` is checked at runtime
inside `parse_with_variant` to cross-bind the type witness on
`FullyParsed<V>` to the variant the upstream `verify_v*` recorded
(test `apk::tests::variant_mismatch_rejected_at_runtime`).

---

## Lean inductive (target shape, P1.9 will land it)

```lean
namespace Axiom.L1.State

inductive SigVariant where
  | v2
  | v3
  | v4

inductive ApkState where
  | unverified
  | sigVerified
  | fullyParsed (v : SigVariant)

end Axiom.L1.State
```

P1.9's translation-validation pass will check:

1. Each `ApkState::NAME` (Rust) matches the Lean constructor suffix
   (string "unverified" / "sig-verified" / "fully-parsed").
2. Each `SigVariant::TAG` (Rust) matches the Lean constructor
   index (`v2 = 2, v3 = 3, v4 = 4`).
3. Each gated method on `Apk<S>` corresponds to a guard predicate
   on the Lean inductive — e.g. `manifest()` is callable iff
   `state = fullyParsed _`.

The Rust-side machine-readable shape lives in `state.rs`'s
`#[test] state_names_match_lean_constructor_suffix` and
`sig_variant_tags_match_lean_indices`. P1.9 will read these as the
oracle for the cross-language check.

---

## State-transition graph

```text
                                                  ┌── Apk<FullyParsed<V2>>
                                                  │
   ┌────────────────────┐  verify_v2     ┌────────┼── parse_v2
   │  Apk<Unverified>   │ ──────────────▶│        │
   └────────────────────┘                │ Apk<   │── parse_v3 ─▶ Apk<FullyParsed<V3>>
            │                            │ Sig    │
            │   verify_v3                │ Verified
            └───────────────────────────▶│ >      │── parse_v4 ─▶ Apk<FullyParsed<V4>>
            │                            │        │
            │   verify_v4                │        │
            └───────────────────────────▶└────────┘
```

Edges consume `self` and return the next state, so the graph is
**strictly forward** — there is no compile-time path that revisits
a state. The compile-fail tests C-04, C-05, C-06, C-09, C-10, C-11,
C-20, C-21 collectively prove the no-revisit property over every
edge. C-22 plus the runtime `variant_mismatch_rejected_at_runtime`
test cover the case where the type-witness on `parse_v*` is allowed
to disagree with the upstream `verify_v*` (a possibility that the
type system can't reject, since both `parse_v2` and `parse_v3` are
in scope on `Apk<SignatureVerified>` — the runtime guard covers it).

---

## Per-state payload (`ApkState::Data`)

P1.8's §I revision moves runtime fields out of a one-size-fits-all
`ApkInner` and into per-state `S::Data` payloads. Each state
carries only the fields that *change* with the state; the
structural entry table lives on the outer `Apk<S>` struct shared
across every state.

| State | `S::Data` | Fields |
|---|---|---|
| `Unverified` | `UnverifiedData` | `captured: CapturedBodies` (3× `Option<Vec<u8>>` for META-INF carrier, AndroidManifest.xml, resources.arsc bytes captured during streaming) |
| `SignatureVerified` | `SignatureVerifiedData` | `manifest_bytes: Option<Vec<u8>>`, `resources_bytes: Option<Vec<u8>>`, `signature_block: SignatureBlock` |
| `FullyParsed<V>` | `FullyParsedData` | `signature_block: SignatureBlock`, `manifest: Manifest`, `resources: Resources` |

Lean side (target shape, P1.9 will reflect):

```lean
namespace Axiom.L1.State

structure CapturedBodies where
  signing_block : Option ByteArray
  manifest      : Option ByteArray
  resources     : Option ByteArray

structure UnverifiedData where
  captured : CapturedBodies

structure SignatureVerifiedData where
  manifest_bytes  : Option ByteArray
  resources_bytes : Option ByteArray
  sig_block       : SignatureBlock

structure FullyParsedData where
  sig_block : SignatureBlock
  manifest  : Manifest
  resources : Resources

end Axiom.L1.State
```

P1.9's TV pass will additionally check that each Rust struct's
field set matches the Lean structure's field set.

## Translation-validation contract (for P1.9)

If P1.9 adds a new constructor to either inductive, this table and
both `state.rs` and (eventually) `State.lean` must be updated in
the same commit. The `state_names_match_lean_constructor_suffix`
test is the build-time canary: it fails if the Rust marker's
`NAME` constant drifts.

The same is true for `SigVariant::TAG` — drift breaks the
`sig_variant_tags_match_lean_indices` test, which P1.9's CI will
run before the cross-language check.
