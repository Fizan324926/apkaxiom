# P1.18 — End-to-End Bench-1K Smoke + Bench-10K Performance Eval

> All Phase 1 KPIs measured live, on real corpora, on reference hardware. Cross-architecture verdicts identical. 24h soak: zero crashes.

**Parent plan:** [../README.md](../README.md) · **PHASE_GATES.md §5:** [../../PHASE_GATES.md#phase-1](../../PHASE_GATES.md#phase-1)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P1.18 |
| Owner(s) | G1 + G2 + G3 + G8 + G13 (all Phase 1 groups) |
| Duration | Weeks 18–22 |
| Critical-path | yes — every Phase-1 KPI lives or dies here |
| Hard prerequisites | P1.15 (IR emitter), P1.16 (verified signing), P1.17 (soundness gate) |

## 2. Goal & Scope

The full Phase-1 stack — verified L0 + L1 + AXIOM-IR-v0.1 emission + signature verification — runs end-to-end on Bench-1K (smoke) and Bench-10K (perf eval). All Phase 1 KPIs from PHASE_GATES.md §5 are measured live and reported on dashboards.

### In scope
- E2E test harness `tests/e2e/phase1.rs`
- Performance dashboards live (Grafana + Pyroscope)
- Reproducibility audit on Bench-1K
- Comparison vs apk-info v0.x baseline
- 24h soak run on Stress-100K (or 10× Bench-10K replay)
- Cross-architecture (x86_64 ↔ ARM64) parity verification

### Out of scope
- AndroZoo benchmark + paper draft (P1.19)
- Phase 2 scope decisions (P1.20)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P1.15** | IR emission |
| **P1.16** | Verified signing on E2E path |
| **P1.17** | Soundness gate green |
| **P1.13/P1.14** | Differential fuzzer running for adversarial inputs |
| **P1.3** | AndroZoo access provisioned |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **All Phase-1 stack** | — | The thing we're measuring |
| **HDR Histogram** | 0.13+ | Latency distribution capture |
| **Pyroscope** | from P1.7 | Continuous profile capture during eval |
| **Prometheus + Grafana** | from P1.7 | Live dashboards |
| **OpenTelemetry collector** | from P1.7 | Trace correlation |
| **iperf3** | from P1.7 | Wire-speed feeder |
| **hyperfine** | from P1.3 | Microbenchmarks |
| **flamegraph** | from P1.3 | Profile diffs |
| **Reference benchmark hardware** | EPYC 9354 / Xeon Gold 6438M (per PHASE_GATES.md App. B) | Where KPIs are measured |
| **ARM64 reference** | Graviton3 c7g.4xlarge or Ampere Altra | Cross-arch parity check |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **AndroZoo** | corpus | **Free academic** | https://androzoo.uni.lu | API key from P1.3; used to curate Bench-10K |
| **DREBIN** | malware corpus | **Free research** | TU Braunschweig | Adversarial inputs |
| **MalwareBazaar** | malware feed | **Free** | https://bazaar.abuse.ch | Recent samples |
| **F-Droid** | clean APKs | **Free** | https://f-droid.org/archive/ | Reference clean corpus |
| **Hetzner / OVH benchmark hosts** | hardware | **Paid** | (from P1.7) | EPYC 9354 dedicated server, ~ €150/mo |
| **AWS Graviton3 c7g.4xlarge** | ARM64 reference | **Paid** $0.58/hr (~ $420/mo if always-on, but only used during P1.18 eval — typically $50/mo) | https://aws.amazon.com/ec2/instance-types/c7g | Or alternative: Ampere Altra at OVH/Hetzner ARM |
| **Grafana Cloud** *(optional)* | hosted dashboards | Free tier 10K series; **paid** $$$ | https://grafana.com/products/cloud/ | We self-host, but Grafana Cloud is an option |
| **Sentry** *(optional)* | crash reporting | Free tier 5K events/mo; **paid** $$$ | https://sentry.io | We can self-host Sentry or use free tier |

**API keys at this sub-phase:**
- AndroZoo (already provisioned in P1.3 — must be live now)
- AWS access key for Graviton3 if using AWS for ARM64 ref (alternative: Hetzner ARM64 dedicated ~ €60/mo, no API key needed)

## 6. System Inventory — Have vs Need

### Already present (after prior sub-phases)
- ✅ Full Phase-1 software stack
- ✅ Pyroscope, Prometheus, Grafana on dev host
- ✅ HDR Histogram (Rust crate)
- ✅ hyperfine, flamegraph, iperf3

### Missing — must procure
- ❌ **Reference benchmark hardware** (Hetzner AX102 EPYC 9354 or equivalent)
- ❌ **ARM64 reference machine** (Hetzner ARM64 dedicated or AWS Graviton3)
- ❌ **Bench-10K corpus** (curated from AndroZoo)
- ❌ **Stress-100K corpus** (sampled from AndroZoo for soak)

### Curation commands

```bash
# Pull AndroZoo metadata (academic API)
curl -O "https://androzoo.uni.lu/api/download?apikey=$ANDROZOO_KEY&sha256=..."

# Curate Bench-10K — stratified sample
buck2 run //tools/corpus-curate -- \
  --source androzoo \
  --target Bench-10K \
  --strata "size:[10MB-50MB] lang:en min-vt:5 max-vt:30" \
  --count 10000

# Curate Stress-100K
buck2 run //tools/corpus-curate -- --source androzoo --target Stress-100K --count 100000
```

## 7. Working Directory & Files Produced

```
apkaxiom/
├── tests/
│   └── e2e/
│       ├── BUCK
│       └── phase1.rs                    # NEW — full E2E test
├── tools/
│   └── corpus-curate/                    # NEW — corpus assembly
│       ├── Cargo.toml
│       └── src/main.rs
├── corpus/
│   ├── bench-1k/                         # ~30 GB
│   ├── bench-10k/                        # ~300 GB
│   └── stress-100k/                      # ~3 TB
├── reports/
│   └── phase1-eval.md                    # NEW — measured numbers + dashboard links
├── monitoring/
│   ├── grafana-dashboards/
│   │   ├── phase1-throughput.json
│   │   ├── phase1-latency.json
│   │   ├── phase1-memory.json
│   │   └── phase1-soundness.json
└── docs/
    └── phase1-eval.md                    # NEW
```

**Storage requirements:** ~ 3.3 TB for full corpora. Hetzner AX102 ships with 2× 1.92 TB NVMe = enough for Bench-10K; Stress-100K spills to HDD or MinIO object store.

## 8. Standalone Output

```bash
# On reference benchmark host
nix develop
buck2 test //tests/e2e:phase1 -- --corpus bench-10k --report reports/phase1-eval.md
# Dashboards live at http://<host>:3000/d/phase1
# Reports include every PHASE_GATES.md §5 KPI, measured + pass/fail
```

## 9. End-to-End Test

Three test surfaces:

### 9.1 Smoke (Bench-1K, 100% reproducible)
- Replay Bench-1K twice; diff every Merkle root + IR + verdict; 100% bit-identical (HARD).

### 9.2 Performance Eval (Bench-10K)
- All §5 KPIs measured live and reported.

### 9.3 24-Hour Soak (Stress-100K replay 10×)
- Continuous workload 24h.
- Zero crashes (HARD per PHASE_GATES.md §5 K7).
- Memory growth ≤ 2 MB/hour (HARD).

### 9.4 Cross-Architecture Parity
- Same Bench-1K on x86_64 + ARM64.
- All verdicts byte-identical (HARD).
- Throughput parity within 25% (HARD).

## 10. Exit Checklist (this is also the Phase 1 KPI gate)

All of PHASE_GATES.md §5 hard gates must be ✅ for ≥ 7 consecutive days:

- [ ] Sustained ≥ 300 APKs/sec on 16-core
- [ ] L0+L1 p99 ≤ 300 ms
- [ ] Peak RSS ≤ 150 MB
- [ ] 1→16 core efficiency ≥ 70%
- [ ] CI byte-identical 100% over 100 PRs
- [ ] Soundness regressions = 0
- [ ] x86_64 ↔ ARM64 within 25%
- [ ] Wire-speed ≥ 500 Mbps single-core sustained 60 min
- [ ] 24h soak: 0 crashes
- [ ] Bench-1K reproducibility 100%
- [ ] Cross-arch Bench-1K verdicts identical
- [ ] Pyroscope continuous profiles archived
- [ ] All Grafana dashboards live and paged on regression

## 11. Hand-Off

| Consumed by | What they need |
|---|---|
| **P1.19** | Numbers for the AndroZoo benchmark and paper |
| **P1.20** | Live KPI dashboard for the gate review meeting |
| **Phase 2 / G4** | Bench-10K + Stress-100K corpora reused as G4 evaluation set |
