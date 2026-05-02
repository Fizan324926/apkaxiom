# P3.6 — cvc5 / Z3 / Bitwuzla / Spacer Integration Layer

> The Rust ↔ SMT bridge. Solver-agnostic API across cvc5, Z3, Bitwuzla, Yices2, Spacer, Eldarica. Per-query solver selection by query class. DRAT-style UNSAT cert capture.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §9](../../../README.md#layer-4) · [../../TECH_STACK.md §6](../../TECH_STACK.md#smt)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P3.6 |
| Owner(s) | G5 |
| Duration | Weeks 2–6 |
| Critical-path | yes |
| Hard prerequisites | P3.1 (solvers pinned via Nix) |

## 2. Goal & Scope

A solver-agnostic Rust integration layer that wraps cvc5 / Z3 / Bitwuzla / Yices2 / Spacer / Eldarica behind a uniform trait. Per-query solver selection happens automatically based on query class (QF_BV, QF_LIA, NIA, CHC, etc.). DRAT-style UNSAT certificate capture is mandatory. Solver pool is process-isolated for crash containment.

### In scope
- `crates/axiom-smt-bridge` — solver-agnostic Rust API
- Process-isolated solver pool (each query in a sandboxed child process)
- Per-query timeout + memory budget
- DRAT / LRAT cert capture from cvc5
- Query-class router (selects solver per query)
- SMT-LIB 2 emit + parse infrastructure
- Solver-side instrumentation (Pyroscope spans per query)

### Out of scope
- CHC encoding of intent resolution (P3.7)
- Symbolic resolver itself (P3.8)
- Abstraction-domain library (P3.10)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P3.1** | All 6 solvers pinned via Nix |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **cvc5** | 1.2+ | Primary SMT |
| **Z3** | 4.13+ | Secondary + Spacer host |
| **Bitwuzla** | latest | QF_BV |
| **Yices2** | 2.6+ | Linear-arithmetic shortcut |
| **Spacer** (in Z3) | bundled | CHC |
| **Eldarica** | latest | Alternative CHC |
| **Pono** | latest | Word-level model checking |
| **smt2parser** (Rust crate) | latest | SMT-LIB 2 parsing |
| **bindgen** | from P1.10 | C-FFI to cvc5/Bitwuzla |
| **jni-rs** | latest | JVM bridge for Eldarica |
| **Glommio** | from P1.7 | Async solver-pool runtime |
| **DRAT-trim** | from P3.3 | Cert verification |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **cvc5 / Z3 / Bitwuzla / Yices2 / Spacer / Eldarica / Pono** | solvers | **Free** OSS | already provisioned | All BSD/MIT/Apache |
| **DRAT-trim** | UNSAT-cert checker | **Free** OSS | already provisioned | |
| **smt2parser** | crate | **Free** OSS | crates.io | |
| **z3.rs / cvc5-rs / bitwuzla-rs** | bindings | **Free** OSS | crates.io | Or hand-rolled FFI via bindgen |

**No new API keys.**

## 6. System Inventory — Have vs Need

### Already present
- ✅ All 6 solvers (P3.1)
- ✅ Rust + bindgen + Glommio
- ✅ DRAT-trim

### Missing
- ❌ Cargo deps: `smt2parser`, `z3.rs`, `cvc5-rs` (or build hand-rolled FFI)

## 7. Features & Functions Delivered (Comprehensive)

### Public Rust API
- `pub trait SmtSolver { fn check(&mut self, q: &Query) -> Result<SmtResult, SolverError>; ... }`
- `pub enum SmtResult { Sat(Model), Unsat(UnsatCert), Unknown(AbstractionDomain) }`
- `pub struct Query { theory: Theory, assertions: Vec<Term>, timeout_ms: u32, capture_cert: bool }`
- `pub fn solve(query: Query) -> SmtResult` — front-door router
- `pub fn solve_chc(clauses: Vec<HornClause>) -> ChcResult` — for Spacer/Eldarica

### Query-class router
- Heuristic: `theory == QF_BV` → Bitwuzla; `theory == QF_LIA` → Yices2; `theory == QF_LRA + arrays` → cvc5; `CHC` → Spacer (with Eldarica as fallback)
- Configurable per-deployment
- Captures router-decision history for audit

### Process isolation
- Each query runs in a sandboxed child process (cgroups + seccomp)
- Memory budget enforced (default 1 GB per query)
- Timeout enforced (default 60 s)
- Crash containment: solver crash never kills the parent

### DRAT/LRAT capture
- cvc5's `--produce-proofs` mode harvested
- DRAT trace persisted to `findings/drat-archive/`
- Independent verification via `drat-trim` on randomly sampled queries (1 % rate)

### Performance instrumentation
- OpenTelemetry span per query (theory, solver, time, result)
- Pyroscope continuous profile of solver-host process
- Prometheus metrics: queries/sec, p50/p95/p99 latency, timeout rate, crash rate

### Pool management
- Process-pool sized by NUMA-aware allocation (per-NUMA-node solver workers)
- Backpressure: queue length bounded; new queries rejected when pool saturated
- Glommio thread-per-core for the dispatcher

### Documentation
- `docs/smt-bridge.md` — API, query-class router, process-isolation design

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Solver-bridge throughput (mixed query mix) | ≥ 200 queries/sec on 16-core | ≥ 800 queries/sec |
| Query p99 latency | ≤ 500 ms | ≤ 100 ms |
| Solver-crash containment | 100 % (no parent crash) | 100 % |
| DRAT cert capture rate (when requested) | 100 % | 100 % |
| Random-sample DRAT verification rate | ≥ 1 % of queries | ≥ 5 % |
| All 6 solvers wrapped behind unified trait | yes | yes |
| Translation-validation round-trip on canonical queries | 100 % | 100 % |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-smt-bridge/
│       ├── Cargo.toml
│       ├── BUCK
│       └── src/
│           ├── lib.rs
│           ├── trait.rs                  # SmtSolver trait
│           ├── router.rs                 # query-class router
│           ├── pool.rs                   # process-isolated pool
│           ├── cvc5.rs / z3.rs / bitwuzla.rs / yices2.rs / spacer.rs / eldarica.rs / pono.rs
│           ├── drat_capture.rs
│           └── error.rs
├── findings/drat-archive/                # NEW — captured UNSAT certs
└── docs/
    └── smt-bridge.md                     # NEW
```

## 10. Standalone Output

```bash
buck2 build //crates/axiom-smt-bridge --release
buck2 run //bench:smt-throughput
# "Throughput: 350 q/sec on 16-core (target ≥200); p99=180ms"
```

## 11. End-to-End Test

```bash
buck2 test //crates/axiom-smt-bridge:full
# - Throughput ≥ 200 q/sec on mixed mix (HARD)
# - p99 ≤ 500 ms (HARD)
# - Solver-crash containment 100% (HARD)
# - DRAT capture + DRAT-trim verification on 1% random sample (HARD)
```

## 12. Exit Checklist

- [ ] All 6 solvers wrapped behind unified trait (HARD)
- [ ] Process isolation operational with cgroups + seccomp (HARD)
- [ ] DRAT capture rate 100 % on requested queries (HARD)
- [ ] DRAT-trim verification on 1 %+ random sample (HARD)
- [ ] Throughput ≥ 200 q/sec on 16-core (HARD)
- [ ] p99 ≤ 500 ms (HARD)
- [ ] Crash containment 100 %
- [ ] OpenTelemetry + Pyroscope instrumentation per query
- [ ] `docs/smt-bridge.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P3.7** | CHC encoding submitted to Spacer / Eldarica via this bridge |
| **P3.8** | Symbolic resolver invokes solvers through this bridge |
| **P3.10** | Abstraction-domain library uses solver for fixpoint queries |
| **P3.12** | UNSAT-cert format from this layer is the foundation for `.axc` |
| **Phase 4 / G7** | DRAT certs lifted into zk-SNARK envelopes |
