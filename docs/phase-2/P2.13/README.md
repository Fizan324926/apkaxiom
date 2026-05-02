# P2.13 — Bundle Differential Testing vs AOSP Installer

> The end-to-end correctness check. Our bundle resolver vs AOSP `pm install` on Cuttlefish, across A11/A12/A13/A14, on every AAB in Bundles-5K. ≥ 99.9% agreement (HARD).

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §7](../../../README.md#layer-2)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P2.13 |
| Owner(s) | G3 + G8 |
| Duration | Weeks 14–18 |
| Critical-path | yes |
| Hard prerequisites | P2.12 (resolver), P1.13/P1.14/P2.17 (Cuttlefish harnesses) |

## 2. Goal & Scope

For every AAB in Bundles-5K, compare the bundle resolver's BehaviorSet against what Cuttlefish A14 (and A11, A12, A13) actually installs. Disagreements logged and classified.

### In scope
- Cuttlefish-driven differential harness (extends P1.13/P1.14)
- Per-AOSP-version install behavior comparison
- Dynamic-feature install scenarios tested (install-time + on-demand activation)
- Disagreement classification (resolver bug / AOSP bug / spec ambiguity)
- Cross-version disagreement reporting

### Out of scope
- Forensic passes (already running by now)
- Phase-3 symbolic reasoning

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P2.12** | Bundle resolver |
| **P1.13/P1.14** | Cuttlefish A8/A11/A14 |
| **P2.17** | Cuttlefish A12/A13 |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Cuttlefish A11/A12/A13/A14 images** | from P1.13, P2.17 | Reference |
| **adb** | from Android SDK Build Tools | Install + introspection |
| **bundletool install-apks** | from P2.10 | Driver for installing AABs on Cuttlefish |
| **Rust** | 1.95 | Differential harness |
| **fjall** | 0.5+ | Persistent disagreement archive (from P1.13) |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **Cuttlefish images** | reference | **Free** OSS | already on KVM nodes | |
| **adb** | client | **Free** | already installed | |
| **bundletool** | tool | **Free** | already installed | |
| **Hetzner KVM nodes (3+ for parallel testing)** | hardware | **Paid** ~ €300–600/mo total | already provisioned | |

**No new API keys.**

## 6. System Inventory — Have vs Need

### Already present (after prior sub-phases)
- ✅ Cuttlefish + KVM nodes
- ✅ Bundle resolver + AAB parser
- ✅ adb (must install — see install command)

### Missing — must install
- ❌ **adb** if not present from Android SDK — `sudo apt-get install -y adb`

```bash
sudo apt-get install -y adb android-tools-adb
```

## 7. Features & Functions Delivered (Comprehensive)

### Differential harness
- `tests/bundle-vs-aosp/` driver
- For each AAB:
  1. Compute BehaviorSet via P2.12 resolver
  2. For each feasible config in BehaviorSet:
     - Boot Cuttlefish image with that config
     - `bundletool install-apks --bundle=<aab> --device=<config>`
     - Verify install succeeded (or matched our expected reject)
     - `adb shell pm dump <package>` — extract installed components
     - Diff against BehaviorSet[config]
- Logs all disagreements

### Cross-version testing
- Same AAB tested across A11, A12, A13, A14
- Cross-version disagreements (e.g., installs on A14 but not A11) flagged separately

### Dynamic-feature scenarios tested
- Install-time delivery — must be present at first install
- On-demand delivery — install base, then `pm install-streaming` the feature module
- Fast-follow — install with deferred feature module install

### Disagreement classification
- Reuses P1.14 classifier
- New categories: `bundle-resolver-bug`, `aosp-installer-quirk`, `bundle-format-ambiguity`

### Findings dashboard
- Grafana view: per-AOSP-version agreement rate, per-config breakdown, top disagreement classes
- Findings persisted in fjall LSM

### Coverage report
- For Bundles-5K: per-AAB pass/fail, per-config pass/fail, aggregate agreement %
- Adversarial bundle samples (bundle-era malware) flagged separately

### Documentation
- `docs/bundle-differential.md`

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Bundles-5K agreement vs AOSP install | ≥ 99.9 % | 100 % |
| Cross-version disagreements found | ≥ 5 | ≥ 30 |
| Dynamic-feature scenarios passing all three delivery types | 100 % | 100 % |
| Differential harness uptime | ≥ 99 % over 14 days | ≥ 99.9 % |
| Disagreement classification accuracy | ≥ 80 % | ≥ 95 % |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── tests/bundle-vs-aosp/
│   ├── Cargo.toml
│   ├── BUCK
│   └── src/
│       ├── main.rs
│       ├── cuttlefish_driver.rs
│       └── differ.rs
├── reports/bundle-differential.md         # NEW — eval results
├── monitoring/grafana-bundle-diff.json    # NEW
└── docs/
    └── bundle-differential.md             # NEW
```

## 10. Standalone Output

```bash
buck2 test //tests/bundle-vs-aosp:bundles-5k
# Reports: per-AOSP-version agreement, top disagreements, classification breakdown
```

## 11. End-to-End Test

```bash
buck2 test //tests/bundle-vs-aosp:bundles-5k -- --aosp-versions A11,A12,A13,A14
# - ≥ 99.9% agreement on Bundles-5K (HARD)
# - All dynamic-feature delivery types pass (HARD)
# - Cross-version disagreements found and reproducible
```

## 12. Exit Checklist

- [ ] Differential harness operational across A11/A12/A13/A14
- [ ] Bundles-5K coverage 100 %
- [ ] Agreement ≥ 99.9 % (HARD)
- [ ] All three dynamic-feature delivery types pass (HARD)
- [ ] Cross-version disagreements found and reproduced
- [ ] Disagreement classification ≥ 80 % accurate
- [ ] Findings dashboard live
- [ ] `docs/bundle-differential.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P2.18** | Bundle differential signals fed into E2E phase |
| **P2.19** | Findings cited in paper |
| **Phase 3 / G5** | Per-config behaviors validated against AOSP |
| **Phase 6 audit** | Differential coverage as evidence |
