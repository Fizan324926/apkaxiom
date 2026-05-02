# P5.2 — AXIOM-IR-v0.4 Native Dialect (DEX SSA + ELF) Design Freeze

> Extend AXIOM-IR with a native dialect so DEX SSA + ARM64/ARMv7 ELF + JNI boundary nodes share one IR with the manifest / resource / symbolic dialects. Frozen RFC, no churn from W4 onward.

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P5.2 |
| Owner(s) | G3 + G9 |
| Duration | Weeks 1–4 |
| Critical-path | **yes** (blocks every native sub-phase) |
| Hard prerequisites | P5.1 |

## 2. Goal & Scope

A single, coherent IR dialect spec covering:
- **DEX SSA values + types** — primitives, references, arrays, methods, classes
- **ELF / native opcodes** — load / store / move / arith / control-flow / call / syscall / atomic / fence
- **JNI boundary nodes** — `jni.call`, `jni.return`, `jni.global_ref`, `jni.local_ref`, `jni.detach_thread`
- **Calling convention metadata** — AAPCS64 (ARM64), AAPCS-VFP (ARMv7), x86-64 SysV (host-only for proof tooling)
- **Type system** — JVM types lifted to AXIOM-IR types, native pointer types with provenance metadata, JNI-handle types
- **Provenance & tainting** — every SSA value carries an origin tag (DEX class, ELF section, JNI bridge); used by L4 + L5
- **Cross-dialect calls** — explicit op for "DEX method calls JNI bridge calls native function"
- **Round-trip property** — lift then re-emit must be byte-equivalent for verified subset (tested via P5.3)

### In scope
- IR dialect spec (`docs/AXIOM-IR-v0.4-native.md`)
- Type-system extensions
- Calling-convention encoding
- Provenance encoding
- JNI boundary nodes
- ADR-0032 — IR-v0.4 freeze

### Out of scope
- Lifter implementation (P5.3, P5.4, P5.5)
- Lean soundness theorems (P5.9)
- Joint analyzer (P5.8)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P5.1** | G3 + G9 onboarded |
| **P3.3** | AXIOM-IR-symbolic dialect (preview) |
| **AXIOM-IR-v0.3** | Frozen Phase-3 baseline |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **MLIR TableGen** | matching LLVM 19 | Dialect generation |
| **MLIR ODS** (operation-definition specification) | LLVM 19 | Op codegen |
| **Lean 4** | pinned | Type-system spec cross-check |
| **mdBook** | latest | RFC publishing |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **MLIR / TableGen / ODS** | tooling | **Free** OSS (Apache 2.0 + LLVM exception) | https://mlir.llvm.org | Already pinned in P5.1 |
| **mdBook** | doc tool | **Free** OSS | https://rust-lang.github.io/mdBook | |

**No new API keys.**

## 6. System Inventory — Have vs Need

### Already present
- ✅ AXIOM-IR-v0.3 (frozen Phase 3)
- ✅ MLIR / LLVM (P5.1)

### Missing — must install
- (none)

## 7. Features & Functions Delivered (Comprehensive)

### Dialect spec (`docs/AXIOM-IR-v0.4-native.md`)
- Op taxonomy (DEX SSA, ELF native, JNI boundary, cross-dialect)
- Type system: JVM types → AXIOM-IR types; native pointer types with provenance; JNI-handle types
- Calling convention encoding for AAPCS64 / AAPCS-VFP / x86-64 SysV
- Memory model: relaxed, acquire, release, seq-cst, fence ops
- Atomics + locks
- Stack-frame model
- Provenance tags on SSA values
- Cross-dialect call op (`crossdial.invoke`) with explicit boundary handoff

### MLIR ODS / TableGen
- ODS files for DEX-SSA dialect
- ODS files for ELF native dialect
- ODS files for JNI boundary nodes
- Generated C++ + Rust bindings

### Type system tests (negative + positive)
- 200+ unit tests covering type compatibility, JNI marshaling, calling-convention conformance

### RFC review
- ≥ 4 weeks RFC-review window before freeze (W1–W4)
- Reviewers: G1, G2, G3, G5, G6, G7, G9, G14
- ADR-0032 — IR-v0.4 freeze

### Provenance encoding
- Every SSA value tagged with: origin dialect, source range (offset in DEX / ELF), JNI-bridge entry id (if any)
- Used downstream by L4 (joint Java+native analyzer) for taint tracking

### Public publication
- `docs/AXIOM-IR-v0.4-native.md` published via mdBook
- Spec carries SHA-256 + Ed25519 sig

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Dialect spec frozen by W4 | yes | yes |
| ≥ 4-week RFC-review window | yes | yes |
| All 8 reviewer leads sign off | yes | yes |
| MLIR ODS / TableGen builds clean | yes | yes |
| 200+ type-system unit tests green | yes | yes |
| ADR-0032 merged | yes | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── docs/
│   ├── AXIOM-IR-v0.4-native.md       # NEW: RFC-style spec
│   └── ADR-0032-ir-v04-freeze.md     # NEW
├── ir/
│   ├── dialects/
│   │   ├── dex-ssa.td                # NEW: ODS
│   │   ├── elf-native.td             # NEW
│   │   └── jni-boundary.td           # NEW
│   └── tests/
│       ├── type-system/              # NEW: 200+ tests
│       └── provenance/               # NEW
└── crates/
    └── axiom-ir/
        └── src/dialects/             # generated bindings
```

## 10. Standalone Output

The IR dialect spec is reusable beyond APKAXIOM — any Android-native analyzer that wants a sound, lossless IR can adopt it.

## 11. End-to-End Test

```bash
buck2 build //ir/dialects:all
buck2 test //ir/tests/type-system:...    # 200+ tests, all green
buck2 test //ir/tests/provenance:...     # provenance tags propagate

# RFC publication
mdbook build docs/
sha256sum docs/AXIOM-IR-v0.4-native.md
```

## 12. Exit Checklist

- [ ] Dialect spec frozen by W4 (HARD)
- [ ] ≥ 4-week RFC-review window completed
- [ ] All 8 leads sign off (G1, G2, G3, G5, G6, G7, G9, G14)
- [ ] ODS / TableGen build clean
- [ ] Type-system unit tests ≥ 200, all green
- [ ] Provenance-encoding tests green
- [ ] Cross-dialect call op tested
- [ ] ADR-0032 merged
- [ ] Spec published via mdBook + SHA-256 + Ed25519 sig

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P5.3** | DEX-SSA dialect frozen |
| **P5.4 / P5.5** | ELF-native dialect frozen |
| **P5.6** | JNI boundary nodes frozen |
| **P5.8** | Cross-dialect call op for joint analyzer |
| **P5.9** | Stable type-system spec for Lean theorems |
