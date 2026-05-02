# Phase 4 — Certificates & Tooling Detailed Plan (M18 → M24)

> The 6 months that take APKAXIOM from "sound symbolic + equivalence reasoning" to "every finding ships with a machine-checkable certificate that a triager runs in milliseconds." 20 sub-phases (P4.1 → P4.20).
> Each sub-phase has its own folder with: identity, goal/scope, dependencies, tools, third-party services & API keys with free/paid status, system inventory, **all features & functions delivered**, **explicit numeric KPIs**, end-to-end test, exit checklist.

This document is the operational complement to:
- [../../README.md](../../README.md) — architecture
- [../ROADMAP.md](../ROADMAP.md) — high-level Phase 4 goals
- [../PHASE_GATES.md](../PHASE_GATES.md#phase-4) — Phase 4 numeric KPI gates
- [../TECH_STACK.md](../TECH_STACK.md) — tech-stack picks (zk-SNARKs, SDKs, GPU)

---

## Table of Contents

1. [Phase 4 Goal Statement](#goal)
2. [What's New in Phase 4 vs Phase 3](#whats-new)
3. [The 20 Sub-Phases at a Glance](#glance)
4. [Sub-Phase Dependency Diagram](#deps)
5. [Cross-Cutting Conditions (always true)](#cross-cutting)
6. [Phase 4 Consolidated Exit Gate](#exit-gate)
7. [Phase 4 Risk Register](#risks)
8. [Definitions](#defs)

---

<a id="goal"></a>
## 1. Phase 4 Goal Statement

By the end of Phase 4 (M24), the project must have:

- **`.axc` certificate format v1** — frozen, RFC-style, public. Cap'n Proto schema, Ed25519-signed, content-addressed. Carries: parser-consistency proof (Lean), reachability witnesses, UNSAT certs (DRAT), equivalence certs (bisim), privacy-invariant proofs (zk-SNARK), provenance metadata.
- **Halo2 zk-SNARK circuits** for the 5 priority privacy invariants — all production-grade with GPU acceleration via sppark/icicle. Plonky3 alternative pipeline benchmarked head-to-head.
- **STARK fallback** (Stwo) operational for post-quantum / regulated-industry deployments.
- **`axiom-verify` reference verifier** — Rust core + Wasm + ARM64 mobile builds. p99 ≤ 100 ms over 10K-cert sample. Cold start ≤ 500 ms.
- **SLSA L4 attestation + reproducible-build verification** end-to-end with F-Droid + manual sample.
- **SDKs**: `axiom-py` (PyO3 + uniffi), `axiom-go` (cgo + uniffi), `axiom-ts` (Wasm + wit-bindgen) — all single-source-of-truth from Rust.
- **Bug-bounty platform pilot** live in production — first platform ingesting `.axc` and rendering findings to triagers in ≤ 2 s.
- **Phase-4 paper drafted** — *"Proof-Carrying APKs: A New Architecture for Mobile App Distribution"* — for **CCS 2027** or **IEEE S&P 2028**.
- **All Phase 4 hard KPIs** from PHASE_GATES.md §8 green for ≥ 7 consecutive days.

---

<a id="whats-new"></a>
## 2. What's New in Phase 4 vs Phase 3

| Area | Phase 3 | Phase 4 |
|---|---|---|
| Layers under measurement | L0–L5 | **L0–L6 — full proof stack** |
| Crypto on the proof path | HACL\* (BLAKE3, SHA-256, Ed25519, RSA, ECDSA) | + **Halo2 + Plonky3 + Binius + Stwo** |
| Output format | bisim + DRAT certs (separate) | **unified `.axc` certificate format** |
| User-facing surface | none — internal only | **`axiom-verify` + 3 SDKs + bug-bounty pilot** |
| GPU usage | none | **GPU-accelerated zk-SNARK proving (10–100×)** |
| Active groups | G1, G2, G3, G4, G5, G6, G8, G13 | + **G7 (Proof Systems & Cryptography), G12 (Supply Chain), G14 (Verifier, SDKs & Tooling)** |
| Headcount | ~32 engineers | **~42 engineers** |
| Paper | IEEE S&P / NDSS (intent resolution) | **CCS / S&P (proof-carrying APKs)** |
| Verification audience | research-internal | **bug-bounty triagers, app-store ingest, CI pipelines** |

---

<a id="glance"></a>
## 3. The 20 Sub-Phases at a Glance

| # | Sub-phase | Owner(s) | Weeks (≈) | Hard dep on |
|---|---|---|---|---|
| [P4.1](./P4.1/README.md) | Phase 3 carry-forward + G7 + G12 + G14 onboarding + Phase 4 kickoff | All + new G7+G12+G14 | W1–W2 | P3.20 |
| [P4.2](./P4.2/README.md) | `.axc` certificate format RFC v1 | G7 | W1–W4 | P4.1 |
| [P4.3](./P4.3/README.md) | zk-SNARK solver-pool: Halo2 / Plonky3 / Binius | G7 | W2–W6 | P4.1 |
| [P4.4](./P4.4/README.md) | Privacy-invariant compilation pipeline (Lean → Halo2) | G7+G1 | W4–W10 | P4.3 |
| [P4.5](./P4.5/README.md) | Privacy invariant 1 — `READ_CONTACTS` Halo2 circuit | G7 | W6–W11 | P4.4 |
| [P4.6](./P4.6/README.md) | Privacy invariant 2 — Network-destination allowlist Halo2 circuit | G7 | W7–W12 | P4.4 |
| [P4.7](./P4.7/README.md) | Privacy invariant 3 — Location-without-network Halo2 circuit | G7 | W8–W13 | P4.4 |
| [P4.8](./P4.8/README.md) | Privacy invariant 4 — Device-identifier read forbidden Halo2 circuit | G7 | W9–W14 | P4.4 |
| [P4.9](./P4.9/README.md) | Privacy invariant 5 — TFLite model integrity Halo2 circuit | G7 | W10–W15 | P4.4 |
| [P4.10](./P4.10/README.md) | STARK / Stwo fallback pipeline (post-quantum) | G7 | W8–W14 | P4.3 |
| [P4.11](./P4.11/README.md) | `axiom-verify` reference verifier core (Rust) | G7+G14 | W6–W14 | P4.2 |
| [P4.12](./P4.12/README.md) | `axiom-verify` Wasm + ARM64 mobile builds | G14 | W12–W18 | P4.11 |
| [P4.13](./P4.13/README.md) | SDK: `axiom-py` (PyO3 + uniffi) | G14 | W14–W18 | P4.11 |
| [P4.14](./P4.14/README.md) | SDK: `axiom-go` (cgo + uniffi) | G14 | W14–W18 | P4.11 |
| [P4.15](./P4.15/README.md) | SDK: `axiom-ts` (Wasm + wit-bindgen) | G14 | W14–W18 | P4.12 |
| [P4.16](./P4.16/README.md) | SLSA L4 attestation + reproducible-build verification | G12 | W6–W18 | P4.1 |
| [P4.17](./P4.17/README.md) | Bug-bounty pilot platform integration | G14 + partner | W16–W22 | P4.11, P4.13 |
| [P4.18](./P4.18/README.md) | Phase-4 E2E: full pipeline + cert + verifier + SDKs + soak + cross-arch | All | W18–W22 | P4.5–P4.10, P4.16, P4.17 |
| [P4.19](./P4.19/README.md) | CCS / S&P paper draft + `.axc` spec publication | All | W20–W24 | P4.18 |
| [P4.20](./P4.20/README.md) | Phase 4 hard-gate review + Phase 5 ADR | Lead + all | W24–W26 | P4.19 |

> **Each sub-phase folder above contains a self-contained README** with the same uniform template as Phases 1–3.

---

<a id="deps"></a>
## 4. Sub-Phase Dependency Diagram

```
                 ┌──────────── P4.1 ──────────────┐
                 │  Onboarding (G7, G12, G14) +   │
                 │  carry-forward                 │
                 └─┬─────┬──────┬─────┬───────────┘
                   │     │      │     │
                   ▼     ▼      ▼     ▼
                 P4.2  P4.3  P4.16  P4.10
                 (axc  (zk    (SLSA)  (STARK
                  RFC) pool)         fallback)
                   │     │      │
                   ▼     ▼      │
                 P4.11  P4.4   │
                 (verify (Lean→Halo2)
                  core)  │
                   │     ├──┬──┬──┬──┐
                   │     ▼  ▼  ▼  ▼  ▼
                   │   P4.5 P4.6 P4.7 P4.8 P4.9 (5 privacy invariants)
                   │
                   ├──► P4.12 (Wasm + mobile)
                   │      │
                   │      ▼
                   │    P4.15 (axiom-ts)
                   │
                   ├──► P4.13 (axiom-py)
                   │
                   ├──► P4.14 (axiom-go)
                   │
                   ▼
                 P4.17 (bug-bounty pilot)
                   │
                   ▼
                 P4.18 (E2E)
                   ▼
                 P4.19 (paper + spec)
                   ▼
                 P4.20 (gate review)
```

---

<a id="cross-cutting"></a>
## 5. Cross-Cutting Conditions (Always True From W1)

| Condition | Owner | Verification |
|---|---|---|
| Buck2 hermetic CI: every PR build byte-identical | G13 | CI gate (continued from P1.1) |
| Lean theorem re-verify on every L1/L4/L5/L6 PR | G1 + G5 + G6 + G7 + G13 | CI gate, fail-closed |
| HACL\* on the verified-crypto path; no generic | G2 | Build-system check |
| All tools pinned via Nix flake (incl. zk-SNARK proving keys) | G13 | `nix flake lock` reviewed quarterly |
| Differential fuzzer ≥ 99 % uptime across 5 AOSP harnesses | G8 | Pyroscope + Prometheus |
| AXIOM-IR-v0.2 spec frozen | G3 | ADR review for any change |
| **NEW: Halo2 / Plonky3 / Binius / Stwo proving keys pinned by content hash** | G7 | Reproducibility gate |
| **NEW: GPU-acceleration test on every cert-emit PR** | G7 + G13 | CI gate |
| **NEW: `axiom-verify` p99 ≤ 100 ms continuous regression test** | G14 | CI gate |
| **NEW: `.axc` v1 wire-format frozen after P4.2** | G7 | ADR for any change |

---

<a id="exit-gate"></a>
## 6. Phase 4 Consolidated Exit Gate

```
Onboarding & RFCs (P4.1, P4.2)
[ ] G7 + G12 + G14 staffed
[ ] Carry-forward debt from Phase 3 resolved or re-classified
[ ] .axc v1 RFC frozen ≥ 4 weeks before P4.18
[ ] zk-SNARK pool integrated (Halo2 + Plonky3 + Binius + Stwo)

Halo2 circuits (P4.4, P4.5–P4.9)
[ ] Lean → Halo2 compilation pipeline operational
[ ] All 5 priority privacy invariants ship as Halo2 circuits
[ ] GPU acceleration via sppark / icicle 10× over CPU baseline
[ ] All circuits have soundness theorems linked to Lean

STARK fallback (P4.10)
[ ] Stwo proving + verification working
[ ] STARK ↔ Halo2 result equivalence verified

Verifier (P4.11, P4.12)
[ ] axiom-verify Rust core p99 ≤ 100 ms over 10K certs
[ ] Wasm build p99 ≤ 300 ms in Chromium 122+
[ ] ARM64 mobile build p99 ≤ 200 ms on Pixel-class

SDKs (P4.13, P4.14, P4.15)
[ ] axiom-py + axiom-go + axiom-ts all pass integration suite
[ ] FFI overhead < 30 % per SDK
[ ] All SDKs generated from single Rust source via uniffi + wit-bindgen + cgo

SLSA + reproducible builds (P4.16)
[ ] SLSA L4 verification operational
[ ] Reproducible-build verification round-trips with F-Droid

Bug-bounty pilot (P4.17)
[ ] Pilot platform live in production
[ ] Ingesting ≥ 500 .axc / hour
[ ] Triager render ≤ 2 s

E2E (P4.18 — PHASE_GATES.md §8 hards)
[ ] L0–L6 sustained ≥ 10 APKs/sec on 16-core
[ ] Verifier service throughput ≥ 3K verifications/sec
[ ] Cold start ≤ 500 ms
[ ] Cert size median ≤ 100 KB
[ ] Cross-arch byte-identical certs 100 %
[ ] 7-day soak: zero crashes
[ ] Reproducibility 100 % across runs + architectures

Publication (P4.19)
[ ] .axc v1 spec publicly published (RFC + Cap'n Proto schema)
[ ] CCS / S&P paper draft ≥ 12 pages
[ ] Reproducibility Docker image published

Phase 5 readiness (P4.20)
[ ] Phase 5 scope ADR approved
[ ] G9 + G10 + G11 hiring plan locked
[ ] Sign-off from all group leads + leadership
[ ] Release tag `phase-4-complete` signed via cosign
```

---

<a id="risks"></a>
## 7. Phase 4 Risk Register

| Risk | Impact | Probability | Sub-phase | Mitigation |
|---|---|---|---|---|
| G7 hiring slow (Halo2 / zk-SNARK cryptographers scarce) | Critical | High | All G7 sub-phases | Pre-Phase-4 sourcing; contractor budget; Plonky3-only fallback |
| Halo2 proving time too slow on real circuits | High | Medium | P4.5–P4.9 | GPU acceleration via sppark / icicle; Binius for hash-heavy circuits; Plonky3 alternative |
| zk-SNARK patent landscape changes (Halo2 has favorable IP, but others) | Medium | Low | P4.5–P4.9 | Quarterly legal review; STARK fallback as patent-clean default |
| `.axc` format spec churns mid-phase | High | Medium | P4.2 | Strong RFC review process; ADR-required for any change post-P4.2 |
| `axiom-verify` p99 fails ≤ 100 ms gate | High | Medium | P4.11 | Profiling continuous via Pyroscope; Wasm vs native budget tunable per platform |
| SLSA L4 reproducibility issues with non-F-Droid samples | Medium | Medium | P4.16 | Document edge cases; F-Droid as authoritative initial corpus |
| Bug-bounty pilot partner pulls out | Medium | Low | P4.17 | Multi-partner conversations early; HackerOne + Bugcrowd in parallel |
| Mobile Wasm performance regression on iOS Safari | Medium | Low | P4.12 | Multi-runtime test matrix; Chromium primary, WebKit secondary |
| Cross-RISC-V parity ≤ 50 % gate slips | Medium | Medium | P4.18 | RISC-V silicon limited; document gracefully |

---

<a id="defs"></a>
## 8. Definitions

Same as Phase 1–3. New terms:

- **`.axc` (Apkaxiom Certificate)** — the unified, signed, content-addressed certificate format produced by Layer 6. Carries all proof artifacts from L1 through L5 plus metadata.
- **Privacy invariant** — a universally-quantified property over an APK's runtime behavior, provable via zk-SNARK without re-executing the APK on the verifier.
- **Proving key (PK) / Verifying key (VK)** — the public parameters of a zk-SNARK scheme; derived from the trusted-setup ceremony.
- **GPU acceleration via sppark / icicle** — NVIDIA / AMD MSM (multi-scalar multiplication) and NTT (number-theoretic transform) kernels that speed up zk-proving 10–100× over CPU.

---

*"Phase 4 makes APKAXIOM's findings citable. Before Phase 4, a finding is an internal artifact; after Phase 4, it's an `.axc` file a bug-bounty triager runs through `axiom-verify` and gets ✅ in milliseconds."*
