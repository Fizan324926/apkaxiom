# P6.10 — G9 Stabilization: Lifter Coverage Extensions for the Long Tail

> Drive native lifter coverage to ≥ 80 % function-level on NDK corpus. ARM64 + ARMv7 long-tail patterns (custom packers, anti-debug stubs, vendor-specific NDK APIs). Round-trip stable.

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P6.10 |
| Owner(s) | G9 |
| Duration | Weeks 1–16 |
| Critical-path | yes |
| Hard prerequisites | P6.1 |

## 2. Goal & Scope

Push native-lifter coverage from Phase-5 ≥ 60 % HARD to v1.0 ≥ 80 % HARD on NDK-100. Long-tail patterns covered: custom packers (Bangcle, Tencent, Qihoo), anti-debug stubs, vendor-specific NDK APIs, more JNI bridges added to the catalog (P5.7 extension).

### In scope
- ARM64 long-tail patterns
- ARMv7 long-tail patterns
- Catalog expansion (libcrypto more variants, libtensorflowlite_jni, libjpeg-turbo, libpng, libwebp, libsoup, libcurl, libavcodec subset)
- Custom-packer detection + unpack heuristics (Bangcle, Tencent, Qihoo)
- Anti-debug stub recognition (no bypass; only annotation)
- JNI catalog: ≥ 100 bridges (up from 50)

### Out of scope
- Pure-Java domain (G2)
- New ISAs (deferred to v1.1)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P6.1** | Stabilization punch-list |
| **All Phase 5 G9 deliverables** | Continued |

## 4. Required Tools, Libraries, and Languages

Same as Phase 5.

## 5. Third-Party Software, Services, Accounts & API Keys

All free OSS.

**No new API keys.**

## 6. System Inventory — Have vs Need

All present.

## 7. Features & Functions Delivered (Comprehensive)

### Coverage extensions
- ARM64 ≥ 80 % function-level on NDK-100 (HARD)
- ARMv7 ≥ 70 % function-level on legacy-NDK-100 (HARD; 50 % was Phase 5)
- DEX ≥ 99 % file-level (HARD; 95 % was Phase 5)

### Catalog expansion
- libcrypto more variants
- libtensorflowlite_jni
- libjpeg-turbo / libpng / libwebp
- libsoup / libcurl / libssl
- libavcodec subset (audio + video tracks)
- 30+ vendor-NDK patterns (Samsung, Huawei, Xiaomi, MIUI specifics)

### Custom-packer detection
- Bangcle / Tencent / Qihoo / 360 packer fingerprints
- IR `native.packed_unknown` op when packer is detected (downstream UNKNOWN)

### Anti-debug stub annotation
- Recognized stubs annotated; no bypass
- Cert evidence: anti-debug-detected flag

### JNI catalog ≥ 100 bridges (extends P5.7)
- More common-bridge patterns (vendor SDKs, ad networks, analytics, payment SDKs, social-media SDKs)

### Reproducibility
- Re-confirmed bytewise

### Documentation
- `docs/g9-stabilization.md`
- Catalog README extended

## 8. KPIs (this sub-phase)

| KPI | HARD |
|---|---|
| ARM64 function-level coverage on NDK-100 | ≥ 80 % |
| ARMv7 function-level coverage on legacy-NDK-100 | ≥ 70 % |
| DEX file-level coverage on Bench-10K | ≥ 99 % |
| JNI catalog bridges | ≥ 100 |
| Custom-packer detection on packer corpus | ≥ 90 % |
| Reproducibility (bytewise) | 100 % |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   ├── axiom-elf-lift/               # extended
│   ├── axiom-jni-bridge/             # extended
│   └── axiom-native-catalog/         # extended
├── tools/
│   └── axiom-packer-detect/          # NEW
└── docs/
    └── g9-stabilization.md           # NEW
```

## 10. Standalone Output

Coverage gains + catalog + packer detector citable in Phase-6 paper.

## 11. End-to-End Test

```bash
buck2 run //tools:axiom-elf-bench -- --corpus ndk-100 --report coverage
# Expect: ≥ 80 % ARM64

buck2 run //tools:axiom-elf-bench -- --corpus legacy-ndk-100 --arch armv7 --report coverage
# Expect: ≥ 70 % ARMv7
```

## 12. Exit Checklist

- [ ] ARM64 coverage ≥ 80 % (HARD)
- [ ] ARMv7 coverage ≥ 70 % (HARD)
- [ ] DEX coverage ≥ 99 % (HARD)
- [ ] JNI catalog ≥ 100 bridges (HARD)
- [ ] Custom-packer detection ≥ 90 %
- [ ] Reproducibility 100 %
- [ ] Documentation `docs/g9-stabilization.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P6.16** | Coverage figures in 50K eval |
| **P6.17** | Lifter scope explained to auditor |
| **P6.20** | "Native lifter ≥ 80 % coverage on NDK corpus" item ✅ for ship gate |
