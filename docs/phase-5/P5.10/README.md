# P5.10 — Android Emulator Orchestration Pool

> Stand up a production-grade emulator pool: AOSP Cuttlefish + Goldfish kernels, KVM ARM Graviton hosts, ≥ 32 steady-state, burst 128, ≤ 120 s cold-start, chaos-drilled weekly.

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P5.10 |
| Owner(s) | G10 |
| Duration | Weeks 1–10 |
| Critical-path | yes (dynamic confirmation needs it) |
| Hard prerequisites | P5.1 |

## 2. Goal & Scope

A pod-based emulator pool addressing:
- ≥ 32 steady-state emulators (HARD)
- Burst to 128 emulators (TARGET)
- Cold-start ≤ 120 s (HARD; ≤ 30 s TARGET)
- Pod-kill chaos drills weekly with recovery ≤ 60 s (HARD; ≤ 15 s TARGET)
- Per-emulator memory budget ≤ 2 GB (HARD; ≤ 1 GB TARGET)
- API levels 26–35 covered
- Cross-arch parity: ARM64 + x86_64 emulators

### In scope
- Cuttlefish + Goldfish kernels build pipeline
- Pod-based orchestration (Kubernetes or Nomad — chosen via ADR-0030)
- Per-pod budget controls (CPU, memory, network)
- Health-check + auto-restart
- Chaos drills: pod kill, network partition, OOM, disk full
- Emulator-checkpoint + restore for cold-start savings
- Cost dashboard (daily / monthly burn)
- Frida-server pre-installed in image
- eBPF host-side observability

### Out of scope
- Frida script library content (P5.11)
- eBPF program library content (P5.12)
- Dynamic confirmation logic (P5.13)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P5.1** | Emulator-pool capex/opex provisioned, Cuttlefish + Goldfish vendored |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **AOSP Cuttlefish** | latest | Cloud-friendly emulator |
| **Goldfish kernels** | matching API levels | Backing kernel |
| **Kubernetes** | 1.30+ (or Nomad 1.7+) | Orchestration |
| **Talos / k0s / k3s** | latest | Lightweight control plane (option) |
| **KubeVirt** | latest | Optional KVM wrapper |
| **Terraform** | 1.7+ | Infra as code |
| **Ansible** | 9+ | Configuration management |
| **Pyroscope + Prometheus + Grafana** | latest | Observability |
| **Velero** | latest | Backup of emulator-pool config |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **AWS Graviton (c7g.4xlarge)** | service | **Paid** ~$0.58/hr ≈ $420/mo per host | https://aws.amazon.com/ec2/graviton | Production |
| **Oracle Cloud Ampere A1** | service | **Free tier** (4 OCPU + 24 GB always-free) + paid extension | https://www.oracle.com/cloud/free | Cost saver |
| **GCP T2A** | service | **Paid** ~$0.50/hr | https://cloud.google.com/compute/docs/general-purpose-machines#t2a | Alternative |
| **Talos / k0s / k3s** | tool | **Free** OSS | various | |
| **KubeVirt** | tool | **Free** OSS | https://kubevirt.io | |
| **Terraform** | tool | **Free** OSS | https://terraform.io | (state stored encrypted; secrets via Vault) |
| **Ansible** | tool | **Free** OSS | https://ansible.com | |
| **Velero** | tool | **Free** OSS | https://velero.io | |
| **Pyroscope / Prometheus / Grafana** | tools | **Free** OSS | various | |
| **Sentry** | service | **Free tier** + paid | https://sentry.io | Error tracking |

**API keys required:** AWS / GCP / Oracle Cloud, Sentry, Vault root token.

## 6. System Inventory — Have vs Need

| Need | Status |
|---|---|
| K8s / Nomad cluster | provision via Terraform |
| Cuttlefish images per API level | build pipeline |
| Frida-server in image | bake into image |

## 7. Features & Functions Delivered (Comprehensive)

### Infra-as-code
- `infra/emulator-pool/terraform/` — provisioning for AWS Graviton + Oracle A1 + optional GCP
- `infra/emulator-pool/ansible/` — host configuration
- `infra/emulator-pool/k8s/` (or Nomad job spec) — pool spec
- Per-pod resource limits

### Emulator image build pipeline
- Cuttlefish base image + Frida-server + custom test ROM
- API levels 26, 28, 30, 31, 33, 34, 35
- Per-arch (ARM64 + x86_64) image tags
- Image deduplication via OCI layer reuse

### Orchestration features
- Pool-size autoscaler
- Cold-start checkpoint + restore (snapshot a booted Android, restart from snapshot)
- Per-pod stable network identity (for Frida attach)
- Health-check probe (boot complete, package manager up, Frida-server reachable)
- Auto-restart on health-fail
- Per-pod taint allowing chaos kills

### Chaos drills
- Weekly pod-kill drill — kill 30 % of pool, verify ≤ 60 s recovery
- Network-partition drill — isolate region, verify graceful degradation
- OOM drill — fill 95 % memory, verify pod restart not host degradation
- Disk-full drill

### Observability
- Per-pod metrics: CPU, memory, network, Frida session count, eBPF program count
- Pyroscope CPU profiles continuous
- Prometheus alerts: pool < 80 %, cold-start > 120 s, restart loop, runaway memory
- Grafana dashboards
- Sentry error tracking

### Cost dashboard
- Daily / monthly burn
- Per-API-level + per-arch breakdown
- Alerts on > 110 % budget

### Documentation
- `docs/emulator-pool.md` — architecture + runbook + chaos-drill catalog

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Steady-state emulator count | ≥ 32 | ≥ 64 |
| Burst capacity | 128 | 256 |
| Cold-start latency | ≤ 120 s | ≤ 30 s |
| Pod-kill recovery | ≤ 60 s | ≤ 15 s |
| Per-emulator memory | ≤ 2 GB | ≤ 1 GB |
| API-level coverage | 26, 28, 30, 31, 33, 34, 35 | same |
| Arch coverage | ARM64 + x86_64 | same |
| Health-check accuracy | 100 % | 100 % |
| Weekly chaos-drill green | yes | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── infra/
│   └── emulator-pool/
│       ├── terraform/                 # NEW
│       ├── ansible/                   # NEW
│       ├── k8s/                       # NEW (or nomad/)
│       └── images/                    # build pipeline (Cuttlefish + API levels)
├── crates/
│   └── axiom-emu-orch/                # NEW: pool client + health-check Rust lib
├── tools/
│   ├── axiom-emu-cli                  # NEW: pool admin
│   └── axiom-emu-bench                # NEW: cold-start bench
└── docs/
    └── emulator-pool.md               # NEW
```

## 10. Standalone Output

A reusable Android emulator pool, deployable independent of APKAXIOM. Open-sourced as `axiom-emu-orch` (AGPL+commercial).

## 11. End-to-End Test

```bash
terraform -chdir=infra/emulator-pool/terraform apply
ansible-playbook infra/emulator-pool/ansible/site.yml
kubectl apply -f infra/emulator-pool/k8s/

buck2 run //tools:axiom-emu-bench -- --target cold-start
# Expect: median ≤ 120 s

buck2 run //tools:axiom-emu-cli -- chaos --pod-kill 30%
# Expect: recovery ≤ 60 s
```

## 12. Exit Checklist

- [ ] Steady-state ≥ 32 (HARD)
- [ ] Cold-start ≤ 120 s (HARD)
- [ ] Pod-kill recovery ≤ 60 s (HARD)
- [ ] Per-emulator memory ≤ 2 GB (HARD)
- [ ] API levels 26, 28, 30, 31, 33, 34, 35 covered (HARD)
- [ ] ARM64 + x86_64 covered (HARD)
- [ ] Weekly chaos drill scheduled + green
- [ ] Pyroscope + Prometheus + Grafana dashboards live
- [ ] Cost dashboard live; alerts configured
- [ ] Velero backups scheduled
- [ ] Documentation `docs/emulator-pool.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P5.11** | Frida-attach hooks ready |
| **P5.12** | eBPF host-side observability ready |
| **P5.13** | Pool consumed by dynamic confirmation bridge |
| **P6 + production** | Pool reused for production canary + chaos drills |
