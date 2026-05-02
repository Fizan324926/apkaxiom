# P5.3 — DEX Bytecode Lifter to AXIOM-IR

> Lift DEX (Dalvik Executable) bytecode losslessly to AXIOM-IR DEX-SSA dialect. Type-checked, SSA-form, round-trippable. ≥ 95 % files coverage on Bench-10K, ≥ 50 MB/s throughput.

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P5.3 |
| Owner(s) | G9 |
| Duration | Weeks 2–10 |
| Critical-path | yes |
| Hard prerequisites | P5.2 |

## 2. Goal & Scope

A production-grade DEX → AXIOM-IR lifter:
- Reads DEX 035–040 (covers all Android versions in scope)
- SSA construction with phi insertion + dominance analysis
- Lossless round-trip on a 1000-class regression sample
- Throughput ≥ 50 MB/s of DEX bytes (HARD; ≥ 150 MB/s TARGET)
- File-level coverage ≥ 95 % on Bench-10K (HARD; ≥ 99 % TARGET)
- Differential validation against Smali / dexdump and angr DEX backend

### In scope
- DEX format parser (header, string ids, type ids, proto ids, field ids, method ids, class defs, code items, debug info)
- Per-instruction lift to DEX-SSA dialect ops
- Type lattice for JVM types
- SSA construction (Cytron's algorithm)
- Loop / dominator analysis
- Try / catch / finally lift to MLIR `cf.region` / structured exceptions
- Multi-DEX support (`classes.dex`, `classes2.dex`, …)
- Diff harness vs Smali / dexdump / angr
- Round-trip emitter (DEX-SSA → DEX) for verification
- Performance: SIMD on the DEX header parse, batch SSA construction

### Out of scope
- ELF lift (P5.4)
- JNI boundary modeling (P5.6)
- Lean theorems (P5.9)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P5.2** | DEX-SSA dialect frozen |
| **P1 / P2** | Manifest + resource dialects (used to resolve class names) |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **MLIR** | LLVM 19 | IR construction |
| **Capstone** | 5.x | Cross-check disassembly |
| **dexdump** (AOSP `dexdump2`) | from AOSP | Reference disassembly |
| **smali / baksmali** | latest | Reference DEX RW |
| **DexGuard / R8 / ProGuard test corpora** | latest | Adversarial inputs |
| **Rust** | 1.84+ | Lifter implementation language |
| **MLIR Python bindings** | matching | Test harness |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **dexdump (AOSP)** | tool | **Free** OSS (Apache 2.0) | AOSP | Already vendored |
| **smali / baksmali** | tool | **Free** OSS (BSD-3) | https://github.com/JesusFreke/smali | |
| **angr** | symex / DEX backend | **Free** OSS | (P5.1) | |
| **Bench-10K corpus** | corpus | **internal** | — | Carry-over from Phase 1 |

**No new API keys.**

## 6. System Inventory — Have vs Need

All needed tooling installed in P5.1 / P5.2.

## 7. Features & Functions Delivered (Comprehensive)

### Crate `axiom-dex-lift`
- `parse_dex(bytes) -> Result<DexFile>` (zero-copy where possible)
- `lift_class(class_def) -> Module` (per-class)
- SSA construction: Cytron's dominance-frontier algorithm
- Phi insertion + iterated rewriting
- Type-lattice inference for unannotated locals
- Try / catch / finally → MLIR `cf.region`
- Switch (packed + sparse) → MLIR `cf.switch`
- Move-result chains → SSA cleanup
- `const-class`, `const-string`, `const-method-handle` → SSA constants
- Multi-DEX classes glued via cross-module references
- Round-trip emitter (DEX-SSA → DEX bytecode) for verification

### Tools
- `axiom-dex-lift-cli` — CLI for round-trip and dump
- `axiom-dex-diff` — diff vs dexdump / smali
- `axiom-dex-bench` — perf harness

### Diff harness
- Compares lifted IR semantics against:
  - Smali round-trip (exact textual)
  - dexdump structural
  - angr DEX backend reachability
- Disagreements categorized: lifter bug / spec ambiguity / reference bug

### Per-instruction lift coverage
- All 256 DEX opcodes covered or explicitly UNKNOWN with reason
- Coverage tracked via reflection over the spec table

### Performance work
- SIMD on the header parse (SSE4.2 / NEON)
- Per-class parallelism via Rayon
- Memory pool for SSA value allocation
- jemalloc / mimalloc allocator profile
- Bench-10K throughput sustained ≥ 50 MB/s on 16-core (HARD)

### Reproducibility
- Deterministic SSA value numbering
- Sorted phi operands
- Bytewise reproducible IR output

### Fuzzing
- LibAFL grammar-fuzzer over DEX format
- Differential fuzz vs angr / dexdump
- ≥ 24h fuzz session per merge

### Documentation
- `docs/dex-lift.md` — semantics + coverage table

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| File-level coverage on Bench-10K | ≥ 95 % | ≥ 99 % |
| Throughput (16-core) | ≥ 50 MB/s | ≥ 150 MB/s |
| Round-trip byte-identity (verified subset 1000 classes) | 100 % | 100 % |
| All 256 DEX opcodes covered or explicit UNKNOWN | 100 % | 100 % |
| Differential disagreements vs angr / dexdump | < 0.5 % | < 0.05 % |
| 24h soak: zero crashes | yes | yes |
| Fuzz disagreements unresolved at merge | < 3 in queue | 0 |
| Bytewise reproducibility across runs / arches | 100 % | 100 % |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-dex-lift/
│       ├── src/
│       │   ├── parser.rs
│       │   ├── ssa.rs
│       │   ├── types.rs
│       │   ├── exceptions.rs
│       │   ├── multi_dex.rs
│       │   └── roundtrip.rs
│       ├── tests/
│       │   ├── roundtrip/                # 1000 classes
│       │   ├── opcodes/                  # all 256
│       │   └── adversarial/
│       └── benches/
├── tools/
│   ├── axiom-dex-lift-cli
│   ├── axiom-dex-diff
│   └── axiom-dex-bench
├── docs/
│   └── dex-lift.md                       # NEW
└── fuzz/
    └── dex-grammar/                      # NEW
```

## 10. Standalone Output

A drop-in DEX-to-MLIR lifter usable beyond APKAXIOM. Open-sourced under AGPL+commercial.

## 11. End-to-End Test

```bash
buck2 build //crates/axiom-dex-lift:...
buck2 test //crates/axiom-dex-lift/tests:...

# Coverage on Bench-10K
buck2 run //tools:axiom-dex-bench -- --corpus bench-10k --threads 16
# Expect: ≥ 95 % files, ≥ 50 MB/s

# Round-trip
buck2 run //tools:axiom-dex-lift-cli -- --roundtrip <classes>
# Expect: 100 % bytewise

# Diff
buck2 run //tools:axiom-dex-diff -- --against dexdump,smali,angr --corpus bench-10k
```

## 12. Exit Checklist

- [ ] File-level coverage on Bench-10K ≥ 95 % (HARD)
- [ ] Throughput on 16-core ≥ 50 MB/s (HARD)
- [ ] Round-trip byte-identity 100 % on 1000-class verified subset
- [ ] All 256 DEX opcodes covered or explicit UNKNOWN
- [ ] Differential disagreement rate < 0.5 %
- [ ] 24h soak: zero crashes
- [ ] Fuzz unresolved < 3 at merge
- [ ] Bytewise reproducibility 100 %
- [ ] CI gates live (coverage, throughput, soundness)
- [ ] Documentation `docs/dex-lift.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P5.6** | DEX SSA available for JNI bridge modeling |
| **P5.8** | DEX SSA + ELF lift available for joint analyzer |
| **P5.9** | DEX → SSA semantics for Lean theorems |
| **L4 / L5 (existing)** | Richer SSA-level info for symbolic + bisim engines |
