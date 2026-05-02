# P5.5 — ARMv7 ELF Lifter (Legacy)

> Extend the ELF lifter to ARMv7 (32-bit) for legacy app coverage. AAPCS-VFP modeling, Thumb-2 ISA, ≥ 50 % function coverage on legacy NDK corpus.

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P5.5 |
| Owner(s) | G9 |
| Duration | Weeks 6–15 |
| Critical-path | yes (for v1.0 scope; can defer to v1.1 if blocked) |
| Hard prerequisites | P5.4 |

## 2. Goal & Scope

ARMv7 (32-bit) ELF lifter extending the P5.4 ARM64 work. Required because a meaningful slice of installed Android apps still ship 32-bit native libraries, especially in emerging-market regions and on older devices.

### In scope
- ELF32 parser
- ARMv7 + Thumb-2 disassembly (VFPv3, NEON, etc.)
- AAPCS-VFP calling convention
- ARM ↔ Thumb mode-switch tracking
- Function-level coverage ≥ 50 % HARD (≥ 80 % TARGET)
- Throughput ≥ 20 MB/s HARD (≥ 60 MB/s TARGET)

### Out of scope
- 64-bit / pure ARM64 patterns (P5.4)
- Lean theorems for ARMv7 (deferred to Phase 6 or v1.1)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P5.2** | ELF-native dialect frozen |
| **P5.4** | ARM64 lifter foundation reused |

## 4. Required Tools, Libraries, and Languages

Same as P5.4 plus:

| Tool | Version | Purpose |
|---|---|---|
| **yaxpeax-arm** (32-bit) | latest | ARMv7 + Thumb disasm |
| **Capstone ARM** | 5.x | Reference disasm |

## 5. Third-Party Software, Services, Accounts & API Keys

Same as P5.4. **No new API keys.**

## 6. System Inventory — Have vs Need

All present from P5.4.

## 7. Features & Functions Delivered (Comprehensive)

### Crate extension `axiom-elf-lift::armv7`
- ELF32 parser
- ARMv7 + Thumb-2 disassembly
- ARM ↔ Thumb mode-switch tracking via low-bit-of-target heuristic
- AAPCS-VFP modeling: argument passing (R0–R3, S0–S15 / D0–D7, stack), return (R0–R1, S0 / D0)
- VFPv3 / NEON SIMD ops
- Soft-float fallback (Android `armeabi` legacy)
- Provenance tags

### Reuse
- CFG recovery + SSA construction reused from P5.4
- Diff harness reused from P5.4
- Round-trip pattern tests

### Tools
- `axiom-elf-lift-cli` extended with `--arch armv7`

### Reproducibility
- Bytewise output stable

### Fuzzing
- LibAFL fuzz on Thumb-2 + ARMv7 mix
- ≥ 24h per merge

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Function-level coverage on legacy-NDK corpus | ≥ 50 % | ≥ 80 % |
| Throughput (16-core) | ≥ 20 MB/s | ≥ 60 MB/s |
| AAPCS-VFP conformance | 100 % | 100 % |
| ARM ↔ Thumb mode switches handled | 100 % | 100 % |
| Diff agreement vs Ghidra (function-level) | ≥ 90 % | ≥ 98 % |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-elf-lift/
│       └── src/armv7/                # NEW
├── corpora/
│   └── legacy-ndk/                   # NEW: 100 ARMv7 NDK libs
├── docs/
│   └── elf-lift-armv7.md             # NEW
└── (tests + benches reuse P5.4 layout)
```

## 10. Standalone Output

ARMv7 lift available as a feature flag on `axiom-elf-lift`.

## 11. End-to-End Test

```bash
buck2 test //crates/axiom-elf-lift/tests:armv7

buck2 run //tools:axiom-elf-bench -- --arch armv7 --corpus legacy-ndk --threads 16
# Expect ≥ 50 % coverage, ≥ 20 MB/s
```

## 12. Exit Checklist

- [ ] Function-level coverage ≥ 50 % (HARD)
- [ ] Throughput ≥ 20 MB/s (HARD)
- [ ] AAPCS-VFP conformance 100 %
- [ ] Mode-switch handling 100 %
- [ ] Diff vs Ghidra ≥ 90 %
- [ ] Fuzz unresolved < 3 at merge
- [ ] Documentation `docs/elf-lift-armv7.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P5.6** | ARMv7 lifts available alongside ARM64 for JNI bridges |
| **P5.7** | ARMv7 binaries can match the common-library catalog |
| **P5.8** | Both 32-bit and 64-bit native code visible to joint analyzer |
