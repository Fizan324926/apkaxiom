# P6.9 — G8 Stabilization: Extended Fuzzing + ≥ 10 CVEs Filed

> Extend fuzzing campaigns through Phase 6, classify disagreements, file ≥ 10 CVEs from accumulated AOSP / model-bug discoveries. Fuzzer 24/7 for the 90-day v1.0 window.

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P6.9 |
| Owner(s) | G8 |
| Duration | Weeks 1–22 |
| Critical-path | yes (≥ 10 CVEs is a v1.0 ship-gate item) |
| Hard prerequisites | P6.1 |

## 2. Goal & Scope

≥ 10 CVEs filed, fuzzer 24/7 across 5 AOSP harnesses + DEX + ELF + ML grammars. Auto-classification of disagreements (AOSP CVE / model bug / spec ambiguity) running fully unattended.

### In scope
- Continuous fuzz across 5+5 harnesses (AOSP A8/A11/A12/A13/A14/A15 + DEX + ELF + ML)
- Auto-classification pipeline
- CVE filing pipeline (AOSP / Google / vendor)
- Disagreement-queue SLAs
- Fuzz dashboards
- Coordinated-disclosure tracker

### Out of scope
- New harnesses
- New mutation strategies (deferred to v1.1)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P6.1** | Stabilization punch-list |
| **All Phase 1–5 G8 deliverables** | Continued |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **AFL++** | latest | Fuzz engine |
| **LibAFL** | latest | Custom fuzz framework |
| **Honggfuzz** | latest | Alt fuzzer |
| **GitHub Issues / OSV Schema** | latest | CVE filing |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **AOSP issuetracker** | account | **Free** | https://issuetracker.google.com | CVE submission |
| **MITRE CVE numbering authority** | service | **Free** | https://cveform.mitre.org | CVE assignment |
| **OSV.dev** | service | **Free** | https://osv.dev | CVE database |

**API keys required:** issuetracker auth, OSV submission token.

## 6. System Inventory — Have vs Need

All present.

## 7. Features & Functions Delivered (Comprehensive)

### Fuzz harnesses
- 5 AOSP versions × 3 grammars (ZIP / AXML / DEX) = 15 harnesses
- ELF grammar (ARM64 + ARMv7) = 2 harnesses
- TFLite grammar = 1 harness
- Total: 18 harnesses, 24/7 uptime

### Auto-classification
- 3-way: AOSP CVE / model bug / spec ambiguity
- ML-assisted classifier on disagreement signatures (BLAKE3 hash of structural diff)
- Per-class auto-route: AOSP CVE → CVE filing pipeline; model bug → group owner; spec ambiguity → ADR drafting

### CVE filing pipeline
- Coordinated disclosure: 90-day standard window
- Per-finding: vendor email + tracker entry + CVE-ID assignment + OSV submission
- v1.0 ship gate: ≥ 10 CVEs filed (HARD)

### SLAs
- Disagreement-queue SLA: < 10 unresolved at any time
- Acknowledgement SLA: vendor reply within 14 days
- Disclosure SLA: 90-day standard

### Dashboards
- Fuzz dashboard: per-harness uptime, throughput, queue
- CVE dashboard: open / acknowledged / fixed / disclosed
- Pyroscope continuous

### Documentation
- `docs/g8-stabilization.md`
- `docs/cve-pipeline.md`

## 8. KPIs (this sub-phase)

| KPI | HARD |
|---|---|
| ≥ 10 CVEs filed by v1.0 ship | yes |
| 5 AOSP × 3 + ELF×2 + TFLite = 18 harnesses 24/7 | yes |
| Fuzz uptime | ≥ 99 % |
| Auto-classification 3-way | green |
| Disagreement queue size | < 10 unresolved |
| Disclosure pipeline operational | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── fuzz/
│   ├── differential/                 # extended
│   ├── elf-grammar/
│   └── tflite-grammar/
├── tools/
│   ├── auto-classify/
│   └── cve-filer/
├── docs/
│   ├── g8-stabilization.md           # NEW
│   └── cve-pipeline.md               # NEW
└── (CVE tracker, OSV submissions)
```

## 10. Standalone Output

Fuzzer + CVEs are independent v1.0 artifacts.

## 11. End-to-End Test

```bash
buck2 run //tools:fuzz-dashboard -- --report
# Expect: 18 harnesses up, ≥ 99 % uptime, queue < 10

buck2 run //tools:cve-filer -- --list-filed
# Expect: ≥ 10 by v1.0
```

## 12. Exit Checklist

- [ ] ≥ 10 CVEs filed (HARD)
- [ ] 18 harnesses 24/7
- [ ] Fuzz uptime ≥ 99 %
- [ ] Auto-classification green
- [ ] Disagreement queue < 10
- [ ] Disclosure pipeline operational
- [ ] Documentation `docs/g8-stabilization.md` + `docs/cve-pipeline.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P6.16** | Fuzz feeds 50K eval edge cases |
| **P6.17** | CVE list reviewed by auditor |
| **P6.20** | "≥ 10 CVEs filed" item ✅ for ship gate |
