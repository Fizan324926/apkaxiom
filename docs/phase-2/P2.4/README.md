# P2.4 — Lean ARSC (Resource Table) Parser Formalization

> Mechanize Android's binary resource table format. ~1,500 LOC. Differential against `aapt2 dump resources` on 5,000+ APKs. Sparse encoding + config qualifiers + complex resource types.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §6 (Layer 1)](../../../README.md#layer-1)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P2.4 |
| Owner(s) | G1 |
| Duration | Weeks 3–8 |
| Critical-path | yes |
| Hard prerequisites | P2.1, P1.6 (ZIP layer) |

## 2. Goal & Scope

`resources.arsc` is formalized in Lean: string pools (per package), resource types, type specs, entry tables, config qualifiers, complex (bag) entries, sparse encoding. Cross-checked against AOSP `aapt2 dump resources` on 5,000+ APKs.

### In scope
- `theorems/Apkaxiom/Arsc/Header.lean`
- `theorems/Apkaxiom/Arsc/Package.lean`
- `theorems/Apkaxiom/Arsc/Type.lean` — type specs + type tables + sparse encoding
- `theorems/Apkaxiom/Arsc/Config.lean` — all config qualifiers (mcc/mnc, locale, density, orientation, screen size, layout, ...)
- `theorems/Apkaxiom/Arsc/Entry.lean` — simple + complex (bag) entries
- `theorems/Apkaxiom/Arsc/Soundness.lean`
- Differential corpus + adversarial cases

### Out of scope
- Rust extraction (P2.6)
- Split-aware resource resolution (P2.10/P2.12 — bundle resolver)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P2.1** | AOSP A12/A13 archaeology — sparse encoding was added in A11; new config qualifiers in A13 |
| **P1.6** | ZIP layer theorems |
| **P1.4 / P2.2** | AXIOM-IR resource dialect |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Lean 4 + mathlib4** | pinned | Theorem prover |
| **AOSP `frameworks/base` ResTable / ResourceTable** | pinned per Android version | Reference |
| **`aapt2 dump resources`** | latest | Reference decoder |
| **`apktool d`** | HAVE | Cross-check |
| **AFL++ + radamsa** | from P1.6 | Adversarial corpus |

## 5. Third-Party Software, Services, Accounts & API Keys

Same as P2.3 — same AOSP + AndroZoo + tooling.

| Item | Type | Free / Paid | Notes |
|---|---|---|---|
| **AOSP source** | reference | **Free** OSS | already synced |
| **Android SDK Build Tools `aapt2`** | binary | **Free** | already installed |
| **AndroZoo** | corpus | **Free academic** | API key from P1.3 |
| **F-Droid archive** | corpus | **Free** | already used |

**No new third-party services or API keys.**

## 6. System Inventory — Have vs Need

### Already present
- ✅ Everything from P2.3

### Missing
- Just Bazel rule extension for ARSC reference build per AOSP version.

```bash
buck2 build //external/aosp:arsc-decoder-A{8,11,12,13,14}
```

## 7. Features & Functions Delivered (Comprehensive)

### Lean theorems
- `parseArscHeader : ByteArray → Except ParseError ArscHeader`
- `parsePackage : ByteArray → ArscHeader → Except ParseError ArscPackage`
- `parseTypeSpec / parseTypeTable` — including **sparse encoding** (sparse type tables added in A11; saves space for partially-populated configurations)
- `parseConfig : ByteArray → Except ParseError ConfigQualifier` — every config qualifier:
  - mcc/mnc
  - locale (BCP 47 + legacy three-letter)
  - layoutDirection (ltr/rtl)
  - smallestScreenWidth, screenWidth, screenHeight
  - screenLayout (size + long + round + dir)
  - colorMode (wide-color-gamut + HDR)
  - orientation, uiMode, density, touchscreen
  - keyboard / keyboardHidden / navigation / navigationHidden
  - sdkVersion / minorVersion
  - **Grammatical gender** (added Android 14)
- `parseEntry : ByteArray → ResType → Except ParseError ArscEntry`
- `parseComplexEntry : ByteArray → ResType → Except ParseError ArscBagEntry` (style/array/plurals/attr)
- `arsc_sound : ∀ bs t, parseArsc bs = ok t → aapt2_v.parse bs ↔ ok t'`
- `arsc_config_match : ConfigQualifier → DeviceState → Bool` — formalizes Android's config-resolution algorithm

### Adversarial cases handled
- Out-of-bounds entry offsets
- Sparse-encoding malformations (declared sparse but actually dense)
- Cyclic complex entries (bag references back to itself)
- Locale field overflow
- Density = 0 (must default per AOSP rules)
- Self-referential resource IDs

### Differential corpus
- ≥ 5,000 ARSC samples from F-Droid + AndroZoo
- ≥ 800 adversarial samples
- All pass: Lean ↔ aapt2 agreement = 100 %

### Documentation
- `docs/lean-arsc.md` covering: sparse vs dense encoding, complex entry semantics, config-resolution rules, version-specific qualifier additions

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Cumulative Lean LOC (ARSC modules) | ≥ 1,500 | ≥ 2,200 |
| Theorem re-verify on CI | ≤ 40 min | ≤ 25 min |
| Differential corpus size | ≥ 5,000 + 800 adversarial | ≥ 10K + 2K |
| Lean ↔ aapt2 agreement | 100 % | 100 % |
| All config qualifiers covered | yes | yes |
| Cross-version agreement on benign | 100 % | 100 % |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── theorems/Apkaxiom/Arsc/
│   ├── Header.lean                       # NEW — ~200 LOC
│   ├── Package.lean                      # NEW — ~300 LOC
│   ├── Type.lean                         # NEW — ~400 LOC (incl. sparse)
│   ├── Config.lean                       # NEW — ~400 LOC (all qualifiers)
│   ├── Entry.lean                        # NEW — ~300 LOC (simple + complex)
│   └── Soundness.lean                    # NEW — ~200 LOC
├── corpus/arsc/
│   ├── valid/                            # 5,000+
│   └── adversarial/                      # 800+
├── tests/differential/src/
│   └── arsc.rs                           # NEW
└── docs/
    └── lean-arsc.md                      # NEW
```

## 10. Standalone Output

```bash
nix develop
buck2 build //theorems:arsc-all
buck2 test //tests/differential:arsc-vs-aapt2
# "5800/5800 ARSC samples Lean ↔ aapt2 agree"
```

## 11. End-to-End Test

Per-version differential against AOSP A8/A11/A12/A13/A14 with config-resolution oracle queries.

## 12. Exit Checklist

- [ ] All ARSC theorems proved (≥ 1,500 LOC)
- [ ] Sparse encoding correctness proved
- [ ] All config qualifiers (incl. A14 grammatical-gender) covered
- [ ] Theorem re-verify on CI ≤ 40 min (HARD)
- [ ] Differential corpus ≥ 5,000 + 800 adversarial
- [ ] 100 % Lean ↔ aapt2 agreement on benign and adversarial (HARD)
- [ ] Cross-version (A8..A14) agreement on benign 100 % (HARD)
- [ ] `docs/lean-arsc.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P2.6** | ARSC Lean modules → Rust extraction target |
| **P2.10** | Resource semantics flow into Schrödinger BehaviorSet |
| **P2.16** | Negative-Space resource anomaly detector reads structural distributions defined here |
| **Phase 3 / G5** | Resource references resolved during symbolic intent resolution |
