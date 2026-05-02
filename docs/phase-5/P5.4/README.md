# P5.4 — ARM64 ELF Lifter to AXIOM-IR (LLVM MLIR)

> Lift ARM64 ELF (Android NDK shared libraries) to AXIOM-IR ELF-native dialect via LLVM MLIR. Function-level coverage ≥ 60 % on NDK-100 corpus, throughput ≥ 25 MB/s.

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P5.4 |
| Owner(s) | G9 |
| Duration | Weeks 3–14 |
| Critical-path | yes |
| Hard prerequisites | P5.2 |

## 2. Goal & Scope

A lifter that takes ARM64 ELF (.so) and emits AXIOM-IR ELF-native dialect, parametrized by AAPCS64 calling convention, capable of handling common Android NDK code patterns (libc, OpenSSL/BoringSSL, dlopen, JNI bridges, common packers).

### In scope
- ELF64 parser (header, sections, symbols, dynamic table, relocations)
- ARM64 disassembly (ARMv8.0–8.4, including BTI / PAC / MTE landing pads)
- Basic-block / CFG recovery
- Per-instruction lift to ELF-native dialect ops
- AAPCS64 calling convention modeling
- Stack-frame model (FP / SP / spill slots)
- Common-library function-summary catalog (built up in P5.7)
- Indirect-call resolution via VTable / PLT / GOT analysis
- Provenance tagging (every SSA value tagged with ELF section + offset)
- Coverage ≥ 60 % function-level on NDK-100 (HARD; ≥ 80 % TARGET)
- Throughput ≥ 25 MB/s ELF bytes (HARD; ≥ 80 MB/s TARGET)
- Diff harness vs Ghidra / angr / BAP

### Out of scope
- ARMv7 lifter (P5.5)
- JNI boundary modeling (P5.6)
- Common-library catalog content (P5.7)
- Lean theorems (P5.9)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P5.2** | ELF-native dialect frozen |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **LLVM / MLIR** | 19 | Lifter foundation |
| **Capstone** | 5.x | Disassembly |
| **Ghidra (headless)** | 11.x | Cross-check |
| **angr** | latest | Symex diff |
| **BAP** | latest | Binary-analysis diff |
| **goblin** (Rust ELF parser) | latest | ELF parsing |
| **iced-x86 / bad64 / yaxpeax-arm** | latest | Alternative disasm libs |

## 5. Third-Party Software, Services, Accounts & API Keys

All vendored / installed in P5.1.

**No new API keys.**

## 6. System Inventory — Have vs Need

All needed installed in P5.1.

## 7. Features & Functions Delivered (Comprehensive)

### Crate `axiom-elf-lift`
- `parse_elf(bytes) -> Result<ElfFile>`
- `disassemble(section, base) -> Vec<Inst>`
- `recover_cfg(insts) -> CFG` (basic blocks, edges, loop headers)
- `lift_function(cfg) -> Module` (per-function MLIR module)
- AAPCS64 modeling: argument passing (X0–X7, V0–V7, NSAA, stack), return (X0–X1, V0–V3), callee-saved (X19–X30, D8–D15)
- Stack-frame model
- BTI landing pads + PAC sign / authenticate ops
- MTE pointer-tag tracking
- VTable + PLT + GOT analysis for indirect-call resolution
- Provenance tags

### Tools
- `axiom-elf-lift-cli`
- `axiom-elf-diff` (vs Ghidra + angr + BAP)
- `axiom-elf-bench`

### Diff harness
- Function-level agreement metric (% functions where lifted CFG ≡ Ghidra CFG)
- Disagreements classified

### Performance
- Per-function parallelism
- LRU disasm cache for repeated bytes
- Bench: ≥ 25 MB/s on NDK-100 16-core

### Reproducibility
- Deterministic basic-block ordering
- Sorted phi operands
- Bytewise reproducible IR output

### Fuzzing
- LibAFL grammar fuzz on ELF
- Differential fuzz vs Ghidra
- ≥ 24h per merge

### Documentation
- `docs/elf-lift.md`

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Function-level coverage on NDK-100 | ≥ 60 % | ≥ 80 % |
| Throughput (16-core) | ≥ 25 MB/s | ≥ 80 MB/s |
| AAPCS64 conformance tests | 100 % | 100 % |
| BTI / PAC / MTE landing-pad tests | 100 % | 100 % |
| Diff agreement vs Ghidra (function-level) | ≥ 95 % | ≥ 99 % |
| 24h soak: zero crashes | yes | yes |
| Bytewise reproducibility | 100 % | 100 % |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-elf-lift/
│       ├── src/
│       │   ├── parser.rs
│       │   ├── disasm.rs
│       │   ├── cfg.rs
│       │   ├── aapcs64.rs
│       │   ├── stack.rs
│       │   ├── bti_pac_mte.rs
│       │   └── indirect.rs
│       ├── tests/
│       └── benches/
├── tools/
│   ├── axiom-elf-lift-cli
│   ├── axiom-elf-diff
│   └── axiom-elf-bench
├── docs/
│   └── elf-lift.md                  # NEW
└── corpora/
    └── ndk-100/                     # 100 ARM64 NDK libraries
```

## 10. Standalone Output

A reusable ARM64 → MLIR lifter, AGPL+commercial. Useful to any binary-analysis pipeline.

## 11. End-to-End Test

```bash
buck2 build //crates/axiom-elf-lift:...
buck2 test //crates/axiom-elf-lift/tests:...

buck2 run //tools:axiom-elf-bench -- --corpus ndk-100 --threads 16
# Expect ≥ 60 % function coverage, ≥ 25 MB/s

buck2 run //tools:axiom-elf-diff -- --against ghidra,angr,bap --corpus ndk-100
```

## 12. Exit Checklist

- [ ] Function-level coverage on NDK-100 ≥ 60 % (HARD)
- [ ] Throughput on 16-core ≥ 25 MB/s (HARD)
- [ ] AAPCS64 conformance 100 %
- [ ] BTI / PAC / MTE conformance 100 %
- [ ] Diff agreement vs Ghidra ≥ 95 %
- [ ] 24h soak zero crashes
- [ ] Bytewise reproducibility 100 %
- [ ] Fuzz unresolved < 3 at merge
- [ ] Documentation `docs/elf-lift.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P5.5** | ELF lifter pipeline reusable for ARMv7 |
| **P5.6** | ELF lift available for JNI boundary modeling |
| **P5.7** | Lifter ready for common-library catalog binding |
| **P5.8** | ELF lift available for joint analyzer |
