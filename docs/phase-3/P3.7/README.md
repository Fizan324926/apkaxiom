# P3.7 — CHC Encoding of Intent Resolution

> Encode Android's intent-resolution algorithm as Constrained Horn Clauses. Spacer / Eldarica solve the recursive resolution. UNSAT certificates emitted on every "no resolution" finding.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §9](../../../README.md#layer-4)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P3.7 |
| Owner(s) | G5 |
| Duration | Weeks 6–11 |
| Critical-path | yes |
| Hard prerequisites | P3.5 (Lean intent-resolution algorithm), P3.6 (SMT bridge) |

## 2. Goal & Scope

Translate the Lean intent-resolution algorithm (P3.5) into a CHC encoding solvable by Spacer (primary) or Eldarica (alt). The encoding is the bridge between Lean's reference semantics and the symbolic resolver (P3.8). UNSAT certificates emitted on "no component resolves" findings — independently checkable.

### In scope
- CHC encoder: Lean reference → SMT-LIB 2 / Horn clauses
- Spacer integration on the new encoding
- Eldarica fallback path
- DRAT capture for UNSAT outcomes
- Initial test corpus from P3.5's adversarial fixtures
- Performance optimization: query-batching, predicate-abstraction hints

### Out of scope
- Symbolic resolver UI / API (P3.8)
- Cross-APK snapshots (P3.9)
- Abstraction refinement (P3.11)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P3.5** | Intent-resolution algorithm in Lean (the source of truth) |
| **P3.6** | SMT bridge (CHC submission infrastructure) |
| **P3.4** | DeviceState type (encoded as CHC variables) |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Spacer** (in Z3) | bundled | Primary CHC solver |
| **Eldarica** | latest | Alt CHC solver |
| **Datalog** notation tools | latest | Optional alternative encoding (Soufflé) |
| **smt2parser** | from P3.6 | SMT-LIB 2 generation |
| **Lean 4** | pinned | Source-of-truth reference |
| **Rust** | 1.95 | Encoder |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **Spacer / Z3** | CHC solver | **Free** OSS | already provisioned | |
| **Eldarica** | CHC solver | **Free** OSS | already provisioned | |
| **Soufflé** *(optional Datalog)* | engine | **Free** OSS | https://souffle-lang.github.io | For some optimizations |

**No new API keys.**

## 6. System Inventory — Have vs Need

### Already present
- ✅ Spacer (in Z3)
- ✅ Eldarica
- ✅ Lean reference (P3.5)

### Missing — optional
- ❌ **Soufflé** (Datalog engine) — `apt-get install -y souffle`

## 7. Features & Functions Delivered (Comprehensive)

### CHC encoder (`crates/axiom-l4-chc-encoder`)
- `pub fn encode(state_model: &PmStateModel, intent: &Intent) -> Vec<HornClause>`
- Variables: `Component(comp_id)`, `Resolves(intent, comp_id, user_id)`, `Installed(apk_id)`, `EnabledComponent(...)`, etc.
- Recursion via Spacer's least-fixed-point semantics
- Handles all intent-filter clauses (action, category, data scheme, MIME type, priority)
- Handles signature-permission gates
- Handles multi-user / work-profile predicates

### Optimization hints
- Predicate abstraction hints emitted as comments in SMT-LIB output
- Query batching: multiple intents over the same state share the encoded `DeviceState` predicates

### UNSAT-certificate capture
- Spacer's `--proof` mode harvested
- DRAT-style certificate persisted with provenance (which Lean theorem this encodes)
- Independently verified via DRAT-trim on 1 % random sample

### Datalog alternative (optional)
- Soufflé encoding for very-large-snapshot queries (Phase 3.9 cross-APK)
- Datalog terminates faster on positive-only queries; fallback for Spacer-timeouts

### Lean ↔ CHC consistency check
- For canonical scenarios: Lean reference computes resolution; CHC encoding queries; outputs must match
- ≥ 200 cross-checked scenarios (drawn from P3.5's adversarial fixtures + P3.2's CVE list)

### Documentation
- `docs/chc-encoding.md` — encoding scheme, predicate set, optimization rationale

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| CHC encoder produces solvable Spacer queries on canonical scenarios | 100 % | 100 % |
| Lean ↔ CHC consistency on ≥ 200 fixtures | 100 % | 100 % |
| Spacer query throughput (single-snapshot intent) | ≥ 50 queries/sec/core | ≥ 200 queries/sec/core |
| Spacer timeout rate | < 5 % | < 1 % |
| DRAT cert capture on UNSAT outcomes | 100 % | 100 % |
| Eldarica fallback works on Spacer-timeout cases | ≥ 70 % | ≥ 90 % |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-l4-chc-encoder/
│       ├── Cargo.toml
│       ├── BUCK
│       └── src/
│           ├── lib.rs
│           ├── encoder.rs                # the main translator
│           ├── predicates.rs             # PM-state predicate set
│           ├── intent_filter.rs
│           ├── priority.rs
│           ├── signature_perm.rs
│           ├── multi_user.rs
│           └── optimizations.rs          # batching, hints
├── corpus/chc-cross-check/                # 200+ Lean ↔ CHC fixtures
├── findings/drat-archive/                 # extended
└── docs/
    └── chc-encoding.md                    # NEW
```

## 10. Standalone Output

```bash
nix develop
buck2 build //crates/axiom-l4-chc-encoder --release
buck2 test //tests/chc-cross-check
# "200/200 fixtures Lean ↔ Spacer agree"
buck2 run //bench:chc-throughput
# "Throughput: 80 q/sec/core; timeout rate: 2.3%"
```

## 11. End-to-End Test

```bash
buck2 test //crates/axiom-l4-chc-encoder:full
# - Lean ↔ CHC consistency 100% on 200+ fixtures (HARD)
# - Spacer throughput ≥ 50 q/sec/core (HARD)
# - Timeout rate < 5% (HARD)
# - DRAT cert on every UNSAT outcome (HARD)
```

## 12. Exit Checklist

- [ ] CHC encoder lands; ≥ 200 fixtures Lean ↔ Spacer agree (HARD)
- [ ] All intent-filter / priority / signature-perm / multi-user predicates encoded
- [ ] Spacer throughput ≥ 50 q/sec/core (HARD)
- [ ] Spacer timeout < 5 % (HARD)
- [ ] Eldarica fallback ≥ 70 % on timeouts (HARD)
- [ ] DRAT cert capture 100 % on UNSAT (HARD)
- [ ] DRAT-trim verifies sample certs (HARD)
- [ ] `docs/chc-encoding.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P3.8** | Single-APK symbolic resolver consumes this encoder |
| **P3.9** | Cross-APK snapshots reuse same predicates with extended state |
| **P3.11** | UNKNOWN-refinement loop layered atop |
| **P3.12** | DRAT certs are the input to cert envelope |
