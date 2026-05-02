# P6.1 — Phase 5 Carry-Forward + Phase 6 Stabilization Kickoff

> No new groups. All 14 groups enter stabilization mode. Strict merge policy enforced. RISC-V CI runner online. 90-day rolling reproducibility / soundness / SLA windows opened. APKAXIOM-Eval-50K corpus locked.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md](../../../README.md) · [../../TECH_STACK.md](../../TECH_STACK.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P6.1 |
| Owner(s) | Project leadership + all G1–G14 leads |
| Duration | Weeks 1–2 |
| Critical-path | **yes** |
| Hard prerequisites | P5.20 (Phase 5 closed) |

## 2. Goal & Scope

A clean Phase-6 start. No hiring, no new groups. Stabilization-mode merge policy enforced. RISC-V CI runner online. 90-day rolling windows opened (reproducibility, soundness regression, verifier SLA). Eval-50K corpus locked. External-audit firm engagement signed.

### In scope
- Stabilization-mode merge policy (`docs/ADR-Phase6-Merge-Policy.md`)
- RISC-V CI runner provisioned (SiFive HiFive Pro P550 or VisionFive 2)
- Eval-50K corpus locked + manifest signed
- External-audit firm engagement signed (T-of-B / NCC / Aleph / Atredis)
- 90-day rolling windows opened: reproducibility / soundness / SLA / fuzzer
- ADR-Phase6-Kickoff
- Per-group stabilization punch-list reviewed + signed
- Phase 6 budget locked
- Press kit drafting underway

### Out of scope
- Per-group stabilization work (P6.2–P6.15)
- 50K eval (P6.16)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P5.20** | Phase 5 closed, ADR-Phase6-Scope, ADR-Phase6-Audit, ADR-Phase6-Eval50K |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **All Phase 1–5 tooling** | (existing, pinned) | Stabilization |
| **RISC-V toolchain (clang / LLVM, gnu-toolchain-rv)** | latest | Cross-arch reproducibility |
| **QEMU-system-riscv64** | latest | Fallback emulated RISC-V CI |
| **SiFive HiFive Pro P550** or **VisionFive 2** | hardware | Native RISC-V CI runner |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **SiFive HiFive Pro P550** | hardware | **Paid** ~$500–1500 (capex) | https://www.sifive.com | Native RISC-V CI |
| **VisionFive 2** (StarFive) | hardware | **Paid** ~$100–200 | https://www.starfivetech.com | Lower-cost alt |
| **QEMU** | tool | **Free** OSS | https://www.qemu.org | Emulated fallback |
| **External audit firm engagement** | service | **Paid** $250–500K | (selected via P5.20 RFP) | ~10-week contract |
| **Cloudflare** | service | **Free tier** + paid | https://www.cloudflare.com | Front for production verifier |
| **HackerOne / Bugcrowd partnership** | service | (existing) | already provisioned | Pilot continued |

**API keys required:** Cloudflare API token (for production deploy in P6.19), audit-firm sandbox creds.

## 6. System Inventory — Have vs Need

### Already present
- ✅ All Phase 5 stack
- ✅ x86_64 + ARM64 CI runners
- ✅ Verifier infra (pilot scale)

### Missing — must install
- ❌ **RISC-V hardware runner** — provision SiFive or VisionFive 2
- ❌ **RISC-V QEMU fallback** — install via apt
- ❌ **External audit firm sandbox** — coordinate with auditor

### Install commands

```bash
# RISC-V QEMU fallback
sudo apt-get install -y qemu-system-misc qemu-user-static

# RISC-V toolchain (Ubuntu 24.04)
sudo apt-get install -y gcc-riscv64-linux-gnu g++-riscv64-linux-gnu \
                        clang-19 lld-19 \
                        binutils-riscv64-linux-gnu

# SiFive HiFive Pro P550 — physical install per vendor instructions
# VisionFive 2 — flash image and connect via SSH
```

## 7. Features & Functions Delivered (Comprehensive)

### Stabilization-mode merge policy (`docs/ADR-Phase6-Merge-Policy.md`)
- Default: `no new features`
- Allowed: safety-critical fix, perf tuning within layer budget, documentation, dependency security update
- Required for any feature merge: leadership ADR + 3 group-lead approvals + audit-trail entry
- Audit log: weekly export to leadership

### RISC-V CI runner
- SiFive HiFive Pro P550 (or VisionFive 2) provisioned + Buck2 + Nix flake bootstrapped
- Reproducibility check: same hermetic build on x86_64 / ARM64 / RISC-V → byte-identical
- QEMU-system-riscv64 fallback for non-hardware-bound checks
- Smoke: build all crates + run unit tests + verify cert byte-identity
- Per-PR reproducibility gate extended to RISC-V (best-effort initially; HARD by P6.14)

### Eval-50K corpus lock
- Manifest signed via cosign
- Per-sample license tracked
- DPO sign-off
- Sample-fetch reproducibility (each cite-able by SHA-256)

### External-audit engagement
- Contract signed
- Sandbox provisioned (read-only mirror of repo + eval cluster)
- NDA in place
- Kickoff meeting scheduled for W4

### 90-day rolling windows
- Reproducibility: 100 % byte-identical CI for 90 days
- Soundness: 0 regressions for 90 days
- Verifier SLA: track readiness for 90-day production-grade window opening in W18 (P6.19)
- Fuzzer disagreements unresolved < 3 in queue, continuous

### Per-group stabilization punch-list
- Per group: top 5 KPIs to drive
- Per item: owner, due date, success criterion
- Reviewed weekly

### Phase-6 kickoff artifacts
- Kickoff meeting minutes
- ADR-Phase6-Kickoff
- ADR-Phase6-Merge-Policy
- ADR-Phase6-RISC-V-CI
- Press kit draft

## 8. KPIs (this sub-phase)

| KPI | HARD |
|---|---|
| RISC-V CI runner online | yes |
| RISC-V smoke green (all crates build, unit tests green) | yes |
| Reproducibility check byte-identical across x86_64 + ARM64 + RISC-V | yes (smoke at this sub-phase; sustained 90 days by P6.20) |
| Eval-50K corpus locked + signed | yes |
| External audit firm contract signed | yes |
| External-audit sandbox provisioned | yes |
| Stabilization-mode merge policy adopted | yes |
| Per-group punch-lists signed | yes |
| 90-day rolling-window dashboards live | yes |
| ADR-Phase6-Kickoff merged | yes |
| Phase 6 budget locked | yes |
| Press kit draft started | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── docs/
│   ├── ADR-Phase6-Kickoff.md
│   ├── ADR-Phase6-Merge-Policy.md
│   ├── ADR-Phase6-RISC-V-CI.md
│   ├── phase-5-carry-forward-resolved.md
│   └── press-kit/                    # NEW
├── infra/
│   └── ci/
│       └── riscv-runner/             # NEW: Terraform + Ansible
├── corpus/
│   └── apkaxiom-eval-50k/            # NEW: manifest only (samples staged separately)
└── meetings/
    ├── 2026-MM-DD-phase6-kickoff.md
    └── 2026-MM-DD-audit-kickoff.md
```

## 10. Standalone Output

ADRs + RISC-V runner + locked corpus + signed audit engagement.

## 11. End-to-End Test

```bash
test -f docs/ADR-Phase6-Kickoff.md
test -f docs/ADR-Phase6-Merge-Policy.md

nix flake check --target riscv64-linux  # RISC-V build reproducible
buck2 build //... --target-platforms //platforms:linux-riscv64

# Cross-arch byte-identity
sha256sum out/x86_64/cert.axc out/aarch64/cert.axc out/riscv64/cert.axc
# Expect: identical

# Eval-50K manifest signed
cosign verify-blob --signature corpus/apkaxiom-eval-50k/manifest.sig corpus/apkaxiom-eval-50k/manifest.toml
```

## 12. Exit Checklist

- [ ] RISC-V CI runner online (HARD)
- [ ] RISC-V smoke green
- [ ] Cross-arch byte-identical smoke (x86_64 + ARM64 + RISC-V)
- [ ] Eval-50K corpus locked + signed (HARD)
- [ ] External audit engagement signed (HARD)
- [ ] External-audit sandbox provisioned
- [ ] Stabilization-mode merge policy live (HARD)
- [ ] Per-group punch-lists signed
- [ ] 90-day rolling windows live
- [ ] ADR-Phase6-Kickoff merged
- [ ] ADR-Phase6-Merge-Policy merged
- [ ] ADR-Phase6-RISC-V-CI merged
- [ ] Phase 6 budget locked
- [ ] Press kit drafting started

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P6.2 .. P6.15** | Per-group stabilization punch-lists |
| **P6.16** | Eval-50K corpus locked |
| **P6.17** | External-audit sandbox + engagement |
| **P6.14** | RISC-V CI baseline |
| **P6.19** | Cloudflare account + cert pinning |
