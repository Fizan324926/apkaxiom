# P3.1 — Phase 2 Carry-Forward + G5 + G6 Onboarding + Phase 3 Kickoff

> Land G5 (Symbolic Execution & Intent Resolver) and G6 (Equivalence & Fingerprinting). Resolve every Phase-2 carry-forward item. Brief the new groups on Lean, AXIOM-IR-v0.2, BehaviorSet, and the verified-supply-chain invariants.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md](../../../README.md) · [../../TECH_STACK.md](../../TECH_STACK.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P3.1 |
| Owner(s) | Project leadership + all Phase-2 groups + new G5 + new G6 |
| Duration | Weeks 1–2 |
| Critical-path | **yes** |
| Hard prerequisites | P2.20 (Phase 2 closed) |

## 2. Goal & Scope

A clean Phase-3 start. G5 (Symbolic Execution + Intent Resolver) and G6 (Equivalence + Fingerprinting) onboarded, all Phase-2 carry-forward debt resolved, and a kickoff that anchors them in the existing Lean / AXIOM-IR / verified-supply-chain invariants.

### In scope
- G5 onboarding (4–5 engineers): program-analysis, SMT modeling, KLEE/angr-style backgrounds
- G6 onboarding (3 engineers): process calculus / refinement-types / abstract-interpretation backgrounds
- Carry-forward debt review per Phase-2 group
- Phase-3 kickoff meeting + decision log
- Phase-3 budget approval; SMT-solver / DiskANN / KVM scaling planning
- Sign Phase-3 charter and infrastructure ramp plan

### Out of scope
- Implementing Phase-2 debt fixes (flow to right Phase-3 sub-phase)
- AOSP archaeology (P3.2 owns)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P2.20** | Phase 2 closed; carry-forward debt list; Phase 3 ADR |
| **P2.10** | Schrödinger formalization (G5 reasons over BehaviorSet) |
| **P2.9** | AXIOM-IR-v0.2 frozen — G5 lifts to AXIOM-IR-symbolic |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Lean 4 + mathlib4** | pinned | Onboarding training |
| **cvc5** | 1.2+ | Primary SMT (G5 onboarding) |
| **Z3** | 4.13+ (HAVE 4.12) | Secondary SMT (upgrade pin) |
| **Bitwuzla** | latest | QF_BV solver |
| **Yices2** | 2.6+ | Fast linear arithmetic |
| **Spacer** (in Z3) | bundled | CHC solver |
| **Eldarica** | latest | Alternative CHC |
| **GitHub team / org** | already provisioned | Add G5 + G6 members |
| **Buildkite agents** | already provisioned | Allocate G5 + G6 share |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **cvc5** | SMT solver | **Free** OSS (BSD-3) | https://cvc5.github.io | Primary engine for G5 |
| **Z3** | SMT solver | **Free** OSS (MIT) | https://github.com/Z3Prover/z3 | HAVE on host, upgrade pin |
| **Bitwuzla** | SMT solver | **Free** OSS (MIT) | https://bitwuzla.github.io | Fastest QF_BV in 2026 |
| **Yices2** | SMT solver | **Free** OSS | https://yices.csl.sri.com | SRI International |
| **Eldarica** | CHC solver | **Free** OSS | https://github.com/uuverifiers/eldarica | Uppsala research |
| **Pono** | model checker | **Free** OSS | https://github.com/upscale-project/pono | Stanford research; for word-level model checking |
| **Coordinated-disclosure CNA partner** | CVE filing | **Free** | continuation from P1.13 | For Phase-3 zero-days from cross-APK analysis |
| **AndroZoo** | corpus | **Free academic** | already provisioned | API key from P1.3 |
| **Hetzner / OVH compute** | hardware | **Paid** ~ €200–600/mo | already provisioned | Solver pool needs more cores |

**No new API keys at this sub-phase** beyond GitHub OAuth tokens for new G5 + G6 hires.

## 6. System Inventory — Have vs Need

### Already present (verified at M0)
- ✅ Z3 4.12 (HAVE)
- ✅ Lean / Lake / mathlib4
- ✅ Phase-1 + Phase-2 software stack

### Missing — must install
- ❌ **cvc5** — `apt-get install -y cvc5` or build from source for latest
- ❌ **Bitwuzla** — build from source
- ❌ **Yices2** — `apt-get install -y yices`
- ❌ **Eldarica** — JAR download
- ❌ **Pono** — build from source

### Install commands

```bash
# cvc5 (latest from upstream)
git clone https://github.com/cvc5/cvc5
cd cvc5 && ./configure.sh production --auto-download && cd build && make -j$(nproc) && sudo make install

# Bitwuzla
git clone https://github.com/bitwuzla/bitwuzla
cd bitwuzla && ./configure.py && cd build && ninja && sudo ninja install

# Yices2
sudo apt-get install -y yices2

# Eldarica
mkdir -p ~/tools/eldarica
curl -L https://github.com/uuverifiers/eldarica/releases/latest/download/eldarica-bin-2.1.tar.gz \
  | tar -xz -C ~/tools/eldarica

# Pono
git clone https://github.com/upscale-project/pono
cd pono && ./contrib/setup-smt-switch.sh && ./configure.sh && cd build && make -j$(nproc)
```

After install, all solvers are pinned via Nix flake (G13 update) and host-installed binaries are not the source of truth.

## 7. Features & Functions Delivered (Comprehensive)

### G5 onboarding deliverables
- **G5 onboarding handbook** (`docs/g5-onboarding.md`) — covers AXIOM-IR-v0.2, Schrödinger BehaviorSet semantics, BLAKE3 + HACL\* invariants, Lean toolchain, code-review norms, SMT-LIB 2 basics, the *no silent UNKNOWN* discipline.
- **G5 group charter** — mission, layer ownership (L4), interfaces with G1/G2/G3/G4/G6, headcount plan to v1.0.
- **G5 first-week tasks per hire** — paired-programming, mathlib4 mini-tutorial, paired SMT-LIB 2 walkthroughs, documented PR mentor.

### G6 onboarding deliverables
- **G6 onboarding handbook** (`docs/g6-onboarding.md`) — covers BSH design philosophy, abstract interpretation, refinement types, bisimulation in process calculus, hash-collision analysis, LSH theory, DiskANN.
- **G6 group charter** — mission, layer ownership (L5), interfaces, headcount plan.
- **G6 first-week tasks** — same paired-programming structure as G5.

### Carry-forward debt resolution
- **Debt review meeting per Phase-2 group** — minuted, action items assigned to specific Phase-3 sub-phases.
- **Debt rollup document** (`docs/phase-2-carry-forward-resolved.md`) — every item from P2.20 with current status.
- **Re-classification ADR** for debt that legitimately defers to Phase 4.

### Solver pool planning
- **Compute capacity plan** — how many cores for solver pool? (Spacer/cvc5 are CPU-bound; budget for ~ 2× existing nodes during Phase 3.)
- **Solver-pinning ADR** (ADR-0014) — cvc5 commit, Z3 commit, Bitwuzla commit, Yices2 version, Spacer version, Eldarica version. All pinned via Nix flake.
- **Timeout discipline document** — every query has a wall-time budget; default 60 s in development, 5 s in production. CI guard prevents unbounded queries.

### Phase-3 kickoff artifacts
- **Kickoff meeting minutes** with decisions on: scope, hiring, infrastructure ramp, paper schedule.
- **Phase-3 communication plan** — internal stand-up cadence, weekly all-hands, paper-writing schedule.

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| G5 onboarded engineers | ≥ 4 (of 4–5) | 5 of 5 |
| G6 onboarded engineers | ≥ 2 (of 3) | 3 of 3 |
| Carry-forward debt items closed or re-classified | 100 % | 100 % |
| All 6 solvers (cvc5, Z3, Bitwuzla, Yices2, Eldarica, Pono) pinned via Nix | yes | yes |
| Solver-timeout discipline doc + CI guard merged | yes | yes |
| ADR-Phase3-Kickoff merged | yes | yes |
| ADR-0014 (solver pinning) merged | yes | yes |
| Phase-3 kickoff sign-off | by leadership + all group leads | same |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── docs/
│   ├── g5-onboarding.md                  # NEW
│   ├── g5-charter.md                     # NEW
│   ├── g6-onboarding.md                  # NEW
│   ├── g6-charter.md                     # NEW
│   ├── phase-2-carry-forward-resolved.md # NEW
│   ├── solver-timeout-discipline.md      # NEW
│   ├── ADR-Phase3-Kickoff.md             # NEW
│   └── ADR-0014-solver-pinning.md        # NEW
├── flake.nix                             # extended — solver pins added
└── meetings/
    ├── 2026-MM-DD-phase3-kickoff.md      # NEW
    └── 2026-MM-DD-debt-review-G{1..14}.md
```

## 10. Standalone Output

The onboarding handbooks + ADRs. Reusable for any future hire onto G5 / G6.

## 11. End-to-End Test

Coordination-heavy; "test" = sign-off + verification:

```bash
# Verification
test -f docs/ADR-Phase3-Kickoff.md
grep -c "^✅ approved by" docs/ADR-Phase3-Kickoff.md  # ≥ 7 (G1-G8 + G13 leads)
test -f docs/ADR-0014-solver-pinning.md
nix flake check  # validates all solvers reproduce
```

## 12. Exit Checklist

- [ ] G5 staffed: ≥ 4 of 4–5 engineers onboarded
- [ ] G6 staffed: ≥ 2 of 3 engineers onboarded
- [ ] G5 onboarding handbook + charter published
- [ ] G6 onboarding handbook + charter published
- [ ] Phase-2 carry-forward debt 100 % closed or re-classified
- [ ] All 6 solvers pinned via Nix flake
- [ ] Solver-timeout discipline doc merged + CI guard active
- [ ] ADR-Phase3-Kickoff merged
- [ ] ADR-0014 (solver pinning) merged
- [ ] Phase-3 kickoff meeting minuted and signed
- [ ] Phase-3 budget approved

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P3.2** | AOSP archaeology kickoff bandwidth |
| **P3.3** | G3 + G5 ready to design AXIOM-IR-symbolic dialect |
| **P3.6** | All solvers pinned and available |
| **P3.13** | G6 ready to start BSH RFC |
| **All P3.x** | Carry-forward debt plan; clean Phase-3 start |
