# APKAXIOM — 3-Year Development Roadmap to v1.0

> The executable plan: where to work first, what depends on what, the hard gates each phase must pass to advance, and the precise definition of "v1.0 ready to deploy."

This document is the temporal companion to [README.md](./README.md). The README defines the architecture and the 14 engineering groups (G1–G14). This document defines **when each group activates, what they ship per phase, and what blocks progression to the next phase**.

---

## Table of Contents

1. [What "v1.0" Means](#v1-definition)
2. [Operating Principles](#principles)
3. [The Critical Path (One Diagram)](#critical-path)
4. [Phase 0 — Pre-Foundation (M−2 to M0)](#phase-0)
5. [Phase 1 — Foundation (M0 to M6)](#phase-1)
6. [Phase 2 — Bundle Era (M6 to M12)](#phase-2)
7. [Phase 3 — Symbolic & Equivalence (M12 to M18)](#phase-3)
8. [Phase 4 — Certificates & Tooling (M18 to M24)](#phase-4)
9. [Phase 5 — Native + Dynamic + ML (M24 to M30)](#phase-5)
10. [Phase 6 — Hardening & v1.0 Release (M30 to M36)](#phase-6)
11. [Continuous Activities (Never Stop)](#continuous)
12. [Decision Points & Re-plan Triggers](#decision-points)
13. [Risk Register](#risks)
14. [Test Pyramid](#test-pyramid)
15. [Definition of Done for v1.0](#dod)
16. [Where to Start On Day 1](#day-1)

---

<a id="v1-definition"></a>
## 1. What "v1.0" Means

v1.0 is not "all features built." v1.0 is **a system you can deploy to a bug-bounty platform on Monday and have it produce certificates that triagers trust without supervision.**

Concretely, v1.0 ships only when **all of the following are simultaneously true**:

| # | Criterion | Measurable test |
|---|---|---|
| 1 | All 14 engineering groups have shipped their owned components | G1–G14 sign-off |
| 2 | The full proof stack (L0–L6 + cross-cutting) is end-to-end functional | Round-trip test: APK → `.axc` → `axiom-verify` ✅ |
| 3 | Zero known soundness gaps in the proof chain | Soundness regression suite green for 90 consecutive days |
| 4 | Reproducible: bit-identical certificates across architectures | x86_64, ARM64, RISC-V produce identical SHA-256 cert hashes |
| 5 | Evaluated on a 50,000+ APK corpus | Public dataset + paper |
| 6 | Independent external security audit completed | No critical findings open (Trail of Bits / NCC / similar) |
| 7 | ≥3 papers accepted at top venues (USENIX / S&P / NDSS / CCS / CAV) | Acceptance letters |
| 8 | ≥10 CVEs filed from differential fuzzing | CVE database entries |
| 9 | Reference verifier production-grade | `axiom-verify` p99 < 100 ms over 10K cert benchmark |
| 10 | Documentation complete | Spec docs for `.axc`, AXIOM-IR, every dialect, and every layer's correctness theorem |

If any one of these is false, **v1.0 does not ship**. Slip the date, do not lower the bar.

---

<a id="principles"></a>
## 2. Operating Principles

These are the rules that govern how the plan is executed. They are not negotiable mid-phase.

1. **Critical path determines staffing, not optimism.** If G1 is behind, hiring more G7 engineers does not help. Re-allocate to the bottleneck.
2. **No layer ships without proof reproducibility.** A theorem that machine-checks on the author's laptop and not on CI is not shipped.
3. **Every phase has a hard gate.** Failing a gate triggers re-plan, not "ship anyway."
4. **Continuous activities never pause.** G8 fuzzing, G13 CI, G1 AOSP archaeology run 24/7 from their start dates through v1.0 and beyond.
5. **Public artifacts at every phase boundary.** A paper, a release, a benchmark dataset. If you can't externalize the work, the work isn't done.
6. **No half-shipped layers.** A layer is either complete behind its theorem, or it is in development. There is no "L4 is mostly done."
7. **Spec before code.** Every layer's input/output type, every IR dialect, every certificate field is specified in writing before implementation starts. Specs are PR-reviewed.
8. **The fuzzer is the final reviewer.** No PR to a parser merges until the differential fuzzer runs against it for ≥24 hours with zero new disagreements introduced.

---

<a id="critical-path"></a>
## 3. The Critical Path (One Diagram)

```
                  ┌────────────────────────────────────────────────────┐
                  │             CRITICAL PATH TO v1.0                  │
                  └────────────────────────────────────────────────────┘

   G13 (infra) ──────────────────────────────────────────────────► v1.0
        │                                                              ▲
        ▼                                                              │
   G1 (Lean) ──► G2 (Rust) ──► G3 (IR/Bundle) ──► G5 (SMT) ──► G7 ───┐│
        │              │              │                          │   ││
        │              │              ▼                          │   ││
        │              │           G4 (Forensics) ──────────────►│   ││
        │              │                                         │   ││
        │              │              ▼                          │   ││
        │              │           G6 (BSH/Bisim) ──────────────►│   ││
        │              ▼                                         │   ││
        │           G8 (Fuzz, continuous from M1) ──────────────►│   ││
        │                                                        │   ││
        ▼                                                        ▼   ││
                                                              G14 ───┤│
                                                                     ││
                          (Phase 5 parallel, late) ─────────────────►││
                          G9, G10, G11, G12 ──────────────────────────┘

        Critical path = the longest chain of strict dependencies.
        Anything off the critical path can slip without delaying v1.0
        (within reason). Anything ON it cannot.
```

**Read it like this:** G13 must exist before anything builds. G1 must produce theorems before G2 can extract. G3 cannot finalize the IR without G1's type-system spec. G5 needs the IR. G7 needs proofs from G1 and witnesses from G5. G14 ships the user surface of G7. Nothing before G7 is removable.

**The off-critical-path groups (G9, G10, G11, G12)** can slip 6 months each without delaying v1.0, but they are still required for v1.0 by the [Definition of Done](#dod). So the real plan is: keep them on schedule, but if any one slips, the response is to *narrow scope*, not delay the ship.

---

<a id="phase-0"></a>
## 4. Phase 0 — Pre-Foundation (M−2 to M0)

**Active groups:** G13 only. Hiring active for G1, G2, G3, G8.
**Headcount:** ~5 engineers (mostly G13 + leadership).

### Goals
1. Stand up the hermetic build substrate. Without G13, nothing else is reproducible.
2. Lock in the proof-assistant choice. Recommendation: **Lean 4** (best Rust extraction tooling, active community, `mathlib` ecosystem). Decided in week 2; revisited never.
3. Hire G1 leads. Lean / Coq PhDs are the scarcest resource on the project. Start in M−2 because the pipeline is 3–6 months.
4. Audit existing apk-info v0.x. G2 leads produce a written report on what stays, what is rewritten, what migrates to the v1.0 refactor.
5. Legal review: AGPL+commercial dual-licensing strategy, zk-SNARK patent landscape (Halo2 has favorable IP; Plonk/Groth16 have ongoing patent disputes), AOSP Apache 2.0 obligations.
6. Lock the decision: Nix vs. Bazel for hermetic builds. Recommendation: **Bazel** (better toolchain support for Rust+Lean+Wasm; Nix is leaner but harder to staff).

### Deliverables
- Bazel-based hermetic build env. `bazel build //...` produces bit-identical outputs on three reference machines.
- Initial repo structure: `theorems/`, `crates/`, `ir/`, `verifier/`, `docs/`, `fuzz/`.
- ADR-0001 (Architecture Decision Record): Lean 4 chosen.
- ADR-0002: Bazel chosen.
- ADR-0003: AGPL + commercial dual-licensing.
- apk-info audit report (~30 pages).
- Hiring funnel: ≥10 G1 candidates interviewed, ≥3 offers extended.

### Phase 0 Gate (must pass to start Phase 1)
- ✅ G13 hermetic build delivers a 3-machine bit-identical reproducibility test.
- ✅ ≥2 G1 leads accepted offers, start dates within M0–M1.
- ✅ apk-info audit report reviewed and accepted by G2 leads.
- ✅ Legal review complete; no blocking IP issues identified.

**If gate fails:** delay M0 by 4–8 weeks. Do not start Phase 1 short on G1 — every later phase compounds the deficit.

---

<a id="phase-1"></a>
## 5. Phase 1 — Foundation (M0 to M6)

**Active groups:** G1, G2, G3, G8, G13.
**Headcount:** ~20 engineers.

This is the most important phase. **If Phase 1 ships clean, the project is on track for v1.0. If Phase 1 slips, every subsequent phase slips by the same amount — there is no recovery later.**

### Goals (in priority order)
1. **Lean 4 mechanization of the trust core.** Specifically:
   - ZIP layer (local file headers, central directory, end-of-central-directory record) for *one* Android version (target: Android 14, the most-deployed in 2026).
   - APK Signing Block parser and verifier (v1, v2, v3, v3.1).
   - The Lean theorems must state: *if `parseApk bytes = ok p`, then Android 14 installs `bytes` with manifest-resolution result `p.manifest`.*
2. **Rust extraction pipeline with translation validation.** Lean → Rust, with a separate translation-validator that compares the extracted Rust against a reference Lean evaluation on a regression corpus.
3. **apk-info v1.0 release** ([README §22](./README.md#apkinfo-integration)):
   - Streaming `Read`-based entry point.
   - Per-Android-version dispatch trait (start with one impl: A14).
   - Type-state phantoms.
   - Merkle commit hooks.
   - AXIOM-IR-v0.1 emitter for the manifest dialect.
4. **AXIOM-IR v0.1 spec frozen.** Manifest dialect + resource dialect. *Frozen* means the type signatures cannot change in Phase 2 without an explicit ADR.
5. **Differential Fuzzing Plant prototype.** Three AOSP version harnesses (A8, A11, A14), AFL++ as the engine, naive byte-mutation fuzzing. Grammar awareness comes in Phase 2.
6. **Hermetic CI gates.** Every PR builds reproducibly within 0 byte difference. Soundness regression suite is enforced.

### Cross-team dependencies
- G2 cannot start parser extraction until G1 ships its first theorem. **G1 leads ship a "hello, world" theorem (parse a trivial ZIP) in the first 30 days** so G2 has something to extract from immediately.
- G3 cannot freeze AXIOM-IR-v0.1 without G1's type-system input. **G3 publishes a draft AXIOM-IR spec in M1; G1 reviews; freeze in M3.**
- G8 cannot fuzz until G2 has a parser. **First fuzzer harness goes live in M2** with a single-version parser, expanded as more land.

### Deliverables
| Deliverable | Owner | Due |
|---|---|---|
| `theorems/zip.lean` (~1500 LOC, ZIP layer formalization) | G1 | M3 |
| `theorems/apk_signing.lean` (~1000 LOC, all 4 schemes) | G1 | M5 |
| `crates/axiom-l0` (streaming ZIP + Merkle commits) | G2 | M3 |
| `crates/axiom-l1-rs` (= apk-info v1.0) | G2 | M5 |
| `docs/AXIOM-IR-v0.1.md` (frozen spec) | G3 | M3 |
| `crates/axiom-ir` (manifest + resource dialects) | G3 | M5 |
| `fuzz/differential` (AFL++ harness for A8/A11/A14) | G8 | M4 |
| Bazel CI: reproducibility + soundness regression gates | G13 | M2 |
| Demo: 1000 known-good APKs parsed across A8/A11/A14 with Lean proof check | G1+G2 | M6 |

### Phase 1 Gate (must pass to start Phase 2)
- ✅ All Phase-1 Lean theorems machine-checked, reproducible on CI in <30 min.
- ✅ apk-info v1.0 released, no perf regression vs. v0.x (10x-Androguard claim preserved as CI gate).
- ✅ AXIOM-IR-v0.1 spec frozen and reviewed by G1, G2, G3, G4 leads.
- ✅ Differential fuzzer producing ≥3 disagreements per week, classified into the 3-way taxonomy (AOSP CVE / model bug / spec ambiguity).
- ✅ Reproducibility CI: 100 consecutive PRs build to byte-identical outputs.
- ✅ Phase-1 paper drafted and ready for submission.

### Risks specific to Phase 1
- **Lean toolchain churn.** Mitigation: vendor a pinned Lean 4 release; G13 owns the upgrade decision.
- **AOSP releases A15 mid-phase.** Mitigation: G2 archaeology absorbs the diff; do not formalize A15 in Phase 1, just track.
- **apk-info v1.0 breaks v0.x users.** Mitigation: v0.x stays released as `apk-info-legacy`; v1.0 is a new crate name (`apk-info` continues but with `v1` major version).
- **G1 understaffing.** This is the single most likely failure mode. Mitigation: aggressive hiring in Phase 0; budget for contractors with `mathlib` track records.

### Publication target
*"Verified Parsing for the Android Package Format"* — submission to **CAV 2026** or **OOPSLA 2026** at end of Phase 1.

---

<a id="phase-2"></a>
## 6. Phase 2 — Bundle Era (M6 to M12)

**Active groups:** G1, G2, G3, G4, G8, G13. (G4 starts.)
**Headcount:** ~24.

### Goals
1. **Schrödinger APK formalization complete.** Bundle composition operator `⊕` defined in Lean, theorems about behavior-set inclusion proven.
2. **Bundle resolver shipped (G3).** Rust implementation handling base APK + ABI splits + density splits + language splits + dynamic feature modules.
3. **Layer 3 forensics (G4).**
   - Shadow Stack: forensic deletion detection.
   - AXML Compiler Provenance Fingerprint.
   - Negative-Space Resource Anomaly.
4. **Lean coverage expands.** AXML parser and ARSC parser formalized (target: A14 only, deferred to Phase 3 for other versions).
5. **Fuzzer maturity.** 5 AOSP version harnesses (A8, A11, A12, A13, A14), grammar-aware mutation strategy.
6. **AXIOM-IR v0.2.** DEX dialect added.

### Cross-team dependencies
- G4 cannot start their forensic passes without the bundle resolver from G3. **G3 ships v0 of the resolver in M7** so G4 has BehaviorSets to analyze.
- G1's AXML/ARSC formalization unblocks G4's AXML Compiler Provenance work (which needs to read both unverified-Rust and verified-Lean parses for differential fingerprinting).

### Deliverables
| Deliverable | Owner | Due |
|---|---|---|
| `theorems/bundle_compose.lean` | G1 | M9 |
| `theorems/axml.lean`, `theorems/arsc.lean` (A14) | G1 | M11 |
| `crates/axiom-l2` (bundle resolver) | G3 | M9 |
| `crates/axiom-l3-shadow` | G4 | M10 |
| `crates/axiom-l3-provenance` + reference corpus (~500 APKs compiled with each toolchain) | G4 | M11 |
| `crates/axiom-l3-negspace` | G4 | M11 |
| Fuzzer extended to A12, A13; grammar-aware strategy | G8 | M9 |
| `docs/AXIOM-IR-v0.2.md` (adds DEX dialect) | G3 | M8 |
| Public benchmark: APKAXIOM-Phase2 evaluated on AndroZoo subset (10K APKs) | G1+G2+G3+G4 | M12 |

### Phase 2 Gate
- ✅ Bundle resolver passes differential testing against AOSP install behavior on 1000+ App Bundles.
- ✅ Forensic FP rate < 1% on a benign 1000-APK corpus.
- ✅ AOSP Lean coverage extended to A12 and A13.
- ✅ AndroZoo subset eval published.
- ✅ Phase-2 paper drafted.

### Publication target
*"Rethinking the Unit of Analysis for Android Security in the App Bundle Era"* — submission to **USENIX Security 2027** or **NDSS 2027**.

---

<a id="phase-3"></a>
## 7. Phase 3 — Symbolic & Equivalence (M12 to M18)

**Active groups:** + G5, G6.
**Headcount:** ~32.

### Goals
1. **Symbolic intent resolver (G5).** cvc5 + Spacer (CHC solver) backend. Models PackageManager state symbolically. Returns reachability proofs, UNSAT certificates, or explicit UNKNOWN.
2. **Behavior Surface Hash spec frozen (G6).** BSH-256 specification published as an RFC-style document.
3. **Bounded bisimulation engine (G6).** k-step bisimulation with abstract domains (numeric, string, type). Produces verifiable witnesses or divergence reports.
4. **Cross-APK device-snapshot prototype.** G5's resolver extended to handle *sets* of installed APKs, not just single APKs.

### Cross-team dependencies
- G5 needs G3's AXIOM-IR with full DEX dialect. **G3 finalizes DEX dialect in M13.**
- G6 needs G5's reachability outputs as the witness for behavioral equivalence (two APKs are equivalent only if they have the same reachable intents). **G5 ships v0 in M14, G6 starts integration in M15.**

### Deliverables
| Deliverable | Owner | Due |
|---|---|---|
| `crates/axiom-l4` (symbolic resolver) | G5 | M16 |
| `docs/BSH-256.md` (RFC-style spec) | G6 | M14 |
| `crates/axiom-l5-bsh` | G6 | M15 |
| `crates/axiom-l5-bisim` | G6 | M17 |
| Cross-APK snapshot prototype | G5 (sub-team) | M18 |
| Ground-truth eval: ≥100 known intent-hijack vulnerabilities reproduced as proofs | G5+G6 | M18 |
| Reproducibility test: BSH stable across ProGuard, R8, DexGuard obfuscators on 1000+ APKs | G6 | M17 |

### Phase 3 Gate
- ✅ Symbolic resolver UNKNOWN rate < 10% on benign 5K corpus.
- ✅ BSH stable (collision rate < 0.01%) across the three obfuscators.
- ✅ Bisimulation engine produces verifiable witnesses for ≥1000 known repackaging pairs.
- ✅ At least 1 zero-day intent-hijack vulnerability discovered via cross-APK analysis (evidence the approach has real-world value).
- ✅ Phase-3 paper drafted.

### Publication target
*"Sound and Complete Intent Resolution for Android"* — submission to **IEEE S&P 2027** or **NDSS 2028**.

---

<a id="phase-4"></a>
## 8. Phase 4 — Certificates & Tooling (M18 to M24)

**Active groups:** + G7, G12, G14.
**Headcount:** ~42.

### Goals
1. **`.axc` certificate format spec (G7).** Public, RFC-style. Versioned. Wire-format stable for v1.0+.
2. **Halo2 zk-SNARK circuits (G7).** Five priority privacy invariants:
   1. *"This APK never reads READ_CONTACTS."*
   2. *"This APK's network destinations are a subset of allowlist X."*
   3. *"This APK never accesses location without prior network."*
   4. *"This APK never reads device identifiers (IMEI, MAC, etc.)."*
   5. *"This APK's ML model has not been tampered with vs. the signed reference."*
3. **`axiom-verify` reference verifier (G7+G14).** Rust + WebAssembly builds. p99 < 100 ms over 10K cert benchmark.
4. **SLSA L4 attestation integration (G12).** Verifies APK against claimed build provenance. Reproducible-build verification (source ↔ APK).
5. **SDKs (G14).** axiom-py, axiom-go, axiom-ts. Each ships with idiomatic API + integration tests.
6. **Bug-bounty platform pilot (G14).** Partnership with HackerOne or Bugcrowd. One pilot platform ingests `.axc` certificates and renders human-readable findings.

### Cross-team dependencies
- G7 cannot finalize the `.axc` format until G1, G5, G6 have stable proof-output formats (Lean proof object, cvc5 UNSAT cert, bisimulation witness). **The proof-output formats are frozen by M19.**
- G14 ships SDKs against the `.axc` spec, so SDK work cannot start until M20.

### Deliverables
| Deliverable | Owner | Due |
|---|---|---|
| `docs/AXC-v1.md` (frozen format spec) | G7 | M20 |
| `circuits/halo2/*` (5 priority invariants) | G7 | M22 |
| `tools/axiom-verify` (Rust + Wasm) | G7+G14 | M22 |
| `crates/axiom-supply-chain` (SLSA L4) | G12 | M22 |
| `sdk/axiom-py`, `sdk/axiom-go`, `sdk/axiom-ts` | G14 | M23 |
| Pilot bug-bounty platform integration | G14 | M24 |
| Phase-4 paper drafted | G7 | M24 |

### Phase 4 Gate
- ✅ `axiom-verify` checks all certs from Phase 1–3 corpus reproducibly.
- ✅ Halo2 proofs verify in < 50 ms each.
- ✅ SLSA L4 round-trips cleanly with at least one F-Droid sample.
- ✅ All three SDKs pass integration tests against the same 1000-APK corpus.
- ✅ Pilot platform accepts and renders ≥10 real `.axc` certificates.

### Publication target
*"Proof-Carrying APKs: A New Architecture for Mobile App Distribution"* — submission to **CCS 2027** or **S&P 2028**.

---

<a id="phase-5"></a>
## 9. Phase 5 — Native + Dynamic + ML (M24 to M30)

**Active groups:** + G9, G10, G11.
**Headcount:** ~52.

### Goals
1. **DEX bytecode lifter to AXIOM-IR (G9).** SSA-form, type-checked, lossless.
2. **ARM64 ELF lifter to AXIOM-IR (G9).** Built on LLVM MLIR. Handles common Android NDK code patterns (JNI bridges, dlopen, common libc, OpenSSL, BoringSSL).
3. **Frida + eBPF dynamic confirmation bridge (G10).** When G5 returns UNKNOWN, drop into a sandboxed Android emulator, run Frida + eBPF traces, refine the static abstraction.
4. **TFLite integrity layer (G11).** Structural model hash, Neural Cleanse, STRIP, adversarial robustness scoring.
5. **Joint Java + native intent analysis.** G5 extended to follow JNI calls into native code, using G9's lift.

### Cross-team dependencies
- G9 needs AXIOM-IR's native dialect from G3. **G3 ships native dialect in M25.**
- G10 needs G5's UNKNOWN findings as inputs. Already available from Phase 3.
- G11 is mostly self-contained; only depends on G2 for raw asset extraction.

### Deliverables
| Deliverable | Owner | Due |
|---|---|---|
| `crates/axiom-dex-lift` | G9 | M27 |
| `crates/axiom-elf-lift` | G9 | M29 |
| `tools/axiom-dynamic` (emulator orchestration) | G10 | M27 |
| Frida + eBPF script library | G10 | M28 |
| `crates/axiom-tflite` | G11 | M28 |
| Joint Java+native intent analyzer (G5+G9 collab) | G5+G9 | M30 |
| Phase-5 paper drafted | G9+G10 | M30 |

### Phase 5 Gate
- ✅ Native lifter handles ≥80% of common Android NDK patterns without manual annotation. (Measured on a 100-APK NDK corpus.)
- ✅ Dynamic bridge resolves ≥50% of G5's UNKNOWN findings.
- ✅ TFLite scanner detects ≥10 planted backdoors (controlled experiment).
- ✅ Joint Java+native analysis discovers ≥1 cross-language vulnerability not visible to Java-only analyzers.

### Publication target
*"Joint Static-Dynamic Analysis of Android Native Code"* — submission to **NDSS 2028** or **RAID 2028**.

---

<a id="phase-6"></a>
## 10. Phase 6 — Hardening & v1.0 Release (M30 to M36)

**Active groups:** All — but **in stabilization mode**. **No new features unless safety-critical.**
**Headcount:** ~52 (no growth).

### Goals
1. **Stabilization sprint.** Bug-fix only. New features are deferred to v1.1.
2. **Full corpus eval.** Run the entire APKAXIOM stack on 50,000+ APKs. Publish results.
3. **External security audit.** Trail of Bits, NCC Group, or equivalent. ~10-week engagement starting M31.
4. **Documentation completeness.** Spec docs for `.axc`, AXIOM-IR (all dialects), every layer's correctness theorem, every group's design rationale.
5. **Production deployment of `axiom-verify`.** As a service. Public API. Rate-limited.
6. **v1.0 release.** Tagged. Signed. Announced.

### Stabilization activities by group
- G1: re-prove all theorems against final Lean toolchain version. No new theorems.
- G2: perf-tune. Memory budgets. No new features.
- G3: IR cleanup. Final dialect freeze.
- G4: FP rate driven below 0.5% via small refinements.
- G5: solver tuning. UNKNOWN rate driven below 5%.
- G6: bisim k-bound tuning per workload.
- G7: circuit gas optimization. Cert size reduction.
- G8: extended fuzzing campaigns. CVE filing.
- G9: lifter coverage extensions for the long tail.
- G10: emulator pool scaling.
- G11: ML model corpus expansion.
- G12: SLSA verification edge cases.
- G13: CI optimization for full-corpus eval.
- G14: SDK polish, docs, examples.

### Deliverables
| Deliverable | Owner | Due |
|---|---|---|
| 50K APK eval results + dataset | All | M33 |
| External audit report | G7+G13 | M34 |
| Migration guide for downstream consumers | G14 | M34 |
| `axiom-verify` production deployment | G14+G13 | M35 |
| v1.0 release announcement + papers | All | M36 |

### v1.0 Ship Gate
This is the hard, non-negotiable checklist from [§1](#v1-definition). All ten items must be ✅. Any ❌ slips the date.

If all ten are ✅: tag `v1.0.0`, sign with the project ed25519 key, publish the announcement. v1.0 ships.

---

<a id="continuous"></a>
## 11. Continuous Activities (Never Stop)

These activities run **24/7 from their start dates through v1.0 and beyond**. They are not scheduled per phase.

| Activity | Owner | Start | Description |
|---|---|---|---|
| Hermetic CI | G13 | M0 | Every PR build is byte-identical. Fail-closed gate on every merge. |
| Soundness regression suite | G13 + G1 | M3 | Every change to L1 must re-verify all Lean theorems. |
| Differential fuzzing | G8 | M2 | Continuous discovery of AOSP CVEs and model bugs. |
| AOSP archaeology | G2 | M0 | Track every libziparchive / PackageParser commit upstream. Re-formalize relevant changes. |
| Performance benchmarks | G2 + G13 | M3 | Every PR runs the perf suite. 10x-Androguard claim is a CI gate. |
| Security review of own code | G7 + G13 | M0 | Quarterly internal review. Annual external audit starting Phase 4. |
| Reproducibility audits | G13 | M3 | Quarterly: re-build all releases on clean machines, verify byte-identity. |

---

<a id="decision-points"></a>
## 12. Decision Points & Re-plan Triggers

These are the moments where the plan **must** be reviewed and possibly re-scoped.

| Decision point | Trigger condition | Response if triggered |
|---|---|---|
| M6 (Phase 1 gate) | Lean coverage < 80% of trust core | Slip Phase 2 by 3 months, do not start G4 |
| M12 (Phase 2 gate) | Bundle resolver fails differential testing on >1% of bundles | Slip Phase 3 by 3 months; G3 reinforced |
| M18 (Phase 3 gate) | UNKNOWN rate > 25% | Scope L4 to a tighter intent fragment; defer cross-APK to v1.1 |
| M24 (Phase 4 gate) | Halo2 perf > 200 ms p99 | Switch evaluation to STARK alternative; G7 reinforced |
| M30 (Phase 5 gate) | Native lifter coverage < 60% | Scope ARM64 only (drop ARMv7) for v1.0; defer to v1.1 |
| M33 (mid-Phase 6) | External audit reports critical finding | Stop the release. Fix. Re-audit. |
| M36 (ship gate) | Any of the 10 v1.0 criteria ❌ | Slip release. Do not ship a degraded v1.0. |

---

<a id="risks"></a>
## 13. Risk Register

The risks that could derail the project, ordered by impact × probability.

| Risk | Impact | Probability | Mitigation |
|---|---|---|---|
| G1 understaffing (Lean PhDs scarce) | Critical | High | Aggressive Phase 0 hiring; contractor budget; partial Coq fallback |
| AOSP semantics changes faster than formalization | High | Medium | G2 archaeology continuous; formalize trust core only, not full PackageParser |
| cvc5 hits hard cases that break solver scaling | High | Medium | Abstraction-domain library to bound state space; Spacer fallback |
| zk-SNARK proving time too high | High | Medium | STARK alternative pipeline maintained from M19; post-quantum bonus |
| zk-SNARK patent landscape changes | Medium | Low | Halo2 has favorable IP; legal review quarterly |
| 3-year runway / funding | Critical | Medium | Phase-2 paper as funding milestone; commercial pilot in Phase 4 |
| External audit finds critical soundness bug | High | Low | Internal review quarterly to surface early; design for soundness from day 1 |
| Talent attrition mid-phase | Medium | Medium | Documentation discipline so any role is replaceable in 8 weeks |
| Reproducibility breaks under scaling | Medium | Medium | G13 budget for tooling; Bazel chosen for this reason |
| AOSP releases major rewrite (e.g., post-Android-15 redesign) | High | Low | Scope v1.0 to A8–A15; A16+ is v1.1 |

---

<a id="test-pyramid"></a>
## 14. Test Pyramid

Every layer of the proof stack has a corresponding layer of testing. Tests are themselves part of the proof — a layer is not "done" until its tests are green.

```
┌──────────────────────────────────────────────────────────┐
│ Production deployment tests (1K real malware, sandboxed) │ ← Phase 6 only
├──────────────────────────────────────────────────────────┤
│ End-to-end integration (50K APK corpus)                  │ ← every phase boundary
├──────────────────────────────────────────────────────────┤
│ Differential fuzzing (continuous, G8)                    │ ← from M2
├──────────────────────────────────────────────────────────┤
│ Property-based tests (QuickCheck-style on grammars)      │ ← every parser
├──────────────────────────────────────────────────────────┤
│ Unit tests (every parser, encoder/decoder, IR pass)      │ ← every PR
├──────────────────────────────────────────────────────────┤
│ Reproducibility tests (every commit, byte-identical)     │ ← every PR
├──────────────────────────────────────────────────────────┤
│ Soundness regression (Lean re-verify on L1 changes)      │ ← every L1 PR
└──────────────────────────────────────────────────────────┘
```

Specific testing rules:
- **Soundness regression** is fail-closed. A PR that breaks a Lean theorem cannot merge under any circumstances.
- **Reproducibility** is fail-closed. A PR that breaks byte-identity cannot merge.
- **Differential fuzzing** is fail-soft but tracked: a PR that introduces new fuzzer disagreements must include either a fix or an issue link explaining why it's intended.
- **End-to-end integration** runs at every phase boundary on the full corpus available at that point.

---

<a id="dod"></a>
## 15. Definition of Done for v1.0

Repeating from §1, but as the executable checklist used at the M36 ship gate.

```
v1.0 ship gate — all items must be ✅

[ ] G1: all Lean theorems for A8–A15 trust core machine-checked & reproducible
[ ] G2: apk-info v2.0 shipped (Lean-extracted parsers); v1.0 retired in favor
[ ] G3: AXIOM-IR all dialects frozen, documented, versioned
[ ] G4: forensic FP rate < 0.5% on 50K-APK benign corpus
[ ] G5: symbolic resolver UNKNOWN rate < 5% on 50K corpus
[ ] G6: bisim engine produces witnesses for known repackaging corpus
[ ] G7: .axc format frozen; Halo2 circuits for 5 priority invariants ship
[ ] G8: ≥10 CVEs filed; fuzzer running 24/7 with auto-classification
[ ] G9: native lifter ≥80% coverage on Android NDK corpus
[ ] G10: dynamic bridge resolves ≥50% of UNKNOWN findings
[ ] G11: TFLite scanner detects backdoors with FP rate < 5%
[ ] G12: SLSA L4 verification works end-to-end with F-Droid samples
[ ] G13: hermetic build, byte-identical across x86_64/ARM64/RISC-V
[ ] G14: SDKs (py, go, ts) published; axiom-verify production-deployed
[ ] Eval: 50K-APK corpus results published as paper + dataset
[ ] Audit: external audit completed, no critical findings open
[ ] Papers: ≥3 accepted at top venues
[ ] Verifier: p99 < 100 ms over 10K-cert benchmark
[ ] Reproducibility: 90 consecutive days byte-identical CI green
[ ] Soundness: 90 consecutive days zero soundness-bug regression
[ ] Documentation: spec docs complete for all formats and theorems
```

If 19/20 are ✅, the answer is **slip the release**. Not ship.

---

<a id="day-1"></a>
## 16. Where to Start On Day 1

Despite the 14 groups and 36-month timeline, the first 90 days reduce to **two actions**:

### Action 1 — Recruit G1 (today, before anything else)

The Lean PhD pipeline is the single longest lead time on the project. Every other resource can be hired in 2–3 months. Lean PhDs take 4–6 months to source, vet, and onboard. **If G1 is not staffed by M0, the whole project slips by the same number of weeks G1 is short.**

This is the most important sentence in this document.

### Action 2 — Begin apk-info v1.0 refactor (G2, in parallel)

Do not wait for G1 to start. The five concrete optimizations of apk-info described in [README §22](./README.md#apkinfo-integration) are valuable on their own *and* are preconditions for Phase 1 integration. G2 leads (recruited from / in coordination with the existing apk-info maintainers at [github.com/delvinru/apk-info](https://github.com/delvinru/apk-info)) start the streaming refactor on day 1.

By the time G1 is ready in M0, G2 is ready to receive their first extracted module.

### What does NOT need to start on day 1

- Anything zk-SNARK related (G7) — Phase 4.
- Anything native code related (G9) — Phase 5.
- Anything dynamic (G10) — Phase 5.
- Anything ML (G11) — Phase 5.
- Symbolic execution (G5), bisimulation (G6) — Phase 3.
- Forensic passes (G4) — Phase 2.

These groups can be hired against later. **Front-loading them wastes runway.** Phase-staggered hiring is part of the plan, not an accident of it.

---

## Appendix A — Phase boundary publications

| Phase | Submitted to | Working title |
|---|---|---|
| 1 | CAV / OOPSLA | Verified Parsing for the Android Package Format |
| 2 | USENIX Security / NDSS | Rethinking the Unit of Analysis for Android Security in the App Bundle Era |
| 3 | IEEE S&P / NDSS | Sound and Complete Intent Resolution for Android |
| 4 | CCS / S&P | Proof-Carrying APKs: A New Architecture for Mobile App Distribution |
| 5 | NDSS / RAID | Joint Static-Dynamic Analysis of Android Native Code |
| 6 (release) | Open dataset paper | The APKAXIOM Corpus: Proof-Stack Evaluation on 50K Android Packages |

Six papers across 36 months at top venues. This is the publication trajectory of a serious research lab, not a side project.

---

## Appendix B — Headcount over time

```
        engineers
         │
      52 ┤                                          ───────────
         │                                          │
         │                                       ┌──┘ Phase 5+6
      42 ┤                              ────────┤
         │                              │
         │                              │ Phase 4
      32 ┤                    ──────────┤
         │                    │
         │                    │ Phase 3
      24 ┤          ──────────┤
         │          │
         │          │ Phase 2
      20 ┤  ────────┤
         │  │
         │  │ Phase 1
       5 ┤──┘ Phase 0
         │
         └────────────────────────────────────────────────────
            M-2   M0    M6    M12    M18    M24    M30    M36
```

Total engineer-months across 3 years: **~1,500** (rough integral).
Total engineer-cost at $250K/eng/yr fully-loaded: **~$31M for 3 years**.

This is realistic for: a Series-B security startup, a national-lab research initiative, a Big Tech research arm with portfolio backing, or a well-funded academic consortium.

This is **not** realistic for: a side project, a typical research grant, a single PhD thesis.

---

*"v1.0 ships when every theorem checks, every cert verifies, every CI gate is green for 90 days, and every external auditor has signed off. Not before."*
