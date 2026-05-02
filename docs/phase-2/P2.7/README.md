# P2.7 — Lean DEX Bytecode Parser (Opcode Subset for Phase 2)

> Mechanize the DEX format in Lean: header, string IDs, type IDs, proto IDs, field IDs, method IDs, classes, code items, and the Phase-2 opcode subset (~120 of 256 opcodes). ~2,000 LOC. Cross-checked against AOSP `dexdump` and `baksmali`.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §6 (Layer 1)](../../../README.md#layer-1)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P2.7 |
| Owner(s) | G1 |
| Duration | Weeks 4–10 |
| Critical-path | yes |
| Hard prerequisites | P2.1 (AOSP archaeology), P2.2 (DEX dialect design) |

## 2. Goal & Scope

DEX format mechanized in Lean: container format (headers, ID tables, code items) plus a Phase-2 opcode subset chosen for downstream Phase-3 G5 needs. The opcode subset is ~120 of 256 opcodes — covers all common Java semantics, intent dispatch, JNI bridging, intent-resolution paths, and the API-call patterns G5 needs. Remaining opcodes are deferred to Phase 5 (G9 native subsystem).

### In scope
- `theorems/Apkaxiom/Dex/Header.lean`
- `theorems/Apkaxiom/Dex/IdTables.lean` — strings, types, protos, fields, methods
- `theorems/Apkaxiom/Dex/Class.lean` — class defs, encoded fields, encoded methods
- `theorems/Apkaxiom/Dex/CodeItem.lean` — code items + try blocks + handlers + debug info
- `theorems/Apkaxiom/Dex/Opcode.lean` — Phase-2 opcode subset semantics
- `theorems/Apkaxiom/Dex/Soundness.lean`
- Adversarial corpus (multi-DEX, MultiDex spec, malformed instruction streams)

### Out of scope
- Full opcode coverage (Phase 5)
- Disassembly to AXIOM-IR-symbolic (Phase 3 G5)
- ART / OAT formats (Phase 5 native)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P2.1** | AOSP archaeology — DEX format changes per version |
| **P2.2** | DEX dialect design (target IR) |
| **P1.6** | ZIP layer (DEX lives inside) |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Lean 4 + mathlib4** | pinned | Theorem prover |
| **AOSP `dalvik`/`art` source** | per Android version | Reference |
| **`dexdump`** | from Android SDK Build Tools | Reference disassembler |
| **`baksmali`** | latest | Cross-check disassembler (smali assembly format) |
| **`apktool`** | HAVE | Cross-check |
| **AFL++ + radamsa** | from P1.6 | Adversarial DEX |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **DEX format spec** | reference | **Free** | https://source.android.com/docs/core/runtime/dex-format | |
| **Dalvik bytecode reference** | reference | **Free** | https://source.android.com/docs/core/runtime/dalvik-bytecode | All 256 opcodes |
| **dexdump** | binary | **Free** | already installed | |
| **baksmali / smali** | binaries | **Free** OSS | https://github.com/JesusFreke/smali | already installed in P2.2 |
| **AndroZoo / F-Droid** | corpora | **Free** | already provisioned | |
| **DREBIN labeled malware** | corpus | **Free research** | TU Braunschweig | Real-world DEX exemplars |

**No new API keys.**

## 6. System Inventory — Have vs Need

### Already present
- ✅ Lean / Lake / mathlib4 cache
- ✅ dexdump, baksmali, apktool
- ✅ AOSP partial sync covers `art/runtime/dex` and `dalvik/dx`

### Missing
- Nothing new system-level.

## 7. Features & Functions Delivered (Comprehensive)

### Lean theorems

#### DEX container
- `parseDexHeader : ByteArray → Except ParseError DexHeader`
- `parseStringIds / parseTypeIds / parseProtoIds / parseFieldIds / parseMethodIds`
- `parseClassDef : ByteArray → DexHeader → ClassDefIndex → Except ParseError DexClassDef`
- `parseCodeItem : ByteArray → ClassDef → MethodIdx → Except ParseError DexCodeItem`
- `parseInstruction : ByteArray → CodeOffset → Except ParseError DexInstruction` — Phase-2 opcode subset
- `parseTryBlocks / parseEncodedHandlers` — exception-handling structure
- `parseDebugInfo` — line numbers + locals (used by P2.15 for AXML provenance signals)
- `parseAnnotations` — class/method/field/parameter annotations (used by G5 for intent-filter equivalents declared in code)
- `parseEncodedArray / parseEncodedValue` — initial-value encoding for static fields
- `parseStaticValues`
- Multi-DEX (`classes.dex`, `classes2.dex`, …) handled by `parseDexBundle`

#### Phase-2 opcode subset (~120 opcodes)
Categories covered:
- nop / move / move-result / return (~15)
- const-* family (literal loads) (~10)
- monitor-enter / monitor-exit / check-cast / instance-of / new-instance / new-array / array-length / fill-array-data (~10)
- if-* family (~10)
- aget / aput / iget / iput / sget / sput (heap & static access) (~24)
- invoke-virtual / invoke-super / invoke-direct / invoke-static / invoke-interface + range variants (~12)
- arithmetic ops (add / sub / mul / div / rem / and / or / xor / shl / shr) for int/long/float/double (~40)

Each opcode: argument types, side-effect signature, exception-throw flag, semantic interpretation.

#### Soundness
- `dex_sound : ∀ bs, parseDex bs = ok dex → dexdump_v.disassemble bs ≡ ok dex'` per AOSP version
- Property-based round-trip on a Phase-2 opcode-subset corpus

### Adversarial cases
- Out-of-bounds string-pool / type-pool offsets
- Invalid code-item-size declarations
- Missing handler entries for declared try blocks
- Self-referential type IDs
- Multi-DEX with conflicting class definitions
- Code items declaring more registers than method signature implies

### Differential corpus
- ≥ 10,000 DEX samples from AndroZoo + F-Droid + DREBIN
- ≥ 1,500 adversarial DEX inputs (radamsa + AFL++ + hand-crafted)

### Documentation
- `docs/lean-dex.md` — opcode coverage matrix, semantic gaps deferred to Phase 5

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Cumulative Lean LOC (DEX modules) | ≥ 2,000 | ≥ 3,000 |
| Theorem re-verify on CI | ≤ 50 min | ≤ 30 min |
| Phase-2 opcode-subset coverage | ≥ 120 opcodes | ≥ 140 opcodes |
| Differential corpus | ≥ 10K + 1.5K adversarial | ≥ 25K + 3K |
| Lean ↔ dexdump agreement on benign | 100 % | 100 % |
| Lean ↔ baksmali agreement on benign | ≥ 99.5 % | ≥ 99.9 % |
| Multi-DEX correctness | 100 % | 100 % |
| Cross-version (A8..A14) agreement | 100 % | 100 % |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── theorems/Apkaxiom/Dex/
│   ├── Header.lean                       # NEW — ~200 LOC
│   ├── IdTables.lean                     # NEW — ~400 LOC
│   ├── Class.lean                        # NEW — ~400 LOC
│   ├── CodeItem.lean                     # NEW — ~400 LOC
│   ├── Opcode.lean                       # NEW — ~600 LOC (Phase-2 subset)
│   ├── Soundness.lean                    # NEW — ~200 LOC
│   └── Multi.lean                        # NEW — multi-DEX
├── corpus/dex/
│   ├── valid/                            # 10K
│   └── adversarial/                      # 1.5K
├── tests/differential/src/
│   └── dex.rs                            # NEW
└── docs/
    └── lean-dex.md                       # NEW
```

## 10. Standalone Output

```bash
nix develop
buck2 build //theorems:dex-all
buck2 test //tests/differential:dex-vs-dexdump
# "11500/11500 DEX samples Lean ↔ dexdump agree"
```

## 11. End-to-End Test

Per-AOSP-version differential against dexdump + baksmali. Multi-DEX bundles tested. Phase-2 opcode-subset coverage verified.

## 12. Exit Checklist

- [ ] DEX theorems proved (≥ 2,000 LOC)
- [ ] Phase-2 opcode subset (≥ 120 opcodes) covered
- [ ] Theorem re-verify on CI ≤ 50 min (HARD)
- [ ] 100 % Lean ↔ dexdump agreement on benign + adversarial (HARD)
- [ ] Multi-DEX correctness 100 %
- [ ] Cross-version (A8..A14) agreement 100 %
- [ ] `docs/lean-dex.md` published
- [ ] Documented opcode-subset rationale (which omitted, why)

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P2.8** | DEX Lean modules → Rust extraction target |
| **P2.10** | DEX semantics for behavior-set component reachability |
| **P2.14** | Shadow Stack reads dangling DEX type indices |
| **Phase 3 / G5** | DEX → AXIOM-IR-symbolic lifting starts here |
