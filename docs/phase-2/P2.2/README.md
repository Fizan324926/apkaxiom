# P2.2 — AXIOM-IR v0.2 Spec Planning + DEX Dialect Design

> Plan the spec freeze of v0.2: DEX dialect added, manifest + resource dialects extended for bundle-era. RFC published, reviewed, ADR drafted.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §13.9 (AXIOM-IR)](../../../README.md#beyond-the-12)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P2.2 |
| Owner(s) | G3 |
| Duration | Weeks 1–3 |
| Critical-path | yes |
| Hard prerequisites | P2.1 (kickoff complete) |

## 2. Goal & Scope

The full design RFC for AXIOM-IR v0.2. Adds the DEX dialect, expands the manifest dialect (bundle-era components, dynamic-feature manifests), expands the resource dialect (split-aware references, runtime-loaded resources). Frozen by P2.9; this sub-phase produces the design.

### In scope
- AXIOM-IR-v0.2 RFC (~ 60–100 pages)
- DEX dialect type set (instructions, types, methods, classes, fields)
- Manifest dialect extensions (split refs, dynamic-feature attrs, asset-pack refs)
- Resource dialect extensions (config-qualifier merging, split-aware lookups)
- Lowering rules updated
- Lean reflection of the v0.2 types planned

### Out of scope
- Implementing the dialect (P2.5/P2.6/P2.8 do this)
- Freezing v0.2 (P2.9)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P1.4** | AXIOM-IR-v0.1 frozen spec |
| **P2.1** | AOSP archaeology delta report — informs v0.2 type set |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **MLIR documentation** | LLVM 19+ | Reference for dialect-design patterns |
| **Cap'n Proto** | from P1.4 | Wire-format extension |
| **PlantUML / Mermaid** | latest | Type-graph diagrams |
| **markdownlint** | latest | RFC consistency |
| **JADX / Smali** | reference | DEX semantics oracle when designing the dialect |
| **dexdump** | from Android SDK Build Tools | DEX inspection |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL / Account | Notes |
|---|---|---|---|---|
| **DEX format spec (Android docs)** | reference | **Free** | https://source.android.com/docs/core/runtime/dex-format | Authoritative |
| **DEX bytecode reference** | reference | **Free** | https://source.android.com/docs/core/runtime/dalvik-bytecode | All 256 opcodes documented |
| **Smali / Baksmali** | tooling | **Free** OSS | https://github.com/JesusFreke/smali | Reference DEX assembler/disassembler |
| **JADX** | DEX → Java decompiler | **Free** OSS | https://github.com/skylot/jadx | Used as oracle |
| **Apache Arrow + DuckDB docs** | reference | **Free** OSS | already in stack | For columnar IR analytics |

**No new API keys, no paid services.** All references are public docs.

## 6. System Inventory — Have vs Need

### Already present
- ✅ Rust + axiom-ir crate from P1.4
- ✅ Cap'n Proto compiler (from P1.4)
- ✅ apktool, dot, mermaid-cli

### Missing — must install
- ❌ **dexdump** — comes with Android SDK Build Tools (already installed in P1.11)
- ❌ **JADX** — `curl -L https://github.com/skylot/jadx/releases/latest/download/jadx-1.5.0.zip -o jadx.zip && unzip jadx.zip`
- ❌ **Smali / Baksmali** — `wget https://github.com/JesusFreke/smali/releases/download/v2.5.2/smali-2.5.2.jar`

```bash
# JADX
mkdir -p ~/tools/jadx && cd ~/tools/jadx
curl -L https://github.com/skylot/jadx/releases/latest/download/jadx-gui-1.5.0.zip -o jadx.zip
unzip jadx.zip && rm jadx.zip

# Smali
mkdir -p ~/tools/smali
curl -L https://github.com/JesusFreke/smali/releases/download/v2.5.2/smali-2.5.2.jar -o ~/tools/smali/smali.jar
curl -L https://github.com/JesusFreke/smali/releases/download/v2.5.2/baksmali-2.5.2.jar -o ~/tools/smali/baksmali.jar
```

## 7. Features & Functions Delivered (Comprehensive)

### AXIOM-IR-v0.2 RFC (`docs/AXIOM-IR-v0.2-RFC.md`)
- **DEX dialect** — the most consequential v0.2 addition:
  - `dex.class` (typed name, superclass, interfaces, modifiers, source-file, annotations)
  - `dex.method` (signature, registers, instructions, exceptions)
  - `dex.field` (type, modifiers, initial value)
  - `dex.instruction` enum covering all 256 Dalvik opcodes (with arity, side effects, exception-throw flags)
  - `dex.type` (primitive, reference, array)
  - `dex.string-pool` (indexed, deduplicated)
  - `dex.intrinsic-call` for known framework intrinsics
- **Manifest dialect extensions:**
  - `manifest.split-ref` (reference to a split APK from the base)
  - `manifest.dynamic-feature` (declares a dynamic-feature module: name, on-demand vs install-time, fusing rules)
  - `manifest.asset-pack-ref` (asset-pack delivery model: install-time, fast-follow, on-demand)
  - `manifest.locale-config` (per-locale split eligibility)
- **Resource dialect extensions:**
  - `resource.config-qualifier-tree` (sparse encoding for ARSC config qualifiers)
  - `resource.split-aware-ref` (resolves at config-resolution time)
  - `resource.runtime-loaded-string` (asset-pack-delivered strings)

### Lowerings updated
- Manifest → Resource (split-ref resolution)
- Manifest → DEX (component-class lookups)
- DEX → AXIOM-IR-symbolic (preview for Phase-3 G5)

### Reference Rust types planned
- Skeleton extensions to `crates/axiom-ir/src/dex/`, `manifest/`, `resource/`
- Cap'n Proto schema delta drafted for v0.2

### Lean reflection plan
- New file `theorems/Apkaxiom/IrV2.lean` planned with type-set theorems

### Diagrams
- DEX dialect type-graph (graphviz)
- Bundle-era manifest extension flowchart (mermaid)
- Lowering flow (manifest ↔ resource ↔ DEX)

### Decision log
- ADR-0010 — DEX dialect granularity (per-instruction vs basic-block)
- ADR-0011 — Asset-pack delivery model representation
- ADR-0012 — Split-aware resource resolution semantics

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| RFC length | ≥ 60 pages | ≥ 100 pages |
| Reviewer sign-off | G1, G2, G3, G4, G5 leads | + G6 lead (preview) |
| All 256 Dalvik opcodes documented | yes | yes |
| Bundle-era manifest extension cases enumerated | ≥ 30 | ≥ 60 |
| Lowering rules complete | manifest↔resource + manifest↔DEX | + DEX↔symbolic preview |
| ADRs drafted | 3 (0010, 0011, 0012) | 5 |
| Mermaid + graphviz diagrams rendered | ≥ 4 | ≥ 8 |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── docs/
│   ├── AXIOM-IR-v0.2-RFC.md              # NEW — full RFC
│   ├── AXIOM-IR-v0.2-text-format.md      # NEW — extends v0.1 textual form
│   ├── ADR-0010-dex-dialect-granularity.md
│   ├── ADR-0011-asset-pack-delivery-model.md
│   └── ADR-0012-split-aware-resource-resolution.md
├── crates/axiom-ir/src/
│   ├── dex/                              # NEW — type skeletons (no impl yet)
│   │   ├── mod.rs
│   │   ├── instruction.rs
│   │   └── opcode.rs
│   ├── manifest/                         # extensions
│   │   ├── split.rs                      # NEW
│   │   ├── dynamic_feature.rs            # NEW
│   │   └── asset_pack.rs                 # NEW
│   └── resource/
│       ├── split_aware.rs                # NEW
│       └── runtime_loaded.rs             # NEW
├── theorems/Apkaxiom/IrV2.lean           # NEW — type-set planning
├── schema/axiom_ir_v0_2.capnp            # NEW — wire-format delta
└── diagrams/
    ├── axiom-ir-v0.2-types.dot
    ├── axiom-ir-v0.2-types.svg
    ├── bundle-manifest-extensions.mmd
    └── bundle-manifest-extensions.svg
```

## 10. Standalone Output

A reviewable RFC + reference type skeletons. Phase-2 implementation sub-phases (P2.5/P2.6/P2.8) consume the RFC directly.

## 11. End-to-End Test

```bash
# Build the type skeletons (must compile, no behavior yet)
buck2 build //crates/axiom-ir
# Lean reflection planning module re-verifies (placeholder theorems)
buck2 build //theorems:ir-v0-2-plan
# RFC reviewer sign-off check
grep -c "^✅ approved by G" docs/sign-offs/P2.2.md  # ≥ 5
```

## 12. Exit Checklist

- [ ] AXIOM-IR-v0.2 RFC ≥ 60 pages, all 256 Dalvik opcodes documented
- [ ] Bundle-era manifest + resource extension cases enumerated
- [ ] All lowering rules drafted
- [ ] ADRs 0010, 0011, 0012 merged
- [ ] G1, G2, G3, G4, G5 lead sign-offs
- [ ] DEX dialect Rust skeleton compiles (no behavior yet)
- [ ] Cap'n Proto schema delta validates
- [ ] Diagrams rendered and embedded
- [ ] Lean reflection planning module placeholder lands

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P2.5, P2.6, P2.8** | DEX dialect + manifest/resource extensions to implement |
| **P2.7** | DEX type set as Lean target |
| **P2.9** | RFC ready to be promoted to frozen spec |
| **P2.10** | Bundle-era extensions inform Schrödinger semantics |
| **Phase 3 / G5** | Symbolic resolver lifts AXIOM-IR-v0.2 — preview lowerings here |
