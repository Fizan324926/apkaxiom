# P5.1 — Phase 4 Carry-Forward + G9 + G10 + G11 Onboarding + Phase 5 Kickoff

> Land G9 (Native Code), G10 (Dynamic Analysis), G11 (ML Security). Resolve all Phase-4 carry-forward debt. Stand up the emulator pool capex/opex. Brief new groups on AXIOM-IR, Lean, the symbolic resolver, BSH/bisim, the cert format, and the *no silent UNKNOWN* discipline.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md](../../../README.md) · [../../TECH_STACK.md](../../TECH_STACK.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P5.1 |
| Owner(s) | Project leadership + all Phase-4 groups + new G9 + G10 + G11 |
| Duration | Weeks 1–2 |
| Critical-path | **yes** |
| Hard prerequisites | P4.20 (Phase 4 closed) |

## 2. Goal & Scope

A clean Phase-5 start. G9 (4 binary-analysis + LLVM MLIR engineers), G10 (3 dynamic-analysis engineers + 1 SRE for the emulator pool), G11 (2–3 ML-security engineers) onboarded. All Phase-4 carry-forward debt resolved. LLVM / MLIR pinned. Emulator farm provisioned. CI gates extended for native lifter, dynamic refinement, and ML scanning.

### In scope
- G9 onboarding (4 engineers)
- G10 onboarding (3 + 1 SRE)
- G11 onboarding (2–3 engineers)
- Carry-forward debt review per Phase-4 group
- ADR-Phase5-Kickoff
- ADR-0029 — DEX SSA + ARM64 / ARMv7 lift strategy
- ADR-0030 — Emulator-pool topology (cloud KVM Graviton vs on-prem)
- ADR-0031 — TFLite scanner ensemble policy
- Phase-5 budget approval
- Hardware ramp: emulator-farm provisioning (≥ 32 emulators steady-state, burst to 128)

### Out of scope
- Implementing Phase-4 debt fixes (flow to right Phase-5 sub-phase)
- AXIOM-IR-v0.4 dialect freeze (P5.2)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P4.20** | Phase 4 closed; carry-forward list; Phase 5 ADR |
| **P4.2** | `.axc` v1 spec (used as cert sink for native + dynamic findings) |
| **P4.11** | `axiom-verify` core (must remain green during Phase 5) |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **LLVM / MLIR** | 18.x or 19.x (latest stable) | Lifter foundation |
| **Capstone** | 5.x | Disassembly cross-check |
| **Ghidra (headless)** | 11.x | Reference disassembler / cross-check |
| **angr** | latest | Reference symbolic execution for diff |
| **BAP** | latest | Reference binary-analysis for diff |
| **Frida** | 16.x or later | Dynamic instrumentation |
| **frida-rs** | latest | Rust bindings to Frida |
| **eBPF (libbpf, bpftrace)** | latest | Kernel-level tracing |
| **CO-RE (BTF)** | kernel 5.4+ | Portable eBPF |
| **Android Emulator (system-images)** | API 26–35 | Emulator pool |
| **AOSP Cuttlefish** | latest | Cloud-friendly emulator |
| **Goldfish kernel** | matching API levels | Emulator backend |
| **TFLite C/C++ runtime** | 2.x latest | Reference TFLite execution |
| **TFLite-Flatbuffers schema** | latest | Model parsing |
| **PyTorch / TensorFlow (eval only)** | stable | Adversarial-robustness baselines |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **LLVM** | compiler / lib | **Free** OSS (Apache 2.0 + LLVM exception) | https://llvm.org | Pinned via Nix |
| **MLIR** | IR framework | **Free** OSS | https://mlir.llvm.org | LLVM project |
| **Capstone** | disassembler | **Free** OSS | https://www.capstone-engine.org | |
| **Ghidra** | RE platform | **Free** OSS (Apache 2.0) | https://ghidra-sre.org | NSA-released |
| **angr** | symex | **Free** OSS (BSD-2) | https://angr.io | Used for diff only |
| **BAP** | binary analysis | **Free** OSS | https://github.com/BinaryAnalysisPlatform/bap | OCaml toolkit |
| **Frida** | instrumentation | **Free** OSS (wxWindows-style) | https://frida.re | |
| **eBPF** | kernel observability | **Free** OSS (kernel) | https://ebpf.io | |
| **AOSP Cuttlefish** | emulator | **Free** OSS (Apache 2.0) | https://android.googlesource.com/device/google/cuttlefish | |
| **AVD / Android Studio Emulator** | emulator | **Free** | https://developer.android.com | |
| **Cloud emulator hosting (AWS Graviton / GCP T2A / Oracle Ampere A1)** | service | **Paid** ~$50–200/mo per emulator (continuous) | various | Recommend: Oracle A1 free tier (4 OCPU + 24 GB) for prototyping; production on AWS Graviton |
| **NVIDIA H100 / L40S** | hardware (carry-over from Phase 4) | **Paid** | (already provisioned) | Re-used for ML scanning |
| **TFLite reference models** | corpus | **Free** OSS | https://www.kaggle.com/models?frameworks=15&framework_type=15&owner=tf | Kaggle Models / TF-Hub |
| **Adversarial robustness toolbox (IBM ART)** | lib | **Free** OSS | https://github.com/Trusted-AI/adversarial-robustness-toolbox | |
| **HackerOne / Bugcrowd partnership** | service (carry-over from Phase 4) | (existing) | already provisioned | Continued |

**Hardware requirement:** emulator farm capex/opex is the Phase-5 financial ramp. 32 always-on emulators ≈ $50–150/mo each → $1.6–4.8K/month steady-state, burst to 128 for E2E ≈ $6–20K/burst-day. Recommend: cloud KVM ARM Graviton or on-prem ARM-server-rack for steady, cloud-burst for E2E.

**API keys / accounts required:**
- AWS / GCP / Oracle Cloud account credentials (emulator pool)
- Kaggle / TF-Hub auth (for ML model corpus pulls)
- GitHub OAuth tokens for new G9 / G10 / G11 hires

## 6. System Inventory — Have vs Need

### Already present
- ✅ All Phase 4 stack
- ✅ HACL\* (BLAKE3, Ed25519, RSA, ECDSA, SHA-256)
- ✅ All SMT solvers + zk-SNARK pool
- ✅ GPU pool (8× H100/L40S)

### Missing — must install
- ❌ **LLVM / MLIR** — clone + build with Buck2 + Nix
- ❌ **Capstone, Ghidra, angr, BAP** — cargo / package deps + JDK for Ghidra
- ❌ **Frida + frida-rs** — Cargo + system pkg
- ❌ **libbpf + bpftrace** — system pkg + kernel headers
- ❌ **Cuttlefish + Goldfish kernels** — Android source partial pull
- ❌ **TFLite runtime + flatbuffers** — Cargo + system pkg
- ❌ **Adversarial Robustness Toolbox** — Python pkg

### Install commands

```bash
# LLVM / MLIR (Ubuntu 24.04, pinned)
wget https://apt.llvm.org/llvm.sh && chmod +x llvm.sh && sudo ./llvm.sh 19 all
sudo apt-get install -y mlir-19-tools libmlir-19-dev

# Capstone
sudo apt-get install -y libcapstone5

# Ghidra (headless)
wget https://github.com/NationalSecurityAgency/ghidra/releases/download/<latest>/ghidra_<latest>.zip
unzip ghidra_<latest>.zip -d third-party/ghidra

# angr / BAP (used for diff only)
pip install angr
opam install bap

# Frida
pip install frida frida-tools
cargo add frida-rs

# eBPF
sudo apt-get install -y libbpf-dev bpftrace linux-headers-$(uname -r)

# Cuttlefish
git clone https://android.googlesource.com/device/google/cuttlefish third-party/cuttlefish

# TFLite runtime
sudo apt-get install -y libtensorflow-lite-dev
cargo add tflite-flatbuffers

# Adversarial Robustness Toolbox
pip install adversarial-robustness-toolbox
```

## 7. Features & Functions Delivered (Comprehensive)

### G9 onboarding deliverables
- **G9 onboarding handbook** (`docs/g9-onboarding.md`) — covers AXIOM-IR-v0.3, Lean toolchain (read-only), DEX semantics, AOSP archaeology, the *lossless lift* discipline, LLVM MLIR primer, JNI boundary semantics, Frida + eBPF basics for cross-team coordination.
- **G9 charter** — mission, layer ownership (DEX + ELF lift, JNI boundary), interfaces with G3/G5/G10, headcount plan to v1.0.
- **G9 first-week tasks per hire**.

### G10 onboarding deliverables
- **G10 onboarding handbook** (`docs/g10-onboarding.md`) — Frida + eBPF + Cuttlefish + Goldfish kernels + the *consent-gated dynamic* discipline + emulator-pool architecture + dynamic-bridge protocol.
- **G10 charter** — mission, layer ownership (dynamic confirmation bridge, emulator orchestration), interfaces with G5.
- **G10 SRE on-call rotation** — 24/7 coverage for emulator pool starting M25.

### G11 onboarding deliverables
- **G11 onboarding handbook** (`docs/g11-onboarding.md`) — TFLite internals, Neural Cleanse, STRIP, adversarial robustness primer, ART toolbox, model-corpus governance.
- **G11 charter**.

### Carry-forward debt resolution
- Per Phase-4-group meeting + minutes
- Debt rollup: `docs/phase-4-carry-forward-resolved.md`
- Re-classification ADR for items deferring to Phase 6

### Emulator pool integration
- Cloud KVM ARM (Graviton or A1) provisioning script
- ≥ 32 steady-state emulators, burst 128
- Cuttlefish + Goldfish kernels build pipeline
- Pyroscope + Prometheus instrumented
- Daily cost dashboard

### CI extensions
- Native lifter soundness regression test (DEX → SSA → DEX round-trip on 1000 sample classes)
- Emulator-pool chaos drill weekly
- TFLite scanner reproducibility (deterministic verdict bits)
- LLVM / MLIR Nix-flake pinning
- BTF-CO-RE eBPF portability check

### Phase-5 kickoff artifacts
- Kickoff meeting minutes
- ADR-Phase5-Kickoff
- ADR-0029 — DEX SSA + ARM64 / ARMv7 lift strategy
- ADR-0030 — Emulator-pool topology
- ADR-0031 — TFLite scanner ensemble policy
- Phase-5 communication plan

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| G9 onboarded engineers | ≥ 4 | 4 |
| G10 onboarded engineers (incl. 1 SRE) | ≥ 4 | 4 |
| G11 onboarded engineers | ≥ 2 (of 2–3) | 3 |
| Carry-forward debt closed or re-classified | 100 % | 100 % |
| LLVM + MLIR pinned via Nix flake, reproducible | yes | yes |
| Emulator pool provisioned: ≥ 32 steady-state | yes | yes |
| Emulator-pool chaos-drill smoke green | yes | yes |
| Frida + eBPF + Cuttlefish + TFLite stack installed + reproducible | yes | yes |
| ADR-Phase5-Kickoff merged | yes | yes |
| ADR-0029 / 0030 / 0031 merged | yes | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── docs/
│   ├── g9-onboarding.md / g9-charter.md            # NEW
│   ├── g10-onboarding.md / g10-charter.md          # NEW
│   ├── g11-onboarding.md / g11-charter.md          # NEW
│   ├── phase-4-carry-forward-resolved.md           # NEW
│   ├── ADR-Phase5-Kickoff.md                        # NEW
│   ├── ADR-0029-dex-arm-lift-strategy.md           # NEW
│   ├── ADR-0030-emulator-pool-topology.md          # NEW
│   └── ADR-0031-tflite-ensemble-policy.md          # NEW
├── flake.nix                                        # extended for LLVM / MLIR / Frida / eBPF / TFLite
├── third-party/
│   ├── ghidra/                                     # vendored
│   └── cuttlefish/                                 # vendored
├── infra/
│   └── emulator-pool/                              # NEW: Terraform + Ansible
└── meetings/
    ├── 2026-MM-DD-phase5-kickoff.md
    └── 2026-MM-DD-debt-review-G{1..14}.md
```

## 10. Standalone Output

Onboarding handbooks + ADRs + emulator pool. The emulator-pool provisioning is reusable for any future research lab; the CI extensions become permanent gates.

## 11. End-to-End Test

```bash
test -f docs/ADR-Phase5-Kickoff.md
grep -c "^✅ approved by" docs/ADR-Phase5-Kickoff.md  # ≥ 9 (G1–G14 leads)
test -f docs/ADR-0029-dex-arm-lift-strategy.md
nix flake check  # validates LLVM + MLIR + Frida + Cuttlefish + TFLite all reproducible
buck2 run //tests/native-smoke:dex-roundtrip      # DEX → SSA → DEX byte-identity on 100 classes
buck2 run //tests/emulator-smoke:cold-start       # ≤ 120 s (HARD)
buck2 run //tests/dynamic-smoke:frida-attach      # ≤ 2 s
buck2 run //tests/ml-smoke:tflite-hash            # ≤ 500 ms
```

## 12. Exit Checklist

- [ ] G9 staffed: ≥ 4 engineers (HARD)
- [ ] G10 staffed: ≥ 4 (incl. 1 SRE) (HARD)
- [ ] G11 staffed: ≥ 2 (HARD)
- [ ] All onboarding handbooks + charters published
- [ ] Phase-4 debt 100 % resolved or re-classified
- [ ] LLVM + MLIR pinned, reproducible
- [ ] Emulator pool ≥ 32 steady-state, chaos-drill smoke green
- [ ] Frida + eBPF + Cuttlefish stack installed + reproducible
- [ ] TFLite + ART installed + reproducible
- [ ] CI extensions live (native lifter SR, chaos drill, TFLite reproducibility)
- [ ] ADR-Phase5-Kickoff merged
- [ ] ADR-0029 / 0030 / 0031 merged
- [ ] Phase-5 budget approved
- [ ] G10 SRE on-call rotation activated

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P5.2** | G9 + G3 ready to design AXIOM-IR-v0.4 native dialect |
| **P5.3 / P5.4 / P5.5** | LLVM + MLIR pinned; G9 ready |
| **P5.10** | Emulator pool live |
| **P5.14** | TFLite stack ready |
| **P5.9** | G1 + G9 ready to start native-lifter Lean theorems |
| **All P5.x** | Carry-forward debt plan; clean Phase-5 start |
