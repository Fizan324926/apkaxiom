# Soundness Regression Suite — Operational Reference

## Purpose

Every PR that touches Lean theorems or extracted Rust must pass the soundness
regression suite before it can merge. The suite re-verifies all theorems and
runs translation validation on the full corpus. It is fail-closed: no override
path exists.

## Scope

| In scope | Out of scope |
|---|---|
| Lean theorem correctness (no sorry, all proofs check) | Performance regression (separate gate) |
| Translation validation — Lean ↔ Rust ↔ extracted byte-identical | Reproducibility regression (P1.1 gate) |
| Signing extraction correctness (P1.16 unit tests) | Fuzzer health (continuous, not per-PR) |

## Running locally

```bash
make soundness          # full suite (all four gates sequentially)
make soundness-sorry-audit   # grep-only, no toolchain needed, <5s
make soundness-lake     # Lake theorem re-verify (requires nix develop)
make soundness-tv       # Translation validator (requires nix develop)
make soundness-signing  # P1.16 signing tests (Rust only, ~30s)
```

For incremental local use, emit a targeted lake build command first:

```bash
bash ci/soundness/changed-modules.sh   # prints: lake build <touched modules>
```

## Gate breakdown

### 1. Sorry audit (`sorry-audit`)

Runs in <5 seconds with no toolchain.

```bash
grep -rn '\bsorry\b' theorems/ --include='*.lean' --exclude-dir='.lake' \
  | grep -v '^ *--'
```

Any hit immediately exits 1. Lean comment lines (`--`) are excluded. The
`lake-verify` step additionally checks Lake's own output for `uses .sorry`
to catch any sorry that slipped through the grep.

### 2. Lake theorem re-verify (`lake-verify`)

```bash
lake build Apkaxiom
```

Re-checks every theorem in `theorems/Apkaxiom/`. With the mathlib Reservoir
cache warm, this completes in under 5 minutes. Cold (cache miss on
`lake-manifest.json`) takes up to 60 minutes due to mathlib compilation.

The CI workflow caches `.lake/build` keyed on `lake-manifest.json`; a
mathlib upgrade is the only event that triggers a cold run.

### 3. Translation validation (`tv-validate`)

```bash
make p19-gates
```

Runs the P1.9 three-way translation validator:
- 1499 LFH corpus vectors: Lean evaluator ↔ hand-Rust ↔ extracted-Rust
- 299 EOCD corpus vectors
- JSON schema check on all evaluator outputs
- Perf-delta within ±2σ of the committed baseline

Any divergence between the three evaluators exits 1.

### 4. Signing extraction tests (`signing-tests`)

```bash
make p116-tests
```

Runs 17 P1.16 unit tests:
- HACL* SHA-256 KAT (4 NIST FIPS 180-4 vectors)
- libcrux vs RustCrypto sha2 cross-check (11 lengths)
- Ed25519 RFC 8032 test vectors (2 positive + 2 rejection)
- ECDSA-P256 DER sig parser
- HACL* chunked digest baseline match
- v2+v3 and v3.1 fixture APK accepts
- RSA-PKCS1-SHA512 8192-bit key corpus APK

## Timing expectations

| Gate | Cold | Cached |
|---|---|---|
| sorry-audit | <5s | <5s |
| lake-verify | ≤60 min | <5 min |
| tv-validate | <5 min | <5 min |
| signing-tests | <1 min | <1 min |
| **Total** | ≤70 min | <12 min |

The CI workflow hard-limits to 90 minutes (`timeout-minutes: 90`).

## Triage guide

| Failing gate | Root cause | Remediation |
|---|---|---|
| sorry-audit | Bare `sorry` in `.lean` file | Revert the theorem or replace sorry with a real proof |
| lake-verify | Proof broke after mathlib bump or theorem edit | Fix the proof; see mathlib upgrade runbook if bump is the cause |
| tv-validate | Lean evaluator ↔ Rust diverged | Check extraction pipeline; re-run `make p19-gates` locally for details |
| signing-tests | HACL* or RustCrypto regression | Check recent changes to `axiom-crypto-hacl` or `axiom-l1-signing-verified` |

## PR merge policy

The `soundness` job is a required status check on the `main` branch. GitHub
branch protection enforces this. No administrator bypass is configured.
A timeout counts as a failure.

## Deliberate-break test

See `ci/deliberate-break-test/README.md` for the quarterly runbook that
confirms the gate is real and not theatrical.
