# P4.1 — Phase 3 Carry-Forward + G7 + G12 + G14 Onboarding + Phase 4 Kickoff

> Land G7 (Proof Systems & Cryptography), G12 (Supply Chain), G14 (Verifier, SDKs & Tooling). Resolve all Phase-3 carry-forward debt. Brief new groups on Lean, AXIOM-IR, BSH, bisim, abstract domains, the verified-supply-chain invariants.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md](../../../README.md) · [../../TECH_STACK.md](../../TECH_STACK.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P4.1 |
| Owner(s) | Project leadership + all Phase-3 groups + new G7 + G12 + G14 |
| Duration | Weeks 1–2 |
| Critical-path | **yes** |
| Hard prerequisites | P3.20 (Phase 3 closed) |

## 2. Goal & Scope

A clean Phase-4 start. G7 (4–5 cryptographers + circuit engineers), G12 (2 supply-chain engineers), G14 (3–4 dev-experience engineers) onboarded. All Phase-3 carry-forward debt resolved. Halo2 / Plonky3 / Binius / Stwo proving keys integrated and pinned. CI gates extended for the new domains.

### In scope
- G7 onboarding (4–5 engineers) — Halo2 / Plonky3 / circuit-design backgrounds
- G12 onboarding (2 engineers) — SLSA / Sigstore / reproducible-build backgrounds
- G14 onboarding (3–4 engineers) — Rust + Wasm + cgo + dev-experience backgrounds
- Carry-forward debt review per Phase-3 group
- ADR-Phase4-Kickoff
- ADR-0021 — zk-SNARK scheme selection per workload
- Phase-4 budget approval
- Hardware ramp: GPU procurement (8 H100/L40S for proving)

### Out of scope
- Implementing Phase-3 debt fixes (flow to right Phase-4 sub-phase)
- `.axc` RFC (P4.2)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P3.20** | Phase 3 closed; carry-forward list; Phase 4 ADR |
| **P3.12** | DRAT cert format pattern |
| **P3.16** | Equiv cert format pattern |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Halo2** | latest from zcash/halo2 | Primary zk-SNARK |
| **Plonky3** | 0.x latest | Alt zk-SNARK |
| **Binius** | latest | Binary-field SNARKs |
| **Stwo** | latest | Post-quantum STARK |
| **sppark** | latest | NVIDIA MSM/NTT kernels |
| **icicle** (Ingonyama) | latest | Cross-platform MSM/NTT |
| **CUDA** | 12.x | NVIDIA GPU runtime |
| **HIP / ROCm** | 6.x | AMD GPU runtime |
| **uniffi** (Mozilla) | 0.27+ | Cross-language bindings |
| **wit-bindgen** | latest | Wasm Component Model |
| **wasm-bindgen** | latest | Wasm-Rust glue |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **Halo2** | zk-SNARK lib | **Free** OSS (MIT/Apache) | https://github.com/zcash/halo2 | Zcash |
| **Plonky3** | zk-SNARK lib | **Free** OSS | https://github.com/Plonky3/Plonky3 | Polygon |
| **Binius** | binary-field SNARKs | **Free** OSS | https://github.com/IrreducibleOSS/binius | Irreducible |
| **Stwo** | STARK | **Free** OSS | https://github.com/starkware-libs/stwo | StarkWare |
| **sppark** | GPU kernels | **Free** OSS (Apache 2.0) | https://github.com/supranational/sppark | Supranational |
| **icicle** | GPU kernels (CUDA + Metal + Vulkan) | **Free** OSS | https://github.com/ingonyama-zk/icicle | Ingonyama |
| **NVIDIA H100 / L40S GPUs** | hardware | **Paid** ~ $25–40K each (capex) or $3–4/hr cloud (~$200–800/mo) | https://lambdalabs.com / https://www.runpod.io | 8 GPUs needed for Phase 4 proving |
| **GitHub team / org** | account | **Paid** (existing) | already provisioned | Add G7 + G12 + G14 |
| **HR / payroll** | service | **Paid** | (org-level) | Outside engineering |
| **Halo2 trusted-setup ceremony participation** | one-off | **Free** | https://zfnd.org/halo2/ | Halo2 doesn't need a trusted setup; mention here for clarity |
| **Patent search service for zk landscape** | service | **Paid** $5–25K | various IP firms | Quarterly legal review |
| **Bug-bounty platform partnership exploration** | service | varies | HackerOne / Bugcrowd | Initial conversations begin in P4.1 to allow Phase-4 pilot in P4.17 |

**Hardware requirement:** GPU procurement is the major Phase-4 capex. 8× H100/L40S = $200–320K outright purchase, OR $1.6–6K/month cloud (cheaper for first 12 months). Recommendation: rent for Phase 4, buy in Phase 5 if utilization sustained.

**No API keys at this sub-phase** beyond GitHub OAuth tokens for new G7/G12/G14 hires + GPU-cloud account credentials.

## 6. System Inventory — Have vs Need

### Already present
- ✅ All Phase-3 stack
- ✅ HACL\* (BLAKE3, Ed25519, RSA, ECDSA, SHA-256)
- ✅ DRAT-trim
- ✅ All SMT solvers

### Missing — must install
- ❌ **CUDA toolkit** — NVIDIA driver + CUDA 12.x SDK
- ❌ **Halo2 / Plonky3 / Binius / Stwo** — Cargo deps + build
- ❌ **sppark / icicle** — clone + build with CUDA
- ❌ **uniffi / wit-bindgen** — Cargo deps

### Install commands

```bash
# CUDA (Ubuntu 24.04)
wget https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2404/x86_64/cuda-keyring_1.1-1_all.deb
sudo dpkg -i cuda-keyring_1.1-1_all.deb
sudo apt-get update && sudo apt-get install -y cuda-toolkit-12-6

# Halo2 / Plonky3 / Binius / Stwo are crates.io / git deps
# Will be added per-crate Cargo.toml in P4.3+

# sppark
git clone https://github.com/supranational/sppark third-party/sppark
cd third-party/sppark && make CUDA=1

# icicle
git clone https://github.com/ingonyama-zk/icicle third-party/icicle
cd third-party/icicle && cargo build --release --features=cuda

# uniffi
cargo install uniffi-bindgen-cli

# wit-bindgen
cargo install wit-bindgen-cli
```

## 7. Features & Functions Delivered (Comprehensive)

### G7 onboarding deliverables
- **G7 onboarding handbook** (`docs/g7-onboarding.md`) — covers AXIOM-IR-v0.2, Lean toolchain, BLAKE3 + HACL\* invariants, DRAT cert format, equiv cert format, the *no silent UNKNOWN* discipline, Halo2 PLONKish arithmetization basics, GPU-acceleration patterns, trusted-setup ceremony procedures.
- **G7 charter** — mission, layer ownership (L6), interfaces with G1/G5/G6/G14, headcount plan to v1.0.
- **G7 first-week tasks per hire**.

### G12 onboarding deliverables
- **G12 onboarding handbook** (`docs/g12-onboarding.md`) — SLSA L4 spec, in-toto attestations, Sigstore, deterministic builds via Bazel/Buck2, F-Droid reference reproducibility.
- **G12 charter** — mission, scope.

### G14 onboarding deliverables
- **G14 onboarding handbook** (`docs/g14-onboarding.md`) — Rust + Wasm + cgo + uniffi + wit-bindgen, the *single source of truth* discipline, dev-experience principles.
- **G14 charter**.

### Carry-forward debt resolution
- Per Phase-3-group meeting + minutes
- Debt rollup: `docs/phase-3-carry-forward-resolved.md`
- Re-classification ADR for items deferring to Phase 5

### zk-SNARK pool integration
- All four zk libraries (Halo2 / Plonky3 / Binius / Stwo) integrated into Buck2 + Nix flake
- Build reproducibility verified across libraries
- Initial benchmarks per library on a toy circuit

### GPU pool plan
- 8× H100/L40S provisioned (cloud or capex)
- sppark + icicle integrated
- CI runs zk-proving smoke test on every G7 PR

### CI extensions
- Halo2 proving-key reproducibility check
- zk-circuit constraint-count regression test
- GPU-acceleration speedup smoke test (CPU vs GPU 10× lower bound)

### Phase-4 kickoff artifacts
- Kickoff meeting minutes
- ADR-Phase4-Kickoff
- ADR-0021 — zk-SNARK scheme selection per workload (Halo2 default, Binius for hash-heavy, Stwo post-quantum, Plonky3 alt)
- Phase-4 communication plan

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| G7 onboarded engineers | ≥ 4 (of 4–5) | 5 |
| G12 onboarded engineers | ≥ 2 (of 2) | 2 |
| G14 onboarded engineers | ≥ 3 (of 3–4) | 4 |
| Carry-forward debt closed or re-classified | 100 % | 100 % |
| All four zk libraries integrated + reproducible builds | yes | yes |
| GPU smoke test green (CPU vs GPU 10× lower bound) | yes | yes |
| ADR-Phase4-Kickoff merged | yes | yes |
| ADR-0021 (zk scheme selection) merged | yes | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── docs/
│   ├── g7-onboarding.md / g7-charter.md      # NEW
│   ├── g12-onboarding.md / g12-charter.md    # NEW
│   ├── g14-onboarding.md / g14-charter.md    # NEW
│   ├── phase-3-carry-forward-resolved.md     # NEW
│   ├── ADR-Phase4-Kickoff.md                 # NEW
│   └── ADR-0021-zk-scheme-selection.md       # NEW
├── flake.nix                                  # extended for zk + CUDA
├── third-party/
│   ├── sppark/                                # vendored
│   └── icicle/                                # vendored
└── meetings/
    ├── 2026-MM-DD-phase4-kickoff.md
    └── 2026-MM-DD-debt-review-G{1..14}.md
```

## 10. Standalone Output

Onboarding handbooks + ADRs + GPU + zk-pool integration. Reusable for any future hire onto G7 / G12 / G14.

## 11. End-to-End Test

```bash
test -f docs/ADR-Phase4-Kickoff.md
grep -c "^✅ approved by" docs/ADR-Phase4-Kickoff.md  # ≥ 9 (G1–G14 leads)
test -f docs/ADR-0021-zk-scheme-selection.md
nix flake check  # validates Halo2 / Plonky3 / Binius / Stwo all reproducible
buck2 run //tests/zk-smoke:cpu-vs-gpu  # 10× speedup on toy circuit
```

## 12. Exit Checklist

- [ ] G7 staffed: ≥ 4 of 4–5 engineers (HARD)
- [ ] G12 staffed: ≥ 2 of 2 (HARD)
- [ ] G14 staffed: ≥ 3 of 3–4 (HARD)
- [ ] All onboarding handbooks + charters published
- [ ] Phase-3 debt 100 % resolved or re-classified
- [ ] All four zk libraries integrated, reproducible
- [ ] GPU pool provisioned (8× H100/L40S)
- [ ] sppark + icicle build green
- [ ] uniffi + wit-bindgen + wasm-bindgen all installed
- [ ] CI extensions live (zk-cert reproducibility, GPU smoke test)
- [ ] ADR-Phase4-Kickoff merged
- [ ] ADR-0021 merged
- [ ] Phase-4 budget approved
- [ ] Bug-bounty pilot conversations initiated

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P4.2** | G7 ready to draft `.axc` RFC |
| **P4.3** | All zk libraries pinned + GPU pool ready |
| **P4.4** | Lean → Halo2 compilation pipeline foundation |
| **P4.16** | G12 ready to start SLSA |
| **P4.17** | Bug-bounty partner conversations underway |
| **All P4.x** | Carry-forward debt plan; clean Phase-4 start |
