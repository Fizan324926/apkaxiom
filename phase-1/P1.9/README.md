# P1.9 — Rust Extraction Pipeline v0.1 + Translation-Validation Harness

> Lean → Rust, with a separate validator that runs both and asserts byte-for-byte equality. Fail-closed CI gate.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../README.md §6 (Layer 1)](../../README.md#layer-1)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P1.9 |
| Owner(s) | G1 + G2 |
| Duration | Weeks 7–11 |
| Critical-path | **yes** — gates every later "verified Rust" claim |
| Hard prerequisites | P1.5 (real Lean theorems), P1.8 (target Rust API + phantom states) |

## 2. Goal & Scope

The Lean → Rust extractor compiles a Lean module into a Rust crate. A **translation validator** runs both the Lean reference evaluator and the extracted Rust on a corpus, asserting byte-for-byte output equality. Discrepancies fail the CI gate.

The extraction pipeline is **the trust boundary** of APKAXIOM. Everything below it is theorems; everything above relies on extraction being faithful. Translation validation is what bridges the two.

### In scope
- `tools/lean-to-rust/` — extractor binary
- `tools/translation-validator/` — runs both sides, diffs outputs
- First real extracted module: `axiom-l0-zip-lfh` from P1.5's Lean
- CI gate: extraction reproducible byte-identical across machines
- Corpus-driven validation: ≥ 1,000 LFH inputs, 100% agreement

### Out of scope
- Full ZIP extraction (P1.12).
- Signing-block extraction (P1.16).
- Production performance optimizations of extracted Rust (later phases).

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P1.2** | Extraction prototype (toy `Nat → Nat`) — proves the pattern works |
| **P1.5** | Real Lean LFH module to extract |
| **P1.8** | Target Rust API surface (phantom states) |
| **P1.4** | AXIOM-IR types Lean and Rust agree on |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Lean 4** | pinned | Source language |
| **Rust** | 1.95 | Target language |
| **OCaml** | 4.14 (from P1.2 opam) | Some Lean tactics need OCaml runtime |
| **proptest** | 1.5+ | Property-based input generation for the validator |
| **insta** | 1.40+ | Snapshot testing for extracted output |
| **rkyv / serde** | from P1.4 | Compare structured outputs |
| **diff-match-patch** | latest | Pretty-print divergences |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **Lean extraction libraries** | research libraries | **Free** OSS | various | We likely write our own; mathlib has some support |
| **F\* extraction → Rust** (Karamel for C, custom for Rust) | reference | **Free** OSS | https://github.com/FStarLang/karamel | Inspiration for the Rust target backend |
| **CompCert OCaml extraction** | reference | **Non-commercial use free; commercial paid** | https://compcert.org/ | Reference design only; we don't link CompCert |
| **CakeML** | verified ML compiler | **Free** OSS | https://cakeml.org | Reference design only |

**No paid third-party. No API keys.** Extraction tooling is research-grade; we vendor or write.

## 6. System Inventory — Have vs Need

### Already present
- ✅ Lean / Lake (P1.2)
- ✅ Rust (P1.1)
- ✅ OCaml (P1.2)

### Missing — must install
- ❌ Just add Rust crates to `Cargo.toml` (`proptest`, `insta`, `diff-match-patch`)

```bash
# Optional: clone Karamel for reference
git clone https://github.com/FStarLang/karamel external/reference/karamel
```

## 7. Working Directory & Files Produced

```
apkaxiom/
├── tools/
│   ├── lean-to-rust/
│   │   ├── Cargo.toml
│   │   ├── BUCK
│   │   └── src/
│   │       ├── main.rs                  # CLI entrypoint
│   │       ├── extractor.rs              # Lean AST → Rust AST
│   │       ├── lean_ast.rs               # Lean syntax parsing
│   │       └── rust_emit.rs              # Rust AST emission
│   └── translation-validator/
│       ├── Cargo.toml
│       ├── BUCK
│       └── src/
│           ├── main.rs
│           ├── lean_runner.rs            # invokes `lake exe` evaluator
│           ├── rust_runner.rs            # invokes extracted crate
│           └── differ.rs                 # diff outputs, classify divergence
├── crates/
│   └── axiom-l0-zip-lfh-verified/        # NEW — auto-generated
│       ├── Cargo.toml
│       ├── BUCK
│       └── src/lib.rs                    # generated from theorems/.../LocalHeader.lean
├── tests/
│   └── translation-validation/
│       └── BUCK                          # runs validator on corpus
└── docs/
    └── extraction-pipeline.md            # NEW — replaces P1.2 draft
```

## 8. Standalone Output

```bash
nix develop
make extract    # tools/lean-to-rust on theorems/.../LocalHeader.lean → crates/axiom-l0-zip-lfh-verified
make tv         # translation-validator on Bench-1K LFH corpus
# Output: "1000/1000 inputs Lean ↔ extracted Rust agree byte-for-byte"
```

## 9. End-to-End Test

```bash
buck2 test //tests/translation-validation:lfh
# Required:
#   - extraction byte-identical across 3 reference machines
#   - validator reports 100% agreement on ≥1,000 inputs
#   - extracted Rust perf within 30% of hand-Rust baseline (TARGET: 10%)
```

CI gate: any change to `theorems/Apkaxiom/Zip/LocalHeader.lean` re-runs extraction + validation. If either fails, PR is blocked.

## 10. Exit Checklist

- [ ] `tools/lean-to-rust` compiles non-trivial Lean module
- [ ] First real extracted module `axiom-l0-zip-lfh-verified` lands
- [ ] Translation validator green on ≥ 1,000 LFH inputs
- [ ] Extraction byte-identical on 3 reference machines (HARD)
- [ ] Extracted Rust perf delta vs hand-Rust ≤ 30% (HARD)
- [ ] CI gate: PRs touching Lean re-run validation, fail-closed
- [ ] `docs/extraction-pipeline.md` published with full pipeline diagram

## 11. Hand-Off

| Consumed by | What they need |
|---|---|
| **P1.12** | Production-ready extractor for full ZIP layer |
| **P1.16** | Same extractor for signing block |
| **P1.17** | Translation validator wired into soundness regression CI gate |
| **All later "verified Rust" claims** | The extraction pipeline is the trust boundary |
