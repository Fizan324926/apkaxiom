# P1.14 — Differential Fuzzing Plant: A8 + A11 Harnesses + Auto Classifier

> Three parallel AOSP harnesses. Disagreements automatically classified into AOSP CVE / model bug / spec ambiguity. Cross-version disagreements are gold.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../README.md §12](../../README.md#continuous)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P1.14 |
| Owner(s) | G8 |
| Duration | Weeks 12–18 |
| Critical-path | no, but feeds Phase 1 KPI gate |
| Hard prerequisites | P1.13 (A14 harness operational) |

## 2. Goal & Scope

Two more Cuttlefish images (A8, A11) added as parallel harnesses. The disagreement classifier is now automated — sorts findings into the 3-way taxonomy: **AOSP CVE candidate / model bug / spec ambiguity**.

Cross-version disagreements (e.g., A8 accepts but A14 rejects) are the highest-value findings — they are direct evidence of evasion-targeting attacks.

### In scope
- A8 + A11 Cuttlefish images, hermetically built
- All 3 harnesses parallel and continuous
- Cross-version differential mode
- Automated classifier with ≥ 80% precision
- Centipede orchestration at scale across the 3 nodes

### Out of scope
- A12, A13, A15 (Phase 2)
- Native code fuzzing (Phase 5)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P1.13** | A14 harness; Nyx; classifier-manual taxonomy |

## 4. Required Tools, Libraries, and Languages

Same as P1.13 plus:

| Tool | Version | Purpose |
|---|---|---|
| **Centipede** | from P1.13 install | Distributed fuzz orchestration |
| **xgboost / scikit-learn** (Python) | latest | Classifier training (rules + simple ML) |
| **OpenTelemetry collector** | from P1.7 | Cross-node trace correlation |

## 5. Third-Party Software, Services, Accounts & API Keys

Same dependencies as P1.13. Additional items:

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **Two more Hetzner / OVH KVM nodes** | hardware | **Paid** (~ €200–600/mo total for 2 more nodes) | https://www.hetzner.com | Required for parallel A8 + A11 harnesses |
| **AOSP A8 source images** | AOSP | **Free** | https://source.android.com | Older AOSP versions; may need archived builds |
| **Object storage for fuzz corpus archive** | storage | **Free** S3-compat (MinIO self-host) or **Paid** AWS S3 (~$23/TB/mo) | self-host MinIO | At fuzzing scale, corpus grows to TBs |

## 6. System Inventory — Have vs Need

### Already present (after P1.13)
- ✅ KVM hardware for one node
- ✅ Cuttlefish + Nyx + AFL++ + Centipede installed

### Missing — must install
- ❌ Two more KVM nodes (procurement task)
- ❌ MinIO (object storage for corpus)
- ❌ AOSP A8 + A11 source — older, may need archived AOSP builds

### Install commands

```bash
# MinIO self-hosted object store (on a separate storage host)
docker run -d --name minio -p 9000:9000 -p 9001:9001 \
  -e MINIO_ROOT_USER=admin -e MINIO_ROOT_PASSWORD=$(openssl rand -base64 32) \
  -v /mnt/storage/minio:/data quay.io/minio/minio server /data --console-address ":9001"

# AOSP A11 (Android 11)
mkdir -p /opt/cuttlefish-images/A11 && cd /opt/cuttlefish-images/A11
# Use AOSP archived build manifest android-11.0.0_rXX
# (build IDs published at https://source.android.com/setup/start/build-numbers)

# Centipede coordinator
buck2 run //fuzz/orchestrator:centipede-coordinator -- --workers 3 --aosp-versions A8,A11,A14
```

## 7. Working Directory & Files Produced

```
apkaxiom/
├── fuzz/
│   ├── orchestrator/
│   │   ├── Cargo.toml
│   │   ├── BUCK
│   │   └── src/main.rs                   # Centipede driver
│   ├── classifier/
│   │   ├── Cargo.toml
│   │   ├── BUCK
│   │   └── src/
│   │       ├── main.rs                   # rules + simple ML
│   │       ├── rules.rs
│   │       └── train.py                  # xgboost training script
│   ├── findings/
│   │   ├── archive.fjall                 # extended
│   │   └── classifications/              # NEW — labeled findings
│   └── dashboards/
│       └── grafana-cross-version.json
├── external/aosp/
│   ├── cuttlefish-A8/
│   ├── cuttlefish-A11/
│   └── cuttlefish-A14/
└── docs/
    └── differential-fuzzer.md             # extended with classifier rules
```

## 8. Standalone Output

```bash
# Across all 3 KVM nodes
buck2 run //fuzz/orchestrator:centipede-coordinator
# Continuous; dashboard shows:
#   - per-node coverage growth
#   - disagreement count by AOSP-version pair
#   - classifier output: CVE / model-bug / ambiguity
```

## 9. End-to-End Test

Sustained 14-day run:
- ≥ 10 disagreements/week classified (HARD per PHASE_GATES.md §5).
- Classifier ≥ 80% precision verified by manual sampling of 100 findings (HARD).
- ≥ 1 cross-version disagreement found and reproduced (HARD).
- 3 harnesses, ≥ 99% uptime each.

## 10. Exit Checklist

- [ ] A8 and A11 Cuttlefish harnesses live
- [ ] All 3 harnesses ≥ 99% uptime over 14 days
- [ ] Classifier ≥ 80% precision (HARD)
- [ ] Cross-version disagreement found and reproduced (HARD)
- [ ] Findings dashboard live with cross-node correlation
- [ ] MinIO corpus archive operational, growing
- [ ] CVE filing pipeline tested with at least one draft submission

## 11. Hand-Off

| Consumed by | What they need |
|---|---|
| **P1.18** | Disagreement count + classification feed Phase-1 KPI gate |
| **P1.20** | Phase 1 ship gate cites fuzz findings |
| **Phase 2 / G8** | Pattern extends to 5 AOSP versions |
