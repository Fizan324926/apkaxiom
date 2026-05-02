# P3.3 — AXIOM-IR-symbolic Dialect Design (Preview)

> The IR dialect for symbolic reasoning. SSA over symbolic values, abstraction-domain markers, UNSAT-certificate carriers. Designed in Phase 3, frozen in Phase 4.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §13.9 (AXIOM-IR)](../../../README.md#beyond-the-12)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P3.3 |
| Owner(s) | G3 + G5 |
| Duration | Weeks 2–6 |
| Critical-path | yes |
| Hard prerequisites | P3.1 |

## 2. Goal & Scope

The AXIOM-IR-symbolic dialect — designed but not frozen in Phase 3. Defines how symbolic values, path conditions, abstraction-domain markers, and UNSAT certificates flow between L4 (G5) and L5 (G6). RFC published, reference Rust types skeleton compiles.

### In scope
- AXIOM-IR-symbolic RFC
- Symbolic value types (`SymVal<T>`, `PathCond`, `Constraint`, `AbsDomain`)
- DEX-symbolic dialect (lifts DEX dialect to symbolic SSA)
- Manifest-symbolic dialect (intent filters as symbolic preds)
- UNSAT-certificate carrier types (DRAT, DRUP, LRAT)
- Reference Rust types skeleton (no implementation in P3.3)
- Lean reflection plan

### Out of scope
- Implementation (P3.7, P3.8 do this)
- Freezing (Phase 4 owns)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P3.1** | G3 + G5 onboarded |
| **P2.9** | AXIOM-IR-v0.2 frozen |
| **P2.2** | AXIOM-IR design pattern (informs symbolic dialect) |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Rust** | 1.95 | Reference types |
| **MLIR docs** | LLVM 19+ | Reference for dialect-design patterns |
| **SMT-LIB 2** | spec | Reference for symbolic-value semantics |
| **DRAT-trim / DRAT format spec** | latest | UNSAT cert carrier reference |
| **PlantUML / Mermaid** | latest | Dialect-flow diagrams |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **DRAT-trim** | UNSAT-cert checker | **Free** OSS | https://github.com/marijnheule/drat-trim | Reference implementation for the cert format we adopt |
| **LRAT-checker** | alternative cert format | **Free** OSS | https://github.com/marijnheule/drat-trim | LRAT is the linearized DRAT successor |
| **MLIR Python bindings (xDSL)** *(reference only)* | reference | **Free** OSS | https://xdsl.dev | For prototyping ideas |

**No new API keys.**

## 6. System Inventory — Have vs Need

### Already present
- ✅ Rust + axiom-ir crate
- ✅ MLIR docs accessible

### Missing — must install
- ❌ **DRAT-trim** — `git clone https://github.com/marijnheule/drat-trim && cd drat-trim && make`

```bash
git clone https://github.com/marijnheule/drat-trim third-party/drat-trim
cd third-party/drat-trim && make
sudo cp drat-trim /usr/local/bin/
```

## 7. Features & Functions Delivered (Comprehensive)

### AXIOM-IR-symbolic RFC (`docs/AXIOM-IR-symbolic-RFC.md`)
- ≥ 50 pages
- **Symbolic value types:**
  - `SymVal<T>` — symbolic value of typed T (Int / String / Object / Reference)
  - `Concrete(T)` — known-concrete value
  - `Symbolic(SymId, AbsDomain)` — fully symbolic with abstraction-domain marker
  - `Approx(T, AbsDomain)` — *typed* over-approximation; never silent
- **PathCond** — first-class path condition; carries the constraint trail leading to a program point
- **Constraint** — typed SMT-style constraint (predicate over `SymVal`s)
- **AbsDomain** — abstraction-domain marker enum (numeric: interval / polyhedra / octagon, string: regular / context-free, type: nominal / structural)
- **DEX-symbolic dialect** — every DEX op becomes a transition between symbolic states
  - `dex.move` → `sym.move`
  - `dex.iget`, `dex.aget` → `sym.heap-load`
  - `dex.invoke-virtual` → `sym.dispatch`
  - `dex.const-string` → `sym.string-const`
  - ...
- **Manifest-symbolic dialect** — intent filters as symbolic predicates
  - `intent.filter` op with predicate-over-Intent-shape
  - `intent.priority` op
  - `intent.action` / `intent.category` / `intent.data-scheme` predicates
- **UNSAT-cert carrier types**
  - `UnsatCert<DRAT>` — DRAT-format clause-deletion proof
  - `UnsatCert<LRAT>` — LRAT (linearized) variant
  - `UnsatCert<Veriz>` — verifiability marker pointing to a checker binary

### Lowering rules
- AXIOM-IR-v0.2 manifest dialect → manifest-symbolic dialect
- AXIOM-IR-v0.2 DEX dialect → DEX-symbolic dialect
- DEX-symbolic → SMT-LIB 2 (consumed by cvc5)

### Lean reflection plan
- `theorems/Apkaxiom/IrSymbolic.lean` — type-set planning
- Soundness theorems planned (formalized in Phase 4)

### Reference Rust types
- `crates/axiom-ir-symbolic/` skeleton — no implementations
- Cap'n Proto schema delta drafted

### Decision log
- ADR-0016 — abstraction-domain set chosen
- ADR-0017 — UNSAT-cert format chosen (DRAT vs LRAT vs newer)
- ADR-0018 — symbolic dialect granularity (per-op vs per-basic-block)

### Diagrams
- AXIOM-IR-symbolic type-graph (graphviz)
- DEX → DEX-symbolic lowering flow (mermaid)
- UNSAT-cert lifecycle (mermaid)

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| RFC length | ≥ 50 pages | ≥ 80 pages |
| Reviewer sign-off | G1, G3, G5, G6, G7 leads | + external SMT-modeling reviewer |
| All abstraction domains documented | yes | + Polyhedra preview |
| All UNSAT-cert formats documented | yes | + zk-SNARK envelope preview |
| ADRs 0016 + 0017 + 0018 merged | 3 | 3 |
| Reference crate skeleton compiles | yes | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── docs/
│   ├── AXIOM-IR-symbolic-RFC.md             # NEW
│   ├── ADR-0016-abstraction-domain-set.md   # NEW
│   ├── ADR-0017-unsat-cert-format.md        # NEW
│   └── ADR-0018-symbolic-dialect-granularity.md  # NEW
├── crates/
│   └── axiom-ir-symbolic/                    # NEW — skeleton only
│       ├── Cargo.toml
│       ├── BUCK
│       └── src/
│           ├── lib.rs
│           ├── value.rs                       # SymVal<T>
│           ├── path_cond.rs
│           ├── constraint.rs
│           ├── abs_domain.rs
│           └── unsat_cert.rs
├── schema/
│   └── axiom_ir_symbolic.capnp                # NEW
├── theorems/Apkaxiom/IrSymbolic.lean          # NEW — planning
├── third-party/drat-trim/                     # vendored
└── diagrams/
    ├── ir-symbolic-types.dot
    └── ir-symbolic-flow.mmd
```

## 10. Standalone Output

```bash
buck2 build //crates/axiom-ir-symbolic       # skeleton compiles
buck2 build //theorems:ir-symbolic-plan      # placeholder theorems
grep -c "^✅ approved by G" docs/sign-offs/P3.3.md  # ≥ 5
```

## 11. End-to-End Test

```bash
# Type skeleton compiles
buck2 build //crates/axiom-ir-symbolic
# Reviewer sign-off
test "$(grep -c '^✅ approved by G' docs/sign-offs/P3.3.md)" -ge 5
```

## 12. Exit Checklist

- [ ] RFC ≥ 50 pages, all abstraction domains documented
- [ ] All UNSAT-cert formats documented
- [ ] G1, G3, G5, G6, G7 lead sign-offs (HARD)
- [ ] Reference Rust skeleton compiles
- [ ] Cap'n Proto schema delta validates
- [ ] ADRs 0016, 0017, 0018 merged
- [ ] Diagrams rendered
- [ ] Lean reflection plan placeholder lands

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P3.7** | DEX-symbolic dialect to encode CHC over |
| **P3.8** | Symbolic-IR types to operate on |
| **P3.10** | AbsDomain markers as the abstraction-domain library's input |
| **P3.12** | UNSAT-cert carrier types |
| **Phase 4 / G7** | Symbolic dialect promoted to AXIOM-IR-v0.3 frozen |
