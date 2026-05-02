# P3.8 — Symbolic Intent Resolver L4 — Single-APK First Cut

> Layer 4 in production for single-APK queries. Returns reachability proof, UNSAT certificate, or explicit UNKNOWN. Never silent over-approximation.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §9](../../../README.md#layer-4)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P3.8 |
| Owner(s) | G5 |
| Duration | Weeks 9–14 |
| Critical-path | yes — the L4 deliverable |
| Hard prerequisites | P3.7 (CHC encoder) |

## 2. Goal & Scope

Layer 4 in production: given an APK's BehaviorSet (from P2.12) + a query (intent), returns either:
- A **reachability proof** (concrete device state + install order witnessing the resolution)
- An **UNSAT certificate** (DRAT cert proving no resolution exists)
- An **UNKNOWN** with explicit abstraction-domain marker

UNKNOWN rate ≤ 25 % on benign 5K (HARD per PHASE_GATES.md §7). Never silent over-approximation.

### In scope
- `crates/axiom-l4` — production crate
- Public API for downstream consumers (Phase 4 G7 cert emitter)
- Reachability-proof construction
- UNSAT-cert linkage to DRAT archive
- Per-query Pyroscope spans
- Error-mode discipline (no panics; UNKNOWN with marker on every fallback)

### Out of scope
- Cross-APK snapshots (P3.9)
- Abstraction refinement (P3.11)
- Bisimulation (P3.15)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P3.7** | CHC encoder |
| **P3.6** | SMT bridge |
| **P3.4** | PM-state model |
| **P2.12** | BehaviorSet (input to L4) |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **All Phase-3 SMT stack** | from P3.6 | Solving |
| **CHC encoder** | from P3.7 | Encoding |
| **Pyroscope** | continuous | Per-query profiling |
| **rkyv / fjall** | latest | Persistent finding archive |

## 5. Third-Party Software, Services, Accounts & API Keys

**No new third-party.** Reuses solver pool + bridge + encoder from prior sub-phases.

## 6. System Inventory — Have vs Need

Nothing new system-level.

## 7. Features & Functions Delivered (Comprehensive)

### Public Rust API
- `pub fn resolve_intent(behavior_set: &BehaviorSet, intent: &Intent, ctx: &ResolutionContext) -> ResolutionOutcome`
- `pub enum ResolutionOutcome { Resolved(Vec<ResolvedComponent>), UnsatProof(UnsatCert), Unknown(AbsDomain, Reason) }`
- `pub struct ResolutionContext { device_state: Option<DeviceState>, caller: PackageId, user_id: UserId, api_level: AndroidVersion, timeout_ms: u32 }`

### Reachability proof construction
- When SAT: build a typed `WitnessExecution { initial_state, install_order, intent_dispatch_trace, resolved_component }`
- Replayable: given the witness, an external verifier can rebuild the state and observe the resolution
- Cryptographically committed via BLAKE3 + signed with Ed25519

### UNSAT-cert linkage
- DRAT cert from solver linked into finding artifact
- Cert provenance: which CHC encoding produced it, which Lean theorem authorizes the encoding
- Stored in `findings/drat-archive/` with content addressing

### UNKNOWN handling
- Every UNKNOWN carries an `AbsDomain` marker (interval / polyhedra / octagon / regular / context-free / nominal / structural)
- Every UNKNOWN carries a `Reason` enum (TimeoutExceeded, MemoryExhausted, AbstractionLimit, ModelExtractionFailed, ...)
- Never panic; never silent over-approximation

### Per-finding outputs
- Reachability witness or UNSAT cert
- Per-finding Merkle commit (BLAKE3)
- Per-finding Pyroscope flamegraph reference
- Solver-history audit trail

### Performance
- Throughput: ≥ 200 queries/sec/16-core
- Per-query p99: ≤ 500 ms
- Memory budget: ≤ 200 MB per query, ≤ 1 GB peak per worker

### Documentation
- `docs/l4-symbolic-resolver.md`

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| L4 query throughput | ≥ 200 q/sec/16-core | ≥ 500 q/sec/16-core |
| L4 query p99 latency | ≤ 500 ms | ≤ 200 ms |
| UNKNOWN rate on benign 5K corpus | ≤ 25 % | ≤ 10 % |
| UNSAT correctness on Malware-1K (vs hand-verified) | 100 % | 100 % |
| Reachability witness replay rate | 100 % (every witness replays cleanly) | 100 % |
| Solver timeout rate | < 5 % | < 1 % |
| Memory per query | ≤ 200 MB | ≤ 80 MB |
| Memory per worker | ≤ 1 GB | ≤ 500 MB |
| Zero panics under fuzzing | yes | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-l4/
│       ├── Cargo.toml
│       ├── BUCK
│       └── src/
│           ├── lib.rs
│           ├── resolver.rs               # main entrypoint
│           ├── witness.rs                # reachability-proof construction
│           ├── unsat_cert.rs
│           ├── unknown.rs                # explicit UNKNOWN
│           ├── context.rs
│           └── error.rs
├── findings/
│   ├── reachability-witnesses/           # NEW
│   └── drat-archive/                     # extended
├── tests/l4-symbolic/                    # NEW
└── docs/
    └── l4-symbolic-resolver.md           # NEW
```

## 10. Standalone Output

```bash
nix develop
buck2 build //crates/axiom-l4 --release
buck2 run //bench:l4-throughput -- --corpus bench-1k
# "Throughput: 280 q/sec on 16-core; p99: 380ms; UNKNOWN: 18.4%"
```

## 11. End-to-End Test

```bash
buck2 test //crates/axiom-l4:full-eval
# - Throughput ≥ 200 q/sec/16-core (HARD)
# - p99 ≤ 500 ms (HARD)
# - UNKNOWN rate ≤ 25% on benign 5K (HARD)
# - UNSAT correctness 100% on Malware-1K (HARD)
# - Reachability witness replay 100% (HARD)
# - Memory ≤ 1 GB per worker (HARD)
```

## 12. Exit Checklist

- [ ] L4 production crate compiles and tests
- [ ] All `ResolutionOutcome` variants covered
- [ ] Reachability witnesses replay cleanly (HARD)
- [ ] UNKNOWN carries AbsDomain + Reason markers
- [ ] Throughput ≥ 200 q/sec/16-core (HARD)
- [ ] p99 ≤ 500 ms (HARD)
- [ ] UNKNOWN rate ≤ 25 % on benign 5K (HARD)
- [ ] UNSAT correctness 100 % on Malware-1K (HARD)
- [ ] No panics under fuzzing
- [ ] `docs/l4-symbolic-resolver.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P3.9** | Cross-APK extends to device-snapshot |
| **P3.11** | UNKNOWN refinement loop layered atop |
| **P3.18** | E2E pipeline measures L4 KPIs |
| **Phase 4 / G7** | Reachability witnesses + UNSAT certs ship in `.axc` |
