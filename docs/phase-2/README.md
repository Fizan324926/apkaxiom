# Phase 2 — Bundle Era Detailed Plan (M6 → M12)

> The 6 months that take APKAXIOM from "verified APK parser" to "verified bundle-era analysis platform with structural forensics." 20 sub-phases (P2.1 → P2.20).
> Each sub-phase has its own folder with: identity, goal/scope, dependencies, tools, third-party services & API keys with free/paid status, system inventory, **all features & functions delivered (comprehensive)**, **explicit numeric KPIs**, end-to-end test, and exit checklist.

This document is the operational complement to:
- [../../README.md](../../README.md) — architecture
- [../ROADMAP.md](../ROADMAP.md) — high-level Phase 2 goals
- [../PHASE_GATES.md](../PHASE_GATES.md#phase-2) — Phase 2 numeric KPI gates
- [../TECH_STACK.md](../TECH_STACK.md) — tech-stack picks

---

## Table of Contents

1. [Phase 2 Goal Statement](#goal)
2. [What's New in Phase 2 vs Phase 1](#whats-new)
3. [The 20 Sub-Phases at a Glance](#glance)
4. [Sub-Phase Dependency Diagram](#deps)
5. [Cross-Cutting Conditions (always true)](#cross-cutting)
6. [Phase 2 Consolidated Exit Gate](#exit-gate)
7. [Phase 2 Risk Register](#risks)
8. [Definitions](#defs)

---

<a id="goal"></a>
## 1. Phase 2 Goal Statement

By the end of Phase 2 (M12), the project must have:

- **Lean coverage extended** to the AXML binary-XML parser, the ARSC resource-table parser, and a DEX bytecode opcode subset. AOSP versions covered: **A8, A11, A12, A13, A14**.
- **AXIOM-IR v0.2** frozen — adds the DEX dialect, expands manifest and resource dialects, finalizes lowerings.
- **Schrödinger APK formal semantics** mechanized in Lean — the bundle composition operator `⊕` and the BehaviorSet inclusion theorems.
- **App Bundle (AAB) parser + bundle resolver** in Rust — base + ABI splits + density splits + language splits + dynamic feature modules + asset packs.
- **Layer 3 forensics** (G4) shipped: Shadow Stack, AXML Compiler Provenance fingerprinting, Negative-Space Resource Anomaly detector. Combined FP rate < 12 %.
- **Differential fuzzing plant** scaled to **5 AOSP versions** (A8, A11, A12, A13, A14) with **Nautilus grammar-aware** mutation. ≥ 30 disagreements/week classified.
- **Bundle differential testing** vs the AOSP installer on ≥ 5,000 App Bundles, ≥ 99.9 % agreement.
- **Phase-2 paper drafted** — *"Rethinking the Unit of Analysis for Android Security in the App Bundle Era"* — for **USENIX Security 2027** or **NDSS 2027**.
- **All Phase 2 hard KPIs** from PHASE_GATES.md §6 green for ≥ 7 consecutive days.

---

<a id="whats-new"></a>
## 2. What's New in Phase 2 vs Phase 1

| Area | Phase 1 | Phase 2 |
|---|---|---|
| AOSP coverage | A14 (+ partial A8/A11) | **A8, A11, A12, A13, A14** — five versions, equally covered |
| Lean trust core | ZIP layer + APK Signing Block | + **AXML parser + ARSC parser + DEX opcode subset** |
| AXIOM-IR | v0.1 (manifest + resource dialects) | **v0.2** — adds DEX dialect, expands manifest + resource |
| Bundle handling | not addressed | **App Bundle parser + bundle resolver + Schrödinger semantics** |
| Forensics | not addressed | **Three Layer-3 sub-passes operational** (Shadow Stack, AXML Provenance, Negative-Space) |
| Fuzzing | 3 AOSP harnesses, manual classification | **5 AOSP harnesses + Nautilus grammar-aware + automated classifier** |
| Active groups | G1, G2, G3, G8, G13 | + **G4 (Structural Forensics)** |
| Headcount | ~20 engineers | **~24 engineers** |
| Paper | CAV / OOPSLA (verified parsing) | **USENIX Security / NDSS (bundle-era unit-of-analysis)** |

---

<a id="glance"></a>
## 3. The 20 Sub-Phases at a Glance

| # | Sub-phase | Owner(s) | Weeks (≈) | Hard dep on |
|---|---|---|---|---|
| [P2.1](./P2.1/README.md) | Phase 1 carry-forward + G4 onboarding + AOSP A12/A13 archaeology kickoff | All + new G4 | W1–W2 | P1.20 |
| [P2.2](./P2.2/README.md) | AXIOM-IR v0.2 spec planning + DEX dialect design | G3 | W1–W3 | P2.1 |
| [P2.3](./P2.3/README.md) | Lean AXML (binary XML) parser formalization | G1 | W2–W7 | P2.1 |
| [P2.4](./P2.4/README.md) | Lean ARSC (resource table) parser formalization | G1 | W3–W8 | P2.1 |
| [P2.5](./P2.5/README.md) | Rust extraction of AXML parser + axiom-l1-rs integration | G1+G2 | W6–W9 | P2.3 |
| [P2.6](./P2.6/README.md) | Rust extraction of ARSC parser + integration | G1+G2 | W7–W10 | P2.4 |
| [P2.7](./P2.7/README.md) | Lean DEX bytecode parser (opcode subset for Phase 2) | G1 | W4–W10 | P2.1, P2.2 |
| [P2.8](./P2.8/README.md) | Rust extraction of DEX parser + DEX dialect emitter | G1+G2 | W9–W12 | P2.7 |
| [P2.9](./P2.9/README.md) | AXIOM-IR v0.2 spec frozen (DEX dialect + extensions) | G3 | W10–W12 | P2.2, P2.8 |
| [P2.10](./P2.10/README.md) | Schrödinger APK formal semantics (Lean) — bundle composition `⊕` | G1+G3 | W6–W12 | P2.5, P2.6 |
| [P2.11](./P2.11/README.md) | App Bundle (AAB) parser — base + ABI/density/language splits | G2+G3 | W8–W14 | P2.5, P2.6 |
| [P2.12](./P2.12/README.md) | Bundle resolver: dynamic feature modules + asset packs | G3 | W12–W17 | P2.10, P2.11 |
| [P2.13](./P2.13/README.md) | Bundle differential testing vs AOSP installer | G3+G8 | W14–W18 | P2.12 |
| [P2.14](./P2.14/README.md) | Layer 3.1 — Shadow Stack (forensic deletion detection) | G4 | W6–W14 | P2.5, P2.6 |
| [P2.15](./P2.15/README.md) | Layer 3.2 — AXML Compiler Provenance fingerprint + classifier | G4 | W8–W16 | P2.5 |
| [P2.16](./P2.16/README.md) | Layer 3.3 — Negative-Space Resource Anomaly detector | G4 | W10–W17 | P2.6 |
| [P2.17](./P2.17/README.md) | Differential Fuzzer scale: A12+A13 harnesses + Nautilus grammar-aware | G8 | W4–W18 | P2.1 |
| [P2.18](./P2.18/README.md) | Phase-2 E2E: Bench-10K rerun + Bundles-5K + 24h soak + cross-arch | All | W18–W22 | P2.12, P2.16 |
| [P2.19](./P2.19/README.md) | USENIX/NDSS paper draft + AndroZoo bundle benchmark publication | All | W20–W24 | P2.18 |
| [P2.20](./P2.20/README.md) | Phase 2 hard-gate review + Phase 3 ADR + carry-forward debt rollup | Lead + all | W24–W26 | P2.19 |

> **Each sub-phase folder above contains a self-contained README** with: identity, goal/scope, hard dependencies, required tools and libraries with version pins, third-party services & API keys with free/paid status, system inventory (have vs need with install commands), comprehensive feature/function list, explicit numeric KPIs, working-directory file tree, standalone output, end-to-end test, and exit checklist.

---

<a id="deps"></a>
## 4. Sub-Phase Dependency Diagram

```
                    ┌─────────── P2.1 ───────────┐
                    │  Onboarding + AOSP arch.   │
                    └───┬──────┬──────┬──────────┘
                        │      │      │
                        ▼      ▼      ▼
                    P2.2    P2.3   P2.4   P2.7   P2.17
                    (IR     (AXML  (ARSC  (DEX   (Fuzzer
                     spec)   Lean) Lean) Lean)   scale)
                       │      │      │      │
                       │      ▼      ▼      ▼
                       │    P2.5   P2.6   P2.8
                       │  (extract)(extract)(extract)
                       │      │      │      │
                       └──────┴──────┴──────┴────► P2.9 (IR v0.2 freeze)
                                           │
                                           ▼
                              P2.10 (Schrödinger Lean) ── P2.11 (AAB parser)
                                           │                  │
                                           └──────┬───────────┘
                                                  ▼
                                           P2.12 (Bundle resolver)
                                                  │
                                                  ▼
                                           P2.13 (Bundle diff vs AOSP)

         (G4 forensics, parallel from W6+)
         P2.14 (Shadow Stack) ──┐
         P2.15 (AXML Provenance)│ ─►  feed into P2.18 E2E
         P2.16 (Negative-Space) ┘

                                    P2.18 (Phase 2 E2E eval)
                                          │
                                          ▼
                                    P2.19 (USENIX/NDSS paper)
                                          │
                                          ▼
                                    P2.20 (Phase 2 gate review)
```

---

<a id="cross-cutting"></a>
## 5. Cross-Cutting Conditions (Always True From W1)

These conditions remain green throughout Phase 2. Any breach is a P0 incident.

| Condition | Owner | Verification |
|---|---|---|
| Buck2 hermetic CI: every PR build byte-identical on 3 reference machines | G13 | CI gate, fail-closed (continued from P1.1) |
| Lean theorem re-verify on every L1 PR | G1 + G13 | CI gate, fail-closed (P1.17) |
| `cargo deny` / `cargo audit` clean | G13 | Per-PR |
| BLAKE3 + SHA-256 + Ed25519 + RSA via HACL\*; never generic | G2 | Build-system check (P1.10/P1.16) |
| All tools pinned via Nix flake | G13 | `nix flake lock` reviewed quarterly |
| Differential fuzzer running ≥ 99 % uptime across all current AOSP versions | G8 | Pyroscope + Prometheus |
| AXIOM-IR-v0.1 spec frozen (until v0.2 freeze in P2.9) | G3 | ADR review for any change |

---

<a id="exit-gate"></a>
## 6. Phase 2 Consolidated Exit Gate

A single checklist combining all 20 sub-phase outcomes plus the PHASE_GATES.md §6 hard KPIs. **Every box ✅ on the live dashboard for ≥ 7 consecutive days for Phase 2 to close and Phase 3 to start.**

```
Onboarding & Foundations (P2.1, P2.2, P2.7 entry)
[ ] G4 staffed and onboarded; first deliverables in flight
[ ] Carry-forward debt from Phase 1 resolved or re-classified to Phase 3
[ ] AOSP A12 + A13 archaeology complete; relevant deltas formalized
[ ] AXIOM-IR v0.2 design RFC published, reviewed by G1, G2, G3, G4, G5 leads

Lean trust-core extension (P2.3, P2.4, P2.7, P2.10)
[ ] AXML parser theorems proved; ≥ 1,500 LOC Lean
[ ] ARSC parser theorems proved; ≥ 1,500 LOC Lean
[ ] DEX opcode-subset theorems proved; ≥ 2,000 LOC Lean
[ ] Schrödinger APK semantics theorems proved; bundle composition `⊕` formalized
[ ] Cumulative Phase-2 Lean LOC ≥ 7,000

Rust extraction (P2.5, P2.6, P2.8)
[ ] AXML extraction byte-identical; translation validator green on 5K corpus
[ ] ARSC extraction byte-identical; translation validator green on 5K corpus
[ ] DEX extraction byte-identical; translation validator green on 10K corpus
[ ] All extractions reproducible bit-identical across 3 reference machines

AXIOM-IR v0.2 (P2.9)
[ ] AXIOM-IR-v0.2 spec frozen ≥ 4 weeks before P2.18
[ ] DEX dialect compiles and round-trips
[ ] Manifest + resource dialect extensions land
[ ] Lean reflection of v0.2 types re-verifies

Bundle handling (P2.11, P2.12, P2.13)
[ ] App Bundle parser: ≥ 99 % coverage on Bundles-5K
[ ] Bundle resolver: dynamic-feature-module discovery rate ≥ 95 %
[ ] Differential vs AOSP installer ≥ 99.9 % agreement
[ ] BehaviorSet memory representation ≤ 2.5× raw bundle size

Layer 3 forensics (P2.14, P2.15, P2.16)
[ ] Shadow Stack: FP rate < 10 % on benign 10K
[ ] AXML Provenance: misidentification rate < 5 %
[ ] Negative-Space: FP rate < 20 % on benign corpus
[ ] Combined forensic FP (any pass fires on benign) < 12 %
[ ] Each forensic pass throughput ≥ 300 APKs/sec on 16-core
[ ] Each forensic pass p99 ≤ 80 ms

Differential Fuzzer (P2.17)
[ ] 5 AOSP harnesses (A8, A11, A12, A13, A14) live, ≥ 99 % uptime each
[ ] Nautilus grammar-aware mutation in production
[ ] Disagreements/week classified ≥ 30 (TARGET: 100+)
[ ] Cross-version disagreements found and reproduced
[ ] ≥ 1 zero-day filed as CVE candidate from Phase-2 fuzzing

E2E (P2.18)
[ ] L0–L3 sustained throughput ≥ 150 APKs/sec on 16-core
[ ] L0–L3 p99 ≤ 800 ms
[ ] Bundle resolution overhead ≤ 60 % over single-APK
[ ] Peak RSS ≤ 300 MB per worker
[ ] 1→16 core efficiency ≥ 70 %
[ ] 24h soak: zero crashes
[ ] Cross-arch (x86_64 ↔ ARM64) verdicts identical
[ ] Reproducibility 100 % bit-identical Merkle + IR + verdict

Publication (P2.19)
[ ] AndroZoo bundle eval published (≥ 5K bundles, ≥ 99 % coverage)
[ ] Reproducibility Docker image published
[ ] USENIX/NDSS paper draft ≥ 12 pages

Phase 3 readiness (P2.20)
[ ] Carry-forward debt logged
[ ] Phase 3 scope ADR approved
[ ] Phase 2 retrospective merged
[ ] Sign-off from G1, G2, G3, G4, G8, G13 leads + leadership
[ ] Release tag `phase-2-complete` signed via cosign
```

---

<a id="risks"></a>
## 7. Phase 2 Risk Register

| Risk | Impact | Probability | Sub-phase exposed | Mitigation |
|---|---|---|---|---|
| G4 hiring slow (forensics + statistical analysis is a niche skill set) | High | Medium | P2.14, P2.15, P2.16 | Pre-Phase-2 sourcing; contractor budget; partial coverage acceptable in v0.x |
| AAB format spec drift (Google ships new bundle features) | High | Low | P2.11, P2.12 | Pin to A14 bundle format; track upstream quarterly |
| DEX opcode coverage insufficient for downstream G5 | High | Medium | P2.7, P2.8 | Scope DEX subset to what G5 needs first; expand opportunistically |
| Schrödinger formalization runs into undecidability | Medium | Medium | P2.10 | Constrain to feasible-configuration space; lift undecidable cases to UNKNOWN |
| Forensic FP rate exceeds gate on real-world corpus | High | Medium | P2.14, P2.15, P2.16 | Budget late-phase tuning sprint; allow scope reduction with ADR |
| Cuttlefish bundle install behavior drifts vs AOSP installer | Medium | Low | P2.13 | Pin Cuttlefish image hashes; quarterly re-validation |
| Nautilus grammar bug-storms (early grammar generates trivial divergences) | Medium | Medium | P2.17 | Triage budget; classifier learns to deduplicate |
| AOSP A12/A13 archaeology reveals breaking semantic changes | High | Medium | P2.1 | G2 archaeology runbook; dedicated arch sprint at W1–W2 |

---

<a id="defs"></a>
## 8. Definitions

Same as [phase-1/README.md §8](../phase-1/README.md#defs). New term:

- **BehaviorSet** — the formal Lean type representing the union of all programs that an App Bundle could materialize across feasible (ABI, density, language, dynamic-feature) configurations. Defined in P2.10. Used by every L3+ layer.

---

*"Phase 2 redefines what an APK is. Phase 1 made parsing sound; Phase 2 makes the unit of analysis right."*
