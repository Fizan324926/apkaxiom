# APKAXIOM — Nation-Grade Tech Stack

> Picked for *what is the most advanced thing that can do this job*, not *what is most popular*.
> Where the cutting edge has a smaller community, harder learning curve, or thinner documentation, we still take it — community is a 2-year problem; soundness and speed are 10-year problems.

This document is the technology companion to [README.md](../README.md) (architecture), [ROADMAP.md](./ROADMAP.md) (timeline), and [PHASE_GATES.md](./PHASE_GATES.md) (KPIs).

---

## Table of Contents

1. [Stack Philosophy](#philosophy)
2. [Languages — The Vertical Stack](#languages)
3. [Verified-Software Supply Chain](#verified-supply-chain)
4. [Performance-Critical Primitives](#perf-primitives)
5. [Zero-Knowledge Proof Systems](#zk-systems)
6. [SMT, CHC & Decision Procedures](#smt)
7. [Native Code & Binary Lifters](#native)
8. [Dynamic Analysis & Concolic Execution](#dynamic)
9. [Fuzzing Infrastructure](#fuzzing)
10. [Build, CI & Reproducibility](#build)
11. [Storage, Indexing & Similarity Search](#storage)
12. [Hashing — Crypto and ZK-Friendly](#hashing)
13. [Observability & Profiling](#obs)
14. [ML Security Stack](#ml)
15. [SDK & Cross-Language Bindings](#sdk)
16. [GPU Acceleration](#gpu)
17. [Concurrency, Async & I/O](#concurrency)
18. [The Refusal List — What We Don't Use](#refusals)
19. [Decision Matrix — Alternatives Considered](#decisions)

---

<a id="philosophy"></a>
## 1. Stack Philosophy

Five operating rules for every technology selection:

1. **Cutting edge over comfortable.** If a 2023–2025 research artifact does the job better, we use it even if it has 200 GitHub stars instead of 20K. We are not optimizing for hireability; we are building a moat.
2. **Verified > tested > heuristic.** Where a verified-software-supply-chain option exists (HACL\*, EverParse, fiat-crypto, seL4-style proofs), it wins.
3. **Low level by default; managed only at the edges.** Anything on the proof path is Rust, Zig, or C++23/26. Python, TypeScript, and Go appear only at FFI surfaces.
4. **Hardware-aware code is the default, not a perf afterthought.** AVX-512 / SVE2 / Neoverse intrinsics, GPU acceleration, io_uring, thread-per-core architecture — these are baseline expectations, not optimizations to get to "later."
5. **No technology stays in the stack on inertia.** Every choice gets re-justified at each phase boundary. If something better lands mid-project, we migrate. ADR review every 6 months.

---

<a id="languages"></a>
## 2. Languages — The Vertical Stack

Listed top-down by abstraction tier. Lower tiers are mandatory; higher tiers exist only at FFI/UI surfaces.

### Tier 0 — Theorem Provers (proofs are first-class artifacts)

| Language | Use | Why this and not alternatives |
|---|---|---|
| **Lean 4** + mathlib4 | Primary mechanization (G1) | Best-in-class extraction tooling; mathlib is the largest formalization corpus; Lean 4's elaborator + tactics are unmatched in 2026 |
| **F\*** | Verified parser combinators (G2 via EverParse), verified crypto (HACL\*) | Microsoft Research's industrial-grade verified-systems language; produces verified C/Rust/OCaml |
| **Rocq** (formerly Coq) | Escape hatch only | Used only when a Lean port doesn't exist for a needed library; must be ADR-justified |
| **Isabelle/HOL** | Reference for some Cogent-style proofs | Read-only — we cite, we don't develop in it |

### Tier 1 — Systems Languages (the proof path)

| Language | Use | Why this and not alternatives |
|---|---|---|
| **Rust 2024 edition** | Default for everything in G2/G3/G4/G5/G6/G7/G9/G12 | Memory safety + zero-cost abstraction + viable Lean extraction target. Non-negotiable for the proof stack |
| **Zig 0.13+** | Hot-path subsystems where Rust's ownership cost is real (allocator-heavy parsers, comptime layout calculations, custom string-pool decoder) | Faster compile, comptime metaprogramming, better C interop than Rust. Used surgically — *not* a Rust replacement |
| **C++26** with concepts, modules, `std::execution`, reflection (P2996) | LLVM/MLIR integration only (G3, G9) | Unavoidable because LLVM/MLIR are C++. We use the *latest* standard, not C++17 |
| **C (restricted, eBPF subset)** | eBPF programs that don't fit Aya yet | Restricted dialect, not freeform C |

### Tier 2 — Domain & Glue (constrained surface)

| Language | Use | Notes |
|---|---|---|
| **TypeScript** (Frida scripts, axiom-ts SDK Wasm wrapper) | Frida hooks; browser-side verifier | Type-safety floor; never on a proof path |
| **Python 3.13+** | G11 (ML security), orchestration only | Used because TFLite/ONNX/PyTorch ecosystems are Python. Hot paths drop to Mojo or Rust |
| **Mojo** | ML hot paths in G11 | MLIR-native, compiles to fast machine code; bridges Python ergonomics to systems performance |
| **Go** | G12 SLSA/in-toto/Sigstore reuse only | Not used elsewhere; Go appears because the supply-chain ecosystem is Go-native |
| **Starlark** (Buck2 BUILD files) | Build configuration | Hermeticity language |

### Tier 3 — Specifications (machine-readable, not executed)

- **EverParse 3D** — declarative binary-format spec → verified parser
- **MLIR TableGen** — dialect declarations
- **SMT-LIB 2** — solver inputs
- **CHC / Datalog** (Soufflé) — symbolic-resolver encodings
- **Halo2 PLONKish** — circuit DSL
- **CDDL / ASN.1** — wire-format specs (e.g., `.axc` certificate)
- **Cap'n Proto schemas** — internal IPC (zero-copy)

### Languages explicitly NOT used (with reasons)

| Language | Why not |
|---|---|
| **Java / Kotlin (for our code)** | Used only as AOSP fuzzing harnesses (G8). Not for our own components |
| **JavaScript** | Frida scripts and TS-Wasm SDK only — never on a proof path |
| **Carbon** | Pre-1.0 in 2026; not stable enough for nation-grade |
| **Nim, D, Crystal** | Too small ecosystems; not a community problem — a tooling problem (no proof-extraction targets, no formal-methods integration) |
| **Haskell** | Lean 4 covers FM needs more directly. Haskell would be a parallel ecosystem with no payoff |
| **Swift** | Apple-centric tooling; cross-platform story too thin |

---

<a id="verified-supply-chain"></a>
## 3. Verified-Software Supply Chain

This is the part most "nation-grade" claims skip. We don't.

| Component | What it gives us | Used for |
|---|---|---|
| **EverParse / 3D** (Microsoft Research) | F\*-verified parser/serializer combinators that compile to verified C or Rust | Alternative compilation target for L1 parsers (alongside Lean→Rust); decision in Phase 0 ADR. Used for verified ZIP and APK Signing Block parsing |
| **HACL\*** (INRIA + Microsoft) | F\*-verified cryptography (Ed25519, RSA, ECDSA, BLAKE3, AES-GCM) | All cryptographic operations on the verified path — APK signature verification, certificate signing, BLAKE3 commits |
| **fiat-crypto** (MIT) | Coq-verified field arithmetic; used in BoringSSL, Firefox NSS | Alternative source for elliptic-curve arithmetic where HACL\* coverage is thinner |
| **EverCrypt** | HACL\*'s agile cryptographic provider with multiplexing | Crypto front-end |
| **CompCert** (INRIA) | Verified C compiler | When extraction goes through C (rare path); fallback for safety-critical kernels |
| **CakeML** | Verified ML compiler (HOL4) | Reference implementation for verified compiler work |
| **seL4-style approach** | Reference for our own verification methodology | We don't use seL4 directly, but we follow its proof-engineering practices |

**The verified path:** APK bytes → EverParse-generated verified Rust parser → AXIOM-IR → SMT/zk-SNARK certificate. Every step on this path is *either* mechanically verified (Lean 4 / F\*) *or* discharges to a checked external proof (cvc5 UNSAT cert, Halo2 verifier).

---

<a id="perf-primitives"></a>
## 4. Performance-Critical Primitives

We pick the most modern primitives, even when they're harder to use.

### Memory allocators

- **snmalloc** (Microsoft Research) — default for shared-memory workloads. Faster than jemalloc on multi-thread workloads.
- **mimalloc** (Microsoft) — alternative for single-thread / low-contention.
- **jemalloc** — only as comparison baseline; not default.

### Async runtimes

- **Glommio** (Datadog, thread-per-core io_uring) — default for Layer 0 streaming and the verifier service.
- **Monoio** (ByteDance, pure thread-per-core io_uring) — alternative; head-to-head bench at Phase 4.
- **Tokio** — only used at FFI edges where work-stealing is required (compatibility with crates that assume Tokio).

### CPU SIMD & specialized kernels

- **`std::simd`** (Rust portable SIMD) — first choice for portable hot paths.
- **AVX-512** intrinsics via `std::arch::x86_64` — required for x86 hot paths (string-pool decode, byte-scan, ZIP central-directory parse).
- **SVE2 / Neoverse-V2** intrinsics — required for ARM64 hot paths.
- **ISPC** (Intel SPMD Program Compiler) — for kernels where SPMD parallelism beats hand-tuned SIMD (parser inner loops over uniform input batches).
- **Highway** (Google, header-only C++ portable SIMD) — alternative considered; rejected in favor of `std::simd` + Rust.

### Lock-free & concurrency primitives

- **crossbeam** — Rust standard for lock-free.
- **kanal** — fastest known Rust channels in 2026.
- **flume** — fallback channels.
- **TigerBeetle's queue patterns** (LMAX-style ring buffers) — for the symbolic-execution work queue (G5).
- **`io_uring` directly** via raw syscalls in places Glommio's abstraction is too thick (rare).

### Layout & data structures

- **archery** for shared persistent data structures.
- **rkyv** for zero-copy archived data (verifier state).
- **Cap'n Proto** for IPC (zero-copy).
- **Apache Arrow** for analytical columnar data (Phase 6 corpus eval).

---

<a id="zk-systems"></a>
## 5. Zero-Knowledge Proof Systems

The ZK landscape changes every six months. We don't bet on one. We use **the most advanced family per workload**.

| System | Use | Why this one |
|---|---|---|
| **Halo2** (Zcash) | Circuit-specific privacy invariants (the 5 Phase-4 circuits) | Mature, Rust-native, well-audited; PLONKish arithmetization is flexible |
| **Plonky3** (Polygon) | Alternative backend for the same circuits | FRI-based, faster proving than Halo2 on many workloads; benchmarked head-to-head at Phase 4 |
| **Jolt** (a16z crypto) | General-purpose zkVM proving — proves "this RISC-V program executed correctly" | Lookup-singularity zkVM; the fastest known general-purpose zk system as of 2025–2026 |
| **SP1** (Succinct Labs) | RISC-V zkVM alternative to Jolt | Production-grade; we A/B test against Jolt at Phase 4 |
| **Binius** (Irreducible) | Binary-field SNARKs — efficient for hash and bit-level workloads | Released 2024; binary fields make BLAKE3 / SHA proofs orders of magnitude cheaper |
| **Nova / HyperNova / SuperNova** (Microsoft Research) | Incrementally verifiable computation (IVC) for streaming claims | Folding schemes that let us prove unbounded computations |
| **STARKs (Winterfell, Stwo)** | Post-quantum fallback | Used when zk-SNARK assumptions are insufficient (regulated industries, long-lived certs) |
| **Lasso** | Lookup arguments at scale | Used inside Jolt; sometimes standalone for our own circuits |
| **Risc Zero** | Comparison only; not in production | We benchmark against; not a dependency |

### Concrete plan

- **Phase 4**: Halo2 for the 5 priority privacy invariants (defaults).
- **Phase 5**: introduce Jolt or SP1 as the general zkVM — proves arbitrary Rust programs, not just hand-written circuits. This is the unlock that lets G7 prove *any* Rust property without bespoke circuit work.
- **Phase 6**: post-quantum STARK alternative (Stwo) verified end-to-end as the fallback profile in `.axc`.

The point: in 2026, **picking one zk system is a category error.** Different proofs need different systems. The `.axc` format carries a `proof_system` field per claim and we pick the best per claim.

---

<a id="smt"></a>
## 6. SMT, CHC & Decision Procedures

### Primary

- **cvc5** — primary SMT engine. Best across QF_BV, QF_LIA, theory combinations. Active research output.
- **Z3** — secondary; better in some QF_NIA cases. Includes Spacer for CHC.
- **Bitwuzla** — fastest QF_BV solver in 2024–2026. Used as the default for bitvector-heavy queries.
- **Yices2** — fastest linear arithmetic; used for queries where domain narrows.

### CHC (Constrained Horn Clauses)

- **Spacer** (in Z3) — primary for the symbolic intent resolver (G5).
- **Eldarica** — alternative; better termination on some instances.
- **GoldenAge / GPDR** variants — research; tracked.

### Model checking (research-frontier, used in G5 sub-problems)

- **Pono** (Stanford) — IC3/PDR for word-level model checking.
- **AVR** — abstraction-refinement model checking.
- **nuXmv** — symbolic model checker.

### Theorem-proving glue

- **PySMT / `smt-rs`** — solver-agnostic API bindings.
- **DRAT / LRAT** — UNSAT certificate format we emit and consume in `.axc`.

---

<a id="native"></a>
## 7. Native Code & Binary Lifters

This is one of the hardest layers. We pick research-frontier tools.

| Tool | Use | Why |
|---|---|---|
| **Ghidra P-Code → MLIR custom dialect** | Primary lifting path for ARM64/ARMv7 ELF | Ghidra's P-Code is the most complete IR available open-source; MLIR is the lowering target |
| **BAP (Binary Analysis Platform)** | Reference + alt path | Rich semantics for x86; OCaml core but battle-tested |
| **angr** | Dynamic-symbolic execution oracle for our static lift | Used as ground truth, not in production |
| **Capstone-rs** | Disassembly only | Wrapped behind P-Code lifting |
| **iced-x86** | x86 disassembly | Used for reverse-engineering checks of native libraries |
| **Reopt (Galois)** | Reference for x86→LLVM lifting research | We don't ship Reopt but we read their papers |
| **Souper (LLVM superoptimization)** | Verified peephole optimizer for the lifted IR | Used to canonicalize lifted code |
| **Triton** (Quarkslab) | Concolic execution layer integrated with the dynamic bridge | Cutting-edge open-source concolic engine |

### Why this matters: native code is the dark matter of APK analysis

Every Java-only analyzer (Androguard, JADX, MobSF, every academic FlowDroid descendant) is blind to .so libraries. APKAXIOM lifts native code to AXIOM-IR alongside DEX, then runs the same Layer 4 symbolic resolver across both. **There is no other tool doing joint Java+native intent analysis at proof grade in 2026.**

---

<a id="dynamic"></a>
## 8. Dynamic Analysis & Concolic Execution

| Tool | Use | Why |
|---|---|---|
| **Cuttlefish** (Google AOSP) | Primary Android emulator | Faster + more accurate than AVD; supports headless cluster deployment |
| **SymQEMU** | Symbolic execution as QEMU TCG instrumentation | Combines QEMU's coverage with symbolic execution; nation-grade choice |
| **Triton** | Concolic execution engine | Refines static UNKNOWNs from G5 |
| **Frida** | High-level Java/Kotlin hooks only | Not for native; native goes through SymQEMU |
| **Aya (Rust eBPF)** | All eBPF programs written in Rust | We do not write eBPF in C |
| **bpftrace** | Ad-hoc tracing in development | Not in production |
| **Pixie** (CNCF) | eBPF-based observability | Reference; we use Aya-based custom probes |
| **DynamoRIO** | Reference for instrumentation research | Not in production |

### Why SymQEMU over plain QEMU + Frida

Frida tells you what *happened*. SymQEMU tells you what *could happen* across symbolic inputs. For UNKNOWN refinement, "could happen" is the actual question.

---

<a id="fuzzing"></a>
## 9. Fuzzing Infrastructure

The differential fuzzing plant (G8) is the continuous oracle. It must be the best-in-class.

| Tool | Use | Why |
|---|---|---|
| **Nyx** (Bochum/TUDA) | Primary: snapshot-based hypervisor fuzzing | Fastest fuzzing technique known for full-system targets. Google Project Zero uses it |
| **Nautilus** | Grammar-aware mutation | We define an APK grammar; Nautilus exercises it semantically |
| **Centipede** (Google) | Distributed coverage-guided fuzzing | Cluster-scale fuzzing |
| **AFL++** | Baseline + standard ecosystem integration | Where Nyx is overkill |
| **libFuzzer** | Per-PR PR-gate fuzzing only | In-process, fast for unit-level harnesses |
| **Honggfuzz** | Alternative comparison | Not in primary path |
| **Fuzzilli** | Reference for grammar-aware mutation techniques | Read-only; we adapt patterns |
| **Gramatron / Token-Level AFL** | Research references for grammar-based fuzzing | Adapted into our APK grammar |

### Why Nyx specifically

Snapshot-based hypervisor fuzzing means we fuzz **the actual AOSP binaries** (libziparchive, PackageParser compiled into Cuttlefish images), not stubs. That's the nation-grade differentiator — every disagreement is a real-world AOSP CVE candidate, not a fixture-fabricated bug.

---

<a id="build"></a>
## 10. Build, CI & Reproducibility

| Tool | Use | Why we picked this and not the popular alternative |
|---|---|---|
| **Buck2** (Meta, Rust-rewritten) | Primary build system | Faster than Bazel, better remote execution, Rust-native (Reindeer integration). State of the art in 2026 |
| **Reindeer** | Cargo → Buck2 conversion | Required for Rust-heavy workspace |
| **Bazel** | Comparison baseline; used for AOSP harness builds | We *had* picked Bazel; we upgraded to Buck2. Bazel only stays where Buck2's AOSP support is thinner |
| **Nix flakes** | Toolchain pinning + escape hatch | For when Buck2's hermeticity isn't enough |
| **Stagex** | Deterministic source-based bootstrap | For the verifier's release artifact (Phase 6) |
| **Trustix** (Tweag) | Build verification network | Cross-check build outputs across independent rebuilders |
| **rules_oci** | OCI container builds | Reproducible images |
| **in-toto** | Build provenance attestations | SLSA L4 |
| **Sigstore (cosign)** | Artifact signing | Standard |

### CI orchestration

- **Buildkite** primary (best agent model for hardware diversity).
- **GitHub Actions** for OSS-facing workflows.
- **DagsterLabs** for the corpus-eval orchestration (DAGs over 50K APKs).

---

<a id="storage"></a>
## 11. Storage, Indexing & Similarity Search

The corpus pipeline at v1.0 handles 50K+ APKs and the BSH index potentially scales to millions. SOTA matters.

| Component | Choice | Why |
|---|---|---|
| **Columnar analytics** | **DuckDB** + **Apache Arrow** | DuckDB is the fastest in-process OLAP engine; Arrow is the universal columnar format |
| **Embedding storage** | **Lance** (LanceDB) | Designed for ML/embedding workloads; columnar + versioned |
| **Archival** | **Parquet** + Zstd | Standard archival columnar |
| **Sparse multi-dim** | **TileDB** | If/when we need sparse high-dim |
| **KV / metadata** | **FoundationDB** | Strict serializability + transactions; SOTA distributed KV |
| **Object store** | **MinIO** (self-host) or S3 | For the 50K APK corpus blobs |
| **Approximate nearest neighbor (ANN)** | **DiskANN** (Microsoft) | Billion-scale ANN; outperforms HNSW at our target scale |
| **HNSW** | **hnswlib** | Comparison baseline; smaller scale |
| **Bloom-filter replacement** | **Ribbon filters** + **Xor filters** | Both faster and smaller than Bloom; used for "have I seen this APK?" indices |
| **Persistent log** | **fjall** (Rust LSM) | LSM tree for the differential-fuzzer disagreement log |

### Why DiskANN over HNSW

At million-APK scale with BSH-256 vectors, HNSW's RAM cost dominates. DiskANN's disk-resident graph index is the production-scale answer.

---

<a id="hashing"></a>
## 12. Hashing — Crypto and ZK-Friendly

| Hash | Use | Why |
|---|---|---|
| **BLAKE3** | Default for content addressing, Merkle trees, integrity | Faster than SHA-2/SHA-3; tree-mode parallel; HACL\* has verified implementation |
| **Poseidon2** | Inside ZK circuits | ZK-friendly arithmetic-friendly hash; faster than original Poseidon |
| **Reinforced Concrete** | Alternative ZK-friendly hash | Even faster for some circuit shapes; benchmarked at Phase 4 |
| **HighwayHash** | Non-cryptographic hash for hot paths | Fast keyed hashing for LSH bucket assignment |
| **SHA-256 / SHA-512** | Only where external compatibility demands it (APK signature schemes) | We verify; we don't add |
| **FNV / xxHash** | Forbidden | Predictable for adversaries; we use HighwayHash |
| **MD5** | Forbidden everywhere | — |

The rule: **outside ZK circuits, BLAKE3. Inside ZK circuits, Poseidon2 or Reinforced Concrete.** The BLAKE3 → Poseidon2 transition is handled by an explicit canonicalization layer at the circuit boundary.

---

<a id="obs"></a>
## 13. Observability & Profiling

| Tool | Use |
|---|---|
| **OpenTelemetry** | Tracing + metrics; the universal substrate |
| **Pyroscope** (Grafana Labs) | Continuous profiling — always-on flamegraphs |
| **Prometheus** | Metrics backend |
| **Grafana** | Dashboards |
| **Aya-based custom eBPF probes** | Per-APK kernel-level instrumentation |
| **Cilium Tetragon** | Runtime security observability |
| **HDR Histogram** | Latency distribution capture (for KPI gates) |
| **flamegraph.pl + cargo-flamegraph** | Manual profiling |
| **`perf` + `perf-tools`** | Standard Linux profiler |
| **Tracy** | Microsecond-resolution profiler for hot loops |

### Continuous profiling (Pyroscope) — non-negotiable

We deploy Pyroscope from Phase 1 onward. Every benchmark run captures a profile. Every regression has a flamegraph diff. This is how Big Tech's perf teams operate; we operate the same way.

---

<a id="ml"></a>
## 14. ML Security Stack

For G11 (model integrity, backdoor detection, adversarial robustness).

| Component | Choice | Why |
|---|---|---|
| **TFLite + ONNX Runtime** | Model parsing | What APKs ship with |
| **PyTorch 2.x with `torch.compile`** | Backdoor detection (Neural Cleanse, STRIP reimplementations) | Dominant ML framework in 2026 |
| **JAX** | Adversarial robustness via differentiable analysis | Function-transform model fits adversarial work |
| **Mojo** | Hot-path inference for embedding extraction | MLIR-native; bridges Python ergonomics to Rust-grade performance |
| **GGML / llama.cpp lineage** | If we ship LLM-assisted analysis | Likely deferred past v1.0 |
| **TensorFlow** | Forbidden for new code | Maintenance-mode; PyTorch dominates |

We do not depend on TensorFlow for new code in 2026. Anything that came in via TFLite parsing is fine; we don't write TF.

---

<a id="sdk"></a>
## 15. SDK & Cross-Language Bindings

We do not hand-write per-language SDKs. We generate.

| Tool | Use |
|---|---|
| **uniffi** (Mozilla) | Single Rust source → Python, Kotlin, Swift, Ruby bindings |
| **Diplomat** (Unicode/Mozilla) | Single Rust source → C, C++, JS/TS bindings |
| **wasm-bindgen** + **wit-bindgen** | Wasm + Component Model — for `axiom-verify` in browsers |
| **PyO3** | When uniffi isn't enough (rare) |
| **napi-rs** | If/when Node.js ABI-native is needed |
| **cgo** | Go-side reuse for SLSA tooling (G12) |

The headline: every SDK comes from a single Rust source of truth. There is no parallel Python or Go re-implementation of the verifier — Wasm or FFI, never reimplementation.

---

<a id="gpu"></a>
## 16. GPU Acceleration

This was missing from the earlier docs. Adding it explicitly.

| Workload | Acceleration |
|---|---|
| **ZK proving (Halo2, Jolt, Plonky3)** | CUDA + HIP via `sppark`, `icicle` (Ingonyama). 10–100× speedup on proof generation. |
| **Bisimulation graph problems** | **cuGraph** (NVIDIA RAPIDS) for large state-space comparisons |
| **LSH / DiskANN search at scale** | GPU-accelerated FAISS as comparison; DiskANN GPU variants |
| **Native ML model analysis (G11)** | Standard PyTorch/JAX GPU |
| **Symbolic-execution-style workloads** | Research only — not production yet |

Compute platforms supported:

- **CUDA** (nVidia) — primary.
- **HIP / ROCm** (AMD) — alternate.
- **Metal** (Apple) — Apple silicon dev workstations.
- **WebGPU / WGSL** — browser-side `axiom-verify` path.
- **OneAPI / SYCL** — Intel hardware future-proofing.

---

<a id="concurrency"></a>
## 17. Concurrency, Async & I/O

| Layer | Choice | Why |
|---|---|---|
| **Async runtime** | **Glommio** (thread-per-core io_uring) | The dominant high-perf model in 2026 |
| **Alt runtime** | **Monoio** | A/B benched at Phase 4 |
| **Tokio** | Edge / FFI compatibility only | Not on the hot path |
| **Lock-free queues** | **kanal** + custom ring buffers | LMAX-style fanout |
| **Thread-per-core** | Architectural default for the verifier service | NUMA-aware, no work-stealing overhead |
| **NUMA-aware allocation** | **snmalloc** + explicit pinning | For 64+ core deployments |
| **Disk I/O** | **io_uring** directly via `liburing-sys` | For Layer 0's streaming spine |
| **Network I/O** | Glommio + io_uring | 25 Gbps NIC saturation is achievable |

**Thread-per-core architecture is the default.** Work-stealing (Tokio's default) introduces unpredictability incompatible with the latency budgets in PHASE_GATES.md.

---

<a id="refusals"></a>
## 18. The Refusal List — What We Don't Use

We are explicit about technologies we deliberately reject, even when popular.

| Rejected | Reason |
|---|---|
| **Java / JVM for our code** | GC pauses incompatible with latency budgets; AOSP harnesses only |
| **Kotlin (for our code)** | Same as above |
| **Bazel as primary** | Buck2 is faster + better remote exec; Bazel stays for AOSP only |
| **HNSW at scale** | DiskANN wins above 10M vectors |
| **Bloom filters** | Xor / Ribbon filters are strictly better |
| **MD5, SHA-1, FNV, xxHash** | Cryptographically inadequate or adversarially weak |
| **TensorFlow for new code** | PyTorch + JAX dominate; TF is maintenance-mode |
| **Tokio on hot paths** | Work-stealing overhead unacceptable; thread-per-core wins |
| **Standard malloc** | snmalloc / mimalloc are Pareto-better |
| **C (for our code, except eBPF)** | Rust is strictly better |
| **Python on the proof path** | Operations and ML only; never on a verified path |
| **Tree-sitter for binary formats** | Designed for source code; we use deku/scroll/EverParse for binary |
| **Closed-source tools** | Quark Pro, IDA Pro, etc. — we cannot ship dependencies on them |
| **Cloud-locked services (Lambda, BigQuery)** | Hermetic builds + multi-cloud portability are mandatory |

---

<a id="decisions"></a>
## 19. Decision Matrix — Alternatives Considered

For every load-bearing choice above, the alternatives evaluated and the deciding criterion.

| Choice | Picked | Alternatives | Deciding criterion |
|---|---|---|---|
| Theorem prover | Lean 4 | Coq, Isabelle/HOL, Agda, F\* | mathlib4 size + Rust extraction quality |
| Verified parser | EverParse / 3D | Lean→Rust direct, Galois Crucible | Industry use (Microsoft QUIC, Hyper-V) |
| Verified crypto | HACL\* | fiat-crypto, libsodium | Coverage of all our crypto needs in one library |
| Systems language | Rust | Zig (used surgically), Ada/SPARK | Ecosystem + Lean extraction support |
| Build system | Buck2 | Bazel, Nix, Pants | Remote execution speed + Rust-native via Reindeer |
| Async runtime | Glommio | Monoio, Tokio, async-std | Thread-per-core io_uring + production maturity |
| Allocator | snmalloc | jemalloc, mimalloc | Multi-thread shared-memory perf |
| Fuzzing engine | Nyx | AFL++, libFuzzer, Honggfuzz | Snapshot-based fuzzing of real AOSP binaries |
| SMT solver | cvc5 | Z3, MathSAT, Yices2 | Best across theory combinations in 2026 |
| ZK system (per workload) | Halo2 / Jolt / Binius | Groth16, Bulletproofs, Risc Zero | Halo2 mature, Jolt fastest general, Binius best for binary fields |
| Native lifter | Ghidra P-Code → MLIR | BAP, Reopt, McSema, RetDec | Ghidra's coverage + MLIR's IR ecosystem |
| Concolic engine | SymQEMU + Triton | KLEE, S2E, angr | Full-system instrumentation via QEMU |
| ANN index | DiskANN | HNSW, ScaNN, FAISS | Billion-scale; disk-resident |
| Filter | Ribbon / Xor | Bloom, Cuckoo | Smaller + faster |
| Hash (general) | BLAKE3 | SHA-3, SHA-256, KangarooTwelve | Speed + tree mode + verified impl |
| Hash (in-circuit) | Poseidon2 | Rescue-Prime, Reinforced Concrete | Maturity + circuit cost |
| GPU ZK acceleration | sppark / icicle | Custom CUDA | Shared community kernel library |
| Cross-language SDK gen | uniffi + Diplomat | hand-written bindings | Single source of truth |
| Continuous profiling | Pyroscope | Datadog Profiler, Parca | OSS + flamegraph diff |
| eBPF language | Aya (Rust) | libbpf-rs (C+Rust), Cilium ebpf-go | Pure-Rust; no C |
| Columnar OLAP | DuckDB | ClickHouse, Apache DataFusion | In-process simplicity at our scale |
| Object store | MinIO / S3 | Ceph, Backblaze | Standard S3 API |
| KV store | FoundationDB | TiKV, CockroachDB, etcd | Strict serializability + tested rigor |

---

## Appendix A — Per-Phase Tech-Stack Activation

| Phase | New tech stack additions |
|---|---|
| Phase 0 | Buck2, Lean 4, mathlib4, Bazel (for AOSP only), Nix, Aya, snmalloc, Glommio, Pyroscope, OpenTelemetry |
| Phase 1 | EverParse / 3D, HACL\*, BLAKE3 (HACL\* impl), `std::simd` baseline, AFL++ + Nautilus |
| Phase 2 | Buck2 remote execution, Nyx (snapshot fuzzing), DuckDB, Arrow |
| Phase 3 | cvc5, Z3 + Spacer, Bitwuzla, Eldarica, DiskANN, Lance, Ribbon filters, Triton (preview) |
| Phase 4 | Halo2, Plonky3 (alt), uniffi, Diplomat, wasm-bindgen + wit-bindgen, sppark/icicle (GPU), Poseidon2 |
| Phase 5 | Jolt or SP1, Ghidra P-Code → MLIR, SymQEMU, Cuttlefish cluster, Aya in production, Mojo for ML hot paths |
| Phase 6 | Stwo (post-quantum STARK), Stagex (deterministic bootstrap), Trustix (build verification network) |

Each addition gets an ADR review at phase start. No tool ships without one.

---

## Appendix B — When We're Wrong

This stack is opinionated. It will be wrong about something. The discipline:

- **ADR review every 6 months.** Each load-bearing choice is re-justified or replaced.
- **Migration cost is not a deciding factor.** If something better lands, we migrate.
- **No technology stays in the stack on inertia.** "We've used X for two years" is not a reason to keep X.

State-of-the-art is a moving target. The stack moves with it.

---

*"The right tool for the job, not the tool we already know how to use."*
