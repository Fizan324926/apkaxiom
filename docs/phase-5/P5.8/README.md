# P5.8 — Joint Java + Native Intent Analyzer

> Extend the symbolic intent resolver (L4) to follow control + data across JNI boundaries into native code. Produce ≥ 1 cross-language vulnerability discovery that pure-Java analyzers miss.

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P5.8 |
| Owner(s) | G5 + G9 |
| Duration | Weeks 12–18 |
| Critical-path | yes |
| Hard prerequisites | P5.6, P5.7 |

## 2. Goal & Scope

A unified joint-language intent resolver that reasons over Java DEX SSA + native ELF SSA + JNI boundary nodes as one control-flow + data-flow graph. Outputs the same shape of evidence (reachability proof, UNSAT cert, UNKNOWN-with-marker) as the Java-only L4 from Phase 3.

### In scope
- Control-flow extension: cross-dialect call op (`crossdial.invoke`) traversed by L4
- Data-flow extension: JNI boundary marshaling rules + provenance flow
- Abstract-domain extension: pointer-provenance + native taint (re-uses Phase-3 abstraction-domain library)
- Native-side intent dispatch: native code calling `startActivity` / `sendBroadcast` etc. via JNI back into Java
- Common-library catalog (P5.7) used to skip re-deriving summarized functions
- ≥ 1 zero-day cross-language vulnerability discovery (HARD)
- p99 ≤ 15 s (HARD; ≤ 5 s TARGET)
- Native intent dispatch resolution rate ≥ 50 % (HARD; ≥ 80 % TARGET)
- JNI boundary modeling coverage ≥ 75 % common patterns (continued from P5.6)

### Out of scope
- Pure-static native intent resolution (handled here as best-effort; UNKNOWNs flow to dynamic confirmation in P5.13)
- Lean theorems for joint analyzer (P5.9)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P5.6** | JNI bridge model |
| **P5.7** | Native common-library catalog |
| **P5.3 / P5.4** | DEX + ELF lift |
| **P3.8** | Single-APK symbolic resolver (Java-only) |
| **P3.10** | Abstract-domain library |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **cvc5** | 1.x latest (P3.6) | Symbolic backend |
| **Spacer / Eldarica** | matching (P3.7) | CHC backend |
| **Bitwuzla / Yices2** | latest | Bitvector / linear backends |
| **MLIR** | 19 | IR |
| **Rust** | 1.84+ | Implementation |

## 5. Third-Party Software, Services, Accounts & API Keys

All vendored / installed in P5.1 + P3.6.

**No new API keys.**

## 6. System Inventory — Have vs Need

All present.

## 7. Features & Functions Delivered (Comprehensive)

### Crate `axiom-l4-joint`
- Cross-dialect CFG construction
- Cross-language SSA traversal
- Native-side intent-dispatch detection: `startActivity` / `sendBroadcast` / `sendOrderedBroadcast` / `bindService` / `startService` / `IntentSender.sendIntent` invoked from native via JNI
- JNI marshaling unrolling
- Abstract-domain extension to native pointer-provenance
- Catalog-summary substitution
- Per-function summarization for un-cataloged native code
- Joint UNKNOWN classification (Java-side / Native-side / Boundary)

### Detection rules library
- Native-side hidden intent dispatch (used to bypass Java-level static analyzers)
- Native-side serialization tampering (e.g., Parcel forging from native)
- Native-side cleartext network destinations
- Native-side device-identifier reads (`ro.serialno`, etc.)

### Tools
- `axiom-l4-joint-cli`
- `axiom-l4-joint-bench`

### Discovery target
- ≥ 1 zero-day cross-language vulnerability (HARD) — coordinated disclosure
- ≥ 5 zero-day candidates surfaced in evaluation (TARGET)

### Performance
- p99 ≤ 15 s end-to-end (HARD; 5 s TARGET) on Bench-1K
- Per-function summary cache to amortize repeated native funcs

### Reproducibility
- Deterministic resolution given same input + same solver version

### Differential
- Compare against Java-only L4: must be a strict superset of decisions
- Joint UNKNOWN rate must not exceed Java-only UNKNOWN rate by more than 5 percentage points

### Documentation
- `docs/joint-analyzer.md`

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Joint analyzer p99 | ≤ 15 s | ≤ 5 s |
| Native intent dispatch resolution | ≥ 50 % | ≥ 80 % |
| JNI boundary coverage (re-confirm) | ≥ 75 % | ≥ 95 % |
| ≥ 1 zero-day cross-language vulnerability discovered | yes | yes |
| Joint UNKNOWN rate over benign 5K | ≤ 25 % + 5 pp Java-only | ≤ 10 % + 3 pp |
| Reproducibility | 100 % | 100 % |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-l4-joint/
│       ├── src/
│       │   ├── crossdial.rs
│       │   ├── native_dispatch.rs
│       │   ├── catalog_subst.rs
│       │   ├── summarize.rs
│       │   └── unknown_class.rs
│       └── tests/
├── tools/
│   ├── axiom-l4-joint-cli
│   └── axiom-l4-joint-bench
└── docs/
    └── joint-analyzer.md             # NEW
```

## 10. Standalone Output

The joint analyzer is a research artifact in itself; published with the Phase-5 paper.

## 11. End-to-End Test

```bash
buck2 test //crates/axiom-l4-joint:...
buck2 run //tools:axiom-l4-joint-bench -- --corpus bench-1k --threads 16
# Expect p99 ≤ 15 s, native dispatch ≥ 50 %

# Replay known cross-language vuln corpus
buck2 run //tools:axiom-l4-joint-cli -- --corpus malware-1k --classify
```

## 12. Exit Checklist

- [ ] p99 ≤ 15 s (HARD)
- [ ] Native dispatch resolution ≥ 50 % (HARD)
- [ ] JNI coverage ≥ 75 % (HARD)
- [ ] ≥ 1 zero-day discovered + disclosed (HARD)
- [ ] Joint UNKNOWN rate within 5 pp of Java-only
- [ ] Reproducibility 100 %
- [ ] Differential vs Java-only L4: strict superset
- [ ] Documentation `docs/joint-analyzer.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P5.13** | UNKNOWNs from joint analyzer feed dynamic confirmation |
| **P5.18** | Joint analyzer in E2E pipeline |
| **P5.19** | Zero-day disclosure case study for paper |
| **L6 cert** | New cert subtype for cross-language vulnerabilities |
