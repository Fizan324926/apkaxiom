# P5.18 — Phase-5 E2E: Full Pipeline + Native + Dynamic + ML + Soak + Cross-Arch

> Run the complete L0–L6 pipeline including native + dynamic + ML on Bench-10K, NDK-100, planted-backdoor zoo, and Repack-2K. 7-day soak. Cross-arch parity. All hard KPIs from PHASE_GATES.md §9 sustained green for ≥ 7 days.

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P5.18 |
| Owner(s) | All groups |
| Duration | Weeks 18–22 |
| Critical-path | yes (gates Phase 5 → Phase 6) |
| Hard prerequisites | P5.5–P5.17 |

## 2. Goal & Scope

A 7-day continuous run of the full Phase-5 pipeline, instrumented end-to-end, on a corpus that exercises every layer + native + dynamic + ML. Must hit all PHASE_GATES.md §9 hard KPIs sustained.

### In scope
- E2E pipeline orchestration (Buck2 + airflow-equivalent)
- Bench-10K (Java side) + NDK-100 (native side) + planted-backdoor zoo (ML side) + Repack-2K (equivalence regression)
- Cross-arch on x86_64 + ARM64
- 7-day soak: continuous load
- Dynamic-bridge enabled in research / pilot mode
- Live KPI dashboards
- Carry-forward debt log
- Reproducibility audit: bytewise certs across runs / arches

### Out of scope
- Paper writing (P5.19)
- Phase 6 planning (P5.20)

## 3. Hard Dependencies on Prior Sub-Phases

All P5.5 through P5.17 must be exit-checked first. Plus P3, P4 stack must remain green.

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Buck2** | (existing) | Build orchestration |
| **Airflow / Dagster** | latest stable | Pipeline orchestration |
| **Pyroscope + Prometheus + Grafana** | (existing) | Observability |
| **Sentry** | (existing) | Error tracking |
| **k6 / Vegeta / wrk2** | latest | Load gen |

## 5. Third-Party Software, Services, Accounts & API Keys

All carried over.

**No new API keys.**

## 6. System Inventory — Have vs Need

All present.

## 7. Features & Functions Delivered (Comprehensive)

### E2E orchestration
- Pipeline DAG: ingest → L0 → L1 → L2 → L3 → L4 (joint) → L5 → L6 → cert emit → verifier → bug-bounty pilot
- Per-stage instrumentation
- Re-runnable from any stage on any APK

### Corpus runs
- Bench-10K: Java side, baseline metric
- NDK-100: native side
- Planted-backdoor zoo: ML side
- Repack-2K: bisim regression
- Adversarial-500: parser confusion
- Stress-100K: stress
- Malware-1K: malware-replay

### Cross-arch
- Same artifact build on x86_64 + ARM64
- Cert SHA-256 byte-identity verified

### 7-day soak
- Continuous load at 90 % nominal
- Automatic recovery from any single-pod failure
- Sentry alert on any P0 incident

### Dynamic-bridge in pilot mode
- Operational on UNKNOWNs from joint analyzer

### Reproducibility audit
- Bytewise-identical certs across runs / arches
- Repeated 5× over the 7 days

### Cost dashboard
- GPU + emulator-pool burn per day

### Carry-forward debt log
- Any KPI miss documented
- Owner + due date

### Documentation
- `docs/phase-5-e2e-results.md`

## 8. KPIs (this sub-phase) — All from PHASE_GATES.md §9

| KPI | HARD | TARGET |
|---|---|---|
| L0–L6 + native sustained throughput, 16-core | ≥ 7 APKs/sec | ≥ 18 APKs/sec |
| Cluster (8-node × 16-core) throughput | ≥ 50 APKs/sec | ≥ 130 APKs/sec |
| Full pipeline incl. native p99 | ≤ 30 s | ≤ 10 s |
| Full pipeline incl. dynamic confirmation p99 | ≤ 120 s | ≤ 45 s |
| Peak RSS per worker | ≤ 1.5 GB | ≤ 800 MB |
| Emulator memory budget | ≤ 2 GB | ≤ 1 GB |
| DEX lift throughput (re-confirm) | ≥ 50 MB/s | ≥ 150 MB/s |
| ARM64 ELF lift throughput (re-confirm) | ≥ 25 MB/s | ≥ 80 MB/s |
| ARMv7 ELF lift throughput | ≥ 20 MB/s | ≥ 60 MB/s |
| DEX coverage on Bench-10K | ≥ 95 % | ≥ 99 % |
| ARM64 coverage on NDK-100 | ≥ 60 % | ≥ 80 % |
| Lift correctness vs reference | ≥ 95 % | ≥ 99 % |
| Emulator cold-start | ≤ 120 s | ≤ 30 s |
| Per-finding dynamic refinement p99 | ≤ 300 s | ≤ 60 s |
| UNKNOWN refinement rate | ≥ 30 % | ≥ 60 % |
| Parallel APKs (emulator) | ≥ 8 | ≥ 16 |
| Backdoor detection precision | ≥ 90 % | ≥ 98 % |
| Backdoor detection recall | ≥ 80 % | ≥ 95 % |
| Cross-language intent analysis p99 | ≤ 15 s | ≤ 5 s |
| JNI boundary modeling coverage | ≥ 75 % | ≥ 95 % |
| Native intent dispatch resolution | ≥ 50 % | ≥ 80 % |
| 7-day soak: zero crashes | yes | yes |
| Cross-arch byte-identical certs | 100 % | 100 % |
| Reproducibility 100 % across runs / arches | 100 % | 100 % |
| K12: Daily native-lifter regression | green for 60 days | green for 90 |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── orch/
│   └── phase-5-e2e/                 # NEW: DAG
├── docs/
│   ├── phase-5-e2e-results.md       # NEW
│   └── phase-5-carry-forward.md     # NEW
└── dashboards/
    └── phase-5/                     # NEW: Grafana JSON
```

## 10. Standalone Output

A reproducible Phase-5 evaluation publishable on Zenodo + sample dataset NDK-100 released under CC-BY-4.0.

## 11. End-to-End Test

```bash
buck2 run //orch:phase-5-e2e -- --corpus all --soak 7d
# At end: every KPI ✅ for ≥ 7 days

# Repro across arches
buck2 build --target-platforms //platforms:linux-x86_64 //... && cosign sign-blob --yes <cert.axc> > x86.sig
buck2 build --target-platforms //platforms:linux-aarch64 //... && cosign sign-blob --yes <cert.axc> > arm.sig
diff <(sha256sum x86_certs/*.axc) <(sha256sum arm_certs/*.axc)
# Expect: identical
```

## 12. Exit Checklist

All KPIs from §8 ✅ for ≥ 7 consecutive days. Plus:

- [ ] All sub-phase exit checks confirmed
- [ ] 7-day soak: zero P0 incidents
- [ ] Cross-arch parity: byte-identical certs
- [ ] Reproducibility audit passed
- [ ] Carry-forward debt log signed off
- [ ] Cost dashboard within budget
- [ ] Documentation `docs/phase-5-e2e-results.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P5.19** | Eval data for paper |
| **P5.20** | Live dashboards for gate review |
| **Phase 6** | Production-grade Phase-5 stack carries into stabilization |
