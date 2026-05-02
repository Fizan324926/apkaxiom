# Phase 5 — Native + Dynamic + ML Detailed Plan (M24 → M30)

> The 6 months that take APKAXIOM from "Java-only proof stack with cert format & SDKs" to "joint Java+native+dynamic+ML proof stack — every layer of an Android app is reasoned over." 20 sub-phases (P5.1 → P5.20).
> Each sub-phase has its own folder with: identity, goal/scope, dependencies, tools, third-party services & API keys with free/paid status, system inventory, **all features & functions delivered**, **explicit numeric KPIs**, end-to-end test, exit checklist.

This document is the operational complement to:
- [../../README.md](../../README.md) — architecture
- [../ROADMAP.md](../ROADMAP.md) — high-level Phase 5 goals
- [../PHASE_GATES.md](../PHASE_GATES.md#phase-5) — Phase 5 numeric KPI gates
- [../TECH_STACK.md](../TECH_STACK.md) — tech-stack picks (lifters, emulators, ML)

---

## Table of Contents

1. [Phase 5 Goal Statement](#goal)
2. [What's New in Phase 5 vs Phase 4](#whats-new)
3. [The 20 Sub-Phases at a Glance](#glance)
4. [Sub-Phase Dependency Diagram](#deps)
5. [Cross-Cutting Conditions (always true)](#cross-cutting)
6. [Phase 5 Consolidated Exit Gate](#exit-gate)
7. [Phase 5 Risk Register](#risks)
8. [Definitions](#defs)

---

<a id="goal"></a>
## 1. Phase 5 Goal Statement

By the end of Phase 5 (M30), the project must have:

- **DEX bytecode lifter to AXIOM-IR** (G9) — SSA-form, type-checked, lossless. Coverage ≥ 95 % files on Bench-10K, ≥ 99 % target.
- **ARM64 ELF lifter to AXIOM-IR** (G9) — built on LLVM MLIR. Handles common Android NDK code patterns (JNI bridges, dlopen, common libc, OpenSSL, BoringSSL). Function-level coverage ≥ 60 % on NDK-100 corpus.
- **ARMv7 ELF lifter** (G9) — legacy support, ≥ 50 % coverage required for v1.0 (drop to v1.1 if blocked).
- **Joint Java + native intent analysis** — G5's symbolic resolver follows JNI calls into native code via G9's lift. ≥ 1 cross-language vulnerability discovered that Java-only analyzers miss.
- **Frida + eBPF dynamic confirmation bridge** (G10) — when L4 returns UNKNOWN, drop into a sandboxed Android emulator, run Frida + eBPF traces, refine the static abstraction. Resolves ≥ 30 % (HARD) / ≥ 60 % (TARGET) of UNKNOWN findings.
- **Android emulator orchestration pool** (G10) — pod-based, chaos-drilled, parallel APKs ≥ 8 simultaneous on 16-core; cold-start ≤ 120 s.
- **TFLite integrity layer** (G11) — structural model hash, Neural Cleanse, STRIP, adversarial robustness scoring. Backdoor detection precision ≥ 90 %.
- **Lean theorems for native lifter soundness** (G1 + G9) — at minimum, the JNI boundary modeling and DEX → SSA correctness theorems checked.
- **AXIOM-IR v0.4 native dialect frozen** (G3 + G9) — SSA values, type system, JNI boundary nodes, calling-convention metadata.
- **Phase-5 paper drafted** — *"Joint Static-Dynamic Analysis of Android Native Code"* — for **NDSS 2028** or **RAID 2028**.
- **All Phase 5 hard KPIs** from PHASE_GATES.md §9 green for ≥ 7 consecutive days.

---

<a id="whats-new"></a>
## 2. What's New in Phase 5 vs Phase 4

| Area | Phase 4 | Phase 5 |
|---|---|---|
| Code reasoned over | Java / DEX (Smali level only) + manifest + resources | **+ DEX SSA + ARM64 ELF + ARMv7 ELF** (full bytecode + native binary) |
| Dynamic data | none (pure static) | **+ Frida + eBPF traces from sandboxed emulator** |
| ML model security | none | **TFLite structural hash + Neural Cleanse + STRIP + adversarial robustness** |
| Solver scope | Java-only intent resolution | **Java + JNI + native intent dispatch** |
| Active groups | + G7, G12, G14 | + **G9 (Native Code), G10 (Dynamic Analysis), G11 (ML Security)** |
| Headcount | ~42 engineers | **~52 engineers** |
| Hardware | GPU pool (zk-proving) | + **emulator farm** (KVM/ARM-on-cloud, ≥ 32 emulators steady-state) |
| Paper | CCS / S&P (proof-carrying APKs) | **NDSS / RAID (joint static-dynamic on Android native)** |
| KPI scope | L0–L6 + verifier + SDKs | **L0–L6 + native subsystem + dynamic + ML** |

---

<a id="glance"></a>
## 3. The 20 Sub-Phases at a Glance

| # | Sub-phase | Owner(s) | Weeks (≈) | Hard dep on |
|---|---|---|---|---|
| [P5.1](./P5.1/README.md) | Phase 4 carry-forward + G9 + G10 + G11 onboarding + Phase 5 kickoff | All + new G9 + G10 + G11 | W1–W2 | P4.20 |
| [P5.2](./P5.2/README.md) | AXIOM-IR-v0.4 native dialect (DEX SSA + ELF) design freeze | G3 + G9 | W1–W4 | P5.1 |
| [P5.3](./P5.3/README.md) | DEX bytecode lifter to AXIOM-IR | G9 | W2–W10 | P5.2 |
| [P5.4](./P5.4/README.md) | ARM64 ELF lifter to AXIOM-IR (LLVM MLIR) | G9 | W3–W14 | P5.2 |
| [P5.5](./P5.5/README.md) | ARMv7 ELF lifter (legacy) | G9 | W6–W15 | P5.4 |
| [P5.6](./P5.6/README.md) | JNI bridge modeling (Java↔native boundary) | G9 + G5 | W6–W14 | P5.3, P5.4 |
| [P5.7](./P5.7/README.md) | Native common-library catalog (libc, OpenSSL, BoringSSL, NDK patterns) | G9 | W4–W14 | P5.4 |
| [P5.8](./P5.8/README.md) | Joint Java + native intent analyzer | G5 + G9 | W12–W18 | P5.6, P5.7 |
| [P5.9](./P5.9/README.md) | Lean theorems for native lifter soundness | G1 + G9 | W4–W18 | P5.2 |
| [P5.10](./P5.10/README.md) | Android emulator orchestration pool | G10 | W1–W10 | P5.1 |
| [P5.11](./P5.11/README.md) | Frida script library + auto-attach | G10 | W6–W14 | P5.10 |
| [P5.12](./P5.12/README.md) | eBPF program library for kernel-level tracing | G10 | W6–W14 | P5.10 |
| [P5.13](./P5.13/README.md) | Dynamic confirmation bridge — UNKNOWN refinement | G5 + G10 | W12–W18 | P5.11, P5.12 |
| [P5.14](./P5.14/README.md) | TFLite model parser + structural integrity hash | G11 | W2–W8 | P5.1 |
| [P5.15](./P5.15/README.md) | Neural Cleanse backdoor scan | G11 | W6–W14 | P5.14 |
| [P5.16](./P5.16/README.md) | STRIP backdoor scan | G11 | W8–W14 | P5.14 |
| [P5.17](./P5.17/README.md) | Adversarial robustness scoring | G11 | W10–W16 | P5.14 |
| [P5.18](./P5.18/README.md) | Phase-5 E2E: full pipeline + native + dynamic + ML + soak + cross-arch | All | W18–W22 | P5.5–P5.17 |
| [P5.19](./P5.19/README.md) | NDSS / RAID paper draft + native+dynamic eval publication | All | W20–W24 | P5.18 |
| [P5.20](./P5.20/README.md) | Phase 5 hard-gate review + Phase 6 ADR | Lead + all | W24–W26 | P5.19 |

> **Each sub-phase folder above contains a self-contained README** with the same uniform template as Phases 1–4.

---

<a id="deps"></a>
## 4. Sub-Phase Dependency Diagram

```
                 ┌──────────── P5.1 ──────────────┐
                 │  Onboarding (G9, G10, G11) +   │
                 │  carry-forward                 │
                 └─┬─────┬──────┬─────┬───────────┘
                   │     │      │     │
                   ▼     ▼      ▼     ▼
                P5.2   P5.10  P5.14   P5.9 (Lean for native)
                (IR     (emu  (TFLite
                native) pool)  hash)
                   │     │      │
                   ▼     │      ├─► P5.15 (Neural Cleanse)
                P5.3 ───┐│      ├─► P5.16 (STRIP)
                (DEX    ││      └─► P5.17 (Adv robustness)
                lift)   ││
                   │    │└──► P5.11 (Frida lib)
                P5.4    │ └──► P5.12 (eBPF lib)
                (ARM64) │       │
                   │    │       ▼
                P5.5    │     P5.13 (Dynamic refine)
                (ARMv7) │       │
                   │    └───────┤
                   ▼            │
                P5.7  ───► P5.6 (JNI bridge)
                (lib              │
                catalog)          ▼
                              P5.8 (Joint Java+native)
                                  │
                                  ▼
                              P5.18 (E2E)
                                  ▼
                              P5.19 (paper)
                                  ▼
                              P5.20 (gate review)
```

---

<a id="cross-cutting"></a>
## 5. Cross-Cutting Conditions (Always True From W1)

| Condition | Owner | Verification |
|---|---|---|
| Buck2 hermetic CI: every PR build byte-identical | G13 | CI gate (continued from P1.1) |
| Lean theorem re-verify on every L1/L4/L5/L6/native PR | G1 + G5 + G6 + G7 + G9 + G13 | CI gate, fail-closed |
| HACL\* on the verified-crypto path; no generic | G2 | Build-system check |
| All tools pinned via Nix flake (incl. lifter, emulator images, TFLite runtime) | G13 | `nix flake lock` reviewed quarterly |
| Differential fuzzer ≥ 99 % uptime across 5 AOSP harnesses + native fuzzer | G8 + G9 | Pyroscope + Prometheus |
| AXIOM-IR-v0.3 spec frozen until P5.2 freezes v0.4 | G3 | ADR review for any change |
| Halo2 / Plonky3 / Binius / Stwo proving keys pinned by content hash | G7 | Reproducibility gate |
| `axiom-verify` p99 ≤ 100 ms continuous regression test | G14 | CI gate |
| **NEW: Native lifter soundness regression test on every G9 PR** | G9 + G13 | CI gate, fail-closed |
| **NEW: Emulator-pool chaos drill weekly (pod kill, network partition, OOM)** | G10 + G13 | Scheduled job |
| **NEW: TFLite scanner reproducibility check (deterministic verdict bits)** | G11 + G13 | CI gate |
| **NEW: AXIOM-IR-v0.4 native dialect frozen after P5.2** | G3 | ADR for any change |

---

<a id="exit-gate"></a>
## 6. Phase 5 Consolidated Exit Gate

A single checklist combining all 20 sub-phase outcomes plus PHASE_GATES.md §9 hard KPIs. **Every box ✅ on the live dashboard for ≥ 7 consecutive days.**

```
Onboarding & Foundations (P5.1, P5.2, P5.9, P5.10)
[ ] G9 + G10 + G11 staffed and onboarded
[ ] Carry-forward debt from Phase 4 resolved or re-classified
[ ] AXIOM-IR-v0.4 native dialect frozen ≥ 4 weeks before P5.18
[ ] Lean native-lifter soundness theorems (DEX SSA + JNI boundary) machine-checked
[ ] Emulator pool operational with ≥ 32 emulators steady-state

Native lifters (P5.3, P5.4, P5.5, P5.6, P5.7)
[ ] DEX lifter ≥ 95 % files coverage on Bench-10K
[ ] DEX lift throughput ≥ 50 MB/s (HARD)
[ ] ARM64 ELF lifter ≥ 60 % function coverage on NDK-100
[ ] ARM64 lift throughput ≥ 25 MB/s (HARD)
[ ] ARMv7 lifter ≥ 50 % coverage (HARD; 80 % TARGET)
[ ] JNI bridge modeling ≥ 75 % common-pattern coverage
[ ] Native common-library catalog: libc + OpenSSL + BoringSSL + 30+ NDK patterns
[ ] Lift correctness ≥ 95 % function-level agreement vs BAP / angr reference

Joint Java + native (P5.8)
[ ] Joint analyzer p99 ≤ 15 s
[ ] Native intent dispatch resolution ≥ 50 %
[ ] ≥ 1 cross-language vulnerability discovered that Java-only analyzers miss (HARD)

Dynamic confirmation (P5.10, P5.11, P5.12, P5.13)
[ ] Emulator cold-start ≤ 120 s (HARD)
[ ] Frida attach latency ≤ 2 s (HARD)
[ ] eBPF load latency ≤ 200 ms (HARD)
[ ] UNKNOWN refinement ≥ 30 % (HARD)
[ ] Per-finding dynamic refinement p99 ≤ 300 s
[ ] Parallel APKs ≥ 8 on 16-core (HARD)

ML / TFLite (P5.14, P5.15, P5.16, P5.17)
[ ] Structural integrity hash ≤ 500 ms / model
[ ] Neural Cleanse scan ≤ 120 s / model
[ ] STRIP scan ≤ 60 s / model
[ ] Adversarial robustness scoring ≤ 300 s / model
[ ] Backdoor detection precision ≥ 90 % (HARD; 98 % TARGET)
[ ] Backdoor detection recall ≥ 80 % (HARD; 95 % TARGET)
[ ] ≥ 10 planted backdoors detected in controlled experiment

E2E (P5.18 — PHASE_GATES.md §9 hards)
[ ] L0–L6 + native sustained ≥ 7 APKs/sec on 16-core
[ ] Cluster (8-node × 16-core) ≥ 50 APKs/sec
[ ] Full pipeline incl. native p99 ≤ 30 s
[ ] Full pipeline incl. dynamic confirmation p99 ≤ 120 s
[ ] Peak RSS ≤ 1.5 GB per worker
[ ] Emulator memory budget ≤ 2 GB / emulator
[ ] 7-day soak: zero crashes
[ ] Cross-arch byte-identical certs 100 %
[ ] Reproducibility 100 %

Publication (P5.19)
[ ] Native+dynamic eval results published
[ ] NDSS / RAID paper draft ≥ 12 pages
[ ] Reproducibility Docker image published
[ ] NDK-100 corpus released

Phase 6 readiness (P5.20)
[ ] Phase 6 stabilization ADR approved
[ ] No new groups; group-level stabilization plans approved
[ ] External-audit RFP issued (Trail of Bits / NCC / Aleph / equivalent)
[ ] APKAXIOM-Eval-50K corpus reviewed and locked
[ ] Phase 6 budget approved
[ ] Sign-off from G1–G14 leads + leadership
[ ] Release tag `phase-5-complete` signed via cosign
```

---

<a id="risks"></a>
## 7. Phase 5 Risk Register

| Risk | Impact | Probability | Sub-phase | Mitigation |
|---|---|---|---|---|
| G9 hiring slow (binary-analysis + LLVM MLIR engineers scarce) | Critical | Medium | All G9 sub-phases | Pre-Phase-5 sourcing; contractor budget; ARM64-only fallback (drop ARMv7 to v1.1) |
| G10 hiring slow (Frida + eBPF + emulator orchestration is a niche skill) | High | Medium | All G10 sub-phases | Partnership with mobile-security firms; dedicated SRE for emulator pool |
| G11 hiring slow (TFLite security + adversarial-ML researchers scarce) | High | Medium | All G11 sub-phases | Contractor budget; academic collaboration |
| Native lifter coverage < 60 % on NDK-100 | High | Medium | P5.4 | Scope to ARM64 only for v1.0; ARMv7 deferred |
| ARM64 ELF instruction-set drift (SVE, BTI, MTE) | Medium | Medium | P5.4 | Track-only for SVE/SME; instrument for BTI/MTE compatibility |
| LLVM MLIR upstream churn breaks lifter | High | Medium | P5.4 | Pin LLVM via Nix flake; incremental upgrade gates |
| Emulator-pool capex/opex blows budget | High | Low | P5.10 | Cloud KVM with ARM Graviton; daily cost dashboards |
| Dynamic confirmation false-positive rate | Medium | Medium | P5.13 | k-step bounded refinement; consent-gated for production traces |
| TFLite backdoor detection precision below 90 % | High | Medium | P5.15, P5.16 | Three-scan ensemble (structural + Neural Cleanse + STRIP); threshold tuning |
| JNI boundary modeling explodes on real apps | High | Medium | P5.6 | Catalog of common bridges; UNKNOWN classification for unmodeled bridges |
| Cross-language analyzer state-space explosion | High | Medium | P5.8 | Reuse abstraction-domain library from Phase 3; per-call summarization |
| Frida detection by anti-tamper SDKs (e.g., DexProtector, Promon) | Medium | Medium | P5.11 | Multi-injection-strategy rotation; emulator with vendor-specific bypass profiles |
| Lean native-lifter soundness theorem complexity | High | Low | P5.9 | Scope to JNI boundary + DEX SSA only in Phase 5; ARM64 in Phase 6 |

---

<a id="defs"></a>
## 8. Definitions

Same as Phase 1–4. New terms:

- **AXIOM-IR-v0.4 native dialect** — SSA-form intermediate representation extending v0.3 with native-side opcodes (load/store/JNI-call/syscall), explicit JNI boundary nodes, and calling-convention metadata.
- **DEX SSA** — Static-Single-Assignment form of the DEX bytecode, with type-checked values and explicit phi nodes; lossless wrt original DEX semantics.
- **NDK-100** — A curated 100-APK corpus of common Android NDK patterns: JNI bridges, dlopen, common libc, OpenSSL, BoringSSL, libcrypto, custom packers.
- **JNI boundary node** — IR node modeling the Java-↔-native handoff; captures argument/return marshaling, ref types, and Java side-effects from native code.
- **Dynamic confirmation bridge** — When the static resolver returns UNKNOWN, the bridge launches a sandboxed emulator, runs Frida + eBPF traces, and refines the static abstraction or escalates to UNKNOWN-with-evidence.
- **Neural Cleanse** — Wang et al. backdoor-detection technique: searches for trigger patterns by reverse-engineering the input space for each output class.
- **STRIP** — Strong Intentional Perturbation; backdoor detection via predicting on perturbed-image ensembles.
- **Structural model hash** — A canonical hash over a TFLite model's graph + tensor shapes + operator types (excluding mutable weights), enabling tamper detection vs a signed reference.

---

*"Phase 5 closes the last gap: every layer of an Android app — Java, native, dynamic, ML — is now reasoned over. Phase 6 stabilizes; Phase 5 completes the analysis stack."*
