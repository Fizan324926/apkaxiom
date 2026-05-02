# P4.4 — Privacy-Invariant Compilation Pipeline (Lean → Halo2)

> The compiler. Take a Lean privacy-invariant theorem, lower to a Halo2 PLONKish circuit. Soundness: Halo2 proof of the circuit ⇒ Lean theorem holds on the witness APK.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §11](../../../README.md#layer-6)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P4.4 |
| Owner(s) | G7 + G1 |
| Duration | Weeks 4–10 |
| Critical-path | yes |
| Hard prerequisites | P4.3 (zk pool) |

## 2. Goal & Scope

A compilation pipeline from Lean privacy-invariant theorems → Halo2 PLONKish circuits. The pipeline is the trust bridge: a Halo2 proof on the circuit cryptographically implies the underlying Lean theorem holds for the witness APK. Soundness theorem mechanized in Lean. 10× faster than hand-coding circuits per invariant.

### In scope
- `crates/axiom-lean-to-halo2` — compiler crate
- Lean → AXIOM-IR-symbolic → constraint system → Halo2 circuit
- Soundness theorem `lean_implies_halo2_circuit`
- 1 toy invariant fully demonstrated end-to-end (used as template for P4.5–P4.9)
- Translation-validation harness: each compiled circuit passed through both Halo2 and a reference SAT solver, results agree

### Out of scope
- The 5 priority privacy invariants (P4.5–P4.9)
- STARK alternative (P4.10)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P4.3** | zk pool API |
| **P3.10** | Abstraction-domain library (constraint targets) |
| **P3.3** | AXIOM-IR-symbolic (intermediate IR) |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Lean 4 + mathlib4** | pinned | Theorem source |
| **Halo2** | latest | Target circuit framework |
| **Rust** | 1.95 | Compiler implementation |
| **AXIOM-IR-symbolic crate** | from P3.3 | Intermediate stage |
| **Translation-validation harness** | from P1.9 | Adapted for circuit ↔ SAT validation |
| **Z3 / cvc5** | from P3.6 | Reference SAT for circuit validation |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **Halo2 / Plonky3 / Binius** | from P4.3 | **Free** OSS | already provisioned | |
| **MathPix / equation-rendering services** | optional | **Paid** $4.99/mo (cheap) | https://mathpix.com | Optional rendering for paper diagrams |

**No new API keys.**

## 6. System Inventory — Have vs Need

### Already present
- ✅ Lean / Lake / mathlib4 / Rust / Halo2 / SMT solvers
- ✅ AXIOM-IR-symbolic types

### Missing
- Nothing new system-level.

## 7. Features & Functions Delivered (Comprehensive)

### Compiler stages
1. **Lean theorem ingestion** — read theorem statement, extract universal quantifier, conclusion predicate
2. **Lower to AXIOM-IR-symbolic** — predicate becomes constraint over symbolic APK state
3. **Lower to constraint system** — PLONK-friendly arithmetic constraints
4. **Lower to Halo2 circuit** — assignments + lookups + custom gates as needed
5. **Generate witness extractor** — given a concrete APK, extract values matching circuit witness shape

### Soundness theorem (mechanized)
- `theorem lean_implies_halo2_circuit : ∀ (theorem T : LeanTheorem) (apk : APK) (witness : ExtractedWitness apk),
   compile T = ok (circuit, vk) →
   halo2_verify circuit vk (extract_witness apk) = true →
   evaluate T apk = true`
- This is the trust bridge — proven once, applied per-invariant

### Translation-validation harness
- For each compiled circuit: generate test witnesses, run Halo2 prover, also encode same predicate to SMT, compare
- ≥ 100 round-trips per circuit
- Discrepancies block CI

### Toy demo invariant
- "Every APK has a non-empty package name" — trivially true, used to demonstrate the full pipeline end-to-end
- Compile → prove → verify

### Performance characteristics
- Compilation time per Lean theorem: ≤ 5 s (one-shot)
- Witness-extraction time per APK: ≤ 100 ms p99
- Compiled circuits used by P4.5–P4.9 directly

### Documentation
- `docs/lean-to-halo2-compiler.md` — design, constraint-encoding patterns, custom-gate inventory
- `docs/lean-to-halo2-soundness-proof.md` — soundness theorem walkthrough

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Compiler produces compilable Halo2 circuit on toy invariant | yes | yes |
| Soundness theorem mechanized in Lean | yes | yes |
| Translation-validation: 100 round-trips per circuit | 100 % agreement | 100 % |
| Compilation time per theorem | ≤ 10 s | ≤ 5 s |
| Witness-extraction p99 | ≤ 200 ms | ≤ 100 ms |
| Reviewer sign-off (G1, G7 leads) | yes | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-lean-to-halo2/
│       ├── Cargo.toml
│       ├── BUCK
│       └── src/
│           ├── lib.rs
│           ├── ingest.rs                  # Lean theorem reader
│           ├── lower_to_ir.rs              # → AXIOM-IR-symbolic
│           ├── lower_to_constraints.rs    # → PLONK constraints
│           ├── lower_to_halo2.rs           # → Halo2 circuit
│           └── witness_extract.rs
├── theorems/Apkaxiom/Halo2/
│   └── Soundness.lean                    # NEW — the trust-bridge theorem
├── corpus/circuits/
│   └── toy-invariant/                    # toy demo
└── docs/
    ├── lean-to-halo2-compiler.md
    └── lean-to-halo2-soundness-proof.md
```

## 10. Standalone Output

```bash
buck2 build //crates/axiom-lean-to-halo2 --release
# Compile toy
buck2 run //tools/cli -- compile-lean-theorem theorems/Apkaxiom/Halo2/ToyDemo.lean
# Outputs: circuit + PK + VK
buck2 test //tests/lean-to-halo2:translation-validation
# "100/100 round-trips on toy-invariant agree"
```

## 11. End-to-End Test

```bash
buck2 test //crates/axiom-lean-to-halo2:full
# - Compiler produces compilable Halo2 (HARD)
# - Soundness theorem re-verifies (HARD)
# - 100 round-trips agree on toy invariant (HARD)
# - Compilation ≤ 10 s (HARD)
# - Witness extract p99 ≤ 200 ms (HARD)
```

## 12. Exit Checklist

- [ ] Compiler produces compilable Halo2 from Lean theorem
- [ ] Soundness theorem mechanized + re-verifies on CI
- [ ] Translation-validation harness on toy invariant 100 % (HARD)
- [ ] Compilation ≤ 10 s per theorem (HARD)
- [ ] Witness extract p99 ≤ 200 ms (HARD)
- [ ] G1 + G7 lead sign-off
- [ ] Documentation published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P4.5–P4.9** | The 5 priority privacy invariants compile through this pipeline |
| **P4.10** | Same pipeline emits Stwo STARK circuits |
| **External community** | Reusable pattern for Lean-grounded zk-SNARKs |
