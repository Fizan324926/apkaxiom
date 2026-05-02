# P6.5 — G4 Stabilization: Forensic FP Rate < 0.5 % on 50K-APK Benign Corpus

> Drive forensic-pass false-positive rate below 0.5 % on the public 50K benign corpus through small refinements (no new passes). Reference-corpus expansion for AXML provenance fingerprints.

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P6.5 |
| Owner(s) | G4 |
| Duration | Weeks 1–14 |
| Critical-path | yes |
| Hard prerequisites | P6.1 |

## 2. Goal & Scope

Drive the combined forensic FP rate (Shadow Stack + AXML provenance + Negative-Space) below 0.5 % on the benign 50K subset. Refinements only — no new passes.

### In scope
- Threshold tuning per pass
- AXML provenance reference-corpus expansion (more compilers, more versions)
- Negative-Space anomaly model retrained on 50K benign
- Shadow Stack edge cases catalogued + suppressed (where benign)
- FP triage + per-class fix
- Cross-confirmation across passes (a pass alone shouldn't fire as cert claim)

### Out of scope
- New forensic passes (deferred to v1.1)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P6.1** | Stabilization punch-list |
| **APKAXIOM-Eval-50K** | Benign subset for FP measurement |
| **All Phase 2 G4 deliverables** | Continued |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **All Phase 2 G4 stack** | (existing) | Continued |
| **Pyroscope / cargo flamegraph** | (existing) | Perf re-confirm |

## 5. Third-Party Software, Services, Accounts & API Keys

All free OSS.

**No new API keys.**

## 6. System Inventory — Have vs Need

All present.

## 7. Features & Functions Delivered (Comprehensive)

### Threshold tuning
- Per-pass thresholds tuned via grid search on 5K validation set (held out from 50K)
- Per-Android-version threshold variants

### Reference-corpus expansion
- AXML provenance: ≥ 1000 reference APKs across compilers (aapt2, R8, ProGuard, DexGuard, etc.)
- Each reference: SHA-256 + tagged compiler version + build flags

### Negative-Space model retrain
- Retrained on 50K benign labels
- Anomaly threshold tuned for FP < 0.2 % per pass

### Shadow Stack edge case suppression
- Cataloged: `<provider>` declarations from manifest-only providers, system app receivers, etc.
- Suppression rules per edge case
- Each rule documented with rationale

### Cross-confirmation
- Cert L6 emits a forensic finding only if ≥ 2 passes agree
- Single-pass findings → "internal signal," not a cert claim

### Documentation
- `docs/forensic-stabilization.md`

## 8. KPIs (this sub-phase)

| KPI | HARD |
|---|---|
| Combined forensic FP rate on benign 50K | < 0.5 % |
| Per-pass FP rate | < 0.3 % |
| AXML provenance reference corpus | ≥ 1000 APKs |
| Negative-Space model retrained on 50K | yes |
| Cross-confirmation: 2-pass-agree rule deployed | yes |
| Per-edge-case suppression rules documented | yes |
| Performance: per-pass throughput re-confirmed (≥ 300 APKs/sec) | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-l3-*/                   # tuned
├── corpora/
│   └── axml-provenance-1k/           # NEW: expanded
├── models/
│   └── negspace-50k.model            # NEW
└── docs/
    └── forensic-stabilization.md     # NEW
```

## 10. Standalone Output

Reference corpus + retrained model usable beyond APKAXIOM.

## 11. End-to-End Test

```bash
buck2 run //tools:axiom-l3-bench -- --corpus eval-50k-benign-only --report fp-rate
# Expect: < 0.5 % combined
```

## 12. Exit Checklist

- [ ] Combined FP < 0.5 % on benign 50K (HARD)
- [ ] Per-pass FP < 0.3 % (HARD)
- [ ] AXML reference corpus ≥ 1000 APKs (HARD)
- [ ] Negative-Space model retrained
- [ ] Cross-confirmation rule deployed
- [ ] Suppression rules documented
- [ ] Performance per pass ≥ 300 APKs/sec
- [ ] Documentation `docs/forensic-stabilization.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P6.16** | Stabilized passes for 50K eval |
| **P6.17** | FP rate evidence for auditor |
| **P6.20** | "Forensic FP rate < 0.5 %" item ✅ for ship gate |
