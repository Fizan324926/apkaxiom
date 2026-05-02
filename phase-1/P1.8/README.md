# P1.8 — apk-info v1.0 Type-State Phantom-Type Guards

> Move runtime checks to the type system. `Apk<Unverified>` cannot call `manifest()`. Misuse becomes a compile error.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../README.md §22](../../README.md#apkinfo-integration)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P1.8 |
| Owner(s) | G2 |
| Duration | Weeks 6–9 |
| Critical-path | yes for apk-info v1.0 chain |
| Hard prerequisites | P1.7 (streaming parser to wrap) |

## 2. Goal & Scope

Parser states encoded as **phantom types**: `Apk<Unverified>`, `Apk<SignatureVerified>`, `Apk<FullyParsed<V>>`. Calling `manifest()` on `Apk<Unverified>` is a compile-time error, not a runtime panic. Zero runtime overhead. The phantom states map 1:1 to Lean inductive constructor branches in P1.5/P1.6 — translation validation is straightforward.

### In scope
- `crates/axiom-l1-rs/src/state.rs` with phantom type definitions
- Public API refactor — every public method gates on type-state
- Compile-fail tests via `trybuild` — at least 20 misuse patterns
- Translation-validation hooks: phantom states ↔ Lean constructor branches

### Out of scope
- Lean reflection of phantom states (P1.9 handles the validator side)
- Cryptographic state transitions (require P1.10's BLAKE3 hooks)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P1.7** | Streaming parser to wrap with phantoms |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Rust** | 1.95 | Implementation |
| **trybuild** | 1.0+ | Compile-fail tests |
| **typestate** (crate, optional) | latest | Reference; we likely roll our own for tight control |
| **diagnostic-namespace** | nightly Rust feature *if needed* | Better error messages on misuse |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **trybuild** | crate | **Free** OSS | crates.io / dtolnay/trybuild | Standard for compile-fail tests |
| **typestate** | reference crate | **Free** OSS | crates.io | Reference implementation pattern |

**No external services. No API keys. Pure-Rust sub-phase.**

## 6. System Inventory — Have vs Need

### Already present
- ✅ Rust 1.95 with phantom types and `PhantomData` in std

### Missing — must install
- Nothing system-level; just add crate deps to `Cargo.toml`.

```toml
# crates/axiom-l1-rs/Cargo.toml
[dev-dependencies]
trybuild = "1"
```

## 7. Working Directory & Files Produced

```
apkaxiom/
├── crates/axiom-l1-rs/
│   ├── src/
│   │   ├── state.rs                    # NEW — phantom types
│   │   └── lib.rs                       # API refactored
│   └── tests/
│       ├── compile-fail/                # NEW — 20+ trybuild misuse patterns
│       │   ├── unverified-manifest.rs
│       │   ├── unverified-resources.rs
│       │   ├── double-verify.rs
│       │   └── ... (20+ files)
│       └── compile-fail.rs              # trybuild driver
└── docs/
    └── type-state.md                    # NEW — design + table mapping to Lean
```

## 8. Standalone Output

```bash
buck2 test //crates/axiom-l1-rs --features compile-fail
# Required: 20/20 compile-fail tests reject as expected
buck2 build //crates/axiom-l1-rs --release
hyperfine 'buck2 run //bench:stream-vs-file -- --apk-corpus corpus/bench-1k'
# Required: perf delta vs P1.7 ≤ 0.1%
```

## 9. End-to-End Test

```bash
cargo test --features compile-fail
# Verifies all misuse patterns reject with the expected error message.
```

## 10. Exit Checklist

- [ ] Phantom types `Apk<Unverified>`, `Apk<SignatureVerified>`, `Apk<FullyParsed<V>>` land
- [ ] All public APIs gated by type-state
- [ ] ≥ 20 compile-fail tests pass with expected error messages
- [ ] Perf delta vs P1.7 ≤ 0.1% (HARD)
- [ ] Translation-validation mapping documented in `docs/type-state.md`
- [ ] No `unsafe` blocks added
- [ ] Lean-side mapping table prepared for P1.9 consumption

## 11. Hand-Off

| Consumed by | What they need |
|---|---|
| **P1.9** | Phantom state ↔ Lean constructor table |
| **P1.10** | Type-state guards on the Merkle hooks (only `SignatureVerified+` can request signing-block commit) |
| **P1.15** | Type-state guards on AXIOM-IR emission (only `FullyParsed<V>` can emit IR) |
