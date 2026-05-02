# P4.3 — zk-SNARK Solver-Pool: Halo2 / Plonky3 / Binius

> The proving infrastructure. Halo2 (default) + Plonky3 (alt) + Binius (binary-field) integrated behind a uniform pool API. GPU acceleration via sppark / icicle. 10× CPU speedup on benchmark circuits.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §11](../../../README.md#layer-6) · [../../TECH_STACK.md §5 (zk-systems)](../../TECH_STACK.md#zk-systems)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P4.3 |
| Owner(s) | G7 |
| Duration | Weeks 2–6 |
| Critical-path | yes |
| Hard prerequisites | P4.1 |

## 2. Goal & Scope

A unified Rust pool that wraps Halo2 / Plonky3 / Binius behind a single `ZkProver` trait. Per-circuit scheme selection. GPU acceleration via sppark (NVIDIA-only) and icicle (cross-platform). Process-isolated GPU workers for crash containment. Proving-key archive in fjall LSM. 10× minimum CPU-to-GPU speedup on benchmark circuits.

### In scope
- `crates/axiom-zk-pool` — pool API
- Halo2 / Plonky3 / Binius integration
- sppark + icicle GPU kernels
- Proving-key + verifying-key archive
- Per-scheme benchmark suite
- Process isolation

### Out of scope
- STARK / Stwo (P4.10)
- Specific privacy circuits (P4.5–P4.9)
- Lean → Halo2 pipeline (P4.4)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P4.1** | All zk libraries pinned via Nix; GPU pool provisioned |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Halo2** | latest | Default zk-SNARK |
| **Plonky3** | 0.x latest | Alt zk-SNARK |
| **Binius** | latest | Binary-field |
| **sppark** | latest | NVIDIA MSM/NTT |
| **icicle** | latest | Cross-platform MSM/NTT |
| **CUDA / HIP / Metal** | versions per platform | GPU runtime |
| **Rust** | 1.95 | Pool implementation |
| **fjall** | 0.5+ | PK/VK archive |
| **HACL\* BLAKE3 + Ed25519** | from P1.10 | Content addressing + signing |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **Halo2 / Plonky3 / Binius** | zk libs | **Free** OSS | already provisioned | |
| **sppark / icicle** | GPU kernels | **Free** OSS | already provisioned | |
| **CUDA driver / toolkit** | GPU runtime | **Free** with NVIDIA hardware | already installed | |
| **8× H100/L40S GPUs** | hardware | **Paid** ~ $200–800/mo cloud, ~ $200–320K capex | already provisioned | From P4.1 |
| **GPU monitoring (NVIDIA DCGM / dcgm-exporter)** | observability | **Free** | https://github.com/NVIDIA/dcgm-exporter | Per-GPU utilization, temp, etc. |

**No new API keys.**

## 6. System Inventory — Have vs Need

### Already present (after P4.1)
- ✅ CUDA toolkit, Halo2, Plonky3, Binius, sppark, icicle

### Missing — must install
- ❌ **dcgm-exporter** for Prometheus integration

```bash
# DCGM exporter for Prometheus
docker run -d --gpus all --rm -p 9400:9400 nvcr.io/nvidia/k8s/dcgm-exporter:latest
```

## 7. Features & Functions Delivered (Comprehensive)

### Public Rust API
- `pub trait ZkProver { fn setup(&self, circuit: &Circuit) -> (ProvingKey, VerifyingKey); fn prove(&self, pk: &ProvingKey, witness: &Witness) -> Proof; fn verify(&self, vk: &VerifyingKey, proof: &Proof, public_input: &PublicInput) -> bool; ... }`
- `pub enum ZkScheme { Halo2, Plonky3, Binius, Stwo }`  (Stwo wired in P4.10)
- `pub fn select_scheme(circuit: &Circuit) -> ZkScheme` — heuristic selection
- `pub fn prove(scheme: ZkScheme, pk: &ProvingKey, witness: &Witness, gpu: bool) -> Proof`

### Scheme-selection heuristic
- Hash-heavy circuits → Binius (binary-field, much cheaper per BLAKE3/SHA op)
- General-purpose → Halo2 (mature, audited)
- Long proofs / streaming → Plonky3 (FRI-based, faster proving)
- Post-quantum / regulated → Stwo (P4.10)

### GPU acceleration
- sppark on NVIDIA — MSM (multi-scalar multiplication) and NTT (number-theoretic transform) kernels
- icicle for cross-platform (CUDA + Metal + Vulkan)
- 10× minimum speedup over CPU baseline (HARD)
- Per-GPU process isolation; one crash doesn't kill pool

### Proving-key / verifying-key archive
- fjall LSM keyed by `(scheme, circuit_digest)`
- Content-addressed, BLAKE3-digested
- Reproducibility: same circuit → byte-identical PK/VK across builds

### Per-scheme benchmark suite
- 10 reference circuits (small / medium / large)
- Per-scheme: prove time, verify time, proof size, PK size, VK size, GPU speedup
- Results dashboarded; regression on every G7 PR

### Pool management
- 8 GPU workers (matched to procured hardware)
- Backpressure: queue length bounded
- NUMA-aware allocation

### Observability
- Per-prove span (OpenTelemetry)
- Pyroscope continuous profile
- DCGM Prometheus metrics: GPU util, temp, memory pressure

### Documentation
- `docs/zk-pool.md`
- `docs/zk-bench-results.md` (regenerated nightly)

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| All 3 schemes (Halo2 + Plonky3 + Binius) operational | yes | yes |
| GPU speedup vs CPU on benchmark circuit | ≥ 10× | ≥ 50× |
| Proving-key archive byte-identity across runs | 100 % | 100 % |
| Pool throughput (mixed circuit mix on 8 GPUs) | ≥ 100 proves/sec | ≥ 500/sec |
| Prove p99 latency on standard 16K-row circuit | ≤ 5 s GPU | ≤ 1.5 s |
| Verify p99 latency | ≤ 20 ms | ≤ 5 ms |
| Process-isolation: GPU crash containment | 100 % | 100 % |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-zk-pool/
│       ├── Cargo.toml
│       ├── BUCK
│       └── src/
│           ├── lib.rs
│           ├── trait.rs                  # ZkProver trait
│           ├── selection.rs              # scheme heuristics
│           ├── halo2_impl.rs
│           ├── plonky3_impl.rs
│           ├── binius_impl.rs
│           ├── gpu_sppark.rs
│           ├── gpu_icicle.rs
│           ├── pool.rs                   # process-isolated worker pool
│           └── archive.rs                # PK/VK fjall archive
├── corpus/zk-benchmark-circuits/         # 10 reference circuits
└── docs/
    ├── zk-pool.md                        # NEW
    └── zk-bench-results.md               # NEW (regenerated nightly)
```

## 10. Standalone Output

```bash
buck2 build //crates/axiom-zk-pool --release --features=cuda
buck2 run //bench:zk-pool-throughput
# "Halo2 GPU prove p99: 3.2s; Plonky3 GPU prove p99: 1.4s; Binius GPU prove p99: 0.8s; verify p99: 4ms"
```

## 11. End-to-End Test

```bash
buck2 test //crates/axiom-zk-pool:full
# - All 3 schemes operational (HARD)
# - GPU speedup ≥ 10× (HARD)
# - PK archive 100% reproducible (HARD)
# - Pool throughput ≥ 100 proves/sec on 8 GPUs (HARD)
# - Verify p99 ≤ 20 ms (HARD)
# - Crash containment 100% (HARD)
```

## 12. Exit Checklist

- [ ] All 3 zk schemes operational
- [ ] GPU acceleration (sppark + icicle) producing ≥ 10× speedup (HARD)
- [ ] PK archive byte-identical 100 % (HARD)
- [ ] Pool throughput ≥ 100 proves/sec on 8 GPUs (HARD)
- [ ] Prove p99 ≤ 5 s, Verify p99 ≤ 20 ms (HARD)
- [ ] Process isolation 100 % crash containment (HARD)
- [ ] DCGM monitoring + alerting live
- [ ] `docs/zk-pool.md` and bench results published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P4.4** | Pool API for circuit compilation pipeline |
| **P4.5–P4.9** | Pool to prove privacy-invariant circuits |
| **P4.10** | STARK / Stwo plugs into the same pool |
| **P4.11** | Verify path used by axiom-verify |
