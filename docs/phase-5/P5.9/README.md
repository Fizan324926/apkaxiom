# P5.9 — Lean Theorems for Native Lifter Soundness

> Establish the proof spine for the native subsystem: DEX → SSA correctness, JNI boundary marshaling correctness, native-summary correctness for the cataloged subset.

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P5.9 |
| Owner(s) | G1 + G9 |
| Duration | Weeks 4–18 |
| Critical-path | yes (any L1-class extension) |
| Hard prerequisites | P5.2 |

## 2. Goal & Scope

Lean 4 theorems sufficient to claim "native lifter and JNI boundary are sound" — this is the proof spine the cert format L6 cites for joint-analysis findings.

### In scope
- DEX → DEX-SSA correctness (semantics-preserving lift)
- JNI boundary marshaling: argument + return + ref-type lifecycle theorems
- Catalog-summary correctness: a small but high-coverage subset (libc memcpy / memcmp / strcmp / open / read / write; OpenSSL EVP_DigestUpdate / EVP_DigestFinal_ex; libsodium crypto_*) — proved-correct vs reference Lean executable models
- Provenance flow soundness: every value crossing the boundary keeps its provenance tag
- ARM64 lifter soundness: scoped to the *AAPCS64-conforming subset* (no inline asm, no SVE/SME) — a useful fragment large enough to cite in v1.0

### Out of scope
- Pure-ARMv7 soundness (deferred to Phase 6 or v1.1)
- Full OpenSSL formalization (huge; we cite a small subset)
- Catalog summaries beyond the proved-subset (v1.1)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P5.2** | AXIOM-IR-v0.4 native dialect (frozen) |
| **P3.4 / P3.5** | Lean theorem-proving infrastructure (re-used) |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Lean 4** | pinned | Theorem proving |
| **mathlib4** | pinned (matching Lean 4) | Math lib |
| **Lean → Rust extraction** | pinned | (existing pipeline) |
| **CompCert / Vellvm-style references** | optional | Cross-validation |

## 5. Third-Party Software, Services, Accounts & API Keys

All Lean tooling vendored / pinned in Phase 1 / 3. **No new API keys.**

## 6. System Inventory — Have vs Need

All present.

## 7. Features & Functions Delivered (Comprehensive)

### Theorems (Lean 4 modules under `theorems/`)
- `theorems/dex_ssa.lean` — DEX → DEX-SSA correctness
  - `lift_preserves_semantics` : ∀ d : DexClass, semantics(lift(d)) ≡ semantics(d)
  - `lift_total_on_well_typed` : every well-typed DEX class lifts to some IR module
  - `lift_deterministic` : same input → same IR
- `theorems/jni_boundary.lean` — JNI marshaling correctness
  - `marshal_round_trip` : marshal(unmarshal(jvalue)) ≡ jvalue
  - `ref_lifecycle_safe` : every local ref's lifetime ⊆ frame
- `theorems/catalog_libc.lean` — proved subset of libc summaries
  - `memcpy_summary_correct`, `memcmp_summary_correct`, `strcmp_summary_correct`, `open_summary_correct`, `read_summary_correct`, `write_summary_correct`
- `theorems/catalog_openssl.lean` — proved subset of OpenSSL summaries
  - `evp_digestupdate_summary_correct`, `evp_digestfinal_summary_correct`
- `theorems/provenance.lean` — provenance soundness
  - `provenance_propagates` : every IR pass preserves provenance tags
- `theorems/arm64_aapcs64_subset.lean` — ARM64 lifter soundness over AAPCS64-conforming subset

### Cross-validation
- Lean executable models compared against reference C / Rust implementations on regression corpus
- Disagreements break CI

### CI gate
- Lean re-verify on every PR touching `theorems/dex_ssa.lean`, `theorems/jni_boundary.lean`, `theorems/catalog_*.lean`, `theorems/provenance.lean`
- Reproducibility: bytewise-identical Lean compiled artifacts across runs / arches

### Documentation
- `docs/native-soundness.md` — theorem index + scope statement + what is *not* claimed for v1.0

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| `lift_preserves_semantics` machine-checked | yes | yes |
| `marshal_round_trip` machine-checked | yes | yes |
| ≥ 6 catalog-summary correctness theorems machine-checked | yes | ≥ 12 |
| `provenance_propagates` machine-checked | yes | yes |
| `arm64_aapcs64_subset` soundness theorem machine-checked | yes | yes |
| Lean re-verify CI green on every relevant PR | 100 % | 100 % |
| Bytewise-identical Lean artifacts | 100 % | 100 % |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── theorems/
│   ├── dex_ssa.lean                  # NEW
│   ├── jni_boundary.lean             # NEW
│   ├── catalog_libc.lean             # NEW
│   ├── catalog_openssl.lean          # NEW
│   ├── provenance.lean               # NEW
│   └── arm64_aapcs64_subset.lean     # NEW
└── docs/
    └── native-soundness.md           # NEW
```

## 10. Standalone Output

The Lean theorems + cross-validation harness are reusable by any binary-analysis project that wants a verified lifter spine.

## 11. End-to-End Test

```bash
buck2 build //theorems:...
# Expect: all .lean files type-check + theorem-check

buck2 test //theorems/cross-validate:...
# Expect: ≥ 1000 cross-validation samples agree
```

## 12. Exit Checklist

- [ ] `lift_preserves_semantics` machine-checked (HARD)
- [ ] `marshal_round_trip` machine-checked (HARD)
- [ ] ≥ 6 catalog-summary correctness theorems (HARD)
- [ ] `provenance_propagates` machine-checked (HARD)
- [ ] `arm64_aapcs64_subset` soundness theorem (HARD)
- [ ] CI re-verify gate green
- [ ] Bytewise-identical artifacts 100 %
- [ ] Cross-validation 1000+ samples agree
- [ ] Documentation `docs/native-soundness.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P5.18** | Soundness theorems cited in E2E |
| **P5.19** | Theorem statements cited in paper |
| **L6 cert** | Cert references theorem hashes |
| **Phase 6** | Re-verify all theorems against final Lean toolchain |
