# P4.10 — STARK / Stwo Fallback Pipeline (Post-Quantum)

> Post-quantum proving for regulated industries + long-lived certs. Stwo as the primary STARK backend. Dual-pipeline: Halo2 for performance, Stwo for post-quantum.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §11](../../../README.md#layer-6) · [../../TECH_STACK.md §5](../../TECH_STACK.md#zk-systems)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P4.10 |
| Owner(s) | G7 |
| Duration | Weeks 8–14 |
| Critical-path | no, but required for v1.0 ship gate |
| Hard prerequisites | P4.3 (zk pool) |

## 2. Goal & Scope

A STARK-based alternative pipeline for the same 5 priority privacy invariants. Stwo (StarkWare's open-source STARK library) as the primary backend. Used when:
- Post-quantum security is required
- Long-lived certs (regulated industries)
- Patent landscape concerns require an SNARK alternative

Stwo + Halo2 outputs are equivalent — same predicate, different proof system. The verifier accepts both.

### In scope
- Stwo integration into zk pool (extends P4.3)
- Stwo-compatible circuit lowering for the 5 priority invariants
- Equivalence-check harness: same predicate ⇒ Halo2 ✓ ⇔ Stwo ✓
- Performance benchmark vs Halo2

### Out of scope
- Other STARK frameworks (e.g. Risc Zero, SP1) — research-tracked, not in production
- Post-quantum signing (deferred to Phase 6)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P4.3** | zk pool framework (Stwo plugs in) |
| **P4.4** | Lean → circuit pipeline (extended for STARK target) |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Stwo** | latest from starkware-libs/stwo | STARK lib |
| **Cairo language tools** *(optional)* | latest | If we cross-check via Cairo |
| **icicle GPU kernels** | latest | STARK-specific MSM/NTT |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **Stwo** | STARK lib | **Free** OSS | https://github.com/starkware-libs/stwo | StarkWare |
| **Cairo** *(optional)* | language | **Free** OSS | https://www.cairo-lang.org | Reference |
| **icicle** | GPU | **Free** OSS | already provisioned | |

**No new API keys.**

## 6. System Inventory — Have vs Need

### Already present
- ✅ All from P4.3 + P4.4
- ✅ icicle (cross-platform GPU)

### Missing
- ❌ Stwo crate — Cargo dep

```bash
# Cargo dep
# crates/axiom-zk-pool/Cargo.toml: stwo = { git = "https://github.com/starkware-libs/stwo" }
```

## 7. Features & Functions Delivered (Comprehensive)

### Stwo integration
- `ZkProver::Stwo` variant in P4.3's pool API
- Stwo prove + verify wrappers
- GPU acceleration via icicle (Stwo team's reference path)

### Lean → Stwo lowering
- Extends P4.4's pipeline with a Stwo target backend
- Same Lean theorem → either Halo2 or Stwo circuit
- Equivalent semantics

### Equivalence-check harness
- Same predicate → encode to Halo2 + Stwo
- For ≥ 100 random witnesses: Halo2 ✓ ⇔ Stwo ✓
- Discrepancy = blocking bug

### Performance benchmark
- All 5 priority circuits (P4.5–P4.9) measured under Stwo
- Reported alongside Halo2 baseline

### `.axc` cert format support
- ProofBlob carries either `Halo2(...)` or `Stwo(...)` per claim
- Verifier (P4.11) handles both

### Documentation
- `docs/stark-stwo-fallback.md`

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Stwo integrated into pool | yes | yes |
| Lean → Stwo lowering for 5 priority circuits | yes | yes |
| Equivalence harness 100 % agreement (Halo2 ⇔ Stwo) | 100 % | 100 % |
| Stwo prove p99 (typical APK, GPU) | ≤ 30 s | ≤ 10 s |
| Stwo verify p99 | ≤ 200 ms | ≤ 50 ms |
| Cert size delta (Stwo vs Halo2) documented | within 3× larger | within 1.5× |
| All 5 priority claims producible via either backend | yes | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-zk-pool/                    # extended with Stwo
│       └── src/stwo_impl.rs
├── crates/axiom-lean-to-halo2/           # extended for Stwo target
│   └── src/lower_to_stwo.rs
├── tests/zk-equivalence/
│   └── halo2-vs-stwo.rs
└── docs/
    └── stark-stwo-fallback.md
```

## 10. Standalone Output

```bash
buck2 run //tools/cli -- prove-no-read-contacts --apk app.apk --backend stwo
# Outputs Stwo proof
buck2 test //tests/zk-equivalence:halo2-vs-stwo
# "100/100 random witnesses Halo2 ⇔ Stwo agree"
```

## 11. End-to-End Test

```bash
buck2 test //crates/axiom-zk-pool:stwo
# - Stwo integration operational (HARD)
# - Equivalence harness 100% (HARD)
# - Prove p99 ≤ 30 s (HARD)
# - Verify p99 ≤ 200 ms (HARD)
```

## 12. Exit Checklist

- [ ] Stwo integrated into zk pool
- [ ] Lean → Stwo lowering extends P4.4's pipeline
- [ ] Equivalence harness 100 % agreement (HARD)
- [ ] Stwo prove p99 ≤ 30 s (HARD)
- [ ] Stwo verify p99 ≤ 200 ms (HARD)
- [ ] Cert size delta documented
- [ ] All 5 priority claims producible via either backend
- [ ] `docs/stark-stwo-fallback.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P4.11** | Verifier handles Stwo proofs |
| **P4.18** | E2E pipeline tests both backends |
| **Phase 6 ship gate** | Post-quantum profile is a v1.0 commitment |
