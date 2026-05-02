# Phase 3 — Symbolic & Equivalence Detailed Plan (M12 → M18)

> The 6 months that take APKAXIOM from "verified bundle-era parser" to "sound-and-complete intent resolution + obfuscation-invariant equivalence proofs." 20 sub-phases (P3.1 → P3.20).
> Each sub-phase has its own folder with: identity, goal/scope, dependencies, tools, third-party services & API keys with free/paid status, system inventory, **all features & functions delivered (comprehensive)**, **explicit numeric KPIs**, end-to-end test, and exit checklist.

This document is the operational complement to:
- [../../README.md](../../README.md) — architecture
- [../ROADMAP.md](../ROADMAP.md) — high-level Phase 3 goals
- [../PHASE_GATES.md](../PHASE_GATES.md#phase-3) — Phase 3 numeric KPI gates
- [../TECH_STACK.md](../TECH_STACK.md) — tech-stack picks (SMT, ZK, equivalence)

---

## Table of Contents

1. [Phase 3 Goal Statement](#goal)
2. [What's New in Phase 3 vs Phase 2](#whats-new)
3. [The 20 Sub-Phases at a Glance](#glance)
4. [Sub-Phase Dependency Diagram](#deps)
5. [Cross-Cutting Conditions (always true)](#cross-cutting)
6. [Phase 3 Consolidated Exit Gate](#exit-gate)
7. [Phase 3 Risk Register](#risks)
8. [Definitions](#defs)

---

<a id="goal"></a>
## 1. Phase 3 Goal Statement

By the end of Phase 3 (M18), the project must have:

- **Symbolic intent resolver (Layer 4)** in production — cvc5 + Spacer (CHC), Bitwuzla as QF_BV backend, Yices2 for linear-arithmetic shortcut. Models PackageManager state symbolically. Returns reachability proofs, UNSAT certificates, or explicit UNKNOWN — never silent over-approximation.
- **First sound-and-complete intent resolver** for a useful fragment of Android's intent system — paper-target language, formally backed.
- **Behavior Surface Hash (BSH-256)** specified as an RFC, frozen, reference Rust implementation shipped, LSH similarity index over **DiskANN** at 1M-vector scale.
- **Bounded bisimulation engine** (k-step, abstract domains: numeric / string / type) — produces verifiable equivalence witnesses or divergence reports. SMT-discharged proof obligations at each transition.
- **Cross-APK device-snapshot prototype** — Layer 4 reasons over *sets* of installed APKs, not just one APK at a time.
- **AXIOM-IR v0.3 (preview)** — symbolic dialect for L4/L5 (no full freeze in Phase 3; freeze in Phase 4).
- **Phase-3 paper drafted** — *"Sound and Complete Intent Resolution for Android"* — for **IEEE S&P 2027** or **NDSS 2028**.
- **All Phase 3 hard KPIs** from PHASE_GATES.md §7 green for ≥ 7 consecutive days.

---

<a id="whats-new"></a>
## 2. What's New in Phase 3 vs Phase 2

| Area | Phase 2 | Phase 3 |
|---|---|---|
| Layers under measurement | L0–L3 | **L0–L5** — adds symbolic resolver + equivalence layer |
| Solvers | none on critical path | **cvc5 + Bitwuzla + Yices2 + Spacer + Eldarica + Pono** (selected per query class) |
| Reasoning | structural / statistical | **symbolic + bisimulation** — proofs not heuristics |
| Equivalence | not addressed | **BSH-256 + bounded bisimulation** — obfuscation-invariant |
| Similarity search | not addressed | **DiskANN at 1M-vector scale + MinHash LSH** |
| Cross-APK analysis | not addressed | **device-snapshot prototype** — reason over installed app sets |
| Active groups | G1, G2, G3, G4, G8, G13 | + **G5 (Symbolic Execution & Intent Resolver), G6 (Equivalence & Fingerprinting)** |
| Headcount | ~24 engineers | **~32 engineers** |
| Paper | USENIX/NDSS (bundle era) | **IEEE S&P / NDSS (sound-and-complete intent resolution)** |
| KPI category newly active | — | **K11 soundness** continues; **K7 stability** tighter; new SMT-specific KPIs |

---

<a id="glance"></a>
## 3. The 20 Sub-Phases at a Glance

| # | Sub-phase | Owner(s) | Weeks (≈) | Hard dep on |
|---|---|---|---|---|
| [P3.1](./P3.1/README.md) | Phase 2 carry-forward + G5 + G6 onboarding + Phase 3 kickoff | All + new G5 + G6 | W1–W2 | P2.20 |
| [P3.2](./P3.2/README.md) | AOSP archaeology extension — intent-filter semantics across A8–A15 | G2 | W1–W4 | P3.1 |
| [P3.3](./P3.3/README.md) | AXIOM-IR-symbolic dialect design (preview) | G3+G5 | W2–W6 | P3.1 |
| [P3.4](./P3.4/README.md) | PackageManager state model in Lean | G1+G5 | W3–W9 | P3.2 |
| [P3.5](./P3.5/README.md) | Intent-filter resolution semantics in Lean | G1+G5 | W5–W11 | P3.4 |
| [P3.6](./P3.6/README.md) | cvc5 / Z3 / Bitwuzla / Spacer integration layer | G5 | W2–W6 | P3.1 |
| [P3.7](./P3.7/README.md) | CHC encoding of intent resolution (Spacer / Eldarica) | G5 | W6–W11 | P3.5, P3.6 |
| [P3.8](./P3.8/README.md) | Symbolic intent resolver L4 — single-APK first cut | G5 | W9–W14 | P3.7 |
| [P3.9](./P3.9/README.md) | Cross-APK device-snapshot prototype | G5 (sub-team) | W12–W17 | P3.8 |
| [P3.10](./P3.10/README.md) | Abstraction-domain library (numeric, string, type) | G5+G6 | W4–W12 | P3.6 |
| [P3.11](./P3.11/README.md) | UNKNOWN handling + abstraction-refinement loop | G5 | W12–W17 | P3.8, P3.10 |
| [P3.12](./P3.12/README.md) | DRAT-style UNSAT certificate emission from cvc5 | G5+G7 (preview) | W11–W15 | P3.7 |
| [P3.13](./P3.13/README.md) | Behavior Surface Hash (BSH-256) RFC freeze | G6 | W4–W12 | P3.1 |
| [P3.14](./P3.14/README.md) | BSH-256 Rust impl + DiskANN similarity index (1M-vector scale) | G6 | W10–W15 | P3.13 |
| [P3.15](./P3.15/README.md) | Bounded bisimulation engine — abstract-domain composition | G6 | W10–W17 | P3.10, P3.13 |
| [P3.16](./P3.16/README.md) | Bisim witness emission + equivalence certificate format | G6+G7 (preview) | W14–W17 | P3.15 |
| [P3.17](./P3.17/README.md) | Layer 5 integration — BSH + bisim + LSH unified | G6 | W15–W18 | P3.14, P3.16 |
| [P3.18](./P3.18/README.md) | Phase-3 E2E: Bench-10K + Repack-2K + Snapshots + Soak + Cross-arch | All | W18–W22 | P3.9, P3.17 |
| [P3.19](./P3.19/README.md) | IEEE S&P / NDSS paper draft + Repack-2K eval publication | All | W20–W24 | P3.18 |
| [P3.20](./P3.20/README.md) | Phase 3 hard-gate review + Phase 4 ADR | Lead + all | W24–W26 | P3.19 |

> **Each sub-phase folder above contains a self-contained README** with the same uniform template as Phase 1 and Phase 2.

---

<a id="deps"></a>
## 4. Sub-Phase Dependency Diagram

```
                    ┌──────────── P3.1 ─────────────┐
                    │  Onboarding (G5, G6) + AOSP   │
                    └─┬───────┬────────┬────────┬───┘
                      │       │        │        │
                      ▼       ▼        ▼        ▼
                  P3.2     P3.3     P3.6     P3.13     P3.10
                  (AOSP)  (IR-sym  (Solver  (BSH RFC) (Abstract
                          dialect) integ.)            domains)
                    │       │        │        │         │
                    ▼       ▼        ▼        ▼         ▼
                  P3.4 ──► P3.5 ──► P3.7 ──► P3.14    P3.15
                  (PM     (IF res.)(CHC)    (BSH+    (Bisim
                  Lean)                      DiskANN) engine)
                                    │                  │
                                    ▼                  ▼
                                  P3.8 ──► P3.9     P3.16
                                  (L4     (Cross-   (Witnesses)
                                  single)  APK)
                                    │                  │
                                    ▼                  ▼
                                  P3.11             P3.17
                                  (UNKNOWN          (L5 integ.)
                                   refine)
                                    │                  │
                                    └──────┬───────────┘
                                           ▼
                                     P3.18 (E2E)
                                           ▼
                                     P3.19 (paper)
                                           ▼
                                     P3.20 (gate review)
                                           │
                  P3.12 (UNSAT certs) ─────┘  (cross-cutting)
```

---

<a id="cross-cutting"></a>
## 5. Cross-Cutting Conditions (Always True From W1)

| Condition | Owner | Verification |
|---|---|---|
| Buck2 hermetic CI: every PR build byte-identical | G13 | CI gate (continued from P1.1) |
| Lean theorem re-verify on every L1/L4/L5 PR | G1 + G5 + G6 + G13 | CI gate, fail-closed |
| HACL\* on the verified-crypto path; no generic | G2 | Build-system check (continued from P1.10/P1.16) |
| All tools pinned via Nix flake | G13 | `nix flake lock` reviewed quarterly |
| Differential fuzzer ≥ 99 % uptime across 5 AOSP harnesses | G8 | Pyroscope + Prometheus |
| AXIOM-IR-v0.2 spec frozen (no churn for L4/L5 work) | G3 | ADR review for any change |
| **NEW: SMT solver pinned per phase via Nix flake** | G13 + G5 | cvc5/Z3/Bitwuzla/Yices2/Spacer/Eldarica all version-pinned |
| **NEW: Solver-query timeout enforced (60s default, 5s in production)** | G5 | CI guard prevents unbounded queries |
| **NEW: BSH spec frozen after P3.13** | G6 | ADR for any change |

---

<a id="exit-gate"></a>
## 6. Phase 3 Consolidated Exit Gate

A single checklist combining all 20 sub-phase outcomes plus PHASE_GATES.md §7 hard KPIs. **Every box ✅ on the live dashboard for ≥ 7 consecutive days.**

```
Onboarding & Foundations (P3.1, P3.2, P3.3, P3.6)
[ ] G5 + G6 staffed and onboarded
[ ] Carry-forward debt from Phase 2 resolved or re-classified to Phase 4
[ ] AOSP archaeology covers intent-filter semantics across A8–A15
[ ] AXIOM-IR-symbolic dialect designed (preview; full freeze in Phase 4)
[ ] cvc5 + Z3 + Bitwuzla + Yices2 + Spacer + Eldarica integrated, pinned via Nix

Lean L4 trust-core (P3.4, P3.5)
[ ] PackageManager state model formalized
[ ] Intent-filter resolution semantics formalized
[ ] Soundness of resolver vs AOSP intent-resolution behavior

Symbolic resolver (P3.7, P3.8, P3.9, P3.11, P3.12)
[ ] CHC encoding of intent resolution complete
[ ] Single-APK symbolic resolver shipped
[ ] Cross-APK device-snapshot prototype shipped
[ ] UNKNOWN handling + refinement loop operational
[ ] DRAT UNSAT certificates emitted from cvc5

Equivalence & fingerprinting (P3.13, P3.14, P3.15, P3.16, P3.17)
[ ] BSH-256 RFC frozen (≥ 4 weeks before P3.18)
[ ] BSH Rust impl + DiskANN similarity index over 1M vectors
[ ] Bounded bisimulation engine + abstract-domain library
[ ] Bisim witness emission + equivalence certificates
[ ] Layer 5 unified surface (BSH + bisim + LSH)

KPIs (PHASE_GATES.md §7)
[ ] L0–L5 sustained ≥ 20 APKs/sec on 16-core
[ ] L0–L5 p99 ≤ 8 s
[ ] Symbolic intent query p99 ≤ 500 ms
[ ] BSH compute p99 ≤ 30 ms
[ ] Bisim per-pair p99 ≤ 2 s
[ ] LSH lookup p99 (1M index) ≤ 200 ms
[ ] L4 UNKNOWN rate ≤ 25 % on benign 5K
[ ] L4 UNSAT correctness 100 %
[ ] BSH collision rate < 0.1 % across 50K APKs
[ ] BSH stability ≥ 90 % across ProGuard/R8/DexGuard
[ ] Bisim TP ≥ 85 %, FP < 1 % on Repack-2K
[ ] Solver timeouts < 5 %
[ ] Peak RSS ≤ 1 GB per worker
[ ] LSH index for 1M APKs ≤ 8 GB
[ ] 1→16 core efficiency ≥ 60 %
[ ] 7-day soak: zero crashes
[ ] Cross-arch verdicts identical
[ ] Reproducibility 100 %

Publication (P3.19)
[ ] Repack-2K eval published
[ ] ≥ 100 known intent-hijack vulnerabilities reproduced as proofs
[ ] ≥ 1 zero-day intent-hijack discovered via cross-APK analysis
[ ] IEEE S&P / NDSS paper draft ≥ 12 pages
[ ] Reproducibility Docker image published

Phase 4 readiness (P3.20)
[ ] Carry-forward debt logged
[ ] Phase 4 scope ADR approved
[ ] G7 hiring plan + start dates
[ ] Phase 4 budget approved
[ ] Phase 3 retrospective merged
[ ] Sign-off from G1, G2, G3, G4, G5, G6, G8, G13 leads + leadership
[ ] Release tag `phase-3-complete` signed via cosign
```

---

<a id="risks"></a>
## 7. Phase 3 Risk Register

| Risk | Impact | Probability | Sub-phase | Mitigation |
|---|---|---|---|---|
| G5 hiring slow (program-analysis + SMT background scarce) | Critical | Medium | All G5 sub-phases | Pre-Phase-3 sourcing; contractor budget; partial Z3-only fallback if Spacer expertise thin |
| cvc5 hits hard cases that break solver scaling | High | Medium | P3.7, P3.8 | Bitwuzla as bitvector backend; Yices2 for linear-arithmetic shortcut; Pono fallback |
| Spacer / Eldarica fails to terminate on real instances | High | Medium | P3.7 | Bounded-iteration mode; explicit timeouts; UNKNOWN classification |
| L4 UNKNOWN rate exceeds gate (> 25 %) | High | Medium | P3.8, P3.11 | Tighter intent-fragment scope; abstraction-refinement budget |
| BSH collision rate exceeds gate | High | Low | P3.13 | RFC review with external cryptographers; include "salt" option |
| DiskANN at 1M-vector scale fails to meet p99 | Medium | Medium | P3.14 | HNSW fallback for smaller deployments; sharding strategy |
| Bisimulation explodes on real inter-component graphs | High | Medium | P3.15 | k-step bound tunable; abstract-domain library shoulders |
| Cross-APK analysis state-space explosion | High | High | P3.9 | Snapshot-budget abstraction; consent-gated for large fleets |
| AOSP intent-filter semantics changes mid-phase | High | Low | P3.5 | Pin to A14 baseline; deltas tracked separately |
| zk-SNARK preview slips into Phase 4 (UNSAT cert format) | Medium | Medium | P3.12 | DRAT-only in Phase 3; zk-SNARK envelope in Phase 4 |

---

<a id="defs"></a>
## 8. Definitions

Same as [phase-1/README.md §8](../phase-1/README.md#defs) and [phase-2/README.md §8](../phase-2/README.md#defs). New terms:

- **Reachability proof** — a concrete device state + install order witnessing that an Intent resolves to a particular component.
- **UNSAT certificate** — DRAT-style proof object emitted by the SMT solver when no witness exists; independently checkable.
- **UNKNOWN (with abstraction-domain marker)** — the resolver's explicit-incompleteness signal; never a silent over-approximation.
- **BSH-256** — the Behavior Surface Hash. A 256-bit canonical hash over the manifest's behavior surface (sorted permissions + intent-filters + exported components + dangerous-API set). Obfuscation-invariant by construction.
- **Bisim witness** — a finite relation between abstract states with discharged side conditions, proving that two APKs are equivalent up to k transitions.
- **Device snapshot** — a set of installed APKs treated as one analysis input; cross-APK reasoning reasons over the union.

---

*"Phase 3 makes APKAXIOM the first APK analyzer that says *here is the proof* instead of *probably yes*."*
