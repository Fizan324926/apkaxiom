# P2.9 — AXIOM-IR v0.2 Spec Frozen

> The freeze. v0.2 spec, DEX dialect, manifest + resource extensions, lowerings — frozen for the rest of Phase 2 and beyond. Reference Rust crate compiles.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §13.9](../../../README.md#beyond-the-12)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P2.9 |
| Owner(s) | G3 |
| Duration | Weeks 10–12 |
| Critical-path | yes |
| Hard prerequisites | P2.2 (RFC), P2.5/P2.6/P2.8 (implementations validate the design) |

## 2. Goal & Scope

The AXIOM-IR-v0.2 spec is **frozen** — no changes for the rest of Phase 2 without an ADR. The reference Rust crate compiles fully. Lean reflection of v0.2 types lands. Round-trip tests pass on 100+ reference IR samples.

### In scope
- Promote `docs/AXIOM-IR-v0.2-RFC.md` to `docs/AXIOM-IR-v0.2.md` (frozen status)
- Reference crate `axiom-ir` v0.2 fully compiles (DEX + manifest extensions + resource extensions)
- Lean reflection module `theorems/Apkaxiom/IrV2.lean` re-verifies
- 100+ reference IR samples round-trip across `serde / rkyv / bincode / Cap'n Proto`
- ADR-0013 — AXIOM-IR v0.2 freeze

### Out of scope
- Implementing emitters (P2.5, P2.6, P2.8 already did)
- v0.3 features (Phase 5)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P2.2** | RFC drafted and reviewed |
| **P2.5** | AXML emitter implemented (validates manifest extensions) |
| **P2.6** | ARSC emitter implemented (validates resource extensions) |
| **P2.8** | DEX dialect implemented (validates DEX dialect design) |

## 4. Required Tools, Libraries, and Languages

Inherited from P2.2 + P1.4. No new tools.

## 5. Third-Party Software, Services, Accounts & API Keys

**No new external dependencies.**

## 6. System Inventory — Have vs Need

Nothing new. Reuses everything from prior sub-phases.

## 7. Features & Functions Delivered (Comprehensive)

### Frozen spec
- `docs/AXIOM-IR-v0.2.md` — exact frozen text of the RFC, with "FROZEN ON YYYY-MM-DD" header
- Versioning policy — `axiom-ir` crate version bumps to `0.2.0`; pre-1.0 SemVer applies
- Migration guide — how a v0.1-only consumer adapts to v0.2

### Reference crate
- `crates/axiom-ir` at v0.2 fully compiles
- All dialects (`manifest`, `resource`, `dex`) compile and round-trip
- Lowerings (`manifest ↔ resource`, `manifest ↔ dex`, preview `dex ↔ symbolic`) compile
- `cargo doc -p axiom-ir` produces complete public API docs

### Lean reflection
- `theorems/Apkaxiom/IrV2.lean` — Lean reflection of all v0.2 types
- Type-set theorems: well-formedness, deduplication invariants
- Re-verifies on CI

### Test coverage
- 100+ hand-written IR samples per dialect, round-trip via `serde + rkyv + bincode + capnp`
- Lowering correctness on 30+ samples per lowering pair
- Snapshot tests via insta

### ADR
- ADR-0013 — AXIOM-IR v0.2 freeze decision, freeze date, post-freeze change-control rules

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| AXIOM-IR-v0.2 spec frozen ≥ 4 weeks before P2.18 | yes | yes |
| Reviewer sign-off | G1, G2, G3, G4 leads | + G5, G6 |
| Reference crate compiles + 100-sample round-trip green | yes | yes |
| Lean reflection module re-verifies | yes | yes |
| Lowering tests green (30+ per pair) | yes | yes |
| ADR-0013 merged | yes | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── docs/
│   ├── AXIOM-IR-v0.2.md                  # promoted from -RFC.md, FROZEN
│   ├── AXIOM-IR-v0.1-to-v0.2-migration.md # NEW
│   └── ADR-0013-axiom-ir-v0.2-freeze.md  # NEW
├── crates/axiom-ir/                       # version bump to 0.2.0; finalized
├── theorems/Apkaxiom/IrV2.lean            # finalized + reflection theorems
└── tests/ir-v0.2/                         # NEW — round-trip tests
    ├── manifest.rs
    ├── resource.rs
    └── dex.rs
```

## 10. Standalone Output

```bash
buck2 build //crates/axiom-ir
buck2 test //tests/ir-v0.2
cargo doc -p axiom-ir
# All green; spec frozen.
```

## 11. End-to-End Test

```bash
buck2 test //tests/ir-v0.2:full
# - 100+ samples per dialect round-trip (HARD)
# - 30+ samples per lowering pair (HARD)
# - Lean reflection module re-verifies (HARD)
```

## 12. Exit Checklist

- [ ] AXIOM-IR-v0.2 spec frozen, FROZEN-ON date set
- [ ] Reviewer sign-off from G1, G2, G3, G4 leads (HARD)
- [ ] Reference crate `axiom-ir` v0.2.0 compiles
- [ ] 100+ samples per dialect round-trip via serde + rkyv + bincode + capnp
- [ ] Lowering tests green (≥ 30 per pair)
- [ ] Lean reflection re-verifies
- [ ] ADR-0013 merged
- [ ] Migration guide v0.1 → v0.2 published
- [ ] Spec must remain unchanged ≥ 4 weeks before P2.18 (HARD)

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P2.10** | Frozen IR types as foundation for Schrödinger BehaviorSet |
| **P2.11, P2.12** | IR types for bundle parser and resolver |
| **P2.14, P2.15, P2.16** | IR as input to forensic passes |
| **Phase 3 / G5** | IR as the source for symbolic resolver |
| **Phase 4 / G7** | `.axc` certificates carry IR-v0.2 commitments |
