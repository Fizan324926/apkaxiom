# P2.6 — Rust Extraction of ARSC Parser + Integration

> ARSC Lean theorems extracted to Rust. Replaces hand-written ARSC parser. Translation validator green on Bench-10K. Performance within 15% of hand-written.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §6](../../../README.md#layer-1)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P2.6 |
| Owner(s) | G1 + G2 |
| Duration | Weeks 7–10 |
| Critical-path | yes |
| Hard prerequisites | P2.4 (ARSC Lean) |

## 2. Goal & Scope

The Lean ARSC parser extracted to Rust and integrated into `axiom-l1-rs`. Translation validator on Bench-10K. Throughput ≥ 3K APKs/sec/core for ARSC alone. All config qualifiers correctly resolved.

### In scope
- Extracted crate `axiom-l1-arsc-verified`
- `axiom-l1-rs` switched to verified ARSC by default
- Translation validator nightly on Bench-10K
- Sparse encoding optimized in extracted Rust
- Config-resolution helper APIs

### Out of scope
- Split-aware resource resolution (P2.12)
- Asset-pack runtime resources (P2.12)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P2.4** | ARSC Lean modules + theorems |
| **P1.9** | Extraction pipeline |
| **P2.2** | AXIOM-IR resource dialect extensions |

## 4. Required Tools, Libraries, and Languages

Same as P2.5. New: `bytemuck` for fast typed-byte access into ARSC chunk bodies; SIMD intrinsics for type-table sparse-bit-scan.

| Tool | Version | Purpose |
|---|---|---|
| **bytemuck** | 1.x | Zero-copy POD access |
| **`std::simd` AVX-512 / SVE2** | nightly compatible 1.95 | Sparse-bit-scan acceleration |
| **proptest, insta** | from earlier | Round-trip testing |

## 5. Third-Party Software, Services, Accounts & API Keys

**No new external dependencies.** Reuses Lean toolchain + AOSP sync + AndroZoo.

## 6. System Inventory — Have vs Need

### Already present
- ✅ Everything from P2.5

### Missing
- Just `bytemuck` and SIMD-feature crate enablement in Cargo.

## 7. Features & Functions Delivered (Comprehensive)

### Extracted Rust API
- `pub fn parse_arsc(bytes: &[u8]) -> Result<ResourceTable, ArscError>`
- `pub fn arsc_to_resource_ir(table: &ResourceTable) -> Result<ResourceIR, LoweringError>`
- `pub struct ResourceTable { packages, string_pool }`
- `pub fn resolve_resource(table: &ResourceTable, id: ResId, config: &ConfigQualifier) -> Option<ResEntry>`
- `pub fn config_match(qualifier: &ConfigQualifier, device: &DeviceState) -> bool`
- All public APIs gated by `Verified` phantom marker

### Sparse encoding optimization
- AVX-512 / SVE2 SIMD bit-scan for sparse type tables
- Performance gain ~ 4× over scalar implementation on dense tables
- Falls back to scalar where SIMD unavailable

### Integration
- `axiom-l1-rs` switches to verified ARSC by default; `cfg(feature = "legacy-arsc")` fallback
- Streaming hooks emit `ParseEvent::ResourceTableStart / ResourceEntry / ResourceTableEnd`
- Merkle commits per ARSC chunk

### Config-resolution helpers
- `pub fn enumerate_feasible_configs(table: &ResourceTable) -> Vec<ConfigQualifier>` — enumerate all configs the APK declares
- `pub fn resolve_for_device(table: &ResourceTable, device: &DeviceState) -> ResolvedView` — get the view a specific Android device would see
- These are the building blocks for P2.10's BehaviorSet construction

### Documentation
- `docs/verified-arsc.md` covers extraction approach, SIMD design, config-resolution semantics

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Translation validator agreement on Bench-10K | 100 % | 100 % |
| Extracted ARSC parser perf delta vs hand-Rust | within 15 % | within 5 % |
| ARSC decode throughput single-core | ≥ 3K APKs/sec | ≥ 8K APKs/sec |
| Sparse-table SIMD path: ≥ 3× scalar speedup | yes | ≥ 4× |
| Reproducibility per-APK | 100 % bit-identical | 100 % |
| Config resolution correctness on AOSP test vectors | 100 % | 100 % |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   ├── axiom-l1-arsc-verified/            # NEW — auto-generated
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   └── axiom-l1-rs/
│       └── src/parser/
│           └── arsc.rs                    # uses verified path
├── tests/translation-validation/
│   └── arsc-bench-10k.rs                  # NEW
└── docs/
    └── verified-arsc.md                    # NEW
```

## 10. Standalone Output

```bash
nix develop
make extract-arsc
buck2 test //tests/translation-validation:arsc-bench-10k
# "10000/10000 ARSC verdicts axiom-l1-arsc-verified ↔ Lean reference agree"
```

## 11. End-to-End Test

```bash
buck2 test //axiom-l1-rs:integration-arsc-bench-10k
# - 100% structure + verdict agreement (HARD)
# - throughput ≥ 3K APKs/sec/core (HARD)
# - perf delta ≤ 15% (HARD)
# - SIMD speedup ≥ 3× on sparse tables (HARD)
```

## 12. Exit Checklist

- [ ] ARSC extracted Rust crate compiles
- [ ] `axiom-l1-rs` defaults to verified ARSC
- [ ] Translation validator 100 % green on Bench-10K (HARD)
- [ ] ARSC throughput ≥ 3K APKs/sec/core (HARD)
- [ ] Perf delta ≤ 15 % vs hand-written (HARD)
- [ ] SIMD sparse-table speedup ≥ 3× (HARD)
- [ ] Config resolution correct on all AOSP test vectors (HARD)
- [ ] Reproducibility 100 %
- [ ] `docs/verified-arsc.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P2.10** | Verified ARSC + config-resolution helpers feed Schrödinger BehaviorSet |
| **P2.11** | Bundle parser handles split resources via this |
| **P2.16** | Negative-Space resource anomaly detector reads structural distributions |
| **Phase 3 / G5** | Resource references resolved during symbolic intent resolution |
