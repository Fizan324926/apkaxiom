# APKAXIOM — Phase Gates: Comprehensive Performance KPI Checklist

> The end-of-phase checklist focused on **speed, performance, scalability, and real-time results.**
> Every KPI is measurable, dated, gated, and tested end-to-end on a public corpus.

This document is the third pillar alongside [README.md](./README.md) (architecture) and [ROADMAP.md](./ROADMAP.md) (temporal plan). The other two define *what* and *when*. **This one defines *how well* — and what blocks progression.**

A phase does not advance because the features are written. A phase advances **only when every hard gate in this document is green for 7 consecutive days**.

---

## Table of Contents

1. [How to Read This Document](#how-to-read)
2. [Hard Gate vs Target Gate](#gate-types)
3. [The 12 KPI Categories](#categories)
4. [Reference Benchmark Corpora](#corpora)
5. [Phase 1 Gate — Parser Foundation](#phase-1)
6. [Phase 2 Gate — Bundle Era](#phase-2)
7. [Phase 3 Gate — Symbolic & Equivalence](#phase-3)
8. [Phase 4 Gate — Certificates & Tooling](#phase-4)
9. [Phase 5 Gate — Native + Dynamic + ML](#phase-5)
10. [Phase 6 Gate — v1.0 Production Ship](#phase-6)
11. [Continuous KPIs (Always-On)](#continuous)
12. [Test Methodology Per Category](#methodology)
13. [Instrumentation Requirements](#instrumentation)
14. [Failure Modes & Re-test Protocol](#failures)

---

<a id="how-to-read"></a>
## 1. How to Read This Document

For each phase you will find:

- A **KPI table** with hard gates and target gates. Every row has: metric, definition, test corpus, pass threshold (hard), pass threshold (target), measurement methodology.
- A **stress profile**: sustained throughput, burst tolerance, soak duration.
- A **real-time profile**: streaming/wire-speed properties.
- A **scalability profile**: single-core → multi-core → multi-machine scaling laws.
- A **stability profile**: crash rate, hang rate, leak rate, MTBF.
- A **reproducibility profile**: byte-identity, cross-architecture parity.

Numbers are committed targets, not aspirations. Targets are **derived from apk-info v0.x baselines**, published Android-tool literature, and the constraints of the proof systems we use. They are revisable only via an ADR review at the start of the next phase.

---

<a id="gate-types"></a>
## 2. Hard Gate vs Target Gate

**Hard gate (HARD):** Must pass for the phase to advance. Failing → phase does not close. Re-plan or re-staff.

**Target gate (TARGET):** Should pass. Failing → file an issue, document the gap, but the phase advances. Targets accumulate as technical debt with explicit owners.

**Failing target gates carry forward.** A target gate failed in Phase 1 must be reopened in Phase 2's review and either closed or escalated to a hard gate.

---

<a id="categories"></a>
## 3. The 12 KPI Categories

Every phase is gated against the same 12 categories. Coverage broadens phase-to-phase as more layers come online.

| # | Category | What it measures |
|---|---|---|
| K1 | **Throughput** | APKs processed per second per machine and per cluster |
| K2 | **Latency** | p50, p95, p99 end-to-end and per-layer |
| K3 | **Memory** | Peak RSS per worker, leak rate, fragmentation under sustained load |
| K4 | **CPU efficiency** | Cycles per APK, branch-miss rate, cache-miss rate, instructions per APK |
| K5 | **Scalability** | Multi-core efficiency, multi-machine linearity, storage scaling |
| K6 | **Real-time / streaming** | Time-to-first-commit, wire-speed inspection bandwidth, streaming decision latency |
| K7 | **Stability** | Crash rate, hang/timeout rate, soak duration, mean time between failures |
| K8 | **Stress / burst** | Spike tolerance, sustained-overload graceful degradation, recovery time |
| K9 | **Cross-platform parity** | Throughput parity x86_64 vs ARM64 vs RISC-V, byte-identical outputs |
| K10 | **Reproducibility** | Bit-identical CI rate, cross-machine reproducibility, cross-time rebuild |
| K11 | **Soundness regression** | Lean theorem re-verify rate, proof-drift incidents, fuzzer disagreement classification |
| K12 | **Real-time test campaigns** | Continuous fuzz, weekly malware replay, production canary, chaos drills |

**Phase 1 measures K1–K11.** K12 starts in Phase 2 (need a population of APKs and a fuzzer warmed up). All 12 categories are measured Phase 2 through Phase 6.

---

<a id="corpora"></a>
## 4. Reference Benchmark Corpora

All KPIs are measured against named, versioned corpora. No KPI is "passed" against a private dataset.

| Corpus | Size | Use | Phase available |
|---|---|---|---|
| **APKAXIOM-Smoke** | 100 APKs | Smoke test on every CI run | M0+ |
| **APKAXIOM-Bench-1K** | 1,000 hand-curated APKs (mix benign + malware + bundles + obfuscated) | Per-PR perf gate | M2+ |
| **APKAXIOM-Bench-10K** | 10,000 APKs sampled from AndroZoo | Phase boundary perf eval | M3+ |
| **APKAXIOM-Stress-100K** | 100,000 APKs | Stress / scalability eval | M6+ |
| **APKAXIOM-Eval-50K** | 50,000 APKs (final dataset, public release with v1.0) | Phase 6 ship-gate eval and paper | M30+ |
| **APKAXIOM-Bundles-5K** | 5,000 App Bundles with realistic split distributions | Phase 2+ | M7+ |
| **APKAXIOM-Malware-1K** | 1,000 known-malware samples (DREBIN + AndroZoo + recent feeds) | Soundness & detection eval | M3+ |
| **APKAXIOM-Repack-2K** | 2,000 known repackaging pairs (original + repacked) | BSH and bisim eval | M12+ |
| **APKAXIOM-Adversarial-500** | 500 hand-crafted parser-confusion APKs (BadPack, ZIP-bomb, malformed signing block, etc.) | Adversarial robustness | M3+ |

Each corpus has a manifest file (`corpus.toml`) specifying sample SHA-256s, license, source, and stratification. CI runs are tagged with the corpus version used.

---

<a id="phase-1"></a>
## 5. Phase 1 Gate — Parser Foundation (M6)

**Scope under measurement:** L0 (streaming ZIP) + L1 (Lean-extracted Rust parsers) on Android 14, with smaller coverage of A8 and A11.

### K1 Throughput

| Metric | Corpus | HARD | TARGET | Methodology |
|---|---|---|---|---|
| Sustained parse throughput, 16-core | Bench-10K | ≥300 APKs/sec | ≥500 APKs/sec | Replay 10K APKs through L0+L1, divide by wall time, repeat 5× and take median |
| Single-core parse throughput | Bench-10K | ≥25 APKs/sec | ≥40 APKs/sec | Single worker, no thread pool |
| Cluster throughput, 8-machine × 16-core | Stress-100K (when available) | ≥2,000 APKs/sec | ≥3,500 APKs/sec | Linear projection accepted in Phase 1 if measured ≥4-machine |

### K2 Latency

| Metric | Corpus | HARD | TARGET | Methodology |
|---|---|---|---|---|
| L0+L1 parse p50 | Bench-1K | ≤50 ms | ≤30 ms | Per-APK histogram |
| L0+L1 parse p95 | Bench-1K | ≤150 ms | ≤80 ms | Per-APK histogram |
| L0+L1 parse p99 | Bench-1K | ≤300 ms | ≤200 ms | Per-APK histogram |
| L0+L1 parse worst-case (max) | Adversarial-500 | ≤2 s | ≤500 ms | Adversarial inputs must not blow up |

### K3 Memory

| Metric | Corpus | HARD | TARGET | Methodology |
|---|---|---|---|---|
| Peak RSS per worker | Bench-1K | ≤150 MB | ≤80 MB | `rusage` peak across run |
| Memory growth under 24h soak | Smoke + replay | ≤2 MB/hour | ≤0.1 MB/hour (effectively zero) | jemalloc stats; `valgrind --tool=massif` snapshot |
| Allocation rate per APK | Bench-1K | ≤200K allocs | ≤80K allocs | jemalloc / mimalloc stats |
| Heap fragmentation after 1M APKs | Soak | <15% | <5% | jemalloc stats |

### K4 CPU efficiency

| Metric | Corpus | HARD | TARGET | Methodology |
|---|---|---|---|---|
| Cycles per typical APK | Bench-1K (excluding malformed) | ≤1B | ≤500M | `perf stat` aggregate / N APKs |
| Branch-miss rate | Bench-1K | <3% | <1.5% | `perf stat` |
| L1 i-cache miss rate | Bench-1K | <5% | <2% | `perf stat` |
| Instructions per cycle (IPC) | Bench-1K | ≥1.8 | ≥2.5 | `perf stat` |

### K5 Scalability

| Metric | HARD | TARGET | Methodology |
|---|---|---|---|
| 1→16 core efficiency | ≥70% | ≥85% | (Throughput at 16c) / (16 × Throughput at 1c) |
| 1→4 machine linearity | ≥80% efficiency | ≥95% efficiency | Network overhead acceptable |
| Async/sync mode parity | within 10% | within 3% | Both modes available |

### K6 Real-time / Streaming

| Metric | HARD | TARGET | Methodology |
|---|---|---|---|
| Time-to-first-Merkle-commit from byte 0 | ≤5 ms | ≤2 ms | Synthetic stream feed at 1 Gbps |
| Streaming decision latency (committed package name) | ≤20 ms (typical APK) | ≤8 ms | Local-header parse only |
| Wire-speed inspection bandwidth | ≥500 Mbps | ≥1 Gbps | Single-core parser sustained against a constant byte stream |
| Backpressure correctness | zero unbounded buffers | same | Adversarial slow-consumer test |

### K7 Stability

| Metric | HARD | TARGET | Methodology |
|---|---|---|---|
| Crash rate | <10 per 1M APKs | 0 per 1M APKs | 24h replay of Stress-100K (10×) |
| Hang/timeout rate | <0.5% | <0.1% | Per-APK 30s wall-time limit |
| 24h soak: monotonic memory | no growth >2 MB/hr | no growth | Tracked above; surfaced as crash test |
| Mean time between failures (MTBF) | ≥48 h continuous | ≥240 h | Continuous worker with input stream |

### K8 Stress / Burst

| Metric | HARD | TARGET | Methodology |
|---|---|---|---|
| 5× burst tolerance for 60s | p99 degrades ≤5× | ≤2× | Inject 5× nominal load for 60s |
| 10× burst tolerance for 60s | no crash, recovery in ≤60s | recovery in ≤30s | Inject 10× nominal load |
| Sustained 90% utilization 24h | no degradation | same | Long soak at high util |

### K9 Cross-platform parity

| Metric | HARD | TARGET | Methodology |
|---|---|---|---|
| x86_64 vs ARM64 throughput parity | within 25% | within 15% | Same Bench-10K, same Bazel build |
| x86_64 vs ARM64 cert byte-identity | 100% | same | Hash the parser output bytewise |

### K10 Reproducibility

| Metric | HARD | TARGET | Methodology |
|---|---|---|---|
| CI byte-identical build rate | 100% over 100 PRs | 100% | Bazel hermetic build |
| Cross-machine rebuild byte-identity | 100% on 3 machines | 100% on 5 machines | Reproducibility audit (G13) |
| Parser output reproducibility | 100% bytewise across runs | same | Replay 100K APKs twice, diff outputs |

### K11 Soundness regression

| Metric | HARD | TARGET | Methodology |
|---|---|---|---|
| Lean theorem re-verify | 100% green on every L1 PR | same | CI gate |
| Proof drift incidents | 0 | 0 | Audit log |
| Fuzzer disagreements unresolved | <3 in queue at gate time | 0 | Fuzzer dashboard |

### Phase 1 Stress Profile

```
sustained_load:        500 APKs/sec/machine, 24h         → MUST PASS
burst_5x:              2500 APKs/sec, 60s                → MUST PASS p99 ≤5× nominal
soak_72h:              continuous Stress-100K replay     → MUST PASS, zero crashes
adversarial_drill:     Adversarial-500 fuzz, 7 days      → MUST PASS, zero memory safety
```

### Phase 1 Hard-Gate Summary (one-line)

> Sustained ≥300 APKs/sec on 16-core, p99 ≤300 ms, ≤150 MB peak RSS, 100% reproducible, zero soundness regressions, 24h soak clean. Pass → Phase 2.

---

<a id="phase-2"></a>
## 6. Phase 2 Gate — Bundle Era (M12)

**Scope under measurement:** L0 + L1 + L2 (bundle resolver) + L3 (forensics).

### K1 Throughput

| Metric | Corpus | HARD | TARGET | Methodology |
|---|---|---|---|---|
| L0–L3 sustained throughput, 16-core | Bench-10K | ≥150 APKs/sec | ≥250 APKs/sec | Same as Phase 1 method |
| Bundle resolution overhead | Bundles-5K | ≤60% over single-APK | ≤30% | Compare BehaviorSet construction time vs single-APK parse |
| Forensic pass throughput each | Bench-10K | ≥300 APKs/sec | ≥500 APKs/sec | Per pass (shadow / provenance / negspace) |

### K2 Latency

| Metric | Corpus | HARD | TARGET |
|---|---|---|---|
| L0–L3 p99 | Bench-1K | ≤800 ms | ≤500 ms |
| Bundle resolution p99 (20-split bundle) | Bundles-5K | ≤3 s | ≤1 s |
| Each forensic pass p99 | Bench-1K | ≤80 ms | ≤30 ms |

### K3 Memory

| Metric | HARD | TARGET |
|---|---|---|
| Peak RSS per worker | ≤300 MB | ≤200 MB |
| BehaviorSet memory representation | ≤2.5× raw bundle size | ≤1.8× |

### K4 CPU efficiency

| Metric | HARD | TARGET |
|---|---|---|
| Cycles per bundle | ≤3B | ≤1.5B |
| IPC | ≥1.8 | ≥2.4 |

### K5 Scalability

| Metric | HARD | TARGET |
|---|---|---|
| 1→16 core efficiency | ≥70% | ≥85% |
| 1→8 machine linearity | ≥75% | ≥90% |
| Storage scaling: BehaviorSet on disk per APK | ≤500 KB | ≤200 KB |

### K6 Real-time / Streaming

| Metric | HARD | TARGET |
|---|---|---|
| Streaming bundle ingest from base APK only | ≤100 ms first-finding | ≤30 ms |
| Wire-speed inspection bandwidth | ≥500 Mbps | ≥1 Gbps |

### Forensic Quality KPIs (Phase-2 specific)

| Metric | HARD | TARGET |
|---|---|---|
| Shadow Stack FP rate on benign 10K | <10% | <3% |
| AXML provenance misidentification rate | <5% | <1% |
| Negative-Space FP rate | <20% | <8% |
| Combined forensic FP (any pass fires) on benign | <12% | <5% |

### Bundle Correctness (Phase-2 specific)

| Metric | HARD | TARGET |
|---|---|---|
| Differential test vs AOSP installer | ≥99.9% agreement on Bundles-5K | 100% |
| Dynamic-feature-module discovery rate | ≥95% | 100% |

### K7 Stability

Same numeric thresholds as Phase 1, applied to L0–L3 stack.

### K8 Stress / Burst

Same as Phase 1, with bundle inputs added.

### K12 Test Campaigns (now active)

| Campaign | HARD | TARGET |
|---|---|---|
| Continuous fuzz: disagreements/week classified | ≥10 | ≥30 |
| Weekly malware-replay run | green for 4 consecutive weeks | green for 8 weeks |
| Adversarial drill (Adversarial-500 + new bundle attacks) | weekly, zero new memory safety | same |

### Phase 2 Hard-Gate Summary

> Sustained ≥150 APKs/sec end-to-end L0–L3, p99 ≤800 ms, bundle correctness ≥99.9% vs AOSP, forensic FP <12% combined, K12 fuzzer green for 30 days. Pass → Phase 3.

---

<a id="phase-3"></a>
## 7. Phase 3 Gate — Symbolic & Equivalence (M18)

**Scope:** L0–L5. Adds the most performance-volatile components — SMT-backed reasoning.

### K1 Throughput

| Metric | Corpus | HARD | TARGET |
|---|---|---|---|
| L0–L5 sustained throughput, 16-core | Bench-10K | ≥20 APKs/sec | ≥40 APKs/sec |
| Symbolic resolver throughput (intent queries/sec) | Bench-1K (avg 20 queries/APK) | ≥200 q/s | ≥500 q/s |
| BSH compute throughput | Bench-10K | ≥1,000 APKs/sec | ≥3,000 APKs/sec |
| Bisim verifications/sec (1000 known pairs) | Repack-2K | ≥2 pairs/sec/core | ≥5 pairs/sec/core |
| LSH similarity search across 1M index | synthetic | ≥1,000 queries/sec | ≥5,000 queries/sec |

### K2 Latency

| Metric | HARD | TARGET |
|---|---|---|
| Full L0–L5 p99 | ≤8 s | ≤5 s |
| Symbolic intent query p99 | ≤500 ms | ≤200 ms |
| BSH compute p99 | ≤30 ms | ≤10 ms |
| Bisim per-pair p99 | ≤2 s | ≤500 ms |
| LSH lookup p99 (1M index) | ≤200 ms | ≤50 ms |

### K3 Memory

| Metric | HARD | TARGET |
|---|---|---|
| Peak RSS per worker | ≤1 GB | ≤500 MB |
| LSH index size for 1M APKs | ≤8 GB | ≤4 GB |
| Solver workspace per query | ≤200 MB | ≤80 MB |

### K4 CPU efficiency

| Metric | HARD | TARGET |
|---|---|---|
| Cycles per APK end-to-end (median) | ≤30B | ≤12B |
| SMT solver instruction efficiency | tracked, baseline established | improving QoQ |

### K5 Scalability

| Metric | HARD | TARGET |
|---|---|---|
| 1→16 core efficiency for full pipeline | ≥60% | ≥80% |
| 1→8 machine linearity for full pipeline | ≥70% | ≥85% |
| LSH index sharding: linear over 8 shards | ≥85% efficiency | ≥95% |

### K6 Real-time / Streaming

| Metric | HARD | TARGET |
|---|---|---|
| Wire-speed indication "candidate match found" | ≤200 ms after byte 0 | ≤50 ms |
| BSH-based similarity query response | ≤300 ms p99 | ≤80 ms |

### Soundness/Quality KPIs (Phase-3 specific)

| Metric | HARD | TARGET |
|---|---|---|
| L4 UNKNOWN rate on benign 5K | ≤25% | ≤10% |
| L4 UNSAT correctness on Malware-1K (vs hand-verified) | 100% | 100% |
| Solver timeout rate | <5% | <1% |
| BSH collision rate across 50K APKs | <0.1% | <0.01% |
| BSH stability across ProGuard/R8/DexGuard repackaging | ≥90% same hash | ≥98% |
| Bisim true positive rate on Repack-2K | ≥85% | ≥95% |
| Bisim false positive rate on benign pairs | <1% | <0.1% |

### K7 Stability

| Metric | HARD | TARGET |
|---|---|---|
| Solver-induced hangs | <0.5% | <0.05% |
| Per-APK 60s timeout enforced | 100% | same |
| 7-day soak with full pipeline | zero crashes | same |

### K8 Stress / Burst

| Metric | HARD | TARGET |
|---|---|---|
| 3× burst (full pipeline) | p99 ≤5× nominal | p99 ≤2× nominal |
| Solver pool exhaustion → graceful queue | yes | yes |

### K12 Test Campaigns

| Campaign | HARD | TARGET |
|---|---|---|
| Weekly SMT solver upgrade regression | green | same |
| Repack-2K nightly run | green for 4 consecutive weeks | green for 8 weeks |
| Continuous fuzz disagreements/week classified | ≥10 | ≥30 |

### Phase 3 Hard-Gate Summary

> ≥20 APKs/sec end-to-end on 16-core, p99 ≤8 s, UNKNOWN rate ≤25%, BSH collision <0.1%, bisim TP ≥85%, FP <1%, 7-day soak zero crashes. Pass → Phase 4.

---

<a id="phase-4"></a>
## 8. Phase 4 Gate — Certificates & Tooling (M24)

**Scope:** Full L0–L6. Verifier as separate measured artifact. SDKs.

### Verifier KPIs (THE HEADLINE)

The verifier is the public-facing artifact. Bug-bounty triagers run it. It must be fast, predictable, and cross-platform.

| Metric | Corpus | HARD | TARGET |
|---|---|---|---|
| `axiom-verify` p50 | 10K cert sample | ≤30 ms | ≤15 ms |
| `axiom-verify` p95 | 10K cert sample | ≤80 ms | ≤40 ms |
| `axiom-verify` p99 | 10K cert sample | ≤100 ms | ≤50 ms |
| `axiom-verify` p99.9 | 10K cert sample | ≤500 ms | ≤200 ms |
| `axiom-verify` cold start | first-cert latency | ≤500 ms | ≤150 ms |
| `axiom-verify` Wasm build p99 | 10K cert sample, in-browser | ≤300 ms | ≤120 ms |
| `axiom-verify` ARM64 mobile p99 | 10K cert sample, on Pixel-class device | ≤200 ms | ≤80 ms |

### Cert Emission

| Metric | HARD | TARGET |
|---|---|---|
| Cert emission p50 | ≤10 s | ≤3 s |
| Cert emission p99 | ≤60 s | ≤20 s |
| Halo2 prove time per circuit p99 | ≤5 s | ≤1.5 s |
| Halo2 verify time per proof p99 | ≤20 ms | ≤5 ms |
| STARK prove time (fallback) | ≤30 s | ≤10 s |

### Cert Size

| Metric | HARD | TARGET |
|---|---|---|
| Typical cert size (median) | ≤100 KB | ≤50 KB |
| Max cert size (p99) | ≤500 KB | ≤200 KB |
| Cert size for "trivial" claim (parser_consistency only) | ≤30 KB | ≤10 KB |

### K1 Throughput (full pipeline)

| Metric | HARD | TARGET |
|---|---|---|
| L0–L6 sustained, 16-core | ≥10 APKs/sec | ≥25 APKs/sec |
| Verifier service throughput, single 16-core node | ≥3,000 verifications/sec | ≥10,000/sec |
| Verifier service cluster (8 nodes) | ≥20K verifications/sec | ≥80K/sec |

### K2 Latency

| Metric | HARD | TARGET |
|---|---|---|
| End-to-end emission p99 | ≤90 s | ≤30 s |
| End-to-end verification p99 | as headline | as headline |

### K6 Real-time / Streaming

| Metric | HARD | TARGET |
|---|---|---|
| Streaming verification of cert during download | ≤50 ms after last byte | ≤10 ms |
| Service warm-up time | ≤5 s | ≤1 s |

### SDK Performance

| Metric | HARD | TARGET |
|---|---|---|
| `axiom-py` verifications/sec, single core | ≥50 | ≥150 |
| `axiom-go` verifications/sec, single core | ≥200 | ≥800 |
| `axiom-ts` (Wasm) verifications/sec, single core | ≥20 | ≥80 |
| FFI overhead vs native Rust | <30% | <10% |

### SLSA Verification (G12)

| Metric | HARD | TARGET |
|---|---|---|
| SLSA L4 verification per APK | ≤2 s | ≤500 ms |
| Reproducible-build verification per APK | ≤30 s | ≤8 s |

### Pilot Bug-Bounty Platform

| Metric | HARD | TARGET |
|---|---|---|
| `.axc` ingestion rate | ≥500/hour | ≥5,000/hour |
| Triager-facing render time | ≤2 s | ≤300 ms |
| Cert→human-readable pipeline p99 | ≤5 s | ≤1 s |

### K9 Cross-platform parity

| Metric | HARD | TARGET |
|---|---|---|
| Verifier x86_64 vs ARM64 throughput | within 30% | within 15% |
| Verifier x86_64 vs RISC-V throughput | within 50% | within 25% |
| All platforms produce byte-identical "verified" judgment | 100% | 100% |

### K10 Reproducibility

| Metric | HARD | TARGET |
|---|---|---|
| Cert SHA-256 deterministic across runs | 100% | 100% |
| Cert SHA-256 deterministic across architectures | 100% | 100% |
| Halo2 proof byte-identity (same statement, same witness) | 100% | 100% |

### Phase 4 Hard-Gate Summary

> Verifier p99 ≤100 ms over 10K certs, emission p99 ≤90 s, cert ≤500 KB max, SDKs all meet throughput floors, byte-identical certs across architectures, pilot platform ingesting ≥500/hour. Pass → Phase 5.

---

<a id="phase-5"></a>
## 9. Phase 5 Gate — Native + Dynamic + ML (M30)

**Scope:** Full pipeline + native code subsystem + dynamic confirmation + ML model security.

### Native Lifter Performance

| Metric | HARD | TARGET |
|---|---|---|
| DEX lift throughput | ≥50 MB/s | ≥150 MB/s |
| ARM64 ELF lift throughput | ≥25 MB/s | ≥80 MB/s |
| ARMv7 ELF lift throughput | ≥20 MB/s | ≥60 MB/s |
| DEX lift coverage on Bench-10K | ≥95% files | ≥99% |
| ARM64 lift coverage on NDK-100 corpus | ≥60% functions | ≥80% |
| Lift correctness vs reference (BAP/angr) | ≥95% function-level agreement | ≥99% |

### Dynamic Bridge Performance

| Metric | HARD | TARGET |
|---|---|---|
| Emulator pool cold-start | ≤120 s | ≤30 s |
| Per-finding dynamic refinement p99 | ≤300 s | ≤60 s |
| Frida script attach latency | ≤2 s | ≤500 ms |
| eBPF program load latency | ≤200 ms | ≤30 ms |
| UNKNOWN resolution rate (refines L4 UNKNOWN to ✓/✗) | ≥30% | ≥60% |
| Dynamic confirmation throughput | ≥1 APK/min/emulator | ≥5 APK/min/emulator |
| Emulator pool: parallel APKs | ≥8 simultaneous on 16-core | ≥16 simultaneous |

### TFLite / ML Layer

| Metric | HARD | TARGET |
|---|---|---|
| Model integrity (structural hash) | ≤500 ms/model | ≤100 ms |
| Neural Cleanse backdoor scan | ≤120 s/model | ≤30 s |
| STRIP scan | ≤60 s/model | ≤15 s |
| Adversarial robustness scoring | ≤300 s/model | ≤60 s |
| Backdoor detection precision (controlled experiment) | ≥90% | ≥98% |
| Backdoor detection recall | ≥80% | ≥95% |

### Joint Java + Native Analysis

| Metric | HARD | TARGET |
|---|---|---|
| Cross-language intent analysis p99 | ≤15 s | ≤5 s |
| JNI boundary modeling coverage | ≥75% common patterns | ≥95% |
| Native intent dispatch resolution rate | ≥50% | ≥80% |

### K1 Throughput (full pipeline incl. native)

| Metric | HARD | TARGET |
|---|---|---|
| L0–L6 + native, 16-core | ≥7 APKs/sec | ≥18 APKs/sec |
| Cluster (8-node × 16-core) | ≥50 APKs/sec | ≥130 APKs/sec |

### K2 Latency

| Metric | HARD | TARGET |
|---|---|---|
| Full pipeline incl. native p99 | ≤30 s | ≤10 s |
| Full pipeline incl. dynamic confirmation p99 | ≤120 s | ≤45 s |

### K3 Memory

| Metric | HARD | TARGET |
|---|---|---|
| Peak RSS per worker (incl. native) | ≤1.5 GB | ≤800 MB |
| Emulator memory budget | ≤2 GB/emulator | ≤1 GB |

### K12 Test Campaigns

| Campaign | HARD | TARGET |
|---|---|---|
| Daily native-lifter regression | green for 60 days | green for 90 |
| Weekly TFLite backdoor zoo replay | green | same |
| Continuous chaos (emulator pod kills) | recovery ≤60 s | ≤15 s |

### Phase 5 Hard-Gate Summary

> Native lift ≥50 MB/s DEX and ≥25 MB/s ELF, ≥60% NDK function coverage, dynamic bridge resolves ≥30% UNKNOWNs, ML scanner ≥90% precision, full pipeline ≥7 APKs/sec on 16-core, p99 ≤30 s static, ≤120 s dynamic. Pass → Phase 6.

---

<a id="phase-6"></a>
## 10. Phase 6 Gate — v1.0 Production Ship (M36)

**This is the ship gate. No "target" column — every line is hard.**

### Production-Grade Verifier SLAs (the headline)

| Metric | HARD |
|---|---|
| `axiom-verify` p99 over 10K cert sample | ≤100 ms |
| `axiom-verify` p99.9 | ≤300 ms |
| Verifier service availability | ≥99.99% over 90-day window |
| Verifier service throughput per cluster | ≥10,000 verifications/sec |
| Verifier cold-start | ≤500 ms |
| Verifier in-browser (Wasm) p99 | ≤300 ms |
| Verifier on mobile ARM64 p99 | ≤200 ms |

### Full Pipeline Production Throughput

| Metric | HARD |
|---|---|
| 50K APK eval completes in ≤72 hours on 100-core cluster | yes |
| Sustained throughput on 100-core cluster | ≥35 APKs/sec |
| Per-APK end-to-end p99 (full pipeline) | ≤30 s |

### Reproducibility (Final)

| Metric | HARD |
|---|---|
| 90 consecutive days byte-identical CI | 100% green |
| Three-architecture (x86_64, ARM64, RISC-V) bit-identical certificates | 100% over 10K samples |
| Re-build of every prior phase release reproduces bit-identical | ≥95% releases |
| Cross-time reproducibility: rebuild Phase-1 release on Phase-6 toolchain | bit-identical |

### Stability (Final)

| Metric | HARD |
|---|---|
| Crash rate over 50K APK eval | <1 per 10M APKs |
| Hang rate (timeout exceeded) | <0.01% |
| Soundness regression incidents over 90 days | 0 |
| MTBF in production | ≥720 hours (30 days) |

### Stress / Burst (Production)

| Metric | HARD |
|---|---|
| 5× burst tolerance, 5 min sustained | p99 ≤2× nominal |
| 10× burst tolerance, 60 s | recovery in ≤30 s |
| Sustained 90% utilization, 7 days | no degradation |
| Chaos drills (pod kill, network partition, disk full) | graceful degradation, recovery ≤60 s |

### Real-time Production

| Metric | HARD |
|---|---|
| Streaming verification latency after last byte | ≤50 ms p99 |
| Wire-speed inspection bandwidth on 16-core | ≥1 Gbps |
| Real-time bug-bounty pilot: median time triager-to-verdict | ≤2 s |

### Cross-platform Parity (Final)

| Metric | HARD |
|---|---|
| Verifier x86_64 vs ARM64 throughput | within 20% |
| Verifier all-arch byte-identical outputs | 100% |
| SDKs all archs all OSes pass integration suite | 100% |

### External Validation (Final)

| Metric | HARD |
|---|---|
| External audit (Trail of Bits / NCC / equivalent): no critical open | yes |
| 50K APK eval results published as paper + dataset | yes |
| ≥3 papers accepted at top venues | yes |
| ≥10 CVEs filed from G8 fuzzing | yes |
| Pilot bug-bounty platform live, ingesting `.axc` in production | yes |

### Documentation (Final)

| Item | HARD |
|---|---|
| `.axc` format specification (RFC-style) | published |
| AXIOM-IR specification (all dialects) | published |
| Every L0–L6 layer's correctness theorem documented | published |
| Per-group design rationale | published |
| Migration guide for downstream consumers | published |

### Phase 6 Hard-Gate Summary (the ship checklist)

```
v1.0 ships only when ALL of the following are simultaneously true:

[ ] axiom-verify p99 ≤100 ms over 10K certs (90 consecutive days green)
[ ] Service availability ≥99.99% over 90 days
[ ] 50K APK eval completes ≤72h on 100-core cluster
[ ] 90 consecutive days byte-identical CI
[ ] Three-arch bit-identical certificates
[ ] Crash rate <1 per 10M APKs
[ ] Soundness regression incidents = 0 over 90 days
[ ] MTBF ≥720h in production
[ ] 5× burst tolerance verified
[ ] Streaming verification ≤50 ms after last byte
[ ] External audit closed, no critical findings
[ ] 50K APK eval published
[ ] ≥3 papers accepted
[ ] ≥10 CVEs filed
[ ] Pilot platform live in production
[ ] All documentation published
[ ] All SDKs all archs pass integration
[ ] Wire-speed ≥1 Gbps verified
[ ] Cross-time reproducibility verified
```

19 items. All hard. Any single ❌ → slip the release.

---

<a id="continuous"></a>
## 11. Continuous KPIs (Always-On)

These run 24/7 from M0 onward. They are not "phase gates" — they are conditions that must remain green continuously. **Going red on a continuous KPI is a P0 incident.**

| KPI | Threshold | Response if red |
|---|---|---|
| CI byte-identical build | 100% on every PR | block all merges; G13 incident |
| Soundness regression | 0 incidents | block all L1 PRs; G1 incident |
| Differential fuzzer disagreements unresolved | ≤10 in queue | G8 reinforced; classification SLA |
| Per-PR perf gate (Bench-1K) | within 5% of baseline | block PR until investigated |
| Per-PR memory gate (Bench-1K) | within 10% of baseline | block PR |
| Daily smoke run | green for 30 consecutive days | block release if broken |
| Weekly nightly run on Bench-10K | green for 4 consecutive weeks | block phase-end review |
| Reproducibility audit (cross-machine) | quarterly, 100% | release block |

---

<a id="methodology"></a>
## 12. Test Methodology Per Category

### K1 Throughput
- Replay specified corpus through the measured stack.
- Wall-clock time on dedicated benchmark hardware (no shared workloads).
- Repeat 5×, report median, surface outliers.
- Exclude warm-up period (first 60 seconds discarded).

### K2 Latency
- Per-APK timestamps captured at layer boundaries.
- HDR Histogram for sub-percentile resolution.
- Reported as full distribution; gates check specific percentiles.

### K3 Memory
- Peak RSS via `getrusage` per worker.
- Long-soak via `valgrind --tool=massif` (offline) and `jemalloc`/`mimalloc` runtime stats.
- 24h soak runs use the production allocator.

### K4 CPU efficiency
- `perf stat` over a 10-minute steady-state window.
- Reported per-APK by dividing by APK count.
- Hardware: dedicated benchmark machine (specified per phase).

### K5 Scalability
- Throughput at 1, 2, 4, 8, 16 cores on a single machine.
- Throughput at 1, 2, 4, 8 machines (each 16 cores).
- Plot efficiency curves; gates check ratios.

### K6 Real-time / Streaming
- Synthetic byte-stream feeder configurable to N Mbps.
- Time-to-first-event measured with monotonic clock.
- Wire-speed test runs against a constant 1 Gbps stream for ≥60 minutes.

### K7 Stability
- Soak tests: 24h, 72h, 7-day, 30-day variants.
- Crash collection via `coredumpctl` or equivalent; classification automated.
- MTBF computed over rolling 30-day window.

### K8 Stress / Burst
- Load generator capable of 10× nominal.
- Burst tests run from steady state, 60s burst, 5-minute recovery observation.
- Chaos drills: explicit failure injection (pod kill, partition, disk full, OOM).

### K9 Cross-platform parity
- Same Bazel-hermetic build artifacts on x86_64, ARM64, RISC-V.
- Same Bench-10K input.
- Reported in normalized cycles per APK + raw throughput.

### K10 Reproducibility
- `bazel build //... && diff` between two clean machines.
- Output bytes hashed (SHA-256) and compared.
- Cross-time: rebuild a 6-month-old release on current toolchain.

### K11 Soundness
- Lean theorems re-verified on every L1 PR via CI gate.
- Audit log of every theorem-statement change.
- Quarterly mathlib upgrade soundness check.

### K12 Test campaigns
- All campaigns scheduled via Bazel + airflow-equivalent orchestration.
- Results dashboarded; failures page on-call.

---

<a id="instrumentation"></a>
## 13. Instrumentation Requirements

Every group ships their layer with instrumentation that supports the KPIs above. This is **mandatory**, not optional. A layer without instrumentation cannot enter a phase gate.

| Required signal | Format | Owner |
|---|---|---|
| Per-APK timing (per layer) | OpenTelemetry spans | every group |
| Per-APK memory delta | counter | G13 lib |
| Allocation rate | jemalloc/mimalloc stats | G13 lib |
| CPU counters | perf or eBPF probes | G13 lib |
| Solver query timing (G5) | OTel + custom solver stats | G5 |
| Lean proof object size (G1, G7) | counter | G1 |
| Halo2 prove/verify timing (G7) | OTel | G7 |
| Emulator session timing (G10) | OTel | G10 |
| End-to-end cert emission timing | OTel root span | G7 |
| Reproducibility audit log | structured JSONL | G13 |

All metrics flow into a single observability stack (recommendation: Prometheus + OpenTelemetry collector + Grafana). Every phase gate review opens against the dashboard, not a slide.

---

<a id="failures"></a>
## 14. Failure Modes & Re-test Protocol

When a phase gate fails:

1. **Identify the failing KPI(s).** A single failed hard gate blocks the phase. Multiple failed targets are acceptable but tracked.
2. **Root-cause within 7 days.** Group lead writes an RCA document.
3. **Plan remediation.** Either fix-in-place (within 4 weeks) or scope-down (defer the failing capability to next phase, requires ADR).
4. **Re-run the full phase gate.** Not just the failing KPI — the whole gate, on the same corpus. KPIs interact; fixing one can break another.
5. **7-day green requirement.** After re-test, KPI must remain green for 7 consecutive days before phase advances.

**A failed hard gate is not a failure of the project.** It is information. The plan is robust if the re-test protocol is followed; the plan fails only if the response is to ship anyway.

---

## Appendix A — KPI Ratchet Table

How key KPIs tighten across phases. Each cell is the hard gate for that phase.

| KPI | P1 | P2 | P3 | P4 | P5 | P6 |
|---|---|---|---|---|---|---|
| Sustained APKs/sec/16-core | ≥300 | ≥150 | ≥20 | ≥10 | ≥7 | ≥7 |
| End-to-end p99 latency | ≤300 ms | ≤800 ms | ≤8 s | ≤90 s | ≤30 s | ≤30 s |
| Verifier p99 | n/a | n/a | n/a | ≤100 ms | ≤100 ms | ≤100 ms |
| Peak RSS per worker | ≤150 MB | ≤300 MB | ≤1 GB | ≤1 GB | ≤1.5 GB | ≤1.5 GB |
| Crash rate per 1M APKs | <10 | <10 | <5 | <2 | <1 | <0.1 |
| Reproducibility (CI byte-identical) | 100% | 100% | 100% | 100% | 100% | 100% |
| Soundness regressions | 0 | 0 | 0 | 0 | 0 | 0 |
| Cross-arch parity | within 25% | within 20% | within 20% | within 20% | within 20% | within 20% |
| Wire-speed inspection | ≥500 Mbps | ≥500 Mbps | ≥500 Mbps | ≥500 Mbps | ≥1 Gbps | ≥1 Gbps |

The throughput drop from Phase 1 to later phases is **expected** — more layers run per APK. The hard gate is "this layer doesn't make us slower than the budget allowed for it." Phase 6 holds Phase 5 throughput because Phase 6 is stabilization, not new features.

---

## Appendix B — Hardware Reference Profile

All KPIs above assume the following reference hardware unless otherwise stated. CI runs on this profile; published numbers cite this profile.

```
Single-machine benchmark host:
  CPU:    AMD EPYC 9354 (16 cores, 32 threads)  or  Intel Xeon Gold 6438M (32C)
  RAM:    256 GB DDR5
  Disk:   2× 4 TB NVMe (RAID-0 for benchmarks)
  Net:    25 Gbps NIC (used for streaming tests)
  OS:     Linux 6.x, kernel-tuned, no swap during runs

Cluster benchmark:
  8 nodes of the above
  Internal: 25 Gbps fabric
  External: 100 Gbps to corpus storage

Mobile reference (Phase 4+):
  Pixel 8 (or equivalent ARM64 mid-range)
  Wasm reference: Chromium 122+ on the above EPYC host

ARM64 server reference:
  AWS Graviton3 c7g.4xlarge or Ampere Altra Q80-30 16C partition

RISC-V reference (Phase 6):
  SiFive HiFive Pro P550 or equivalent RVA22 implementation
```

If reference hardware is unavailable for any phase gate review, the phase **does not advance** until either the hardware is procured or an ADR documents the substitution and its scaling-factor analysis.

---

*"A layer is ready when its slowest, hottest, dirtiest case still meets its KPI. Not when the happy path is green."*
