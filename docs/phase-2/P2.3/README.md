# P2.3 — Lean AXML (Binary XML) Parser Formalization

> Mechanize the Android AXML format in Lean. ~1,500 LOC. Cross-checked against AOSP `aapt2` and `apktool` on 5,000+ AXML samples across A8–A14.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §6 (Layer 1)](../../../README.md#layer-1)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P2.3 |
| Owner(s) | G1 |
| Duration | Weeks 2–7 |
| Critical-path | yes |
| Hard prerequisites | P2.1 (AOSP A12/A13 archaeology), P1.6 (ZIP layer Lean — AXML lives inside ZIP) |

## 2. Goal & Scope

The Android binary AXML format is formalized in Lean 4: string pool, resource map, namespaces, start/end tags, attributes, CDATA, the XML tree itself. A theorem states `Lean.parseAxml accepts iff aapt2 dump xmltree accepts on the same bytes`, with consistent decoded structure.

### In scope
- `theorems/Apkaxiom/Axml/StringPool.lean`
- `theorems/Apkaxiom/Axml/ResourceMap.lean`
- `theorems/Apkaxiom/Axml/Tree.lean` (start tags, end tags, attributes, CDATA)
- `theorems/Apkaxiom/Axml/Soundness.lean` (cross-version + cross-tool agreement)
- Adversarial AXML corpus: ≥ 1,000 BadPack-class manipulations (oversized string pool, malformed offsets, type-mismatched attributes)
- Differential against `aapt2 dump xmltree`, `apktool d`, AOSP A8/A11/A12/A13/A14 internal AXML decoder

### Out of scope
- Rust extraction (P2.5)
- Resource table parser (P2.4)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P2.1** | AOSP A12/A13 archaeology delta — AXML changes across versions |
| **P1.6** | ZIP layer theorems (AXML lives inside ZIP) |
| **P1.4** | AXIOM-IR manifest dialect (Lean reflection) |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Lean 4 + mathlib4** | pinned | Theorem prover |
| **AOSP `frameworks/base` AXML decoder** | pinned per Android version | Reference implementation; pull via repo tool |
| **`aapt2 dump xmltree`** | from Android SDK Build Tools | Production-grade reference decoder |
| **`apktool d`** | latest (HAVE) | Cross-check decoder |
| **AFL++ + radamsa** (from P1.6) | latest | Adversarial corpus generation |
| **Hypothesis** (Python) | latest | Property-based input generation |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **AOSP `frameworks/base` AXML decoder** | source | **Free** OSS (Apache 2.0) | https://android.googlesource.com/platform/frameworks/base | Already partially synced |
| **Android SDK Build Tools `aapt2`** | binary | **Free** | from P1.11 SDK install | Already installed |
| **apktool** | binary | **Free** OSS (Apache 2.0) | already installed (HAVE) | Cross-check |
| **AndroZoo** | corpus | **Free academic** | https://androzoo.uni.lu | API key from P1.3 |
| **F-Droid signed APK archive** | corpus | **Free** | https://f-droid.org/archive/ | Diverse manifest examples |

**No new API keys.** AndroZoo key reused.

## 6. System Inventory — Have vs Need

### Already present
- ✅ Lean 4 + mathlib4
- ✅ AFL++, radamsa, Hypothesis
- ✅ aapt2, apktool

### Missing
- Just need to extend Bazel sub-workspace to expose AOSP AXML decoder per version.

```bash
# Bazel rule extending external/aosp/BUILD to build AXML decoder library per version
buck2 build //external/aosp:axml-decoder-A8 \
            //external/aosp:axml-decoder-A11 \
            //external/aosp:axml-decoder-A12 \
            //external/aosp:axml-decoder-A13 \
            //external/aosp:axml-decoder-A14
```

## 7. Features & Functions Delivered (Comprehensive)

### Lean theorems
- `parseStringPool : ByteArray → Except ParseError StringPool` — handles UTF-8 / UTF-16 dual encoding, sorted/unsorted flag, 64-style "huge string" overflow handling.
- `parseResourceMap : ByteArray → Except ParseError ResourceMap` — maps string-pool indices to resource IDs (R.attr.x).
- `parseAxmlTree : ByteArray → StringPool → ResourceMap → Except ParseError AxmlTree` — recursive tree parser with proper nesting + attribute typing.
- `parseAxml : ByteArray → Except ParseError AxmlDocument` — top-level orchestration (LFH-extracted bytes → typed `AxmlDocument`).
- `axml_sound : ∀ bs doc, parseAxml bs = ok doc → ∀ v ∈ {A8..A14}, aapt2_v.parse bs ↔ ok doc'` — soundness theorem with version-stratification.
- `axml_round_trip : ∀ doc, encodeAxml doc → decodeAxml ≡ ok doc` — round-trip identity for canonical inputs.

### Adversarial cases handled
- **Oversize string pool offsets** — Lean rejects, AOSP rejects, agreement enforced.
- **Malformed namespace stack** — bracket-mismatch in start/end tags.
- **Cyclic attribute references** — Lean rejects with explicit `CycleDetected` error.
- **Type-mismatched attributes** — string where integer expected (and vice versa).
- **UTF-8/UTF-16 oscillation** — flag flip mid-string-pool.
- **Empty string-pool with non-empty references** — must reject.
- **Resource-map references beyond bounds**.

### Differential corpus
- ≥ 5,000 valid AXML samples (extracted from F-Droid + AndroZoo APKs).
- ≥ 1,000 adversarial samples (radamsa + AFL++ mutations + hand-crafted CVE-style attacks).
- Per-AOSP-version reference outputs cached for fast comparison.

### Lean reflection of AXIOM-IR manifest dialect
- `axml_to_manifest_ir : AxmlDocument → AxmlError ⊕ Manifest.Document` — preview lowering used by P2.5 emitter.

### Documentation
- `docs/lean-axml.md` — design notes, invariants, edge cases, encoding-rule subtleties.

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Cumulative Lean LOC (AXML modules) | ≥ 1,500 | ≥ 2,000 |
| Theorem re-verify on CI | ≤ 35 min | ≤ 20 min |
| Differential corpus size | ≥ 5,000 valid + 1,000 adversarial | ≥ 10K + 2K |
| Lean ↔ aapt2 agreement on all corpora | 100 % | 100 % |
| Lean ↔ apktool agreement on benign corpus | ≥ 99.5 % | ≥ 99.9 % |
| Cross-version (A8..A14) agreement on benign corpus | 100 % | 100 % |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── theorems/Apkaxiom/Axml/
│   ├── StringPool.lean                    # NEW — ~400 LOC
│   ├── ResourceMap.lean                   # NEW — ~300 LOC
│   ├── Tree.lean                          # NEW — ~600 LOC
│   ├── Soundness.lean                     # NEW — ~200 LOC
│   └── ToManifest.lean                    # NEW — preview lowering
├── corpus/axml/
│   ├── valid/                             # 5,000+ samples
│   └── adversarial/                       # 1,000+
├── tests/differential/src/
│   └── axml.rs                            # NEW — diff vs aapt2/apktool/AOSP
└── docs/
    └── lean-axml.md                       # NEW
```

## 10. Standalone Output

```bash
nix develop
buck2 build //theorems:axml-all
buck2 test //tests/differential:axml-vs-aapt2
# "6000/6000 AXML samples Lean ↔ aapt2 agree"
```

## 11. End-to-End Test

```bash
# Per-version differential
for v in A8 A11 A12 A13 A14; do
  buck2 test //tests/differential:axml-vs-aosp-$v
done
# All must report 100% agreement on benign + adversarial corpora
```

## 12. Exit Checklist

- [ ] AXML parser theorems proved (≥ 1,500 LOC)
- [ ] Theorem re-verify on CI ≤ 35 min (HARD)
- [ ] Differential corpus ≥ 5,000 valid + ≥ 1,000 adversarial
- [ ] 100 % agreement Lean ↔ aapt2 on all corpora (HARD)
- [ ] 100 % cross-version agreement on benign corpus (HARD)
- [ ] All adversarial cases reject in both Lean and AOSP
- [ ] `docs/lean-axml.md` published
- [ ] Mathlib4 cache hit rate ≥ 90 % on AXML CI runs

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P2.5** | AXML Lean modules → Rust extraction target |
| **P2.10** | AXML decoder is the basis for manifest portion of Schrödinger BehaviorSet |
| **P2.15** | AXML provenance fingerprint reads structural micro-features defined here |
| **Phase 3 / G5** | Manifest semantics flow into symbolic resolver |
