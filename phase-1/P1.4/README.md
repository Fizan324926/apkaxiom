# P1.4 — AXIOM-IR v0.1 Draft Spec (Manifest + Resource Dialects)

> Freeze the IR before anyone emits it. The spec is the contract; every layer above L1 reads from it.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../README.md §13.9 (AXIOM-IR)](../../README.md#beyond-the-12)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P1.4 |
| Owner(s) | G3 (AXIOM-IR & Bundle Resolver) |
| Duration | Weeks 2–6 |
| Critical-path | **yes** — every emitter and consumer awaits the freeze |
| Hard prerequisites | P1.1 (Buck2 + Rust), P1.2 (Lean type-system primitives for the Lean side of the spec) |

## 2. Goal & Scope

A frozen specification of AXIOM-IR v0.1 covering the **manifest dialect** and the **resource dialect**, with full type signatures, lowering rules between dialects, and a reference Rust implementation of the IR data structures. The spec is *frozen*: it cannot change in Phase 2 without an explicit ADR review.

AXIOM-IR is the universal currency between layers — apk-info / `axiom-l1-rs` emits it (P1.15), Layer 3 forensics consume it (Phase 2), Layer 4 symbolic resolver reasons over it (Phase 3). Get the type signatures wrong here and every downstream layer eats the cost.

### In scope
- AXIOM-IR core: SSA-form, dialect concept, type system, value/operation/region/block primitives.
- **Manifest dialect** — covers `AndroidManifest.xml` semantics: package, application, components, intent filters, permissions.
- **Resource dialect** — covers `resources.arsc`: types, configurations, resource references, string pools.
- Lowering rules (manifest ↔ resource references).
- Reference Rust implementation: `crates/axiom-ir` with `serde` round-trip support.
- Lean reflection of the IR types (so Lean theorems about IR-validity are well-typed).
- IR human-readable text format (analogous to MLIR's textual form).

### Out of scope
- DEX dialect (Phase 2).
- Native code (Phase 5).
- Resource encoding bit-perfectly (the dialect is *semantically* faithful, not byte-identical re-encoding — that's P1.15 round-trip).

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P1.1** | Buck2 build of Rust crates; Nix flake |
| **P1.2** | Lean type-system primitives (`Std`, basic mathlib) for IR-side theorems |
| **P1.3** | `axiom-l1-rs` spec — defines what fields the manifest emitter must surface (drives manifest-dialect type set) |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Rust** | 1.95+ | Reference IR implementation |
| **serde** | 1.0+ | IR serialization |
| **rkyv** | 0.7+ | Zero-copy archived IR (used by verifier for fast deserialization) |
| **bincode** | 2.x | Compact binary IR format |
| **Lean 4** | from P1.2 | IR type reflection in Lean |
| **graphviz** (`dot`) | 2.42+ | Type-system diagrams |
| **mermaid-cli** | latest | IR-flow diagrams in spec |
| **MLIR** (read-only reference) | LLVM 19+ | Design-pattern reference; we do **not** depend on MLIR runtime here |
| **Cap'n Proto** schema compiler | latest | Wire-format definitions for inter-process IR transmission |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL / Account | Notes |
|---|---|---|---|---|
| **MLIR documentation** | reference | **Free** OSS (Apache 2.0) | https://mlir.llvm.org | Inspiration only; we don't link MLIR libraries |
| **xDSL** (Python MLIR ecosystem) | reference | **Free** OSS | https://xdsl.dev | Useful for prototyping dialect ideas |
| **serde / rkyv / bincode** | crates | **Free** OSS | crates.io | Standard Rust ecosystem |
| **Cap'n Proto** | RPC + serialization | **Free** OSS (MIT) | https://capnproto.org | Wire format |
| **Apache Arrow** (read) | reference for columnar IR analysis | **Free** OSS | https://arrow.apache.org | Used in Phase 6 for corpus analysis; design considered now |
| **GitHub Discussions on the AXIOM-IR spec** | community feedback | **Free** | repo Discussions tab | Used to solicit feedback from Phase-2/3 group leads |

**No API keys, no paid services.** All dependencies are crates.io OSS or self-hosted reference docs.

## 6. System Inventory — Have vs Need

### Already present
- ✅ Rust 1.95 / cargo
- ✅ Lean / Lake (from P1.2)
- ✅ Buck2 / Bazel (from P1.1)
- ✅ git, gh

### Missing — must install
- ❌ **mermaid-cli** (`npm install -g @mermaid-js/mermaid-cli`)
- ❌ **graphviz** (already listed in P1.3; install if not done)
- ❌ **capnp** (Cap'n Proto compiler) — `sudo apt-get install -y capnproto`

### Install commands

```bash
# Mermaid CLI (for IR flow diagrams)
npm install -g @mermaid-js/mermaid-cli

# Cap'n Proto compiler
sudo apt-get install -y capnproto libcapnp-dev

# Add Rust crates to workspace (deferred until reading the spec)
# In crates/axiom-ir/Cargo.toml:
#   serde = { version = "1", features = ["derive"] }
#   rkyv = { version = "0.7", features = ["validation"] }
#   bincode = "2"
```

## 7. Working Directory & Files Produced

```
apkaxiom/
├── docs/
│   ├── AXIOM-IR-v0.1.md               # NEW — full spec (~80–120 pages)
│   ├── AXIOM-IR-text-format.md        # NEW — human-readable IR syntax
│   ├── AXIOM-IR-versioning.md         # NEW — how dialects evolve post-freeze
│   └── ADR-0006-axiom-ir-v0.1.md      # NEW
├── crates/
│   └── axiom-ir/
│       ├── Cargo.toml
│       ├── BUCK
│       └── src/
│           ├── lib.rs                  # crate root
│           ├── core.rs                 # value / operation / region / block
│           ├── manifest/               # manifest dialect types
│           │   ├── mod.rs
│           │   ├── components.rs
│           │   ├── intent_filter.rs
│           │   └── permission.rs
│           ├── resource/                # resource dialect types
│           │   ├── mod.rs
│           │   ├── string_pool.rs
│           │   └── config.rs
│           └── lowering.rs              # manifest ↔ resource lowerings
├── theorems/
│   └── Apkaxiom/
│       └── Ir.lean                     # NEW — Lean reflection of IR types
├── schema/
│   └── axiom_ir_v0_1.capnp              # NEW — Cap'n Proto schema for wire format
└── diagrams/
    ├── axiom-ir-types.dot
    ├── axiom-ir-types.svg
    ├── axiom-ir-flow.mmd
    └── axiom-ir-flow.svg
```

### Manifest dialect (excerpt)

```rust
// crates/axiom-ir/src/manifest/components.rs
#[derive(Debug, Clone, Serialize, Deserialize, Archive)]
pub enum Component {
    Activity {
        name: ComponentName,
        exported: Tribool,
        intent_filters: Vec<IntentFilter>,
        permission: Option<PermissionRef>,
        // ...
    },
    Service { /* ... */ },
    BroadcastReceiver { /* ... */ },
    ContentProvider { /* ... */ },
}

#[derive(Debug, Clone, Serialize, Deserialize, Archive)]
pub enum Tribool { True, False, Default }
```

## 8. Standalone Output

A frozen spec document and a compiling reference crate. Two artifacts:

1. **`docs/AXIOM-IR-v0.1.md`** — readable in isolation; cited by every Phase 2+ paper.
2. **`crates/axiom-ir`** — `cargo build && cargo test -p axiom-ir` passes; `serde` round-trip on 100 hand-written manifests succeeds.

## 9. End-to-End Test

```bash
buck2 test //crates/axiom-ir
# Includes:
#   - 100 hand-written manifest IR samples round-trip via serde + rkyv + bincode
#   - 50 hand-written resource IR samples round-trip
#   - manifest↔resource lowering preserves semantics on 30 samples
#   - Lean reflection module re-verifies (theorems/Apkaxiom/Ir.lean)
```

The 100 hand-written manifests cover edge cases — empty intent filters, deeply nested components, permission groups, resource references that resolve and that don't. They become regression fixtures for P1.15.

## 10. Exit Checklist

- [ ] `docs/AXIOM-IR-v0.1.md` ≥ 80 pages, complete spec
- [ ] Spec frozen — no changes for ≥ 4 weeks before P1.15 begins (HARD)
- [ ] Reviewer sign-off from G1, G2, G3, G4 leads
- [ ] `crates/axiom-ir` compiles under Buck2
- [ ] 100-sample serde + rkyv + bincode round-trip green
- [ ] Manifest↔resource lowering semantics-preserving on 30 samples
- [ ] Lean reflection module `Apkaxiom.Ir` re-verifies on CI
- [ ] Cap'n Proto schema compiles and round-trips
- [ ] ADR-0006 merged
- [ ] Mermaid + graphviz diagrams rendered and embedded in spec

## 11. Hand-Off

| Consumed by | What they need |
|---|---|
| **P1.5, P1.6** | Lean reflection of IR types — IR-side theorems are stated against this |
| **P1.15** | Reference crate `axiom-ir` to emit into; manifest-dialect type set |
| **Phase 2 / G4** | Forensic passes operate on `BehaviorSet<axiom_ir::*>` |
| **Phase 3 / G5** | Symbolic resolver lifts manifest-dialect to AXIOM-IR-symbolic |
| **Phase 4 / G7** | `.axc` certificate carries IR commitments |
