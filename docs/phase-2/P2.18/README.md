# P2.18 — Phase-2 E2E: Bench-10K Rerun + Bundles-5K + 24h Soak + Cross-Architecture

> All Phase 2 KPIs measured live, on real corpora, on reference hardware. Bundle-era pipeline through L0–L3. 24h soak: zero crashes. Cross-arch verdicts identical.

**Parent plan:** [../README.md](../README.md) · **PHASE_GATES.md §6:** [../../PHASE_GATES.md#phase-2](../../PHASE_GATES.md#phase-2)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P2.18 |
| Owner(s) | All Phase 2 groups (G1, G2, G3, G4, G8, G13) |
| Duration | Weeks 18–22 |
| Critical-path | yes |
| Hard prerequisites | P2.12 (resolver), P2.16 (final forensic pass), P2.17 (fuzzer at scale) |

## 2. Goal & Scope

The full Phase-2 stack — verified L0 + L1 (with AXML + ARSC + DEX) + AXIOM-IR-v0.2 emission + bundle resolver + L3 forensics — runs end-to-end. All Phase 2 KPIs from PHASE_GATES.md §6 measured live and reported on dashboards.

### In scope
- E2E test harness extending `tests/e2e/phase1.rs` to `phase2.rs`
- Bundle handling tested
- All 3 forensic passes engaged
- Performance dashboards updated
- Reproducibility audit on Bench-1K + Bundles-5K
- 24h soak run on Stress-100K (or Bench-10K replay 10×)
- Cross-architecture (x86_64 ↔ ARM64) parity

### Out of scope
- AndroZoo paper publication (P2.19)
- Phase 3 scope decisions (P2.20)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P2.12** | Bundle resolver |
| **P2.13** | Bundle differential signals |
| **P2.14, P2.15, P2.16** | Forensic passes operational |
| **P2.17** | Differential fuzzer at scale (signals continuous) |

## 4. Required Tools, Libraries, and Languages

Same as P1.18 + the new Phase-2 components.

| Tool | Version | Purpose |
|---|---|---|
| **Full Phase-2 stack** | from prior sub-phases | The thing under measurement |
| **HDR Histogram, Pyroscope, Prometheus, Grafana** | from P1.18 | Dashboards + profiling |
| **Reference benchmark hardware** | EPYC 9354 (from P1.18) | KPI measurement |
| **ARM64 reference (Graviton3 or Hetzner ARM)** | from P1.18 | Cross-arch parity |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **AndroZoo** | corpus | **Free academic** | already provisioned | Bench-10K + Bundles-5K |
| **Hetzner / OVH benchmark hosts** | hardware | **Paid** ~ €150–500/mo | already provisioned | EPYC + ARM64 |
| **MinIO** | object store | **Free** OSS self-host | already provisioned | Stress-100K archive |
| **VirusTotal** | optional ground truth | Free tier; **paid** | from P1.3 | Spot-check forensic findings |

**No new API keys.**

## 6. System Inventory — Have vs Need

### Already present
- ✅ Full Phase-2 stack
- ✅ All Phase-1 monitoring infrastructure

### Missing
- May need to expand Bench-10K and Bundles-5K corpora if not already curated.

```bash
buck2 run //tools/corpus-curate -- --target Bench-10K --refresh
buck2 run //tools/corpus-curate -- --target Bundles-5K --refresh
buck2 run //tools/corpus-curate -- --target Stress-100K --refresh
```

## 7. Features & Functions Delivered (Comprehensive)

### E2E test harness (`tests/e2e/phase2.rs`)
- Reads APK or AAB
- Runs L0 (streaming) → L1 (verified parsers) → L2 (bundle resolver) → L3 (3 forensic passes)
- Emits AXIOM-IR-v0.2
- Captures all signals (parser outputs, bundle resolutions, forensic findings, fuzzer disagreements)
- Per-stage timing reported
- Memory + CPU instrumented

### Per-KPI test cases
For each KPI in PHASE_GATES.md §6:
- Test case
- Pass threshold check
- Dashboard panel
- Alert rule (Prometheus)

### Bundle-aware testing
- Bundles-5K driven through full pipeline
- Per-config behavior measured
- Dynamic-feature-module discovery rate captured
- BehaviorSet memory representation measured

### Forensic-pass integration
- All 3 passes run on every input
- Combined FP rate measured
- Per-pass throughput measured

### 24h soak
- Replay Stress-100K continuously for 24 hours
- Crash rate measured (HARD: zero crashes)
- Memory growth tracked (HARD: ≤ 2 MB/hour)

### Cross-arch parity
- Same Bench-1K and Bundles-1K subset on x86_64 + ARM64 (Hetzner ARM or AWS Graviton3)
- Verdicts byte-identical (HARD)
- Throughput parity within 25% (HARD)

### Performance dashboards (Grafana)
- Per-layer throughput
- Per-layer p50/p95/p99 latency
- Memory + CPU usage curves
- Forensic-pass FP/recall live
- Differential-fuzzer findings in/out flow
- Reproducibility CI gate status

### Reports
- `reports/phase2-e2e-eval.md` — full numerical breakdown vs PHASE_GATES.md §6
- `reports/phase2-bundles-eval.md` — Bundles-5K specific results
- Reproducibility report — bit-identical Merkle + IR + verdict on Bench-1K twice

## 8. KPIs (this sub-phase — all PHASE_GATES.md §6 hards)

| KPI | HARD | TARGET |
|---|---|---|
| L0–L3 sustained throughput, 16-core | ≥ 150 APKs/sec | ≥ 250 APKs/sec |
| L0–L3 p99 latency | ≤ 800 ms | ≤ 500 ms |
| Bundle resolution overhead | ≤ 60 % | ≤ 30 % |
| Bundle resolution p99 (20-split) | ≤ 3 s | ≤ 1 s |
| Forensic pass throughput each | ≥ 300 APKs/sec | ≥ 500 APKs/sec |
| Per forensic pass p99 | ≤ 80 ms | ≤ 30 ms |
| Peak RSS per worker | ≤ 300 MB | ≤ 200 MB |
| BehaviorSet memory representation | ≤ 2.5× raw | ≤ 1.8× |
| Combined forensic FP on benign | < 12 % | < 5 % |
| Differential test vs AOSP installer | ≥ 99.9 % | 100 % |
| 1→16 core efficiency | ≥ 70 % | ≥ 85 % |
| 24h soak: 0 crashes | yes | yes |
| Cross-arch verdicts identical | 100 % | 100 % |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── tests/e2e/phase2.rs                 # NEW
├── reports/
│   ├── phase2-e2e-eval.md              # NEW
│   └── phase2-bundles-eval.md          # NEW
├── monitoring/grafana-dashboards/
│   ├── phase2-throughput.json
│   ├── phase2-latency.json
│   ├── phase2-bundle.json
│   └── phase2-forensics.json
└── corpus/                             # corpora updated
    ├── bench-10k/
    ├── bundles-5k/
    └── stress-100k/
```

## 10. Standalone Output

```bash
# On reference benchmark host
nix develop
buck2 test //tests/e2e:phase2 -- --corpus bench-10k --bundles bundles-5k --report reports/phase2-e2e-eval.md
# Dashboards live at http://<host>:3000/d/phase2
```

## 11. End-to-End Test

```bash
buck2 test //tests/e2e:phase2-bench-10k
buck2 test //tests/e2e:phase2-bundles-5k
buck2 test //tests/e2e:phase2-soak-24h
buck2 test //tests/e2e:phase2-cross-arch
# All HARD KPIs above must pass; ≥ 7 days green for sub-phase to close
```

## 12. Exit Checklist

All PHASE_GATES.md §6 hard gates ✅ for ≥ 7 consecutive days:

- [ ] Sustained ≥ 150 APKs/sec on 16-core (HARD)
- [ ] L0–L3 p99 ≤ 800 ms (HARD)
- [ ] Bundle resolution overhead ≤ 60 % (HARD)
- [ ] Bundle resolution p99 ≤ 3 s for 20-split (HARD)
- [ ] All forensic-pass throughputs ≥ 300 APKs/sec (HARD)
- [ ] Combined forensic FP < 12 % (HARD)
- [ ] Differential vs AOSP install ≥ 99.9 % (HARD)
- [ ] Peak RSS ≤ 300 MB (HARD)
- [ ] BehaviorSet memory ≤ 2.5× raw (HARD)
- [ ] 1→16 core efficiency ≥ 70 % (HARD)
- [ ] CI byte-identical 100 % (carry-forward HARD)
- [ ] 24h soak: zero crashes (HARD)
- [ ] Cross-arch verdicts identical (HARD)
- [ ] Reproducibility 100 % on Bench-1K + Bundles-1K
- [ ] All Grafana dashboards live and paged

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P2.19** | Phase-2 numbers for paper |
| **P2.20** | Live KPI dashboard for gate review meeting |
| **Phase 3 / G5** | Bench-10K + Bundles-5K corpora for symbolic-resolver eval |
