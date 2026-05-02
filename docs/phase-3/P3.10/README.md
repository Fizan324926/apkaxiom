# P3.10 — Abstraction-Domain Library (Numeric, String, Type)

> A reusable library of abstract domains: intervals, polyhedra, octagons (numeric); regular, context-free (string); nominal, structural (type). Used by L4 + L5.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §10 (L5)](../../../README.md#layer-5)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P3.10 |
| Owner(s) | G5 + G6 |
| Duration | Weeks 4–12 |
| Critical-path | yes (consumed by P3.11, P3.15) |
| Hard prerequisites | P3.6 (SMT bridge for fixpoint queries) |

## 2. Goal & Scope

A typed library of abstract domains used by both Layer 4 (UNKNOWN refinement) and Layer 5 (bisimulation). Each domain comes with: representation, `join`/`meet`/`widen`, soundness, decidability bounds, and an SMT-encoding that drops back to the bridge from P3.6.

### In scope
- Numeric: Interval, Polyhedra (PPL), Octagon (Apron), ZonotopeQ
- String: Regular, Context-free (in bounded form), Bounded-string lengths
- Type: Nominal, Structural, Refinement-typed (limited)
- `Approx<T>` typed wrapper with abstraction-marker discipline
- Widening / narrowing for fixpoint termination
- SMT round-trip: domain → SMT-LIB 2 → solver → reified result

### Out of scope
- Polynomial domains (Phase 5)
- Probabilistic domains (deferred past v1.0)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P3.6** | SMT bridge for fixpoint queries |
| **P3.3** | AXIOM-IR-symbolic dialect — AbsDomain markers |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **PPL (Parma Polyhedra Library)** | 1.2+ | Polyhedra |
| **Apron** | 0.9.15+ | Octagon + interval + polyhedra (alt) |
| **GMP / MPFR** | system | PPL/Apron deps |
| **Rust** | 1.95 | Library impl |
| **bindgen** | from P1.10 | C-FFI to PPL/Apron |
| **regex / regex-automata** | latest Rust crates | Regular-string domain |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **PPL (Parma Polyhedra Library)** | numeric library | **Free** OSS (GPL — care needed for licensing!) | https://www.bugseng.com/ppl | LGPL/GPL — we link dynamically and document |
| **Apron** | numeric library | **Free** OSS (LGPL) | https://antoinemine.github.io/Apron/ | LGPL — clean for our use |
| **GMP / MPFR** | bignum / float | **Free** OSS (LGPL) | https://gmplib.org / https://www.mpfr.org | system deps |
| **regex-automata** | crate | **Free** OSS | crates.io | Pure Rust |

**Licensing note:** PPL is GPL — we use it via dynamic linking with documented separation. Apron LGPL is the safer default; we prefer Apron and only fall back to PPL where it has features Apron lacks.

## 6. System Inventory — Have vs Need

### Already present
- ✅ Rust + bindgen + GMP (system)

### Missing — must install
- ❌ **PPL** — `apt-get install -y libppl-dev`
- ❌ **Apron** — build from source

### Install commands

```bash
sudo apt-get install -y libppl-dev libgmp-dev libmpfr-dev

# Apron
git clone https://github.com/antoinemine/apron
cd apron && ./configure --no-cxx --no-java --no-ocaml
make -j$(nproc) && sudo make install
```

## 7. Features & Functions Delivered (Comprehensive)

### Public Rust API
- `pub trait AbstractDomain<T> { fn bottom() -> Self; fn top() -> Self; fn join(&self, other: &Self) -> Self; fn meet(&self, other: &Self) -> Self; fn widen(&self, other: &Self) -> Self; fn includes(&self, value: T) -> bool; ... }`
- `pub struct Approx<T, D: AbstractDomain<T>> { value: D, marker: AbsDomainMarker, history: Vec<AbsTransition> }` — typed approximation with provenance

### Numeric domains
- **Interval** — `[lo, hi]` with `Inf`/`-Inf` handling
- **Polyhedra** (via PPL or Apron) — convex hull + linear constraints
- **Octagon** (via Apron) — relational `±x ± y ≤ c` constraints
- **ZonotopeQ** — quasi-zonotope (rational coefficients)
- All carry widening operators with appropriate convergence

### String domains
- **Regular** — represented as DFA via `regex-automata`
- **Context-free** (bounded depth) — for grammar-style strings
- **Bounded-length** — `String[≤ k]` cap

### Type domains
- **Nominal** — exact class name set
- **Structural** — method-signature compatibility class
- **Refinement-typed** (limited) — for known invariants

### Typed `Approx<T, D>` discipline
- Every operation that produces an `Approx` records the transition
- Rust type system prevents accidental "promotion" of `Approx` to concrete value
- AXIOM-IR-symbolic operations consume and produce `Approx<T, D>`

### SMT round-trip
- Each domain provides `to_smtlib2: &Self -> String`
- `from_smt_model: SmtModel -> Self` reconstructs an over-approximation
- Used by L4 fixpoint queries and L5 bisimulation discharge

### Documentation
- `docs/abstract-domains.md` — design, decidability bounds, when-to-use guidance

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| All 4 numeric + 3 string + 3 type domains implemented | yes | yes |
| Fixpoint convergence on canonical loops | 100 % within budget | 100 % |
| `Approx<T, D>` discipline enforced by type system | yes | yes |
| SMT round-trip on each domain | 100 % preserves over-approximation | 100 % |
| Library throughput (per-domain micro-benchmarks) | within 30 % of Apron native | within 10 % |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-abstract-domains/
│       ├── Cargo.toml
│       ├── BUCK
│       └── src/
│           ├── lib.rs
│           ├── trait.rs                  # AbstractDomain trait
│           ├── numeric/
│           │   ├── interval.rs
│           │   ├── polyhedra.rs          # PPL/Apron FFI
│           │   ├── octagon.rs            # Apron FFI
│           │   └── zonotope_q.rs
│           ├── string/
│           │   ├── regular.rs
│           │   ├── context_free.rs
│           │   └── bounded_length.rs
│           ├── type_domain/
│           │   ├── nominal.rs
│           │   ├── structural.rs
│           │   └── refinement.rs
│           ├── approx.rs                 # Approx<T, D>
│           └── smt_roundtrip.rs
├── tests/abstract-domains/
│   └── canonical-fixpoints.rs
└── docs/
    └── abstract-domains.md               # NEW
```

## 10. Standalone Output

```bash
buck2 build //crates/axiom-abstract-domains --release
buck2 test //tests/abstract-domains
# All canonical fixpoints converge; SMT round-trips green
```

## 11. End-to-End Test

```bash
buck2 test //crates/axiom-abstract-domains:full
# - 4 numeric + 3 string + 3 type domains operational (HARD)
# - All canonical fixpoints converge within budget (HARD)
# - SMT round-trip preserves over-approximation 100% (HARD)
# - Type system enforces Approx<T, D> discipline (HARD)
```

## 12. Exit Checklist

- [ ] All 4 numeric domains implemented (Interval, Polyhedra, Octagon, ZonotopeQ)
- [ ] All 3 string domains implemented
- [ ] All 3 type domains implemented
- [ ] `Approx<T, D>` type-state enforced
- [ ] Fixpoint convergence on canonical loops 100 % (HARD)
- [ ] SMT round-trip preserves over-approximation 100 % (HARD)
- [ ] Library throughput within 30 % of Apron native (HARD)
- [ ] Licensing documented (PPL vs Apron split)
- [ ] `docs/abstract-domains.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P3.11** | UNKNOWN refinement uses these domains |
| **P3.15** | Bisimulation discharge proceeds over abstract states |
| **Phase 5 / G9** | Native-code lifter outputs in these domains |
