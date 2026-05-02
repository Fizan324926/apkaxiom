# Phase 6 — Hardening & v1.0 Release Detailed Plan (M30 → M36)

> The 6 months that take APKAXIOM from "feature-complete research stack" to **v1.0 — a system you can deploy to a bug-bounty platform on Monday and have it produce certificates triagers trust without supervision.** 20 sub-phases (P6.1 → P6.20). Stabilization mode: no new features unless safety-critical.
> Each sub-phase has its own folder with: identity, goal/scope, dependencies, tools, third-party services & API keys with free/paid status, system inventory, **all features & functions delivered**, **explicit numeric KPIs**, end-to-end test, exit checklist.

This document is the operational complement to:
- [../../README.md](../../README.md) — architecture
- [../ROADMAP.md](../ROADMAP.md) — high-level Phase 6 goals + v1.0 Definition of Done
- [../PHASE_GATES.md](../PHASE_GATES.md#phase-6) — Phase 6 (v1.0 ship) numeric KPI gates
- [../TECH_STACK.md](../TECH_STACK.md) — tech-stack picks

---

## Table of Contents

1. [Phase 6 Goal Statement](#goal)
2. [What's New in Phase 6 vs Phase 5](#whats-new)
3. [The 20 Sub-Phases at a Glance](#glance)
4. [Sub-Phase Dependency Diagram](#deps)
5. [Cross-Cutting Conditions (always true)](#cross-cutting)
6. [Phase 6 (v1.0) Consolidated Ship Gate](#exit-gate)
7. [Phase 6 Risk Register](#risks)
8. [Definitions](#defs)

---

<a id="goal"></a>
## 1. Phase 6 Goal Statement

By the end of Phase 6 (M36), the project must have:

- **All 20 v1.0 ship-gate items from PHASE_GATES.md §10 ✅** for ≥ 90 consecutive days. No "target" column; every line is hard.
- **Stabilization, not new features.** Each group runs a focused punch-list to drive their layer to production-grade.
- **APKAXIOM-Eval-50K** — the public 50,000-APK eval — completes in ≤ 72 h on 100-core cluster, results published as paper + dataset.
- **External security audit closed.** Trail of Bits / NCC / Aleph or equivalent — ~10-week engagement with no critical findings open.
- **Documentation complete** — `.axc` format spec, AXIOM-IR all dialects, every L0–L6 layer's correctness theorem, every group's design rationale, migration guide.
- **Production deployment of `axiom-verify` as a service** — public API, rate-limited, ≥ 99.99 % availability over 90-day window.
- **≥ 3 papers accepted** at top venues (USENIX / S&P / NDSS / CCS / CAV / OOPSLA / RAID / FSE / PLDI / IEEE TDSC / etc.).
- **≥ 10 CVEs filed** from G8 fuzzing.
- **Pilot bug-bounty platform live in production**, ingesting `.axc` certs in real triager flow.
- **v1.0 tag released**, signed via cosign, announced.

---

<a id="whats-new"></a>
## 2. What's New in Phase 6 vs Phase 5

| Area | Phase 5 | Phase 6 |
|---|---|---|
| Mode | feature development | **stabilization — no new features unless safety-critical** |
| Active groups | + G9 + G10 + G11 | **all 14 (G1–G14) — but in stabilization mode** |
| Headcount | ~52 | **~52 (no growth — focus, not expansion)** |
| Eval scale | NDK-100 + Bench-10K + planted-backdoor zoo | **APKAXIOM-Eval-50K (public, full corpus)** |
| External validation | none | **external security audit (Trail of Bits / NCC / Aleph)** |
| Verifier deployment | pilot | **production-grade SLA: ≥ 99.99 % over 90 days** |
| Crash rate | <1 per 10M APKs (target) | **<1 per 10M APKs (HARD)** |
| Reproducibility | spot-checked | **90 consecutive days byte-identical CI green** |
| Soundness regression | continuous | **0 incidents in 90-day window** (HARD) |
| Documentation | per-feature | **complete spec docs for all formats and theorems** |
| Cross-arch | x86_64 + ARM64 | **+ RISC-V (final architecture parity)** |

---

<a id="glance"></a>
## 3. The 20 Sub-Phases at a Glance

| # | Sub-phase | Owner(s) | Weeks (≈) | Hard dep on |
|---|---|---|---|---|
| [P6.1](./P6.1/README.md) | Phase 5 carry-forward + Phase 6 stabilization kickoff | All | W1–W2 | P5.20 |
| [P6.2](./P6.2/README.md) | G1 stabilization: re-prove all theorems against final Lean toolchain | G1 | W1–W14 | P6.1 |
| [P6.3](./P6.3/README.md) | G2 stabilization: perf-tune, memory budgets, no new features | G2 | W1–W14 | P6.1 |
| [P6.4](./P6.4/README.md) | G3 stabilization: AXIOM-IR final-dialect freeze + v1.0 spec publication | G3 | W1–W12 | P6.1 |
| [P6.5](./P6.5/README.md) | G4 stabilization: forensic FP rate < 0.5 % | G4 | W1–W14 | P6.1 |
| [P6.6](./P6.6/README.md) | G5 stabilization: solver tuning, UNKNOWN rate < 5 % | G5 | W1–W16 | P6.1 |
| [P6.7](./P6.7/README.md) | G6 stabilization: bisim k-bound tuning per workload | G6 | W1–W14 | P6.1 |
| [P6.8](./P6.8/README.md) | G7 stabilization: circuit gas optimization + cert size reduction | G7 | W1–W14 | P6.1 |
| [P6.9](./P6.9/README.md) | G8 stabilization: extended fuzzing campaigns + ≥ 10 CVEs filed | G8 | W1–W22 | P6.1 |
| [P6.10](./P6.10/README.md) | G9 stabilization: lifter coverage extensions for the long tail | G9 | W1–W16 | P6.1 |
| [P6.11](./P6.11/README.md) | G10 stabilization: emulator pool scaling + chaos drills | G10 | W1–W14 | P6.1 |
| [P6.12](./P6.12/README.md) | G11 stabilization: ML model corpus expansion + production scanning | G11 | W1–W14 | P6.1 |
| [P6.13](./P6.13/README.md) | G12 stabilization: SLSA edge cases + reproducible-build coverage expansion | G12 | W1–W14 | P6.1 |
| [P6.14](./P6.14/README.md) | G13 stabilization: CI optimization + RISC-V parity + 50K eval pipeline | G13 | W1–W18 | P6.1 |
| [P6.15](./P6.15/README.md) | G14 stabilization: SDK polish + production verifier service deployment | G14 | W1–W18 | P6.1 |
| [P6.16](./P6.16/README.md) | APKAXIOM-Eval-50K corpus run + dataset release | All | W12–W22 | P6.2–P6.15 |
| [P6.17](./P6.17/README.md) | External security audit (Trail of Bits / NCC / Aleph) | G7 + G13 + leadership | W6–W22 | P6.1 |
| [P6.18](./P6.18/README.md) | Documentation completeness | All | W12–W22 | P6.2–P6.15 |
| [P6.19](./P6.19/README.md) | Production deployment of `axiom-verify` service + open-data paper | G14 + G13 | W18–W24 | P6.16, P6.17 |
| [P6.20](./P6.20/README.md) | v1.0 ship-gate review + tag + release announcement | Lead + all | W24–W26 | P6.17, P6.19 |

> **Each sub-phase folder above contains a self-contained README** with the same uniform template as Phases 1–5.

---

<a id="deps"></a>
## 4. Sub-Phase Dependency Diagram

```
                 ┌──────────── P6.1 ──────────────┐
                 │  Stabilization kickoff +       │
                 │  carry-forward                 │
                 └─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬───┘
                   │ │ │ │ │ │ │ │ │ │ │ │ │ │
                   ▼ ▼ ▼ ▼ ▼ ▼ ▼ ▼ ▼ ▼ ▼ ▼ ▼ ▼
                  P6.2..P6.15  (one per group: G1..G14 stabilization)
                   │ │ │ │ │ │ │ │ │ │ │ │ │ │
                   └─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─► P6.16 (50K eval)
                                                    │
                                P6.17 (audit) ──────┤
                                                    │
                                P6.18 (docs) ───────┤
                                                    ▼
                                              P6.19 (production deploy)
                                                    ▼
                                              P6.20 (v1.0 ship)
```

---

<a id="cross-cutting"></a>
## 5. Cross-Cutting Conditions (Always True From W1)

| Condition | Owner | Verification |
|---|---|---|
| Buck2 hermetic CI: every PR build byte-identical (continued) | G13 | CI gate |
| Lean theorem re-verify on every L1/L4/L5/L6 PR (continued) | G1 + G5 + G6 + G7 + G9 + G13 | CI gate, fail-closed |
| HACL\* on the verified-crypto path (continued) | G2 | Build-system check |
| All tools pinned via Nix flake (continued) | G13 | `nix flake lock` reviewed quarterly |
| Differential fuzzer ≥ 99 % uptime (continued) | G8 | Pyroscope + Prometheus |
| AXIOM-IR-v0.4 spec frozen (will become v1.0 in P6.4) | G3 | ADR |
| `axiom-verify` p99 ≤ 100 ms continuous regression test | G14 | CI gate |
| **NEW: NO new features merge unless safety-critical, with leadership ADR** | All | Merge-policy enforcement |
| **NEW: 90-day reproducibility audit window opens** | G13 | Audit log |
| **NEW: 90-day soundness-regression-zero window opens** | G1 | Audit log |
| **NEW: 90-day verifier SLA window opens** | G14 + G13 | Service availability metric |
| **NEW: External-auditor sandbox provisioned** | G13 + leadership | Audit-firm onboarding pack |
| **NEW: RISC-V CI runner online** | G13 | Cross-arch reproducibility check |
| **NEW: APKAXIOM-Eval-50K corpus governance lock** | leadership + DPO | Manifest + license tracking |

---

<a id="exit-gate"></a>
## 6. Phase 6 (v1.0) Consolidated Ship Gate

This is the **v1.0 ship gate** — every line is hard, every line must be ✅ for ≥ 90 consecutive days. Mirrors PHASE_GATES.md §10 + ROADMAP §15.

```
Stabilization (P6.2 .. P6.15)
[ ] G1 — all theorems re-verify on final Lean toolchain
[ ] G2 — apk-info v2.0 perf-tuned, memory-budgeted, no perf regression
[ ] G3 — AXIOM-IR v1.0 spec frozen + published
[ ] G4 — forensic FP rate < 0.5 % on 50K-APK benign corpus
[ ] G5 — symbolic resolver UNKNOWN rate < 5 % on 50K corpus
[ ] G6 — bisim engine produces witnesses for known repackaging corpus
[ ] G7 — .axc format frozen; circuits gas-optimized; cert size median ≤ 50 KB
[ ] G8 — ≥ 10 CVEs filed; fuzzer 24/7 with auto-classification
[ ] G9 — native lifter ≥ 80 % coverage on Android NDK corpus
[ ] G10 — dynamic bridge resolves ≥ 50 % UNKNOWN findings
[ ] G11 — TFLite scanner detects backdoors with FP rate < 5 %
[ ] G12 — SLSA L4 verification end-to-end with F-Droid + Play-store-style
[ ] G13 — hermetic build, byte-identical across x86_64 / ARM64 / RISC-V
[ ] G14 — SDKs (py, go, ts) published; axiom-verify production-deployed

KPIs (PHASE_GATES.md §10)
[ ] axiom-verify p99 ≤ 100 ms over 10K cert sample (90 days green)
[ ] Service availability ≥ 99.99 % over 90 days
[ ] 50K APK eval completes ≤ 72 h on 100-core cluster
[ ] Sustained throughput on 100-core cluster ≥ 35 APKs/sec
[ ] Per-APK end-to-end p99 (full pipeline) ≤ 30 s
[ ] 90 consecutive days byte-identical CI
[ ] Three-arch (x86_64 + ARM64 + RISC-V) bit-identical certs 100 % over 10K samples
[ ] Re-build of every prior phase release reproduces bit-identical ≥ 95 %
[ ] Cross-time reproducibility verified
[ ] Crash rate < 1 per 10M APKs
[ ] Hang rate < 0.01 %
[ ] Soundness regression incidents = 0 over 90 days
[ ] MTBF ≥ 720 h in production
[ ] 5× burst tolerance, 5 min sustained, p99 ≤ 2× nominal
[ ] 10× burst tolerance, 60 s, recovery ≤ 30 s
[ ] Sustained 90 % util, 7 days, no degradation
[ ] Streaming verification ≤ 50 ms p99 after last byte
[ ] Wire-speed inspection ≥ 1 Gbps on 16-core
[ ] Real-time bug-bounty pilot: median triager-to-verdict ≤ 2 s
[ ] Verifier x86_64 vs ARM64 throughput within 20 %
[ ] All-arch byte-identical outputs 100 %

External Validation
[ ] External audit (Trail of Bits / NCC / Aleph): no critical open
[ ] 50K APK eval results published as paper + dataset
[ ] ≥ 3 papers accepted at top venues
[ ] ≥ 10 CVEs filed from G8 fuzzing
[ ] Pilot bug-bounty platform live in production, ingesting .axc

Documentation
[ ] .axc format specification (RFC-style) published
[ ] AXIOM-IR specification (all dialects) published
[ ] Every L0–L6 layer's correctness theorem documented
[ ] Per-group design rationale published
[ ] Migration guide for downstream consumers published

Release
[ ] v1.0 tag created, signed via cosign
[ ] Public release announcement
[ ] Press / academic kit
```

If 19/20 are ✅, the answer is **slip the release.** Not ship.

---

<a id="risks"></a>
## 7. Phase 6 Risk Register

| Risk | Impact | Probability | Sub-phase | Mitigation |
|---|---|---|---|---|
| External audit finds a critical soundness bug | Critical | Low | P6.17 | Internal review quarterly to surface early; design-for-soundness from day 1; budget reserved for fix-and-re-audit |
| 50K eval reveals a long-tail crash rate above gate | High | Medium | P6.16 | Bench-10K → Stress-100K progression already in place; long-tail backlog reserved |
| Reproducibility regression on RISC-V | High | Medium | P6.14 | Early CI on RISC-V from W1; SiFive HiFive Pro P550 procurement scheduled |
| Verifier SLA below 99.99 % over 90 days | High | Medium | P6.19 | Multi-region deployment; failover; cost-of-availability budgeted |
| New feature smuggled in mid-phase ("just one fix") | Medium | High | All | Strict merge policy + leadership ADR for any non-stabilization PR |
| Talent attrition mid-phase | Medium | Medium | All | Documentation discipline so any role replaceable in 8 weeks |
| AOSP A16 release destabilizes parser | Medium | Low | P6.3 | Scope v1.0 to A8–A15; A16 is v1.1 |
| Audit firm capacity / scheduling slip | Medium | Medium | P6.17 | Multi-firm RFP issued in P5.20; backup engagement pre-negotiated |
| Lean toolchain final-version churn | High | Low | P6.2 | Pin in W4; freeze upgrade until v1.0 |
| Eval-50K corpus license issues | High | Low | P6.16 | DPO governance; per-sample license tracking |
| Verifier under DDoS at production launch | Medium | Medium | P6.19 | Cloudflare front + per-cert rate limit + token-based ingest |
| Bug-bounty pilot churn (partner shifts) | Medium | Low | P6.16 / production | Multi-partner conversations from Phase 4; secondary platform on standby |

---

<a id="defs"></a>
## 8. Definitions

Same as Phase 1–5. New terms:

- **APKAXIOM-Eval-50K** — The public 50,000-APK evaluation corpus released with v1.0. Stratified across benign / malware / bundles / obfuscated / NDK-heavy. Composition + license recorded in a manifest, governed by the data-protection officer.
- **v1.0 ship gate** — The 20-item hard checklist from PHASE_GATES.md §10 + ROADMAP §15. All items must be ✅ for ≥ 90 consecutive days for the release to ship.
- **External security audit** — A ~10-week engagement with an independent firm (Trail of Bits, NCC Group, Aleph Research, or Atredis) reviewing the entire stack for soundness, cryptographic correctness, supply-chain integrity, side-channels, and operational hardening. No critical findings open at v1.0 ship.
- **Production verifier SLA** — The `axiom-verify` service hosted publicly, with ≥ 99.99 % availability over 90-day rolling window, p99 ≤ 100 ms, and rate-limited public API.
- **Stabilization mode** — A merge policy in which only safety-critical fixes, performance tuning within an existing layer's budget, and documentation updates are accepted. New features require a leadership ADR; the default is "no."

---

*"Phase 6 doesn't add features. It earns the right to call the result v1.0. Every theorem re-verified, every cert reproducible, every CI gate green for 90 days, every external auditor signed off. Not before."*
