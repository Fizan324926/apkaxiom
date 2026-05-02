# P1.16 — Rust Extraction of APK Signing Block (HACL\* Signature Path)

> Lean signing theorems extracted to Rust. All crypto routes through HACL\*. 100% verdict agreement with apksigner on Bench-10K.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../README.md §6](../../README.md#layer-1)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P1.16 |
| Owner(s) | G1 + G2 |
| Duration | Weeks 14–18 |
| Critical-path | yes |
| Hard prerequisites | P1.11 (Lean signing theorems), P1.12 (extraction pipeline working on ZIP) |

## 2. Goal & Scope

APK Signing Block v1/v2/v3/v3.1 verifiers extracted from Lean to Rust. All cryptographic operations route through HACL\* (verified Ed25519, RSA-PKCS1, RSA-PSS, ECDSA, BLAKE3, SHA-256). Translation validator passes on the 2,500-APK signing corpus.

### In scope
- Extracted crate `axiom-l1-signing-verified`
- HACL\* crypto bindings for the four needed primitives (Ed25519, RSA, ECDSA, SHA-256)
- Translation validator integration
- Performance benchmark vs apksigner

### Out of scope
- Third-party signing-block formats (Stamp, Channel, Vasdolly, Packer NG) — Phase 2.

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P1.11** | Full signing-block Lean theorems |
| **P1.12** | Production extraction pipeline + translation validator |
| **P1.10** | HACL\* infrastructure for crypto |

## 4. Required Tools, Libraries, and Languages

Same as P1.10 + P1.11 + P1.12. New: per-primitive HACL\* binding for the additional crypto:

| Tool | Version | Purpose |
|---|---|---|
| **HACL\* Ed25519** | from hacl-star | Verified Ed25519 verification |
| **HACL\* RSA-PKCS1, RSA-PSS** | from hacl-star | Verified RSA signature verification |
| **HACL\* ECDSA P-256** | from hacl-star | Verified ECDSA verification |
| **fiat-crypto** | latest | Backup for ECDSA P-384 (HACL\* coverage thinner) |

## 5. Third-Party Software, Services, Accounts & API Keys

Same as P1.10 + P1.11. **No new third-party.**

## 6. System Inventory — Have vs Need

### Already present (after prior sub-phases)
- ✅ HACL\* + EverCrypt
- ✅ Lean signing theorems
- ✅ Extraction pipeline
- ✅ apksigner (from P1.11)

### Missing
- Rust bindings to HACL\* for Ed25519 / RSA / ECDSA — extension of `axiom-blake3-hacl` crate.

```bash
# No new system installs; extend existing crate
# crates/axiom-crypto-hacl/Cargo.toml — adds bindings for Ed25519/RSA/ECDSA
```

## 7. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   ├── axiom-crypto-hacl/                # NEW — superset of axiom-blake3-hacl
│   │   ├── Cargo.toml
│   │   ├── BUCK
│   │   ├── build.rs
│   │   ├── wrapper.h
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── blake3.rs                 # from P1.10
│   │       ├── sha256.rs                 # NEW
│   │       ├── ed25519.rs                # NEW
│   │       ├── rsa.rs                    # NEW (PKCS1 + PSS)
│   │       └── ecdsa.rs                  # NEW
│   └── axiom-l1-signing-verified/        # NEW — auto-generated
│       ├── Cargo.toml
│       └── src/lib.rs
└── tests/
    └── translation-validation/
        └── signing.rs                     # NEW — Bench-10K signing diff
```

## 8. Standalone Output

```bash
nix develop
make extract-signing               # Lean signing → Rust
buck2 test //tests/translation-validation:signing
# "10000/10000 verdicts axiom-l1-signing-verified ↔ apksigner agree"
buck2 run //bench:signing-throughput
# Reports verifications/sec/core
```

## 9. End-to-End Test

```bash
buck2 test //tests/translation-validation:signing-bench-10k
# Required:
#   - 100% verdict agreement with apksigner (HARD)
#   - signature verification throughput ≥ 1,000 APKs/sec/core (HARD)
#   - verified vs hand-written perf delta ≤ 20%
#   - HACL* on the path (no generic crypto), build-system check
```

## 10. Exit Checklist

- [ ] All 4 signing schemes extracted
- [ ] HACL\* SHA-256 / RSA / Ed25519 / ECDSA bindings landed
- [ ] Translation validator green on 10,000-APK signing corpus (HARD)
- [ ] Throughput ≥ 1,000 APKs/sec/core (HARD per PHASE_GATES.md §5)
- [ ] Bench-10K verdict agreement = 100% with apksigner (HARD)
- [ ] HACL\* in use, generic crypto banned (build-system check enforces)

## 11. Hand-Off

| Consumed by | What they need |
|---|---|
| **P1.17** | Soundness regression suite covers signing extraction |
| **P1.18** | Verified signing on E2E path |
| **Phase 2 / G12 SLSA** | Verified signature path is the foundation for SLSA |
| **Phase 4 / G7** | `.axc` certificates carry HACL\*-verified signatures |
