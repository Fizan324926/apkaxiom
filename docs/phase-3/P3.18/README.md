# P3.18 — Phase-3 E2E: Bench-10K + Repack-2K + Snapshots + Soak + Cross-Architecture

> All Phase 3 KPIs measured live, on real corpora, on reference hardware. Full L0–L5 pipeline. 7-day soak. Cross-arch verdicts identical.

**Parent plan:** [../README.md](../README.md) · **PHASE_GATES.md §7:** [../../PHASE_GATES.md#phase-3](../../PHASE_GATES.md#phase-3)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P3.18 |
| Owner(s) | All Phase 3 groups (G1, G2, G3, G4, G5, G6, G8, G13) |
| Duration | Weeks 18–22 |
| Critical-path | yes |
| Hard prerequisites | P3.9 (cross-APK), P3.17 (L5 unified) |

## 2. Goal & Scope

The full Phase-3 stack — verified L0 + L1 (with all Phase-2 dialects) + L2 (bundle) + L3 (forensics) + L4 (symbolic resolver, single + cross-APK + refinement) + L5 (BSH + LSH + bisim) — runs end-to-end. All Phase 3 KPIs measured live and reported.

### In scope
- E2E test harness extending `tests/e2e/phase2.rs` to `phase3.rs`
- Bench-10K + Repack-2K + Snapshot-1K corpora through full pipeline
- 7-day soak run on Stress-100K
- Cross-architecture (x86_64 ↔ ARM64) parity
- Reproducibility audit
- LSH index built over AndroZoo subset (1M vectors)
- Reachability/UNSAT/equiv certs all archived

### Out of scope
- Paper publication (P3.19)
- Phase 4 scope decisions (P3.20)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P3.9** | Cross-APK device-snapshot prototype |
| **P3.11** | UNKNOWN refinement |
| **P3.12** | DRAT certs |
| **P3.16** | Equiv certs |
| **P3.17** | L5 unified |

## 4. Required Tools, Libraries, and Languages

Same as P2.18 + the new Phase-3 components.

| Tool | Version | Purpose |
|---|---|---|
| **Full Phase-3 stack** | from prior sub-phases | The thing under measurement |
| **HDR Histogram, Pyroscope, Prometheus, Grafana** | from P1.18 | Dashboards + profiling |
| **Reference benchmark hardware** | EPYC 9354 | KPI measurement |
| **ARM64 reference** | Graviton3 / Hetzner ARM | Cross-arch parity |
| **MinIO** | latest | 1M-vector LSH index storage |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **AndroZoo** | corpus | **Free academic** | already provisioned | Bench-10K + Snapshot-1K + 1M-vector LSH index |
| **Hetzner / OVH / AWS Graviton3** | hardware | **Paid** ~ €150–500/mo | already provisioned | Reference hardware |
| **MinIO** | object store | **Free** OSS | already provisioned | LSH index archive |
| **MalwareBazaar / DREBIN / Repack-2K** | corpora | **Free** | already provisioned | Adversarial inputs |

**No new API keys.**

## 6. System Inventory — Have vs Need

### Already present
- ✅ Full Phase-3 stack
- ✅ All monitoring infrastructure

### Missing
- 1M-vector LSH index requires storage; ensure ≥ 100 GB available on the storage host (already in Phase 1.18 provisioning)

## 7. Features & Functions Delivered (Comprehensive)

### E2E test harness (`tests/e2e/phase3.rs`)
- Reads APK / AAB / device snapshot
- Runs L0 (streaming) → L1 → L2 (bundle) → L3 (forensics) → L4 (symbolic + refinement) → L5 (unified)
- Emits all certs: reachability witness, UNSAT cert, equiv cert
- Captures all signals + KPIs

### Per-KPI test cases
For each KPI in PHASE_GATES.md §7:
- Test case
- Pass threshold check
- Dashboard panel
- Alert rule (Prometheus)

### Cross-APK snapshot eval
- Snapshot-1K corpus (1000 realistic device snapshots)
- Run through L4 cross-APK
- Verify zero-day(s) reproducible

### LSH 1M-vector eval
- Build LSH index over 1M AndroZoo APKs
- Measure index size, build throughput, lookup latency

### Bisim eval on Repack-2K
- TP/FP measurement
- Per-bisim-cert verification

### 7-day soak
- Replay Stress-100K continuously for 7 days
- Crash rate measured (HARD: zero crashes)
- Memory growth tracked

### Cross-arch parity
- Same Bench-1K + Snapshot-100 on x86_64 + ARM64
- Verdicts byte-identical (HARD)
- Throughput parity within 25% (HARD)

### Performance dashboards (Grafana)
- Per-layer throughput / latency / memory
- L4 UNKNOWN rate
- L5 cert emit rate
- LSH lookup distribution
- Solver pool utilization

### Reports
- `reports/phase3-e2e-eval.md`
- `reports/phase3-snapshots-eval.md`
- `reports/phase3-repack-2k-eval.md`

## 8. KPIs (this sub-phase — all PHASE_GATES.md §7 hards)

| KPI | HARD | TARGET |
|---|---|---|
| L0–L5 sustained throughput, 16-core | ≥ 20 APKs/sec | ≥ 40 APKs/sec |
| L0–L5 p99 | ≤ 8 s | ≤ 5 s |
| Symbolic intent query p99 | ≤ 500 ms | ≤ 200 ms |
| BSH compute p99 | ≤ 30 ms | ≤ 10 ms |
| Bisim per-pair p99 | ≤ 2 s | ≤ 500 ms |
| LSH lookup p99 (1M index) | ≤ 200 ms | ≤ 50 ms |
| L4 UNKNOWN rate (post-refinement) | ≤ 25 % | ≤ 10 % |
| BSH collision rate (50K APKs) | < 0.1 % | < 0.01 % |
| BSH stability (ProGuard/R8/DexGuard) | ≥ 90 % | ≥ 98 % |
| Bisim TP on Repack-2K | ≥ 85 % | ≥ 95 % |
| Bisim FP on benign pairs | < 1 % | < 0.1 % |
| Solver timeout rate | < 5 % | < 1 % |
| Peak RSS per worker | ≤ 1 GB | ≤ 500 MB |
| LSH index size (1M) | ≤ 8 GB | ≤ 4 GB |
| 1→16 core efficiency | ≥ 60 % | ≥ 80 % |
| 7-day soak: 0 crashes | yes | yes |
| Cross-arch verdicts identical | 100 % | 100 % |
| ≥ 100 known intent-hijack vulnerabilities reproduced as proofs | yes | ≥ 200 |
| ≥ 1 zero-day intent-hijack from cross-APK | yes | ≥ 5 |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── tests/e2e/phase3.rs                   # NEW
├── reports/
│   ├── phase3-e2e-eval.md                # NEW
│   ├── phase3-snapshots-eval.md          # NEW
│   └── phase3-repack-2k-eval.md          # NEW
├── monitoring/grafana-dashboards/
│   ├── phase3-throughput.json
│   ├── phase3-latency.json
│   ├── phase3-l4.json
│   └── phase3-l5.json
└── corpus/
    ├── bench-10k/
    ├── repack-2k/
    ├── snapshot-1k/
    └── lsh-1m-index/                     # 1M-vector LSH index
```

## 10. Standalone Output

```bash
nix develop
buck2 test //tests/e2e:phase3 -- --corpus bench-10k --snapshots snapshot-1k --report reports/phase3-e2e-eval.md
# Dashboards live; reports written
```

## 11. End-to-End Test

```bash
buck2 test //tests/e2e:phase3-bench-10k
buck2 test //tests/e2e:phase3-repack-2k
buck2 test //tests/e2e:phase3-snapshots
buck2 test //tests/e2e:phase3-soak-7d
buck2 test //tests/e2e:phase3-cross-arch
# All HARD KPIs above must pass; ≥ 7 days green
```

## 12. Exit Checklist

All PHASE_GATES.md §7 hard gates ✅ for ≥ 7 consecutive days:

- [ ] L0–L5 sustained ≥ 20 APKs/sec on 16-core (HARD)
- [ ] L0–L5 p99 ≤ 8 s (HARD)
- [ ] Symbolic query p99 ≤ 500 ms (HARD)
- [ ] BSH compute p99 ≤ 30 ms (HARD)
- [ ] Bisim p99 ≤ 2 s (HARD)
- [ ] LSH p99 ≤ 200 ms on 1M (HARD)
- [ ] L4 UNKNOWN ≤ 25% (HARD)
- [ ] BSH collision < 0.1% (HARD)
- [ ] BSH stability ≥ 90% (HARD)
- [ ] Bisim TP ≥ 85%, FP < 1% (HARD)
- [ ] Solver timeout < 5% (HARD)
- [ ] Peak RSS ≤ 1 GB (HARD)
- [ ] LSH 1M-index ≤ 8 GB (HARD)
- [ ] 1→16 core ≥ 60% (HARD)
- [ ] 7-day soak: 0 crashes (HARD)
- [ ] Cross-arch verdicts identical (HARD)
- [ ] ≥ 100 known intent-hijacks reproduced as proofs (HARD)
- [ ] ≥ 1 zero-day from cross-APK reproduced (HARD)
- [ ] All cert types (reachability, UNSAT, equiv) emitted and verified

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P3.19** | Phase-3 numbers for paper |
| **P3.20** | Live KPI dashboard for gate review |
| **Phase 4 / G7** | All cert types ready for `.axc` envelope |
