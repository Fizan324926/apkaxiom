# P2.8 — Rust Extraction of DEX Parser + DEX Dialect Emitter

> DEX parser extracted to Rust. AXIOM-IR DEX dialect emitter lands. Bench-10K DEX coverage ≥ 95 % files. Translation validator green. Performance within 20% of hand-written.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §6](../../../README.md#layer-1)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P2.8 |
| Owner(s) | G1 + G2 |
| Duration | Weeks 9–12 |
| Critical-path | yes |
| Hard prerequisites | P2.7 (DEX Lean), P2.5 + P2.6 (extraction pattern proven) |

## 2. Goal & Scope

DEX parser extracted from Lean to Rust, integrated into `axiom-l1-rs`. AXIOM-IR DEX dialect emitter produces typed IR for every parsed class. Translation validator green on Bench-10K (DEX-bearing APKs). Throughput ≥ 100 MB/s DEX bytes single-core.

### In scope
- Extracted crate `axiom-l1-dex-verified`
- AXIOM-IR DEX dialect emitter
- Multi-DEX (`classes.dex`, `classes2.dex`, …) handled
- Translation validator nightly
- Performance regression gate

### Out of scope
- Full opcode coverage (Phase 5)
- Symbolic-IR lifting (Phase 3 G5)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P2.7** | DEX Lean modules |
| **P1.9, P2.5** | Extraction pipeline |
| **P2.2** | DEX dialect design (target IR) |

## 4. Required Tools, Libraries, and Languages

Inherited from P1.9 + P2.5 + P2.6. New: aggressive vectorization (`std::simd` for ULEB128 batch decoding) and `bumpalo` arena allocator for IR construction.

| Tool | Version | Purpose |
|---|---|---|
| **bumpalo** | 3.x | Arena allocator for transient DEX-class IR construction |
| **`std::simd`** | nightly-compatible 1.95 | ULEB128 batch decode |
| **insta** | 1.40+ | Snapshot fixtures |

## 5. Third-Party Software, Services, Accounts & API Keys

**No new external dependencies.** Reuses Lean toolchain + AOSP sync + AndroZoo.

## 6. System Inventory — Have vs Need

### Already present
- ✅ Everything from P2.5/P2.6/P2.7

### Missing
- Just `bumpalo` and SIMD nightly-compatible Cargo features.

## 7. Features & Functions Delivered (Comprehensive)

### Extracted Rust API
- `pub fn parse_dex(bytes: &[u8]) -> Result<DexFile, DexError>`
- `pub fn parse_dex_bundle(bytes: &[(&str, &[u8])]) -> Result<DexBundle, DexError>` — multi-DEX entrypoint
- `pub fn dex_to_ir(dex: &DexFile) -> Result<DexIR, LoweringError>`
- `pub struct DexFile { header, string_pool, types, protos, fields, methods, classes }`
- `pub struct DexClass { name, superclass, interfaces, fields, methods, source_file, annotations }`
- `pub struct DexMethod { name, signature, code_item, exceptions, annotations }`
- `pub struct DexInstruction { opcode, operands, location }`
- All public APIs gated by phantom `Verified`

### AXIOM-IR DEX dialect emission
- `dex.class` ops emitted per class
- `dex.method` ops with full operand types
- `dex.instruction` ops with operand SSA values
- Annotations preserved
- String-pool deduplication propagated to IR string-pool dialect

### Performance optimizations
- Bumpalo arena allocator for transient IR construction (massive speedup on large DEX files)
- SIMD ULEB128 batch decode (DEX is ULEB128-heavy)
- Zero-copy string-pool access via `bytemuck`

### Multi-DEX handling
- Detects `classes.dex` + `classes2.dex` + … pattern
- Builds unified `DexBundle` with global indices
- MultiDex spec compliance (Android < 5.0 limitations documented)

### Documentation
- `docs/verified-dex.md` covers Phase-2 opcode-subset rationale, multi-DEX semantics, IR emission contract

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Translation validator agreement on Bench-10K | 100 % | 100 % |
| DEX coverage on Bench-10K (parses without error) | ≥ 95 % files | ≥ 99 % |
| DEX parse throughput single-core | ≥ 100 MB/s | ≥ 250 MB/s |
| Extracted vs hand-Rust perf delta | within 20 % | within 8 % |
| AXIOM-IR DEX-dialect round-trip on snapshots | ≥ 95 % byte-identical | ≥ 99 % |
| Multi-DEX correctness | 100 % | 100 % |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   ├── axiom-l1-dex-verified/             # NEW — auto-generated
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   └── axiom-l1-rs/
│       └── src/
│           ├── parser/dex.rs              # uses verified path
│           └── ir/dex.rs                  # NEW — DEX dialect emitter
├── tests/translation-validation/
│   └── dex-bench-10k.rs                   # NEW
└── docs/
    └── verified-dex.md                    # NEW
```

## 10. Standalone Output

```bash
nix develop
make extract-dex
buck2 test //tests/translation-validation:dex-bench-10k
# "Coverage: 95.4% (9540/10000) — translation-validator green on covered set"
buck2 run //bench:dex-throughput
# "DEX parse: 165 MB/s single-core (HARD ≥ 100 MB/s)"
```

## 11. End-to-End Test

```bash
buck2 test //axiom-l1-rs:integration-dex-bench-10k
# - 100% structure agreement on covered set (HARD)
# - DEX coverage ≥ 95% files (HARD)
# - throughput ≥ 100 MB/s/core (HARD)
# - perf delta ≤ 20% vs hand-written (HARD)
# - DEX-dialect round-trip ≥ 95% byte-identical (HARD)
```

## 12. Exit Checklist

- [ ] DEX extracted Rust crate compiles
- [ ] `axiom-l1-rs` defaults to verified DEX
- [ ] Translation validator 100 % green on covered set (HARD)
- [ ] DEX coverage ≥ 95 % files (HARD)
- [ ] DEX throughput ≥ 100 MB/s/core (HARD)
- [ ] Perf delta ≤ 20 % (HARD)
- [ ] DEX-dialect round-trip ≥ 95 % (HARD)
- [ ] Multi-DEX correctness 100 %
- [ ] `docs/verified-dex.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P2.9** | DEX dialect ready for IR-v0.2 freeze |
| **P2.10** | DEX semantics for BehaviorSet reachability over Schrödinger configurations |
| **P2.14** | Shadow Stack uses dangling DEX type-index detection |
| **Phase 3 / G5** | DEX dialect input for symbolic resolver |
| **Phase 5 / G9** | Native code subsystem extends opcode coverage and joins Java side |
