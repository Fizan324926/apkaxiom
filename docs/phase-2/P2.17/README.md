# P2.17 — Differential Fuzzer Scale: A12+A13 Harnesses + Nautilus Grammar-Aware

> Five AOSP harnesses live (A8, A11, A12, A13, A14). Nautilus grammar-aware mutation in production. ≥ 30 disagreements/week classified. First Phase-2 zero-day filed.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §12](../../../README.md#continuous) · [../../TECH_STACK.md §9](../../TECH_STACK.md#fuzzing)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P2.17 |
| Owner(s) | G8 |
| Duration | Weeks 4–18 (continuous after launch) |
| Critical-path | no, but required for Phase-2 KPI |
| Hard prerequisites | P1.13/P1.14 (3-harness baseline), P2.1 (A12/A13 archaeology) |

## 2. Goal & Scope

The fuzzing plant scales from 3 to 5 AOSP harnesses (adds A12 + A13) and adopts **Nautilus** grammar-aware mutation in production. Goal: ≥ 30 disagreements/week classified, ≥ 1 zero-day CVE candidate filed in Phase 2.

### In scope
- A12 + A13 Cuttlefish hermetic images
- 5-node KVM cluster (or 3-node with 2 harnesses each)
- Nautilus grammar-aware mutation engine in production
- Centipede orchestration across 5 nodes
- Cross-version disagreement reporting (any pair-wise of the 5)
- Bundle-era fuzzing (extends to AABs in P2.13's wake)

### Out of scope
- A15 (Phase 3 if released; otherwise Phase 4)
- Native-code fuzzing (Phase 5)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P1.13/P1.14** | 3-harness baseline (A8/A11/A14) |
| **P2.1** | AOSP A12/A13 sync ready |
| **P2.12** | Bundle resolver (for AAB-fuzzing extension) |

## 4. Required Tools, Libraries, and Languages

Same as P1.13/P1.14 plus:

| Tool | Version | Purpose |
|---|---|---|
| **Nautilus** | research code | Grammar-aware mutation in production |
| **Centipede** | from P1.13 | Distributed orchestration |
| **2 additional KVM nodes** | hardware | A12 + A13 harnesses |
| **MinIO** (from P1.14) | latest | Corpus archive |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **Nautilus** | grammar fuzzer | **Free** OSS | https://github.com/nautilus-fuzz/nautilus | Bochum |
| **Centipede** | orchestration | **Free** OSS | https://github.com/google/centipede | Google |
| **2 additional Hetzner KVM nodes** | hardware | **Paid** ~ €200–400/mo | https://www.hetzner.com | |
| **MITRE CNA / partner CNA** | CVE allocation | **Free** | https://cveform.mitre.org | Required for filing zero-days |
| **Google Android Security Rewards** | bug bounty | **Free** | https://www.google.com/about/appsecurity/android-rewards/ | Up to $1M for severe findings |

**Hardware:** add 2 more KVM-enabled nodes (or expand existing nodes' load).

## 6. System Inventory — Have vs Need

### Already present
- ✅ 3 KVM nodes from P1.13/P1.14
- ✅ Cuttlefish + Nyx + AFL++
- ✅ Centipede installed

### Missing — must procure
- ❌ 2 more KVM nodes for A12 + A13 (procurement)
- ❌ Nautilus integration into our grammar (existing APK grammar adapted)

```bash
# Add A12 + A13 Cuttlefish images on new nodes (analogous to P1.13/P1.14)
# Nautilus integration:
git clone https://github.com/nautilus-fuzz/nautilus
# Adapt our APK grammar to Nautilus's input format
buck2 build //fuzz/grammars:apk-nautilus
```

## 7. Features & Functions Delivered (Comprehensive)

### 5 Cuttlefish harnesses
- A8, A11, A12, A13, A14, all running 24/7 with ≥ 99 % uptime
- Hermetically built (Bazel sub-workspace)
- Centipede coordinator orchestrates inputs across all 5

### Nautilus grammar-aware mutation
- Custom APK grammar in Nautilus format
- Hierarchical mutation: ZIP envelope → AXML → ARSC → DEX
- Significantly higher coverage than pure byte-mutation (target: 5–10× discovery rate)

### Bundle-era fuzzing
- AAB grammar developed (extends APK grammar)
- Bundles tested via P2.12 resolver + AOSP install
- Captures bundle-era evasion attempts

### Disagreement classifier (extended from P1.14)
- New categories: `bundle-resolver-bug`, `bundle-fusion-edge-case`
- Cross-version pairings: every pair of (A8, A11, A12, A13, A14) reported separately

### CVE filing pipeline
- Confirmed zero-day → `tools/cve-draft` produces standardized advisory
- Submission to MITRE CNA or partner CNA
- Coordinated disclosure protocol with Google Android Security
- ≥ 1 zero-day filed by end of P2.17 (HARD per Phase-2 plan)

### Findings dashboard (Grafana)
- Per-AOSP-version coverage growth
- Per-pair disagreement counts
- Classifier output distribution
- Top-N most-frequent disagreement signatures

### Documentation
- `docs/differential-fuzzer.md` extended with Nautilus + 5-harness operations
- `docs/cve-filing-runbook.md` covers coordinated disclosure

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| 5 harnesses live, ≥ 99 % uptime each | yes | ≥ 99.9 % |
| Disagreements/week classified | ≥ 30 | ≥ 100 |
| Cross-version disagreements found | ≥ 5 distinct pairs | every pair |
| Classifier precision | ≥ 80 % | ≥ 95 % |
| Zero-day CVE candidate filed | ≥ 1 | ≥ 5 |
| Nautilus coverage growth vs AFL++ baseline | ≥ 3× | ≥ 10× |
| Bundle-era disagreement detection | ≥ 1 found | ≥ 5 |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── fuzz/
│   ├── orchestrator/                    # extended for 5 harnesses
│   ├── grammars/
│   │   ├── apk-nautilus.json            # NEW — Nautilus grammar
│   │   └── aab-nautilus.json            # NEW — bundle grammar
│   ├── classifier/                      # extended categories
│   ├── findings/
│   │   └── cve-drafts/                  # NEW
│   └── dashboards/
│       └── grafana-five-harness.json
├── external/aosp/
│   ├── cuttlefish-A12/                  # NEW
│   └── cuttlefish-A13/                  # NEW
├── tools/cve-draft/                     # NEW — CVE advisory drafter
└── docs/
    ├── differential-fuzzer.md           # extended
    └── cve-filing-runbook.md            # NEW
```

## 10. Standalone Output

```bash
# Across 5 KVM nodes:
buck2 run //fuzz/orchestrator:centipede-coordinator -- --harnesses A8,A11,A12,A13,A14
# Continuous; dashboard at http://orchestrator:3000/d/five-harness
```

## 11. End-to-End Test

Sustained 4-week run:
- ≥ 30 disagreements/week classified consistently (HARD)
- ≥ 1 cross-version disagreement reproducible from each pairing tested (HARD)
- ≥ 1 zero-day CVE candidate drafted and filed (HARD)
- All 5 harnesses ≥ 99 % uptime (HARD)

## 12. Exit Checklist

- [ ] A12 + A13 Cuttlefish harnesses live
- [ ] All 5 harnesses ≥ 99 % uptime over 4 weeks (HARD)
- [ ] Nautilus grammar-aware mutation in production
- [ ] APK + AAB grammars built
- [ ] ≥ 30 disagreements/week classified (HARD)
- [ ] Cross-version disagreements found across all pairs (HARD)
- [ ] ≥ 1 zero-day CVE candidate filed (HARD)
- [ ] Bundle-era disagreement detected
- [ ] CVE filing pipeline documented + tested
- [ ] `docs/cve-filing-runbook.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P2.18** | Differential fuzzer findings inform Phase-2 KPI gate |
| **P2.20** | Phase-2 ship gate cites zero-day count |
| **Phase 3 / G8** | Continues to scale; eventually hits 7+ AOSP versions |
