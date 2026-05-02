# P6.11 — G10 Stabilization: Emulator Pool Scaling + Chaos Drills

> Scale emulator pool from 32 to 128 steady-state. Drive UNKNOWN refinement rate ≥ 50 %. Continuous chaos drills. Cold-start ≤ 30 s TARGET hit consistently.

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P6.11 |
| Owner(s) | G10 |
| Duration | Weeks 1–14 |
| Critical-path | yes |
| Hard prerequisites | P6.1 |

## 2. Goal & Scope

Pool scaled, refinement rate driven up, chaos drills run weekly with zero unscheduled downtime over the 90-day v1.0 window.

### In scope
- Pool steady-state ≥ 128 (HARD; up from 32)
- Cold-start ≤ 30 s (TARGET sustained → HARD for v1.0)
- UNKNOWN refinement ≥ 50 % HARD (was ≥ 30 %)
- Weekly chaos drills (pod kill 30 % / network partition / OOM / disk full / kernel-bug-injection)
- Per-pod observability: Pyroscope continuous + Prometheus + Sentry
- Cost dashboard: per-API-level + per-arch
- Multi-region failover

### Out of scope
- New emulator features
- New Frida scripts (G10 in P5.11)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P6.1** | Stabilization punch-list |
| **All Phase 5 G10 deliverables** | Continued |

## 4. Required Tools, Libraries, and Languages

Same as Phase 5.

## 5. Third-Party Software, Services, Accounts & API Keys

All free OSS / paid (existing).

**No new API keys.**

## 6. System Inventory — Have vs Need

| Need | Status |
|---|---|
| Cloud-budget capacity for 128 pods | request increase |
| Multi-region failover | provision second region |

## 7. Features & Functions Delivered (Comprehensive)

### Pool scaling
- Steady-state ≥ 128 (HARD)
- Burst to 512 (TARGET)
- Multi-region: primary AWS Graviton + secondary Oracle A1 / GCP T2A
- Failover SLA: ≤ 5 min cold-region warm-up

### Cold-start tuning
- ≤ 30 s sustained
- Snapshot + restore optimization
- Pre-warmed pool of 16 idle pods at all times

### UNKNOWN refinement driver
- Driver heuristics improved: more comprehensive deeplink fuzz seeds, smarter Monkey strategies, grammar-aware Intent fuzz with constraint propagation
- Refinement rate ≥ 50 % HARD on full UNKNOWN distribution

### Chaos drills
- Weekly: pod kill 30 % / network partition / OOM / disk full
- Monthly: full-region failover drill
- Continuous chaos: random 1 % pod kill (production canary)

### Observability
- Per-pod Pyroscope CPU profile continuous
- Sentry error tracking + alerting
- Cost dashboard with per-day burn limits

### Documentation
- `docs/g10-stabilization.md`

## 8. KPIs (this sub-phase)

| KPI | HARD |
|---|---|
| Steady-state pool size | ≥ 128 |
| Burst capacity | ≥ 512 |
| Cold-start | ≤ 30 s sustained |
| UNKNOWN refinement rate | ≥ 50 % |
| Pod-kill recovery | ≤ 15 s |
| Multi-region failover | ≤ 5 min |
| Weekly chaos drill green | yes |
| 90-day uninterrupted-uptime window | green |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── infra/
│   └── emulator-pool/                # scaled
├── crates/
│   └── axiom-emu-orch/               # multi-region
└── docs/
    └── g10-stabilization.md          # NEW
```

## 10. Standalone Output

128-pod multi-region pool reusable beyond APKAXIOM.

## 11. End-to-End Test

```bash
kubectl get pods -n emulator-pool | wc -l
# Expect: ≥ 128

buck2 run //tools:axiom-emu-bench -- --target cold-start
# Expect: median ≤ 30 s

buck2 run //tools:axiom-dynamic-bench -- --corpus eval-50k --report unknown-refinement
# Expect: ≥ 50 %
```

## 12. Exit Checklist

- [ ] Pool ≥ 128 steady (HARD)
- [ ] Cold-start ≤ 30 s (HARD)
- [ ] UNKNOWN refinement ≥ 50 % (HARD)
- [ ] Multi-region failover ≤ 5 min
- [ ] Weekly chaos drill green
- [ ] 90-day uptime window green (continuous)
- [ ] Cost dashboard within budget
- [ ] Documentation `docs/g10-stabilization.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P6.16** | Pool scale used for 50K eval |
| **P6.17** | Pool architecture explained to auditor |
| **P6.20** | "Dynamic bridge resolves ≥ 50 % UNKNOWN findings" item ✅ for ship gate |
