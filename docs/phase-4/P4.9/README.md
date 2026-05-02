# P4.9 — Privacy Invariant 5: TFLite Model Integrity Halo2 Circuit

> *"The TFLite model bundled in this APK has digest H — and was not tampered with vs. the signed reference."* The cryptographic foundation for ML-supply-chain verification.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §13.3](../../../README.md#beyond-the-12)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P4.9 |
| Owner(s) | G7 |
| Duration | Weeks 10–15 |
| Critical-path | yes |
| Hard prerequisites | P4.4 |

## 2. Goal & Scope

A Halo2 circuit proving the APK's bundled TFLite model has a specific structural digest, computed in a way invariant under standard quantization noise. Used for ML-supply-chain verification: an app store can verify the model wasn't backdoored after vendor upload.

### In scope
- `theorems/Apkaxiom/PrivacyInvariants/TfliteIntegrity.lean`
- Halo2 circuit `crates/axiom-circuit-tflite-integrity`
- Structural model-hash extractor (independent of quantization noise)
- End-to-end demo on synthetic + real TFLite-bearing APKs

### Out of scope
- Backdoor detection (Phase 5 / G11)
- ONNX coverage (Phase 5)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P4.4** | Lean → Halo2 pipeline |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **TFLite parser** | latest crates.io | Read bundled .tflite |
| **Halo2 + Poseidon2** | from P4.5 | Circuit |
| **HACL\* SHA-256** | from P1.10 | Outer hash |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **TFLite format spec** | reference | **Free** | https://www.tensorflow.org/lite | |
| **TensorFlow Lite Rust crate** | parser | **Free** OSS | crates.io | |

**No new API keys.**

## 6. System Inventory — Have vs Need

### Already present
- ✅ Halo2 + Poseidon2 + Rust + HACL\*

### Missing
- ❌ TFLite Rust crate — Cargo dep

## 7. Features & Functions Delivered (Comprehensive)

### Lean theorem
```lean
theorem tflite_integrity (apk : APK) (claimed_digest : Hash) :
  ∀ (model : TFLiteModel), apk.bundled_model = some model →
    structural_hash model = claimed_digest
```

### Structural model-hash
- Topology hash: layer types + shapes + connection graph
- Operator hash: each op's type + param shapes
- Weight-shape hash (NOT weight values — quantization-invariant)
- Final: BLAKE3 over the structured triple

### Halo2 circuit
- Public input: claimed digest
- Private witness: model parsed structure
- Constraints: structural-hash computation matches claimed digest
- In-circuit Poseidon2 hashing
- Circuit size: target ≤ 2^17 rows

### Structural vs full hash
- Full hash (BLAKE3 over raw bytes) is brittle to quantization
- Structural hash captures what matters for "is this the same model architecture?"
- Optional companion claim: "and the weights match digest H_weights" — which IS sensitive to quantization (used when both vendor and store use same quantization)

### Demo
- 100 sample TFLite-bearing APKs from F-Droid + AndroZoo
- Per-APK: extract structural hash, prove circuit, verify

### Documentation
- `docs/circuit-tflite-integrity.md`

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Lean theorem mechanized | yes | yes |
| Structural-hash quantization-invariance verified on 50 quantized variants | 100 % | 100 % |
| Prove p99 | ≤ 5 s | ≤ 1.5 s |
| Verify p99 | ≤ 20 ms | ≤ 5 ms |
| Demo on 100 TFLite-bearing APKs | ≥ 95 % provable | ≥ 99 % |
| Cert size | ≤ 30 KB | ≤ 10 KB |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── theorems/Apkaxiom/PrivacyInvariants/
│   └── TfliteIntegrity.lean
├── crates/
│   └── axiom-circuit-tflite-integrity/
└── docs/
    └── circuit-tflite-integrity.md
```

## 10. Standalone Output

```bash
buck2 run //tools/cli -- prove-tflite-integrity --apk app.apk --expected-digest 0x...
```

## 11. End-to-End Test

```bash
buck2 test //crates/axiom-circuit-tflite-integrity:demo
# - Quantization-invariance 100% (HARD)
# - Demo provable ≥ 95% (HARD)
# - Prove p99 ≤ 5 s (HARD)
# - Verify p99 ≤ 20 ms (HARD)
```

## 12. Exit Checklist

- [ ] Lean theorem mechanized
- [ ] Structural-hash quantization-invariance verified (HARD)
- [ ] Prove p99 ≤ 5 s (HARD)
- [ ] Verify p99 ≤ 20 ms (HARD)
- [ ] Demo provable ≥ 95 % on TFLite-bearing APKs (HARD)
- [ ] Cert ≤ 30 KB (HARD)
- [ ] Documentation published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P4.11** | Verifier handles claim type |
| **P4.17** | Bug-bounty + ML-app pilot |
| **Phase 5 / G11** | Builds on this for backdoor detection |
