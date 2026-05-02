# P1.2 — Lean 4 Toolchain & Extraction Prototype

> Lean 4 + mathlib4 vendored. First "hello" theorem re-verifies on CI. Lean → Rust extraction round-trips on a trivial example.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../README.md](../../README.md) · [../../TECH_STACK.md](../../TECH_STACK.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P1.2 |
| Owner(s) | G1 (Formal Methods Core) |
| Duration | Weeks 2–4 |
| Critical-path | **yes** — gates every Lean theorem in P1.5+ |
| Hard prerequisites | P1.1 (Nix flake slot for Lean) |

## 2. Goal & Scope

A pinned Lean 4 toolchain, mathlib4 vendored at a specific commit, and a "hello, world" Lean theorem that builds and re-verifies on CI in under 10 minutes. The extraction prototype produces a Rust file from a trivial Lean function and proves operational equivalence on a single test input.

This is **not** the real ZIP-layer formalization — that is P1.5/P1.6. It is the *bring-up* that proves the Lean toolchain plus our extraction pipeline work end-to-end on a trivial case before we invest in real theorems.

### In scope
- `lean-toolchain` file pinning Lean 4 4.x.y.
- `lakefile.toml` declaring mathlib4 dependency.
- One module `theorems/Apkaxiom/Hello.lean` with a single theorem.
- Lean → Rust prototype on a `Nat → Nat` function.
- Translation-validator harness skeleton (real validation lands in P1.9).
- mathlib4 build cache via Cachix.

### Out of scope
- Real ZIP formalization (P1.5).
- Real signing-block formalization (P1.11).
- Production extraction pipeline (P1.9).
- AOSP differential check (P1.5+).

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P1.1** | `flake.nix` with a slot for Lean 4 toolchain; Buck2 workspace; Cachix binary cache |

## 4. Required Tools, Libraries, and Languages

| Tool | Pinned version | Purpose |
|---|---|---|
| **Lean 4** | 4.x.y, pinned via `lean-toolchain` | Theorem prover |
| **Lake** | bundled with Lean 4 | Lean's package manager / build tool |
| **elan** | latest | Lean toolchain manager (host bootstrap) |
| **mathlib4** | pinned commit SHA | Lean math library — Phase-1 dependencies (`Std`, basic data structures) |
| **Rust** | 1.95+ (already pinned by P1.1) | Extraction target |
| **OCaml** | 4.14+ via `opam` | Host language for Lean tactics; needed only for advanced extraction work |
| **opam** | 2.x | OCaml package manager |
| **VSCode** + **Lean 4 extension** | latest | Local dev experience (not required on CI) |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL / Account | Notes |
|---|---|---|---|---|
| **Lean Prover** | proof assistant | **Free** OSS (Apache 2.0) | https://leanprover.github.io | Microsoft Research / community |
| **mathlib4** | Lean math library | **Free** OSS (Apache 2.0) | https://github.com/leanprover-community/mathlib4 | Largest formalization corpus |
| **Lean community Zulip** | discussion / Q&A | **Free** | https://leanprover.zulipchat.com | Where to get help; account = email |
| **Lean Reservoir** (mathlib4 cache) | binary cache for olean files | **Free** | https://reservoir.lean-lang.org | Speeds mathlib4 builds 10–50× |
| **Cachix** | Nix binary cache | Free 5 GB / Paid $5+/mo | https://www.cachix.org | Optional layer over Reservoir |
| **F* (FStar)** | for HACL\* and EverParse work later | **Free** OSS | https://fstar-lang.org | Not needed in P1.2 but pinned now to avoid Phase-3 churn |

**No API keys at this sub-phase.** Lean Reservoir is anonymous read-only; mathlib4 is public; Cachix free tier is sufficient.

## 6. System Inventory — Have vs Need

### Already present
- ✅ Rust 1.95 / cargo
- ✅ git, gh
- ✅ Nix from P1.1

### Missing — must install
- ❌ **elan** (bootstraps Lean 4 / Lake)
- ❌ **lean** / **lake** (managed by elan)
- ❌ **opam** (OCaml package manager — for Lean tactics work)
- ❌ **OCaml 4.14+** (via opam)

### Install commands

```bash
# 1) elan (Lean toolchain manager)
curl -sSf https://raw.githubusercontent.com/leanprover/elan/master/elan-init.sh | sh -s -- -y --default-toolchain none
source $HOME/.elan/env
# elan auto-fetches Lean 4 when 'lean-toolchain' is read in repo

# 2) opam + OCaml
sudo apt-get install -y opam
opam init --bare --disable-sandboxing -y
opam switch create 4.14.2
eval $(opam env)

# 3) Verify
lean --version    # should print Lean 4.x.y once toolchain is pinned in repo
ocaml --version   # 4.14.2
```

After bootstrap, the `flake.nix` from P1.1 is updated to provide Lean/Lake/elan/opam through Nix, and host installs are not the source of truth.

### Adding Lean to flake.nix (delta from P1.1)

```nix
# flake.nix excerpt
{
  inputs.lean4.url = "github:leanprover/lean4/v4.x.y";
  outputs = { self, nixpkgs, lean4 }: {
    devShells.default = nixpkgs.mkShell {
      buildInputs = [
        lean4.packages.${system}.lean-all
        nixpkgs.legacyPackages.${system}.opam
        nixpkgs.legacyPackages.${system}.ocaml
        # ... plus everything from P1.1
      ];
    };
  };
}
```

## 7. Working Directory & Files Produced

```
apkaxiom/
├── lean-toolchain                     # NEW — single line: "leanprover/lean4:v4.x.y"
├── lakefile.toml                      # NEW — Lake project manifest
├── theorems/
│   ├── BUCK                           # NEW — Buck2 rule wrapping Lake
│   ├── Apkaxiom.lean                  # NEW — root namespace
│   └── Apkaxiom/
│       └── Hello.lean                 # NEW — first theorem
├── tools/
│   └── lean-to-rust/                  # NEW — extraction prototype
│       ├── Cargo.toml
│       └── src/main.rs
├── crates/
│   └── axiom-extract-hello/           # NEW — extracted Rust
│       ├── Cargo.toml
│       └── src/lib.rs                 # auto-generated from Hello.lean
└── docs/
    ├── lean-setup.md                  # NEW
    └── extraction-architecture.md     # NEW (initial draft)
```

### Hello.lean (the actual content)

```lean
namespace Apkaxiom.Hello

/-- A trivial proposition for bring-up. -/
theorem zero_is_zero : (0 : Nat) = 0 := rfl

/-- A trivial computable function. To be extracted. -/
def double (n : Nat) : Nat := 2 * n

theorem double_correct : ∀ n, double n = n + n := by
  intro n
  unfold double
  omega

end Apkaxiom.Hello
```

The extractor produces:

```rust
// crates/axiom-extract-hello/src/lib.rs (auto-generated)
pub fn double(n: u64) -> u64 { 2 * n }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn double_zero() { assert_eq!(double(0), 0); }
    #[test] fn double_seven() { assert_eq!(double(7), 14); }
}
```

## 8. Standalone Output

A buildable Lean module + auto-generated Rust crate proving the toolchain works end-to-end:

```bash
nix develop
buck2 build //theorems:hello              # Lean re-verify
buck2 build //tools/lean-to-rust:bin       # build extractor
buck2 run //tools/lean-to-rust -- theorems/Apkaxiom/Hello.lean \
   > crates/axiom-extract-hello/src/lib.rs
buck2 test //crates/axiom-extract-hello    # extracted Rust passes its tests
```

## 9. End-to-End Test

```yaml
# .github/workflows/lean-bringup.yml
jobs:
  lean-hello:
    steps:
      - uses: actions/checkout@v4
      - uses: nixbuild/nix-quick-install-action@v27
      - run: nix develop --command lean --version
      - run: nix develop --command lake build       # builds Hello.lean
      - run: nix develop --command buck2 test //crates/axiom-extract-hello
      - run: |
          # mathlib4 cache hit rate must exceed 90%
          nix develop --command lake exe cache get
```

The CI step times the full Lean+extract+test cycle. **HARD: ≤ 10 minutes total.**

## 10. Exit Checklist

- [ ] `lean-toolchain` pinned to specific Lean 4 release
- [ ] `lakefile.toml` declares `Std` + `mathlib` (mathlib4) dependency, pinned commit
- [ ] `Hello.lean` re-verifies on CI in ≤ 10 min
- [ ] Mathlib4 cache hit rate ≥ 90% on warm CI runs
- [ ] Extraction prototype produces compiling Rust from `double` function
- [ ] Extracted Rust test (`double_zero`, `double_seven`) passes
- [ ] Translation-validation harness skeleton merged (no real validation yet)
- [ ] `flake.nix` updated to provide Lean via Nix (not host-installed)
- [ ] G1 onboarding doc `docs/lean-setup.md` published

## 11. Hand-Off

| Consumed by | What they need |
|---|---|
| **P1.4** | Lean type-system primitives for AXIOM-IR's manifest-dialect type formalization |
| **P1.5** | Working `lake build` of `Apkaxiom.*` modules; `Buck2` rule wrapping Lake |
| **P1.6** | Same as P1.5 |
| **P1.9** | Extraction prototype as the basis for the production translation validator |
| **P1.11** | Lean toolchain in CI; mathlib cache |
| **P1.17** | Lean re-verify wired into the soundness regression CI gate |
