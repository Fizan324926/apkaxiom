# P3.11 — UNKNOWN Handling + Abstraction-Refinement Loop

> When L4 says UNKNOWN, refine the abstraction and try again. CEGAR-style refinement loop. Drives UNKNOWN rate down from initial ~25% to target ≤10%.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §9](../../../README.md#layer-4)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P3.11 |
| Owner(s) | G5 |
| Duration | Weeks 12–17 |
| Critical-path | yes (drives the L4 UNKNOWN gate down) |
| Hard prerequisites | P3.8 (single-APK L4), P3.10 (abstract domains) |

## 2. Goal & Scope

A CEGAR-style (Counter-Example Guided Abstraction Refinement) loop that, when L4 returns UNKNOWN, attempts refinement with a stronger abstract domain or with predicate-abstraction hints. Drives the UNKNOWN rate down from the initial ≤25% to the target ≤10% on benign 5K.

### In scope
- `crates/axiom-l4-refinement` — CEGAR loop
- Abstraction-refinement strategies (interval → octagon → polyhedra)
- Counter-example mining from solver UNSAT proofs
- Predicate-abstraction hint generation
- Refinement budget (max iterations, max time)
- Telemetry: which strategy succeeded, how often

### Out of scope
- Symbolic execution itself (P3.8 owns)
- Bisimulation refinement (P3.15)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P3.8** | L4 single-APK base + UNKNOWN markers |
| **P3.10** | Abstract domains for refinement strategies |
| **P3.6** | SMT bridge for re-querying with refined abstractions |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **All Phase-3 SMT stack** | from P3.6 | Re-querying |
| **Abstract-domain library** | from P3.10 | Refinement targets |
| **CEGAR pattern reference** | research | Berkeley/Lal/Reps tradition |

## 5. Third-Party Software, Services, Accounts & API Keys

**No new external dependencies.** Reuses solver pool + abstract-domain library.

## 6. System Inventory — Have vs Need

Nothing new system-level.

## 7. Features & Functions Delivered (Comprehensive)

### CEGAR refinement loop
- Input: an L4 query that returned UNKNOWN
- Output: either Refined-SAT, Refined-UNSAT, or Refined-UNKNOWN (after budget exhausted)
- Strategies tried in order:
  1. **Strengthen numeric domain**: interval → octagon → polyhedra
  2. **Strengthen string domain**: bounded-length → regular → context-free
  3. **Predicate abstraction**: extract predicates from solver-trace, restrict abstraction
  4. **Symbolic re-execution** with the refined abstraction
- Budget: max 3 strategy escalations, max 60 s wall time per query

### Counter-example mining
- When SMT returns SAT with a spurious witness (witness lifts up to abstract state but the abstract state is over-approximate), extract the spurious-witness pattern
- Use the pattern to generate a refinement predicate
- Re-query with refinement

### Predicate-abstraction hints
- Lift solver trace into predicate set
- Hint included in next CHC encoding (helps Spacer)

### Telemetry
- Per-query: which strategies tried, which succeeded, time spent
- Aggregated: strategy-success rate by query class
- Used to inform Phase-4 model improvements

### Public Rust API
- `pub fn refine(query: Query, initial_unknown: Unknown) -> RefinedOutcome`
- `pub enum RefinedOutcome { Sat(Witness), Unsat(UnsatCert), StillUnknown(AbsDomain, Reason) }`

### Documentation
- `docs/cegar-refinement.md`

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Refinement reduces UNKNOWN rate on benign 5K from ~25% to | ≤ 25 % final | ≤ 10 % final |
| Refinement budget never exceeded silently | yes (always returns explicit StillUnknown) | yes |
| Refinement throughput overhead vs base L4 | ≤ 3× | ≤ 1.5× |
| Refinement-strategy success-rate distribution recorded | yes | yes |
| Spurious-witness patterns reduced query-class-wide | yes | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-l4-refinement/
│       ├── Cargo.toml
│       ├── BUCK
│       └── src/
│           ├── lib.rs
│           ├── cegar.rs                  # main loop
│           ├── strategies.rs
│           ├── counter_example_mining.rs
│           ├── predicate_abstraction.rs
│           └── budget.rs
├── tests/refinement/
│   └── benign-5k-eval.rs
└── docs/
    └── cegar-refinement.md               # NEW
```

## 10. Standalone Output

```bash
buck2 build //crates/axiom-l4-refinement --release
buck2 test //tests/refinement:benign-5k-eval
# "Initial UNKNOWN: 22.4%; refined: 8.1%"
```

## 11. End-to-End Test

```bash
buck2 test //crates/axiom-l4-refinement:full-eval
# - Final UNKNOWN ≤ 25% on benign 5K (HARD)
# - Refinement-overhead ≤ 3× base L4 (HARD)
# - No silent budget overflow (HARD)
```

## 12. Exit Checklist

- [ ] CEGAR loop operational
- [ ] All 4 strategies implemented
- [ ] Counter-example mining yields refinement predicates
- [ ] Final UNKNOWN ≤ 25 % on benign 5K (HARD)
- [ ] Throughput overhead ≤ 3× base L4 (HARD)
- [ ] Telemetry recorded; strategy-success-rate distribution documented
- [ ] `docs/cegar-refinement.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P3.18** | Refined L4 measured in E2E |
| **Phase 4 / G7** | Refined certs ship in `.axc` |
| **Phase 5+** | Native-code symbolic execution can plug into the same loop |
