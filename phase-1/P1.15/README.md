# P1.15 — apk-info v1.0 AXIOM-IR-v0.1 Emitter

> apk-info parses; AXIOM-IR-v0.1 is what comes out. Round-trip on Bench-1K: parse → IR → re-encode ≥ 95% byte-identical.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../README.md §22](../../README.md#apkinfo-integration)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P1.15 |
| Owner(s) | G2 + G3 |
| Duration | Weeks 12–17 |
| Critical-path | yes |
| Hard prerequisites | P1.4 (AXIOM-IR-v0.1 frozen), P1.10 (commit chain), P1.7 (streaming parser) |

## 2. Goal & Scope

`axiom-l1-rs` emits AXIOM-IR-v0.1 (manifest dialect + resource dialect) for every parsed APK. Round-trip test: parse → IR → re-encode → byte-identical original. ≥ 95% on Bench-1K.

### In scope
- `axiom_l1_rs::ir` module emitting both dialects
- Round-trip test on Bench-1K
- IR output deterministic (same APK → same IR bytes)
- IR → JSON debug serialization for human inspection

### Out of scope
- DEX dialect emission (Phase 2)
- Lossless re-encoding (some manifest edge cases are intentionally lossy in the dialect)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P1.4** | AXIOM-IR-v0.1 frozen spec + reference Rust crate |
| **P1.7** | Streaming parser ParseEvent stream as input |
| **P1.10** | Commit chain hooks at IR-emit points |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Rust** | 1.95 | Implementation |
| **axiom-ir** crate | from P1.4 | Target IR types |
| **serde + rkyv** | from P1.4 | IR serialization |
| **insta** | 1.40+ | Snapshot testing for IR output |
| **proptest** | 1.5+ | Property-based round-trip testing |

## 5. Third-Party Software, Services, Accounts & API Keys

**No external dependencies introduced. All in-house tooling and crates.io OSS.**

## 6. System Inventory — Have vs Need

### Already present
- ✅ Rust + axiom-ir + axiom-l1-rs crates from prior sub-phases

### Missing
- Nothing system-level; just add `insta = "1.40"` to Cargo dev-dependencies.

## 7. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-l1-rs/
│       └── src/
│           └── ir/
│               ├── mod.rs               # NEW
│               ├── manifest.rs           # NEW — manifest dialect emitter
│               ├── resource.rs           # NEW — resource dialect emitter
│               └── reencode.rs           # NEW — round-trip back to bytes
├── tests/
│   └── ir-roundtrip/
│       ├── BUCK
│       ├── src/main.rs
│       └── snapshots/                    # insta snapshots for 100 reference APKs
└── docs/
    └── ir-emitter.md                     # NEW
```

## 8. Standalone Output

```bash
buck2 build //crates/axiom-l1-rs --release
buck2 test //tests/ir-roundtrip
# Output: "964/1000 byte-identical, 36 documented exceptions" (≥95%)
```

## 9. End-to-End Test

For every APK in Bench-1K:
1. Parse to AXIOM-IR-v0.1.
2. Re-encode IR → bytes.
3. Compare to original.
4. ≥ 95% must be byte-identical (HARD).
5. Documented exceptions classified by dialect-lossiness reason.

```bash
buck2 test //tests/ir-roundtrip:bench-1k
# Reports: byte-identity rate, IR emission overhead, IR determinism
```

## 10. Exit Checklist

- [ ] Manifest dialect emitter lands
- [ ] Resource dialect emitter lands
- [ ] Round-trip ≥ 95% byte-identical on Bench-1K (HARD)
- [ ] IR emission overhead ≤ 15% throughput hit vs no-IR (HARD)
- [ ] IR output deterministic — same APK twice → bit-identical IR
- [ ] insta snapshots for 100 reference APKs land in repo
- [ ] `docs/ir-emitter.md` published

## 11. Hand-Off

| Consumed by | What they need |
|---|---|
| **P1.18** | IR emission is part of E2E pipeline measured |
| **Phase 2 / G3 + G4** | IR is the input to bundle resolver and forensic passes |
| **Phase 3 / G5** | Symbolic resolver lifts manifest dialect into AXIOM-IR-symbolic |
