# P3.15 — Bounded Bisimulation Engine — Abstract-Domain Composition

> The first principled answer to "is this the same APK, repackaged?" Bisim witness over abstract states. SMT-discharged proof obligations. TP ≥85%, FP <1%.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §10.2 (Bisim)](../../../README.md#layer-5)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P3.15 |
| Owner(s) | G6 |
| Duration | Weeks 10–17 |
| Critical-path | yes |
| Hard prerequisites | P3.10 (abstract domains), P3.13 (BSH RFC; bisim uses BSH as coarse filter) |

## 2. Goal & Scope

A bounded (k-step) bisimulation engine that proves two APKs behaviorally equivalent up to k transitions. Operates on abstract states (per P3.10's library). SMT-discharged proof obligations at each transition (per P3.6's bridge). Outputs a verifiable witness or an explicit divergence report.

### In scope
- `crates/axiom-l5-bisim` — bounded bisimulation engine
- Inter-component communication graph extraction (from BehaviorSet)
- API call-trace abstraction
- k-step bisimulation game over abstract states
- SMT-discharged proof obligations
- Witness emission (P3.16) — preview here

### Out of scope
- Witness format finalization (P3.16 owns)
- Layer 5 unified surface (P3.17)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P3.10** | Abstract domains — bisim discharge runs over them |
| **P3.13** | BSH used as coarse filter (skip bisim if BSH already mismatches) |
| **P3.6** | SMT bridge for proof-obligation discharge |
| **P2.8** | DEX dialect — call-trace extraction from |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **All Phase-3 SMT stack** | from P3.6 | Discharge |
| **Abstract-domain library** | from P3.10 | State representation |
| **petgraph** | latest Rust crate | ICC graph |
| **rkyv / fjall** | latest | Witness archive |
| **Process calculus reference** | research | π-calculus, LOTOS bisimulation literature |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **mCRL2** | reference bisim toolset | **Free** OSS | https://www.mcrl2.org | Reference for some bisimulation algorithms |
| **CADP** | bisim tools | **Free** academic | https://cadp.inria.fr | Reference; not required at runtime |
| **petgraph / Rust ecosystem** | crates | **Free** OSS | crates.io | |

**No new API keys.**

## 6. System Inventory — Have vs Need

### Already present
- ✅ All Phase-3 SMT + abstract-domain library
- ✅ Rust ecosystem

### Missing
- Optional: **mCRL2** for reference comparison — `apt-get install -y mcrl2`

## 7. Features & Functions Delivered (Comprehensive)

### Public Rust API
- `pub fn bisim_check(a: &BehaviorSet, b: &BehaviorSet, k: u32) -> BisimOutcome`
- `pub enum BisimOutcome { Equivalent(Witness), Divergent(DivergencePoint), Inconclusive(Reason) }`
- `pub struct Witness { relation: Vec<(AbstractState, AbstractState)>, transitions: Vec<MatchedTransition>, smt_certs: Vec<UnsatCert> }`
- `pub struct DivergencePoint { step: u32, a_state: AbstractState, b_state: AbstractState, distinguishing_observation: Observation }`

### ICC graph extraction
- For each BehaviorSet: extract inter-component communication graph
- Nodes: components (Activity / Service / Receiver / Provider)
- Edges: intent dispatch (annotated with intent shape)
- Hyper-edges: cross-app intent dispatch

### Call-trace abstraction
- Concrete DEX call traces lifted to abstract traces (per P3.10's domains)
- API renaming via canonical mapping (handles obfuscated names)
- Each step: (call-site, callee-class-canonical, callee-method-canonical, args-abs)

### Bisimulation game
- Standard (1-counter) bisimulation up to k transitions
- Quotient by API renaming (handles obfuscated method names)
- SMT discharge at each step: prove abstract-state pair is bisimilar
- Coarse filter: if BSH-256 mismatches, skip (early reject)

### Witness emission
- DAG of matched (a-step, b-step) pairs with SMT-discharge cert per pair
- Replayable: external verifier rebuilds the abstraction and re-checks
- Cryptographically committed (BLAKE3 + Ed25519)

### Divergence reporting
- When bisim fails: minimal-distinguishing observation
- Used by analyst to understand the difference
- Includes step number + state pair + observation

### Performance optimizations
- BSH-256 prefilter (massive speedup)
- Memoization of abstract-state-pair bisim queries
- Parallel discharge across SMT pool

### Documentation
- `docs/bisim-engine.md`

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Bisim verifications/sec on 1000 known repackaging pairs (Repack-2K) | ≥ 2 pairs/sec/core | ≥ 5 pairs/sec/core |
| Bisim per-pair p99 | ≤ 2 s | ≤ 500 ms |
| Bisim true-positive rate on Repack-2K | ≥ 85 % | ≥ 95 % |
| Bisim false-positive rate on benign pairs | < 1 % | < 0.1 % |
| Witnesses replayable by external verifier | 100 % | 100 % |
| 1000 known repackaging pairs verified in ≤ 10 min total | yes | ≤ 5 min |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-l5-bisim/
│       ├── Cargo.toml
│       ├── BUCK
│       └── src/
│           ├── lib.rs
│           ├── icc_graph.rs              # inter-component comm graph
│           ├── call_trace.rs             # DEX → abstract trace
│           ├── bisim_game.rs              # main loop
│           ├── discharge.rs               # SMT discharge
│           ├── witness.rs
│           ├── divergence.rs
│           └── memo.rs
├── corpus/repack-2k/                      # known repackaging pairs
├── findings/bisim-witnesses/              # NEW
├── tests/bisim/
│   └── repack-2k-eval.rs
└── docs/
    └── bisim-engine.md                    # NEW
```

## 10. Standalone Output

```bash
buck2 build //crates/axiom-l5-bisim --release
buck2 run //bench:bisim-throughput
# "Throughput: 3.4 pairs/sec/core; p99: 1.1s; TP: 91%; FP: 0.6%"
```

## 11. End-to-End Test

```bash
buck2 test //crates/axiom-l5-bisim:repack-2k-eval
# - Throughput ≥ 2 pairs/sec/core (HARD)
# - p99 ≤ 2 s (HARD)
# - TP ≥ 85% on Repack-2K (HARD)
# - FP < 1% on benign pairs (HARD)
# - 100% witness replay (HARD)
# - 1000 pairs in ≤ 10 min total (HARD)
```

## 12. Exit Checklist

- [ ] Bisim engine compiles and tests
- [ ] ICC graph extraction operational
- [ ] Call-trace abstraction respects P3.10 domain discipline
- [ ] SMT-discharge per transition
- [ ] Bisim throughput ≥ 2 pairs/sec/core (HARD)
- [ ] p99 ≤ 2 s (HARD)
- [ ] TP ≥ 85 % on Repack-2K (HARD)
- [ ] FP < 1 % on benign pairs (HARD)
- [ ] Witnesses 100 % replayable (HARD)
- [ ] BSH-256 prefilter integrated
- [ ] `docs/bisim-engine.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P3.16** | Witness format finalized + cert envelope |
| **P3.17** | L5 unified surface combines bisim + BSH + LSH |
| **P3.18** | Bisim E2E measured in Phase-3 gate |
| **Phase 4 / G7** | Bisim witnesses ship in `.axc` certs |
