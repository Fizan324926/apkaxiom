# P4.18 — Phase-4 E2E: Full Pipeline + Cert + Verifier + SDKs + Soak + Cross-Architecture

> All Phase 4 KPIs measured live. Full L0–L6 pipeline in production. Verifier p99 ≤ 100 ms over 10K certs. 7-day soak. Cross-arch byte-identical certs.

**Parent plan:** [../README.md](../README.md) · **PHASE_GATES.md §8:** [../../PHASE_GATES.md#phase-4](../../PHASE_GATES.md#phase-4)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P4.18 |
| Owner(s) | All Phase 4 groups |
| Duration | Weeks 18–22 |
| Critical-path | yes |
| Hard prerequisites | P4.5–P4.10, P4.16, P4.17 |

## 2. Goal & Scope

The full Phase-4 stack — verified L0–L5 + L6 cert emission (Halo2 + Stwo) + axiom-verify (Rust + Wasm + ARM64) + SDKs + SLSA + bug-bounty pilot — runs end-to-end. All Phase 4 KPIs measured live and reported.

### In scope
- E2E test harness extending `tests/e2e/phase3.rs` to `phase4.rs`
- Full pipeline produces `.axc` certs on Bench-10K + Bundles-5K
- Verifier benchmark on 10K-cert sample
- All 5 priority privacy-invariant claims emitted + verified
- 7-day soak run
- Cross-arch (x86_64 / ARM64 / RISC-V where available) parity
- Reproducibility audit

### Out of scope
- Paper publication (P4.19)
- Phase 5 scope decisions (P4.20)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P4.5–P4.10** | All zk circuits + STARK fallback |
| **P4.11/P4.12** | Verifier core + Wasm + mobile |
| **P4.13/P4.14/P4.15** | All 3 SDKs |
| **P4.16** | SLSA + reproducibility |
| **P4.17** | Bug-bounty pilot live |

## 4. Required Tools, Libraries, and Languages

Same as P3.18 + Phase-4 components.

| Tool | Version | Purpose |
|---|---|---|
| **Full Phase-4 stack** | from prior sub-phases | The thing under measurement |
| **All monitoring infra** | from P1.18 | Dashboards |
| **Reference benchmark hardware** | EPYC 9354 + Graviton3 + RISC-V P550 | KPI measurement |
| **8× H100/L40S GPUs** | from P4.1 | zk proving |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **AndroZoo + DREBIN + F-Droid + Repack-2K** | corpora | **Free** | already provisioned | |
| **Hetzner / OVH / AWS Graviton3** | hardware | **Paid** | already provisioned | |
| **RISC-V P550 / Ampere AltraMax** | RISC-V hardware | **Paid** ~$5–15K capex (boards exist by 2027) | https://www.sifive.com / https://amperecomputing.com | For RISC-V cross-arch parity |
| **MinIO / PostgreSQL / Lance** | storage | **Free** OSS | already provisioned | |

**No new API keys.** RISC-V hardware procurement is the new infrastructure item; Phase-4 plan accommodates either physical board or QEMU-rv64 emulation as a fallback.

## 6. System Inventory — Have vs Need

### Already present
- ✅ Full Phase-4 stack
- ✅ All monitoring + benchmark hardware (excl. RISC-V)

### Missing
- RISC-V hardware (procure or fall back to QEMU emulation)

```bash
# QEMU RISC-V fallback
sudo apt-get install -y qemu-system-misc
# RISC-V cross-compile target
rustup target add riscv64gc-unknown-linux-gnu
```

## 7. Features & Functions Delivered (Comprehensive)

### E2E test harness (`tests/e2e/phase4.rs`)
- Reads APK / AAB
- Runs L0 → L1 → L2 → L3 → L4 → L5 → L6
- Emits `.axc` cert with all claim types
- Verifier verifies the cert
- All KPIs captured

### Full-pipeline cert generation
- Per APK: parse, resolve, fingerprint, prove all 5 priority invariants, emit `.axc`
- Cert size + emission time tracked
- Halo2 vs Stwo dual-pipeline tested

### Verifier benchmark
- 10K-cert sample → run through `axiom-verify` core, Wasm, mobile, all 3 SDKs
- p50 / p95 / p99 / p99.9 captured per platform
- Cross-platform parity verified

### Cross-architecture parity
- Bench-1K through full pipeline on x86_64 + ARM64 + RISC-V (or QEMU)
- Cert SHA-256 byte-identical (HARD)
- Verdicts byte-identical (HARD)

### Reproducibility audit
- 50 runs of full pipeline on Bench-100
- Cert byte-identity 100 % (HARD per PHASE_GATES.md K10)
- Halo2 proof byte-identity 100 % (HARD)

### 7-day soak
- Bench-10K replay continuously for 7 days
- Crash rate measured (HARD: zero)
- Memory growth tracked

### Pilot validation
- During this 4-week period, P4.17's pilot is live and accumulating data
- Cross-cutting metric: ≥ 1000 verifications during E2E phase

### Performance dashboards (Grafana)
- Per-layer throughput / latency / memory
- L6 cert emission split (per-claim-type)
- Verifier service throughput across all platforms
- SDK throughput per language
- SLSA coverage rate
- Pilot platform metrics

### Reports
- `reports/phase4-e2e-eval.md`
- `reports/phase4-verifier-benchmark.md`
- `reports/phase4-cross-arch-parity.md`
- `reports/phase4-pilot-results.md`

## 8. KPIs (this sub-phase — all PHASE_GATES.md §8 hards)

| KPI | HARD | TARGET |
|---|---|---|
| L0–L6 sustained throughput, 16-core | ≥ 10 APKs/sec | ≥ 25 APKs/sec |
| End-to-end emission p99 | ≤ 90 s | ≤ 30 s |
| Verifier service throughput, single 16-core node | ≥ 3,000 verifications/sec | ≥ 10,000/sec |
| `axiom-verify` p99 over 10K cert sample | ≤ 100 ms | ≤ 50 ms |
| `axiom-verify` cold start | ≤ 500 ms | ≤ 150 ms |
| Wasm p99 in Chromium 122+ | ≤ 300 ms | ≤ 120 ms |
| ARM64 mobile p99 | ≤ 200 ms | ≤ 80 ms |
| Cert size median | ≤ 100 KB | ≤ 50 KB |
| Cert size p99 | ≤ 500 KB | ≤ 200 KB |
| All 3 SDKs throughput floors met | yes | yes |
| Cross-arch byte-identical certs | 100 % | 100 % |
| 7-day soak: 0 crashes | yes | yes |
| Reproducibility 100 % across runs + architectures | yes | yes |
| Bug-bounty pilot ingestion ≥ 500/hour | yes | ≥ 5K/hour |
| SLSA L4 verifier round-trips | ≥ 100 F-Droid | ≥ 1000 |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── tests/e2e/phase4.rs                  # NEW
├── reports/
│   ├── phase4-e2e-eval.md
│   ├── phase4-verifier-benchmark.md
│   ├── phase4-cross-arch-parity.md
│   └── phase4-pilot-results.md
├── monitoring/grafana-dashboards/
│   ├── phase4-l6-emission.json
│   ├── phase4-verifier.json
│   ├── phase4-sdks.json
│   └── phase4-pilot.json
└── corpus/
    ├── bench-10k/
    ├── bundles-5k/
    └── stress-100k/
```

## 10. Standalone Output

```bash
nix develop
buck2 test //tests/e2e:phase4 -- --corpus bench-10k --report reports/phase4-e2e-eval.md
# Dashboards live; reports written
```

## 11. End-to-End Test

```bash
buck2 test //tests/e2e:phase4-bench-10k
buck2 test //tests/e2e:phase4-cross-arch
buck2 test //tests/e2e:phase4-soak-7d
buck2 test //tests/e2e:phase4-verifier-benchmark
buck2 test //tests/e2e:phase4-sdks-throughput
# All HARD KPIs above must pass; ≥ 7 days green
```

## 12. Exit Checklist

All PHASE_GATES.md §8 hard gates ✅ for ≥ 7 consecutive days:

- [ ] L0–L6 sustained ≥ 10 APKs/sec on 16-core (HARD)
- [ ] End-to-end emission p99 ≤ 90 s (HARD)
- [ ] Verifier service ≥ 3K/sec on single 16-core node (HARD)
- [ ] `axiom-verify` p99 ≤ 100 ms over 10K certs (HARD)
- [ ] Cold start ≤ 500 ms (HARD)
- [ ] Wasm p99 ≤ 300 ms (HARD)
- [ ] ARM64 mobile p99 ≤ 200 ms (HARD)
- [ ] Cert size median ≤ 100 KB, max ≤ 500 KB (HARD)
- [ ] All 3 SDKs throughput met (axiom-py ≥ 50, axiom-go ≥ 200, axiom-ts ≥ 20) (HARD)
- [ ] Cross-arch byte-identical certs 100 % (HARD)
- [ ] 7-day soak: 0 crashes (HARD)
- [ ] Reproducibility 100 % (HARD)
- [ ] Pilot ingestion ≥ 500/hour live (HARD)
- [ ] SLSA L4 verifier round-trips operational (HARD)
- [ ] All 5 priority claim types emit + verify
- [ ] Halo2 + Stwo dual-pipeline both produce verifiable certs

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P4.19** | Phase-4 numbers for paper |
| **P4.20** | Live KPI dashboard for gate review |
| **Phase 5 / G7** | Phase-4 stack as foundation for additional zk circuits |
