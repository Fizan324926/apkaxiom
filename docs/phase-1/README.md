# Phase 1 — Foundation Detailed Plan (M0 → M6)

> The first 6 months of APKAXIOM, broken into **20 sub-phases (P1.1 → P1.20)**.
> Each sub-phase has a single source of truth in this document: working state, features ready, KPIs (numeric, drawn from [PHASE_GATES.md](../PHASE_GATES.md)), end-to-end test requirement, documentation deliverable, and an exit checklist.
> A sub-phase is **NOT done** until every checkbox below it is ✅ on the live CI dashboard for ≥7 consecutive days.

This document is the operational complement to:
- [../README.md](../../README.md) — architecture
- [../ROADMAP.md](../ROADMAP.md) — high-level Phase 1 goals
- [../PHASE_GATES.md](../PHASE_GATES.md) — Phase 1 numeric KPI gates
- [../TECH_STACK.md](../TECH_STACK.md) — tech-stack picks

---

## Table of Contents

1. [Phase 1 Goal Statement](#goal)
2. [The 20 Sub-Phases at a Glance](#glance)
3. [Sub-Phase Dependency Diagram](#deps)
4. [Cross-Cutting Conditions (always true)](#cross-cutting)
5. [Sub-Phases P1.1 through P1.20 — full detail](#detail)
6. [Phase 1 Consolidated Exit Gate](#exit-gate)
7. [Phase 1 Risk Register](#risks)
8. [Definitions](#defs)

---

<a id="goal"></a>
## 1. Phase 1 Goal Statement

By the end of Phase 1 (M6), the project must have:

- A **Lean 4 mechanization of the trust core** (ZIP layer + APK Signing Block v1/v2/v3/v3.1) for **Android 14**, with smaller initial coverage of A8 and A11.
- A **Rust extraction pipeline with translation validation** that produces machine-checkable Rust from the Lean theorems.
- **apk-info v1.0** released as `axiom-l1-rs`, with: streaming reader, per-Android-version dispatch trait, type-state phantom types, BLAKE3 Merkle commit hooks, and AXIOM-IR-v0.1 manifest-dialect emitter.
- **AXIOM-IR v0.1 spec frozen** (manifest dialect + resource dialect).
- **Differential Fuzzing Plant prototype** with three AOSP version harnesses (A8, A11, A14), Nyx-based snapshot fuzzing.
- **Hermetic CI substrate** (Buck2 primary, Bazel for AOSP harnesses, Nix flakes for toolchain pinning), with reproducibility and soundness regression as fail-closed CI gates.
- **All Phase 1 KPIs** from PHASE_GATES.md §5 green for ≥7 consecutive days.
- **Phase-1 paper drafted** for CAV / OOPSLA submission.

---

<a id="glance"></a>
## 2. The 20 Sub-Phases at a Glance

| # | Sub-phase | Owner(s) | Weeks (≈) | Hard dependency on |
|---|---|---|---|---|
| [P1.1](./P1.1/README.md) | Hermetic build foundation (Buck2 + Nix + reproducibility CI) | G13 | W1–W3 | — |
| [P1.2](./P1.2/README.md) | Lean 4 toolchain & mathlib4 vendoring + extraction prototype | G1 | W2–W4 | P1.1 |
| [P1.3](./P1.3/README.md) | apk-info v0.x audit & v1.0 architecture spec | G2 | W1–W3 | — |
| [P1.4](./P1.4/README.md) | AXIOM-IR v0.1 draft spec (manifest + resource dialects) | G3 | W2–W6 | P1.2 (type system) |
| [P1.5](./P1.5/README.md) | Lean ZIP layer — local file headers + EOCD | G1 | W3–W7 | P1.2 |
| [P1.6](./P1.6/README.md) | Lean ZIP layer — central directory + offsets | G1 | W6–W10 | P1.5 |
| [P1.7](./P1.7/README.md) | apk-info v1.0 streaming reader trait | G2 | W3–W7 | P1.3 |
| [P1.8](./P1.8/README.md) | apk-info v1.0 type-state phantom-type guards | G2 | W6–W9 | P1.7 |
| [P1.9](./P1.9/README.md) | Rust extraction pipeline v0.1 + translation-validation harness | G1+G2 | W7–W11 | P1.5, P1.8 |
| [P1.10](./P1.10/README.md) | apk-info v1.0 BLAKE3 Merkle commit hooks (HACL\* verified) | G2 | W8–W11 | P1.7 |
| [P1.11](./P1.11/README.md) | Lean APK Signing Block v1/v2/v3/v3.1 | G1 | W9–W14 | P1.6 |
| [P1.12](./P1.12/README.md) | Rust extraction of full ZIP layer (replace hand-written) | G1+G2 | W11–W14 | P1.6, P1.9 |
| [P1.13](./P1.13/README.md) | Differential Fuzzing Plant — Cuttlefish A14 harness via Nyx | G8 | W4–W12 | P1.1 |
| [P1.14](./P1.14/README.md) | Differential Fuzzing Plant — A8 + A11 harnesses + classifier | G8 | W12–W18 | P1.13 |
| [P1.15](./P1.15/README.md) | apk-info v1.0 AXIOM-IR-v0.1 emitter (manifest + resource) | G2+G3 | W12–W17 | P1.4, P1.10 |
| [P1.16](./P1.16/README.md) | Rust extraction of APK Signing Block (HACL\* signature path) | G1+G2 | W14–W18 | P1.11, P1.12 |
| [P1.17](./P1.17/README.md) | Soundness regression suite as fail-closed CI gate | G1+G13 | W12–W20 | P1.9, P1.16 |
| [P1.18](./P1.18/README.md) | End-to-end Bench-1K smoke + Bench-10K perf eval | All | W18–W22 | P1.15, P1.16, P1.17 |
| [P1.19](./P1.19/README.md) | Public AndroZoo benchmark + Phase-1 paper draft | All | W20–W24 | P1.18 |
| [P1.20](./P1.20/README.md) | Phase 1 hard-gate review + Phase 2 ADR | Leadership + all | W24–W26 | P1.19 |

> **Each sub-phase folder above contains a self-contained README** with: identity, goal/scope, hard dependencies, required tools and libraries with version pins, **third-party services & API keys with free/paid status**, system inventory (what's on the host vs what to install with concrete commands), working-directory file tree, standalone output, end-to-end test, and exit checklist.

**6 months = ~26 weeks.** Many sub-phases overlap; the table is approximate scheduling, not strict serial order. Hard dependencies are non-negotiable.

---

<a id="deps"></a>
## 3. Sub-Phase Dependency Diagram

```
                  ┌──────────────── P1.1 ──────────────────┐
                  │  Hermetic build (Buck2/Nix/CI repro)  │
                  └───┬───────────────┬───────────┬────────┘
                      │               │           │
                      ▼               ▼           ▼
                P1.2 (Lean)     P1.3 (audit)  P1.13 (Nyx A14 harness)
                      │               │           │
                      ▼               ▼           │
                P1.4 (IR spec)  P1.7 (stream)    │
                      │               │           │
                      ▼               ▼           │
                P1.5 (ZIP LFH)  P1.8 (type-state)│
                      │               │           │
                      ▼               ▼           │
                P1.6 (ZIP CDR) ─►P1.10 (Merkle)──┤
                      │               │           ▼
                      └──┬────────────┘     P1.14 (A8+A11 + classifier)
                         ▼
                   P1.9 (extraction pipeline)
                         │
                         ▼
                   P1.12 (extracted ZIP)
                         │
                         ▼
                   P1.11 (APK Signing Block Lean) ──► P1.16 (extracted)
                         │
                         ▼
                  P1.15 (IR emitter) ◄── P1.4, P1.10
                         │
                         ▼
                  P1.17 (soundness regression CI)
                         │
                         ▼
                  P1.18 (Bench-1K + Bench-10K e2e)
                         │
                         ▼
                  P1.19 (AndroZoo bench + paper draft)
                         │
                         ▼
                  P1.20 (Phase 1 gate review + Phase 2 ADR)
```

---

<a id="cross-cutting"></a>
## 4. Cross-Cutting Conditions (Always True From Day 1)

These are not sub-phases; they are conditions that must remain green at all times. Going red is a P0 incident.

| Condition | Owner | Verification |
|---|---|---|
| Buck2 hermetic CI: every PR build byte-identical on 3 reference machines | G13 | CI gate, fail-closed |
| Lean theorem re-verify on every L1 PR | G1+G13 | CI gate, fail-closed |
| `cargo deny` / `cargo audit` clean | G13 | Per-PR |
| BLAKE3 implementation is HACL\* verified, not generic | G2 | Build-system check |
| All tools pinned via Nix flake (Lean, Rust, Buck2, Bazel-for-AOSP) | G13 | `nix flake lock` reviewed quarterly |
| Crash-rate dashboard live; >0 paged | G13 | Pyroscope + Prometheus alerts |

---

<a id="detail"></a>
## 5. Sub-Phases P1.1 through P1.20 — Full Detail

Each sub-phase entry below has: **Owner** · **Working state** · **Features ready** · **KPIs (numeric, from PHASE_GATES)** · **End-to-end test** · **Documentation** · **Exit checklist**.

A sub-phase advances only when **every checkbox is ✅ for ≥7 consecutive days**.

---

### P1.1 — Hermetic Build Foundation

**Owner.** G13 (Platform Infrastructure)
**Estimated duration.** Weeks 1–3
**Dependencies.** None — foundation work.

**Working state.**
The repository builds reproducibly on 3 reference machines via Buck2. Toolchains (Rust, Lean 4, Buck2 itself, Bazel-for-AOSP, mathlib4 commit) are pinned via Nix flake. Every PR triggers a reproducibility check that compares output bytes from 3 independent builders.

**Features ready.**
- Buck2 workspace with Reindeer (Cargo→Buck2 conversion) operational
- `nix flake.nix` pinning all toolchains
- Bazel sub-workspace under `external/aosp/` ready to receive AOSP harnesses (Phase P1.13)
- `BUILD` files for placeholder crates (`axiom-l0`, `axiom-l1-rs`, `axiom-ir`)
- Reproducibility test harness: `make repro-check` runs `bazel build //... && diff` between two clean machines (and reports SHA-256 of every output)
- Per-PR perf-regression gate scaffolding (will hold real numbers once P1.7 lands)

**KPIs (PHASE_GATES.md §5 K10 Reproducibility).**
| Metric | HARD | TARGET |
|---|---|---|
| CI byte-identical build rate | 100% over 30 PRs | 100% over 60 PRs |
| Cross-machine rebuild byte-identity | 100% on 3 machines | 100% on 5 machines |
| CI build wall time | ≤25 min | ≤15 min |

**End-to-end test.**
A demo PR that adds a no-op file change builds reproducibly. Outputs are diffed across 3 reference machines (1× x86_64 Linux, 1× ARM64 Linux, 1× x86_64 macOS dev workstation). Hash equality is required for the PR to merge.

**Documentation.**
- `docs/build-and-run.md` — how to clone and `make repro-check`
- ADR-0002: "Buck2 chosen as primary build, Bazel for AOSP only"
- ADR-0004: "Nix flake as toolchain pin source of truth"

**Exit checklist.**
- [ ] Buck2 + Reindeer building all current crates
- [ ] `nix flake.nix` pins Lean 4, Rust, Buck2, Bazel, mathlib4
- [ ] 30 consecutive PRs land with byte-identical CI builds
- [ ] Reproducibility tested on x86_64 + ARM64 + macOS dev workstation
- [ ] CI build wall time ≤25 min p99
- [ ] ADR-0002 and ADR-0004 merged
- [ ] G13 docs published in `docs/`

---

### P1.2 — Lean 4 Toolchain & Extraction Prototype

**Owner.** G1 (Formal Methods Core)
**Estimated duration.** Weeks 2–4
**Dependencies.** P1.1.

**Working state.**
A Lean 4 toolchain pinned in Nix flake, mathlib4 vendored at a specific commit, and a "hello, world" Lean theorem (`theorem zip_local_header_size : ∀ h : ZipLocalHeader, h.size = 30`) that builds and re-verifies on CI. The extraction prototype produces a Rust file from a trivial Lean function and proves equivalence on a single test input.

**Features ready.**
- `theorems/lean-toolchain` pinned to Lean 4 4.x.y
- `lakefile.toml` with mathlib4 dependency
- First Lean module `theorems/Apkaxiom/Hello.lean`
- First extraction proof of concept: Lean `Nat → Nat` function → Rust `fn(u64) -> u64`
- Translation-validator harness skeleton (no real validation yet)

**KPIs.**
| Metric | HARD | TARGET |
|---|---|---|
| Lean theorem re-verify time on CI | ≤10 min | ≤5 min |
| Lean ↔ Rust extraction round-trip on 1 example | works | works |
| Mathlib4 build cache hit rate | ≥90% | ≥99% |

**End-to-end test.**
Run `bazel test //theorems:hello && cargo test -p axiom-extract-hello`. Lean theorem checks; extracted Rust passes its own test.

**Documentation.**
- `docs/lean-setup.md` — local dev environment
- `docs/extraction-architecture.md` — the Lean→Rust pipeline (initial draft)

**Exit checklist.**
- [ ] Lean 4 + mathlib4 pinned in Nix flake
- [ ] `theorems/Apkaxiom/Hello.lean` re-verifies on CI in <10 min
- [ ] Extraction prototype produces compiling Rust from Lean function
- [ ] Translation-validation harness skeleton merged
- [ ] G1 onboarding docs published

---

### P1.3 — apk-info v0.x Audit & v1.0 Architecture Spec

**Owner.** G2 (Parser Engineering)
**Estimated duration.** Weeks 1–3
**Dependencies.** None — runs in parallel with P1.1.

**Working state.**
A 30-page audit report on the upstream `apk-info` codebase ([github.com/delvinru/apk-info](https://github.com/delvinru/apk-info)) is finalized and reviewed. A v1.0 architecture spec for `axiom-l1-rs` (the APKAXIOM fork/successor) is approved by G1, G2, G3 leads.

**Features ready.**
- `docs/apk-info-audit.md` — full audit report (what stays, what's rewritten, what migrates)
- `docs/axiom-l1-rs-spec.md` — v1.0 spec covering streaming reader, per-version trait, type-state, Merkle hooks, IR emitter
- Migration path from apk-info v0.x → axiom-l1-rs v1.0 documented (with timeline)
- ADR-0005: "axiom-l1-rs as the engineering beachhead — not a rewrite"

**KPIs.**
- Audit report ≥30 pages, reviewed by ≥3 senior engineers (no perf KPIs at this sub-phase — design only)

**End-to-end test.**
N/A (design phase). Outputs are documents reviewed by leads.

**Documentation.**
- Audit + spec listed above

**Exit checklist.**
- [ ] Audit report finalized and signed off by G1, G2, G3 leads
- [ ] axiom-l1-rs v1.0 spec approved
- [ ] Migration path documented with timeline
- [ ] ADR-0005 merged

---

### P1.4 — AXIOM-IR v0.1 Draft Spec (Manifest + Resource Dialects)

**Owner.** G3 (AXIOM-IR & Bundle Resolver)
**Estimated duration.** Weeks 2–6
**Dependencies.** P1.2 (need Lean type-system input).

**Working state.**
A frozen specification of AXIOM-IR v0.1 covering the manifest and resource dialects, with type signatures, lowerings between dialects, and a reference Rust implementation of the IR data structures.

**Features ready.**
- `docs/AXIOM-IR-v0.1.md` — full spec
- `crates/axiom-ir` skeleton with manifest + resource dialect types
- `serde` round-trip tests for IR serialization
- Lowering stubs (manifest → resource, no real semantics yet)

**KPIs.**
| Metric | HARD | TARGET |
|---|---|---|
| Spec frozen and unchanged for ≥4 weeks before P1.15 starts | yes | yes |
| Reviewer sign-off | G1, G2, G3, G4 leads | + G5 lead |
| `axiom-ir` crate compiles + serde round-trips on 100 hand-written manifests | yes | yes |

**End-to-end test.**
`cargo test -p axiom-ir` — round-trip manifest IR serialization on 100 hand-crafted samples.

**Documentation.**
- `docs/AXIOM-IR-v0.1.md` (the spec itself)
- ADR-0006: "AXIOM-IR v0.1 dialect set"

**Exit checklist.**
- [ ] AXIOM-IR-v0.1.md spec frozen; signed off by G1, G2, G3, G4 leads
- [ ] `crates/axiom-ir` compiles
- [ ] 100-sample serde round-trip test green
- [ ] ADR-0006 merged

---

### P1.5 — Lean ZIP Layer: Local File Headers + EOCD

**Owner.** G1
**Estimated duration.** Weeks 3–7
**Dependencies.** P1.2.

**Working state.**
The ZIP local file header (LFH) structure and the end-of-central-directory record (EOCD) are formalized in Lean 4. A theorem states that parsing an LFH-prefixed byte sequence yields the typed structure that AOSP `libziparchive` would produce for that same prefix.

**Features ready.**
- `theorems/Apkaxiom/Zip/LocalHeader.lean` — ~600 LOC Lean
- `theorems/Apkaxiom/Zip/Eocd.lean` — ~400 LOC Lean
- Theorem: `parseLfh_sound : ∀ bs, parseLfh bs = ok h → libziparchive_parseLfh bs = ok h`
- Property-based test corpus (≥1000 hand-fuzzed LFHs) where Lean and reference match

**KPIs.**
| Metric | HARD | TARGET |
|---|---|---|
| Theorems machine-checked on CI | ≤15 min | ≤8 min |
| ≥1,000 sample LFHs verified Lean ↔ libziparchive match | 100% | 100% |
| Lean LOC | ≥1,000 LOC | — |

**End-to-end test.**
Property-based test (Hypothesis-style) generates 1,000 random LFHs and 100 valid EOCDs; each is parsed by Lean (reference evaluator) and by AOSP's `libziparchive` (compiled inside the Bazel sub-workspace from P1.1). Outputs must agree byte-for-byte.

**Documentation.**
- `docs/lean-zip-layer.md` — design notes, invariants, edge cases

**Exit checklist.**
- [ ] LFH theorem stated and proved
- [ ] EOCD theorem stated and proved
- [ ] Property-based corpus ≥1,000 inputs, 100% Lean↔AOSP agreement
- [ ] Lean module re-verifies on CI in ≤15 min
- [ ] Lean LOC count ≥1,000

---

### P1.6 — Lean ZIP Layer: Central Directory + Offsets

**Owner.** G1
**Estimated duration.** Weeks 6–10
**Dependencies.** P1.5.

**Working state.**
The ZIP central directory record (CDR) is formalized in Lean 4, including offset arithmetic and the relationship between CDR entries and LFHs. A theorem states the consistency: the offset in the CDR points to a valid LFH.

**Features ready.**
- `theorems/Apkaxiom/Zip/CentralDirectory.lean` — ~1,000 LOC Lean
- `theorems/Apkaxiom/Zip/Consistency.lean` — connecting LFH+CDR+EOCD
- Theorem: `cdr_lfh_offset_valid : ∀ cdr lfh, parseCdr bs = ok cdr → cdr.offset_to_lfh < bs.size ∧ parseLfh (bs.drop cdr.offset_to_lfh) = ok lfh`
- Adversarial test corpus (≥500 malformed ZIPs from public BadPack-class samples)

**KPIs.**
| Metric | HARD | TARGET |
|---|---|---|
| Theorem re-verify on CI | ≤25 min | ≤15 min |
| Adversarial corpus rejection (Lean rejects iff AOSP rejects) | 100% agreement | 100% |
| Cumulative Lean LOC | ≥2,000 LOC | — |

**End-to-end test.**
Run the full adversarial corpus through both Lean and AOSP A14 `libziparchive`. They must agree on accept/reject for every input.

**Documentation.**
- `docs/lean-zip-layer.md` updated with CDR/consistency notes

**Exit checklist.**
- [ ] CDR theorem stated and proved
- [ ] Cross-record consistency theorem proved
- [ ] ≥500 adversarial inputs, 100% Lean↔AOSP agreement
- [ ] Cumulative Lean LOC ≥2,000

---

### P1.7 — apk-info v1.0 Streaming Reader Trait

**Owner.** G2
**Estimated duration.** Weeks 3–7
**Dependencies.** P1.3.

**Working state.**
`axiom-l1-rs` exposes a streaming entry point: `ApkParser::from_reader<R: io::Read>` produces an event stream as bytes arrive, without loading the file into memory.

**Features ready.**
- `crates/axiom-l1-rs/src/stream.rs` — async streaming parser using **Glommio** runtime
- Event-stream API emitting `ParseEvent` variants
- Backpressure correctness verified
- Bench harness `bench/stream-vs-file.rs` comparing streaming vs file-load

**KPIs (PHASE_GATES.md §5 K6 Real-time + K1 Throughput).**
| Metric | HARD | TARGET |
|---|---|---|
| Time-to-first-Merkle-commit from byte 0 | ≤5 ms | ≤2 ms |
| Streaming decision latency (committed package name) | ≤20 ms typical APK | ≤8 ms |
| Wire-speed inspection bandwidth single-core | ≥500 Mbps | ≥1 Gbps |
| Streaming-vs-file-load throughput parity | within 5% | within 1% |

**End-to-end test.**
A synthetic byte-stream feeder (1 Gbps constant rate) drives the streaming parser. Time-to-first-event measured; sustained throughput measured for 60 minutes. No unbounded buffer growth.

**Documentation.**
- `docs/streaming-architecture.md`

**Exit checklist.**
- [ ] `ApkParser::from_reader` lands
- [ ] Glommio runtime integrated
- [ ] Time-to-first-commit ≤5 ms p99 on Bench-1K
- [ ] Wire-speed test sustains ≥500 Mbps for 60 min
- [ ] Streaming-vs-file-load throughput within 5%

---

### P1.8 — apk-info v1.0 Type-State Phantom-Type Guards

**Owner.** G2
**Estimated duration.** Weeks 6–9
**Dependencies.** P1.7.

**Working state.**
Parser states are encoded as phantom types: `Apk<Unverified>`, `Apk<SignatureVerified>`, `Apk<FullyParsed<V>>`. Misuse (e.g., calling `manifest()` on `Apk<Unverified>`) is a compile-time error, not a runtime panic.

**Features ready.**
- `crates/axiom-l1-rs/src/state.rs` — phantom type definitions
- API refactor: every public method gates on type-state
- Compile-fail tests (`trybuild`) ensuring misuse is rejected at compile time
- Translation-validation hooks: phantom states correspond to Lean inductive constructor branches

**KPIs.**
| Metric | HARD | TARGET |
|---|---|---|
| Zero runtime overhead vs untyped (≤0.1% perf delta) | yes | yes |
| Compile-fail tests covering 20+ misuse patterns | ≥20 | ≥40 |
| Translation-validation maps all phantom states to Lean constructors | 100% | 100% |

**End-to-end test.**
`cargo test --features compile-fail` runs the trybuild suite. Every misuse pattern fails to compile with the expected error message.

**Documentation.**
- `docs/type-state.md`

**Exit checklist.**
- [ ] Phantom type set landed
- [ ] All public APIs gated by type-state
- [ ] ≥20 compile-fail tests pass
- [ ] Perf delta vs untyped ≤0.1%
- [ ] Translation-validation mapping documented

---

### P1.9 — Rust Extraction Pipeline v0.1 + Translation Validator

**Owner.** G1 + G2
**Estimated duration.** Weeks 7–11
**Dependencies.** P1.5, P1.8.

**Working state.**
The Lean → Rust extractor compiles a Lean module into a Rust crate. A separate translation-validator runs both the Lean reference evaluator and the extracted Rust on a corpus, verifying byte-for-byte output equality. Discrepancies fail the CI gate.

**Features ready.**
- `tools/lean-to-rust` — extractor binary
- `tools/translation-validator` — runs both, diffs outputs
- First real extracted module: `axiom-l0-zip-lfh` from P1.5's Lean
- CI gate: extraction reproducible bit-identical across machines

**KPIs.**
| Metric | HARD | TARGET |
|---|---|---|
| Extracted Rust output byte-identical across runs | 100% | 100% |
| Translation-validation corpus size | ≥1,000 inputs | ≥10,000 |
| Translation-validation passes | 100% | 100% |
| Extracted Rust ↔ Lean perf delta on Bench-1K | within 30% | within 10% |

**End-to-end test.**
`make extract && make tv` extracts the LFH parser to Rust, runs the translation validator on 1,000 LFH inputs, asserts 100% agreement, and benchmarks both. Performance ratio reported.

**Documentation.**
- `docs/extraction-pipeline.md` (replaces the P1.2 draft)

**Exit checklist.**
- [ ] `lean-to-rust` extractor compiles a non-trivial Lean module
- [ ] First real extracted module (LFH parser) lands
- [ ] Translation validator green on 1,000+ inputs
- [ ] Extraction byte-identical on 3 reference machines
- [ ] Extraction documented end-to-end

---

### P1.10 — apk-info v1.0 BLAKE3 Merkle Commit Hooks (HACL\* Verified)

**Owner.** G2
**Estimated duration.** Weeks 8–11
**Dependencies.** P1.7.

**Working state.**
Every parse step in `axiom-l1-rs` emits a BLAKE3 hash of the byte range it consumed, chained into a Merkle commitment tree. The BLAKE3 implementation is the **HACL\*-verified** version, not a generic Rust port.

**Features ready.**
- HACL\* BLAKE3 bindings (`crates/axiom-blake3-hacl`)
- `CommitChain` API on the parser
- Merkle Patricia Trie for the commit chain
- Performance: BLAKE3 hashing does not bottleneck streaming throughput

**KPIs.**
| Metric | HARD | TARGET |
|---|---|---|
| BLAKE3 hashing throughput single-core | ≥1.5 GB/s | ≥3 GB/s |
| Streaming throughput with hashing on | within 10% of without | within 3% |
| Merkle root reproducibility across runs | 100% bit-identical | 100% |
| HACL\* BLAKE3 in use (not generic) | yes | yes |

**End-to-end test.**
Parse Bench-1K twice, compute Merkle roots, assert bit-identity. Compare streaming throughput with and without hashing; ratio must satisfy KPI.

**Documentation.**
- `docs/merkle-commits.md`

**Exit checklist.**
- [ ] HACL\* BLAKE3 bindings landed
- [ ] CommitChain API on streaming parser
- [ ] Merkle root reproducible bit-identical
- [ ] Throughput KPI met (≤10% impact)
- [ ] Merkle root format documented

---

### P1.11 — Lean APK Signing Block v1/v2/v3/v3.1

**Owner.** G1
**Estimated duration.** Weeks 9–14
**Dependencies.** P1.6.

**Working state.**
All four APK signing schemes are formalized in Lean 4. A theorem states that Lean's `verifySignature` returns `true` iff `apksigner verify` accepts the signature.

**Features ready.**
- `theorems/Apkaxiom/Signing/V1.lean` — JAR signing
- `theorems/Apkaxiom/Signing/V2.lean` — APK Signature Scheme v2
- `theorems/Apkaxiom/Signing/V3.lean` — APK Signature Scheme v3
- `theorems/Apkaxiom/Signing/V3_1.lean` — APK Signature Scheme v3.1
- Cross-scheme dispatch theorem
- Soundness theorem against AOSP `apksigner` reference

**KPIs.**
| Metric | HARD | TARGET |
|---|---|---|
| Theorems re-verify on CI | ≤45 min | ≤25 min |
| Cumulative Lean LOC after this sub-phase | ≥4,000 LOC | — |
| ≥2,000 signed APKs Lean↔apksigner agreement | 100% | 100% |
| All 4 schemes covered | yes | yes |

**End-to-end test.**
A corpus of 2,000 signed APKs (mix of all 4 schemes) is verified by Lean and by AOSP's `apksigner`. They must agree on accept/reject for every input. Includes adversarial samples (modified signatures, length extension attempts, downgrade attacks).

**Documentation.**
- `docs/lean-signing.md`

**Exit checklist.**
- [ ] All 4 signing schemes formalized
- [ ] Cross-scheme dispatch proved
- [ ] 2,000-sample agreement test green
- [ ] Theorems re-verify in ≤45 min on CI
- [ ] Adversarial signature samples included in test corpus

---

### P1.12 — Rust Extraction of Full ZIP Layer

**Owner.** G1 + G2
**Estimated duration.** Weeks 11–14
**Dependencies.** P1.6, P1.9.

**Working state.**
The full Lean ZIP layer (LFH + CDR + EOCD + consistency) is extracted to Rust and replaces the hand-written ZIP parser inside `axiom-l0`. The translation validator passes on the full Bench-1K corpus.

**Features ready.**
- Extracted crate `axiom-l0-zip-verified`
- `axiom-l0` switched to use the verified crate by default (with a feature flag for fallback)
- Translation validator runs nightly on Bench-10K
- Performance: verified extraction is within 10% of hand-written Rust

**KPIs (PHASE_GATES.md §5 K1 + K2).**
| Metric | HARD | TARGET |
|---|---|---|
| L0 sustained throughput (verified path) | ≥250 APKs/sec on 16-core | ≥400 APKs/sec |
| L0 p99 latency (verified path) | ≤80 ms | ≤40 ms |
| Verified vs hand-written perf delta | within 15% | within 5% |
| Translation validator on Bench-10K nightly | green | green |

**End-to-end test.**
Run Bench-10K through verified `axiom-l0`. Compare per-APK output bytes to the hand-written reference (where it exists). Compare aggregate throughput to the unverified baseline.

**Documentation.**
- `docs/verified-l0.md`

**Exit checklist.**
- [ ] Full ZIP layer extracted
- [ ] `axiom-l0` defaults to verified
- [ ] Translation validator green on Bench-10K
- [ ] Verified perf within 15% of hand-written
- [ ] Throughput ≥250 APKs/sec on 16-core

---

### P1.13 — Differential Fuzzing Plant — Cuttlefish A14 Harness via Nyx

**Owner.** G8 (Differential Fuzzing Plant)
**Estimated duration.** Weeks 4–12
**Dependencies.** P1.1.

**Working state.**
A Cuttlefish A14 image is wrapped as a Nyx fuzzing harness. The fuzzer mutates ZIP/APK byte structures and observes whether the A14 install pipeline accepts or rejects the input. Disagreements with our `axiom-l0` parser are logged.

**Features ready.**
- Cuttlefish A14 hermetic image (built via Bazel sub-workspace)
- Nyx snapshot + harness wrapper
- APK grammar (initial) for Nautilus-style mutation guidance
- Disagreement classifier — manual taxonomy at first, automated by P1.14
- Fuzzing dashboard (Grafana, fed by Prometheus)

**KPIs (PHASE_GATES.md §5 K12 — early surfacing).**
| Metric | HARD | TARGET |
|---|---|---|
| Nyx harness operational | yes | yes |
| Fuzzer running 24/7 | ≥99% uptime | ≥99.9% |
| Disagreements logged per week | ≥5 (raw) | ≥30 |
| Fuzz-corpus persistence | yes (rkyv) | yes |

**End-to-end test.**
Run the fuzzer for 7 consecutive days. At end of week, ≥5 distinct (verified-by-replay) disagreements between A14 install pipeline and `axiom-l0`. Each disagreement reproducible byte-for-byte.

**Documentation.**
- `docs/differential-fuzzer.md`

**Exit checklist.**
- [ ] Cuttlefish A14 image hermetically built
- [ ] Nyx wrapper operational
- [ ] APK grammar drafted
- [ ] 7-day continuous run with ≥99% uptime
- [ ] ≥5 disagreements logged with reproducer

---

### P1.14 — Differential Fuzzing Plant — A8 + A11 Harnesses + Auto Classifier

**Owner.** G8
**Estimated duration.** Weeks 12–18
**Dependencies.** P1.13.

**Working state.**
Two more Cuttlefish images (A8, A11) added as parallel harnesses. The disagreement classifier is now automated — sorts findings into the 3-way taxonomy: AOSP CVE candidate / model bug / spec ambiguity.

**Features ready.**
- 3 parallel harnesses running 24/7
- Cross-version differential mode (A8 ↔ A11 ↔ A14 disagreements surfaced)
- Automated classifier (rules-based, with manual override)
- Dashboard categorizing findings

**KPIs.**
| Metric | HARD | TARGET |
|---|---|---|
| Disagreements/week classified | ≥10 | ≥30 |
| Classifier precision (validated by manual review) | ≥80% | ≥95% |
| 3 harnesses, ≥99% uptime each | yes | yes |
| Cross-version disagreements (A8 vs A14) found | ≥1 | ≥5 |

**End-to-end test.**
Sustained 14-day run: ≥10 disagreements/week classified with ≥80% precision verified by manual sampling.

**Documentation.**
- `docs/differential-fuzzer.md` updated with classifier rules

**Exit checklist.**
- [ ] A8 and A11 Cuttlefish harnesses live
- [ ] Classifier automated (≥80% precision)
- [ ] 14-day sustained run, KPIs met
- [ ] Cross-version disagreement found and reproduced
- [ ] Findings dashboard live

---

### P1.15 — apk-info v1.0 AXIOM-IR-v0.1 Emitter

**Owner.** G2 + G3
**Estimated duration.** Weeks 12–17
**Dependencies.** P1.4, P1.10.

**Working state.**
`axiom-l1-rs` emits AXIOM-IR-v0.1 (manifest dialect + resource dialect) for every parsed APK. Round-trip test: parse → IR → re-encode → byte-identical original (where the dialect is lossless).

**Features ready.**
- `axiom_l1_rs::ir` module emitting both dialects
- Round-trip test on Bench-1K
- IR output deterministic (same APK → same IR bytes always)
- IR → JSON debug serialization for human inspection

**KPIs.**
| Metric | HARD | TARGET |
|---|---|---|
| Round-trip byte-identity on Bench-1K | ≥95% APKs | ≥99% |
| IR emission overhead vs no-IR | ≤15% throughput hit | ≤5% |
| IR output deterministic | 100% reproducible | 100% |

**End-to-end test.**
For every APK in Bench-1K: parse → IR → re-encode → diff against original. ≥95% must be byte-identical (the rest are documented exceptions where the dialect is intentionally lossy).

**Documentation.**
- `docs/ir-emitter.md`

**Exit checklist.**
- [ ] Manifest dialect emitter lands
- [ ] Resource dialect emitter lands
- [ ] Round-trip test ≥95% byte-identical on Bench-1K
- [ ] IR emission overhead ≤15%
- [ ] IR deterministic across runs

---

### P1.16 — Rust Extraction of APK Signing Block

**Owner.** G1 + G2
**Estimated duration.** Weeks 14–18
**Dependencies.** P1.11, P1.12.

**Working state.**
APK Signing Block v1/v2/v3/v3.1 verifiers extracted from Lean to Rust. All cryptographic operations route through HACL\* (verified Ed25519, RSA, ECDSA, BLAKE3). The translation validator passes on the 2,000-APK signing corpus.

**Features ready.**
- Extracted crate `axiom-l1-signing-verified`
- HACL\* crypto bindings for the four needed primitives
- Translation validator integration
- Performance benchmark vs apksigner

**KPIs.**
| Metric | HARD | TARGET |
|---|---|---|
| Signature verification throughput | ≥1,000 APKs/sec/core | ≥3,000 APKs/sec/core |
| Verified vs hand-written perf delta | within 20% | within 8% |
| Translation validator on 2,000 signed APKs | 100% agreement | 100% |
| HACL\* crypto on the verified path (no generic) | yes | yes |

**End-to-end test.**
Run signing-block verification across Bench-10K. Compare verdicts (accept/reject) byte-for-byte against `apksigner verify`. 100% agreement required.

**Documentation.**
- `docs/verified-signing.md`

**Exit checklist.**
- [ ] All 4 signing schemes extracted
- [ ] HACL\* crypto integrated
- [ ] Translation validator green on signing corpus
- [ ] Throughput ≥1,000 APKs/sec/core
- [ ] Bench-10K verdict agreement with apksigner = 100%

---

### P1.17 — Soundness Regression Suite as Fail-Closed CI Gate

**Owner.** G1 + G13
**Estimated duration.** Weeks 12–20
**Dependencies.** P1.9, P1.16.

**Working state.**
Every PR that touches Lean theorems or extracted Rust must pass the soundness regression suite — re-verify of all theorems plus translation-validation on the full Bench-1K corpus. The gate is **fail-closed**: a red gate blocks merge, no override.

**Features ready.**
- CI workflow `.github/workflows/soundness.yml`
- `make soundness` runs the full suite locally
- 30+ PRs land with green gates (proves the gate is real, not theatrical)
- Quarterly mathlib4 upgrade dry-run

**KPIs (PHASE_GATES.md §5 K11).**
| Metric | HARD | TARGET |
|---|---|---|
| Lean theorem re-verify | 100% green on every L1 PR | same |
| Proof-drift incidents | 0 | 0 |
| Soundness regression CI wall-time | ≤90 min p99 | ≤45 min |
| 30 consecutive PRs land with green soundness | yes | yes |

**End-to-end test.**
Inject a deliberate proof-breaking PR (sandbox branch) — confirm CI blocks merge with the expected error. Then revert. Confirm CI returns green.

**Documentation.**
- `docs/soundness-regression.md`

**Exit checklist.**
- [ ] Soundness CI workflow live
- [ ] `make soundness` documented
- [ ] 30 consecutive PRs green
- [ ] Deliberate-break test confirms fail-closed
- [ ] Quarterly mathlib4 upgrade runbook documented

---

### P1.18 — End-to-End Bench-1K Smoke + Bench-10K Performance Eval

**Owner.** All Phase 1 groups (G1, G2, G3, G8, G13)
**Estimated duration.** Weeks 18–22
**Dependencies.** P1.15, P1.16, P1.17.

**Working state.**
Full L0 + L1 (verified) + AXIOM-IR-v0.1 emission + signature verification runs end-to-end on Bench-1K (smoke) and Bench-10K (perf eval). All Phase 1 KPIs from PHASE_GATES.md §5 are measured and reported.

**Features ready.**
- E2E test harness `tests/e2e/phase1.rs`
- Performance dashboards live (Grafana + Pyroscope)
- Reproducibility audit on Bench-1K (every APK → same Merkle root + same IR + same verdict)
- Comparison vs apk-info v0.x baseline

**KPIs (PHASE_GATES.md §5 — full Phase 1 gate set).**
All §5 KPIs apply. Highlights:
| Metric | HARD | TARGET |
|---|---|---|
| Sustained parse throughput, 16-core | ≥300 APKs/sec | ≥500 APKs/sec |
| L0+L1 parse p99 | ≤300 ms | ≤200 ms |
| Peak RSS per worker | ≤150 MB | ≤80 MB |
| 24h soak: zero crashes | yes | yes |
| Bench-1K reproducibility | 100% | 100% |
| 1→16 core efficiency | ≥70% | ≥85% |

**End-to-end test.**
- 24h soak: replay Stress-100K (or repeat Bench-10K 10×) continuously. Zero crashes.
- Reproducibility: replay Bench-1K twice; diff every Merkle root + IR; 100% bit-identical.
- Cross-architecture: same Bench-1K on x86_64 and ARM64; verdicts identical.

**Documentation.**
- `docs/phase1-eval.md` — published numbers + dashboard links

**Exit checklist.**
- [ ] E2E test harness lands
- [ ] Bench-1K smoke green, 100% reproducible
- [ ] Bench-10K perf eval published
- [ ] All §5 hard KPIs met
- [ ] 24h soak green, zero crashes
- [ ] Cross-arch verdicts identical

---

### P1.19 — Public AndroZoo Benchmark + Phase-1 Paper Draft

**Owner.** All Phase 1 groups
**Estimated duration.** Weeks 20–24
**Dependencies.** P1.18.

**Working state.**
APKAXIOM-Phase1 evaluated on a 10K AndroZoo subset. Numbers published. Phase-1 paper drafted, ready for submission to **CAV 2026** or **OOPSLA 2026** ("Verified Parsing for the Android Package Format").

**Features ready.**
- AndroZoo 10K subset run, results dashboarded
- Comparison: APKAXIOM-Phase1 vs apk-info v0.x vs Androguard
- Paper draft (~10 pages) for top-venue submission
- Internal demo: 1,000 known-good APKs parsed across A8/A11/A14 with Lean proof check passing

**KPIs.**
| Metric | HARD | TARGET |
|---|---|---|
| AndroZoo 10K eval coverage (parsed without error) | ≥99% | ≥99.5% |
| Comparison numbers vs apk-info v0.x: no regression | yes | better |
| Comparison numbers vs Androguard: ≥10× faster | yes | ≥15× |
| Paper draft length | ≥10 pages | ≥12 pages |

**End-to-end test.**
The AndroZoo 10K subset run is itself the test. ≥99% coverage required.

**Documentation.**
- `papers/phase1-cav.tex` — paper draft
- `docs/phase1-eval.md` updated with AndroZoo numbers

**Exit checklist.**
- [ ] AndroZoo 10K eval complete, ≥99% coverage
- [ ] Comparison numbers vs apk-info + Androguard published
- [ ] Paper draft ≥10 pages
- [ ] Internal demo run cleanly (1K APKs across A8/A11/A14)
- [ ] Public benchmark dashboard live

---

### P1.20 — Phase 1 Hard-Gate Review + Phase 2 ADR

**Owner.** Leadership + all Phase 1 groups
**Estimated duration.** Weeks 24–26
**Dependencies.** P1.19.

**Working state.**
Every PHASE_GATES.md §5 hard gate reviewed against the live dashboard. Failed targets logged as carry-forward debt. Phase 2 scope ADR written and approved.

**Features ready.**
- Phase 1 gate review meeting (recorded + minuted)
- Filed ADR-Phase2-Scope: which P1 target gates carry forward, scope adjustments, hiring asks
- Phase 1 retrospective document
- Sign-off from G1, G2, G3, G8, G13 leads + leadership

**KPIs.**
| Metric | HARD |
|---|---|
| All §5 hard gates ✅ for ≥7 consecutive days | yes |
| All §5 target gates either met or documented as carry-forward debt | yes |
| Phase 2 scope ADR approved | yes |
| Phase 1 retrospective complete | yes |

**End-to-end test.**
The dashboard itself is the test. The gate review walks through every line of PHASE_GATES.md §5 and verifies each item against live data.

**Documentation.**
- `docs/phase1-retrospective.md`
- ADR-Phase2-Scope

**Exit checklist (this is the Phase 1 ship gate — all hard).**
- [ ] PHASE_GATES.md §5 K1 throughput hard gates met (≥7 days green)
- [ ] PHASE_GATES.md §5 K2 latency hard gates met
- [ ] PHASE_GATES.md §5 K3 memory hard gates met
- [ ] PHASE_GATES.md §5 K4 CPU efficiency hard gates met
- [ ] PHASE_GATES.md §5 K5 scalability hard gates met
- [ ] PHASE_GATES.md §5 K6 real-time hard gates met
- [ ] PHASE_GATES.md §5 K7 stability hard gates met (zero soundness regressions; <10 crashes/1M)
- [ ] PHASE_GATES.md §5 K8 stress/burst hard gates met
- [ ] PHASE_GATES.md §5 K9 cross-platform parity met
- [ ] PHASE_GATES.md §5 K10 reproducibility 100%
- [ ] PHASE_GATES.md §5 K11 soundness regressions = 0
- [ ] AXIOM-IR-v0.1 spec frozen and unchanged ≥4 weeks
- [ ] apk-info v1.0 (`axiom-l1-rs`) released, no perf regression vs v0.x
- [ ] Differential fuzzer ≥10 disagreements/week classified, ≥99% uptime
- [ ] Bench-1K E2E smoke green; Bench-10K perf eval published
- [ ] AndroZoo 10K eval published
- [ ] Phase-1 paper drafted, ready for submission
- [ ] Phase 2 scope ADR approved
- [ ] Phase 1 retrospective merged
- [ ] Sign-off from G1, G2, G3, G8, G13 leads + leadership

---

<a id="exit-gate"></a>
## 6. Phase 1 Consolidated Exit Gate

A single checklist combining everything above. **Every box must be ✅ on the live dashboard for ≥7 consecutive days for Phase 1 to close and Phase 2 to start.**

```
Foundations (P1.1, P1.2)
[ ] Buck2 hermetic CI: 30 consecutive PRs byte-identical
[ ] Nix flake pins all toolchains
[ ] Lean 4 + mathlib4 vendored, "hello" theorem re-verifies in <10 min

Specs (P1.3, P1.4)
[ ] apk-info v0.x audit signed off
[ ] axiom-l1-rs v1.0 spec approved
[ ] AXIOM-IR-v0.1 spec frozen ≥4 weeks before P1.18

Lean ZIP layer (P1.5, P1.6)
[ ] LFH + EOCD theorems proved, Lean↔AOSP agreement on ≥1,000 inputs
[ ] CDR theorem proved, adversarial corpus 100% Lean↔AOSP agreement
[ ] Cumulative Lean LOC ≥2,000

apk-info v1.0 refactor (P1.7, P1.8, P1.10, P1.15)
[ ] Streaming reader with Glommio, time-to-first-commit ≤5 ms p99
[ ] Type-state phantom types, 20+ compile-fail tests pass
[ ] HACL*-verified BLAKE3 Merkle commits, ≥1.5 GB/s
[ ] AXIOM-IR-v0.1 emitter, ≥95% byte-identical round-trip on Bench-1K

Lean Signing Block (P1.11)
[ ] All 4 signing schemes formalized
[ ] 2,000-APK Lean↔apksigner agreement = 100%

Extraction pipeline (P1.9, P1.12, P1.16)
[ ] Lean→Rust extractor lands, byte-identical across machines
[ ] Verified ZIP layer replaces hand-written; perf within 15%
[ ] Verified signing block; 100% verdict agreement with apksigner

Differential Fuzzing Plant (P1.13, P1.14)
[ ] 3 Cuttlefish harnesses (A8, A11, A14) running ≥99% uptime
[ ] Classifier ≥80% precision; ≥10 disagreements/week
[ ] Cross-version disagreement found and reproduced

Soundness regression (P1.17)
[ ] CI fail-closed gate live, deliberate-break test confirmed
[ ] 30 consecutive PRs land with green soundness gate

E2E and eval (P1.18, P1.19)
[ ] Bench-1K E2E smoke 100% reproducible
[ ] Bench-10K perf eval all hard KPIs met
[ ] 24h soak: zero crashes
[ ] Cross-arch (x86_64 ↔ ARM64) verdicts identical
[ ] AndroZoo 10K eval ≥99% coverage published
[ ] CAV/OOPSLA paper draft ≥10 pages

Final Phase 1 KPIs (PHASE_GATES.md §5)
[ ] Sustained ≥300 APKs/sec on 16-core
[ ] L0+L1 p99 ≤300 ms
[ ] Peak RSS ≤150 MB
[ ] 1→16 core efficiency ≥70%
[ ] CI byte-identical 100% over 100 PRs
[ ] Soundness regressions = 0
[ ] x86_64 ↔ ARM64 throughput within 25%
[ ] Wire-speed ≥500 Mbps single-core sustained 60 min

Phase 2 readiness (P1.20)
[ ] All carry-forward debt logged
[ ] Phase 2 scope ADR approved
[ ] Phase 1 retrospective merged
[ ] All group leads + leadership signed off
```

---

<a id="risks"></a>
## 7. Phase 1 Risk Register

Specific to Phase 1 — broader risks live in [../ROADMAP.md §13](../ROADMAP.md#risks).

| Risk | Impact | Probability | Sub-phase exposed | Mitigation |
|---|---|---|---|---|
| G1 understaffing — Lean PhDs not hired by M0 | Critical | High | All G1 sub-phases | Pre-Phase-0 hiring; contractor budget; ADR for partial Coq fallback |
| Lean toolchain churn breaks extraction mid-phase | High | Medium | P1.2, P1.9, P1.12 | Vendor pinned Lean; G13 owns toolchain upgrade decision |
| AOSP releases A15 mid-phase, harness work spills | Medium | Medium | P1.13, P1.14 | A15 deferred to Phase 2; G2 archaeology absorbs the diff |
| HACL\* BLAKE3 perf insufficient | High | Low | P1.10, P1.16 | Fallback to BLAKE3 reference impl + property-based equivalence test against HACL\* |
| Translation-validator finds extraction bugs late | High | Medium | P1.9, P1.12 | Run validator nightly from P1.9 onward; weekly review |
| Cuttlefish image build fails reproducibly | Medium | Low | P1.13 | AVD fallback documented; G13 invests in build-system support |
| AXIOM-IR-v0.1 spec changes mid-phase under pressure | High | Medium | P1.4 | ADR-required for any post-freeze change; G3 lead has veto |
| Bench-10K perf eval falls short of HARD gate | Critical | Low | P1.18 | Profiling continuous via Pyroscope; weekly perf review |
| 24h soak finds memory leak late | High | Low | P1.18 | Soak runs nightly from P1.7 onward |

---

<a id="defs"></a>
## 8. Definitions

- **Working state.** What exists in the repo (binaries, modules, tests, docs) when this sub-phase is complete.
- **Features ready.** Concrete deliverables a user (or downstream sub-phase) can rely on.
- **KPIs.** Numeric thresholds drawn from [PHASE_GATES.md §5](../PHASE_GATES.md#phase-1). HARD blocks; TARGET is best-effort.
- **End-to-end test.** A specific test (or suite) whose passing demonstrates the sub-phase actually works as advertised.
- **Documentation.** Markdown / spec / paper artifacts published in this sub-phase.
- **Exit checklist.** The pass/fail items reviewed at sub-phase close. **Every item must be ✅ for ≥7 consecutive days for the sub-phase to close.**

---

*"A sub-phase closes when its slowest, hottest, dirtiest case meets its KPI on a live dashboard. Not when the happy path is green."*
