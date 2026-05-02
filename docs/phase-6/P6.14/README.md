# P6.14 — G13 Stabilization: CI Optimization + RISC-V Parity + 50K Eval Pipeline

> Drive CI to handle 50K eval in ≤ 72 h on 100-core cluster. RISC-V CI parity. 90 consecutive days byte-identical CI. Reproducibility audit quarterly.

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P6.14 |
| Owner(s) | G13 |
| Duration | Weeks 1–18 |
| Critical-path | yes |
| Hard prerequisites | P6.1 |

## 2. Goal & Scope

The infrastructure that the v1.0 ship gate sits on: 50K eval pipeline running in ≤ 72 h, RISC-V achieving cross-arch parity, 90-day byte-identical CI green.

### In scope
- 50K eval pipeline orchestration: 100-core cluster fan-out
- RISC-V CI parity: smoke → soak → byte-identical certs (100 % over 10K samples)
- 90-day byte-identical CI window opens (target: green by P6.20)
- Reproducibility audit run quarterly + before v1.0 ship
- Cluster cost dashboard
- Per-PR CI < 15 min total HARD
- Backups + disaster-recovery drilled

### Out of scope
- New tooling (deferred)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P6.1** | RISC-V runner + audit windows |
| **All Phase 1–5 G13 deliverables** | Continued |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Buck2** | (existing) | Build |
| **Bazel** | (existing) | AOSP harness builds |
| **Nix flakes** | (existing) | Pinning |
| **Airflow / Dagster** | latest | 50K orchestration |
| **k8s** | (existing) | Compute |
| **Velero** | (existing) | Backup |
| **Karpenter / cluster-autoscaler** | latest | Auto-scaling |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **AWS Graviton 100-core cluster** | service | **Paid** ~$0.58/hr/host × 7 hosts × 72 h burst | https://aws.amazon.com/ec2/graviton | 50K eval burst |
| **Karpenter / cluster-autoscaler** | tool | **Free** OSS | https://karpenter.sh | |
| **Velero** | tool | **Free** OSS | https://velero.io | |
| **Pyroscope / Prometheus / Grafana** | tools | (existing) | | |

**API keys required:** AWS / GCP burst capacity reservations.

## 6. System Inventory — Have vs Need

| Need | Status |
|---|---|
| 100-core cluster burst budget | request from leadership |
| Karpenter installed | install |
| Velero scheduled | configure |

## 7. Features & Functions Delivered (Comprehensive)

### 50K eval pipeline
- Airflow / Dagster DAG: fan-out 50K APKs across 100 cores with sharding
- Per-shard isolation; per-stage instrumentation
- Re-runnable from any stage
- Idempotent
- Cost-budget guard (auto-pause at 110 % budget)

### Cluster auto-scaling
- Karpenter scales 0 → 100 cores for eval, back to 0 after
- Cost dashboard: per-day burn

### RISC-V CI parity
- Smoke: every PR builds and passes unit tests on RISC-V
- Soak: weekly full pipeline on RISC-V (sample subset due to hardware)
- Byte-identical certs across x86_64 / ARM64 / RISC-V on 10K-sample
- HARD by P6.20: cross-arch parity 100 %

### 90-day byte-identical CI window
- Audit log of every PR's bytewise-build status
- Continuous green required
- Any regression breaks the v1.0 ship gate

### Per-PR CI < 15 min
- Compile + test + perf gate + memory gate + soundness re-verify (incremental) ≤ 15 min HARD
- Cache hit rate ≥ 90 %

### Backups + DR
- Velero backups daily
- Quarterly DR drill (full-cluster bootstrap from backup)

### Cross-time reproducibility
- Re-build of every prior phase release reproduces bit-identical (≥ 95 %)
- Cross-time test: rebuild Phase-1 release on Phase-6 toolchain

### Documentation
- `docs/g13-stabilization.md`
- `docs/50k-eval-runbook.md`

## 8. KPIs (this sub-phase)

| KPI | HARD |
|---|---|
| 50K eval pipeline ≤ 72 h on 100-core cluster | yes |
| Per-PR CI total time | ≤ 15 min |
| RISC-V smoke green per PR | yes |
| RISC-V soak weekly green | yes |
| Cross-arch byte-identity (x86_64 / ARM64 / RISC-V) | 100 % over 10K samples |
| 90 consecutive days byte-identical CI green | yes (continuous) |
| Cross-time rebuild bit-identical | ≥ 95 % releases |
| DR drill quarterly | green |
| Cost dashboard within budget | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── orch/
│   └── 50k-eval/                     # NEW
├── infra/
│   ├── ci/                           # extended
│   ├── eval-cluster/                 # NEW: Karpenter + Velero
│   └── dr/                           # NEW
├── docs/
│   ├── g13-stabilization.md          # NEW
│   └── 50k-eval-runbook.md           # NEW
└── audit/
    └── reproducibility-90-day.jsonl  # NEW
```

## 10. Standalone Output

50K eval pipeline + DR runbook + 90-day audit log.

## 11. End-to-End Test

```bash
# 50K eval dry run on a 5K sample
buck2 run //orch:50k-eval -- --sample 5k --target 100-core-cluster
# Expect: ≤ 7.2 h linear projection

# Cross-arch byte-identity
diff <(sha256sum out/x86_64/cert.axc) <(sha256sum out/aarch64/cert.axc) <(sha256sum out/riscv64/cert.axc)
# Expect: identical

# DR drill
buck2 run //infra/dr:bootstrap-from-backup -- --dry-run
```

## 12. Exit Checklist

- [ ] 50K eval pipeline ≤ 72 h projection (HARD)
- [ ] Per-PR CI ≤ 15 min (HARD)
- [ ] RISC-V parity 100 % (HARD)
- [ ] 90-day byte-identical CI green (continuous)
- [ ] Cross-time rebuild ≥ 95 % bit-identical
- [ ] Quarterly DR drill green
- [ ] Cost dashboard within budget
- [ ] Documentation `docs/g13-stabilization.md` + `docs/50k-eval-runbook.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P6.16** | 50K eval pipeline ready |
| **P6.17** | CI / build / repro evidence for auditor |
| **P6.19** | Cluster auto-scaling for production verifier |
| **P6.20** | "Hermetic build, byte-identical across x86_64 / ARM64 / RISC-V" + "90 days byte-identical CI" items ✅ for ship gate |
