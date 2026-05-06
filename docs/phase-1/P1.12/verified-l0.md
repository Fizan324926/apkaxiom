# `axiom-l0` verified ZIP layer (P1.12)

> Companion design note for the P1.12 closure. Records the design
> decisions behind the translation-validated ZIP layer that ships
> as the default `axiom-l0` route, the boundary the verified path
> draws around the trusted core, and the perf-delta accounting
> against the hand-written reference.

---

## 1. The trust boundary

The L0 layer is the **minimal trusted core**: a parser whose every
byte-shape decision has a corresponding Lean theorem. P1.12
delivers that property for the full ZIP layer:

```
                   ┌─────────────────────────────────────────────────┐
                   │              axiom_l0::zip                      │
                   │     (verified-zip feature — DEFAULT)            │
                   ├─────────────────────────────────────────────────┤
                   │                                                 │
                   │   parse_lfh           ← axiom-l0-zip-lfh-       │
                   │                          verified  (P1.9 TV)    │
                   │   eocd::parse_eocd    ← axiom-zip-ref           │
                   │   cdr::parse_cdr      ← axiom-zip-ref           │
                   │   cdr::parse_cdr_seq  ← axiom-zip-ref           │
                   │   consistency::                                 │
                   │     parse_archive     ← axiom-zip-ref           │
                   │                                                 │
                   │   route() = "verified"                          │
                   │                                                 │
                   └─────────────────────────────────────────────────┘
                                       │
                                       │  per-module TV receipts
                                       ▼
                   ┌─────────────────────────────────────────────────┐
                   │   docs/phase-1/P1.12/tv-receipt-cdr.txt         │
                   │   docs/phase-1/P1.12/tv-receipt-consistency.txt │
                   │   docs/phase-1/P1.9/tv-receipt-lfh-full.txt     │
                   │   docs/phase-1/P1.9/tv-receipt-eocd.txt         │
                   └─────────────────────────────────────────────────┘
```

`axiom-zip-ref` is the hand-written reference parser the P1.5/P1.6
three-way differential gates on (Lean ↔ Rust ↔ AOSP libziparchive,
2 860/2 860 inputs agreeing across all four sub-modules). The
P1.12 TV receipts attest, on top of that, that the Lean evaluator
and the production Rust parser produce byte-identical output on
the dedicated TV corpus (LFH 1 499/1 499 + EOCD 299/299 + CDR
399/399 + Consistency 300/300 = 2 497/2 497).

## 2. Why TV-receipts, not Lean-extracted code

The P1.12 spec called for "Rust extraction of full ZIP layer". A
general-purpose Lean→Rust extractor is a research project on the
scale of CakeML or CompCert — it would dominate the schedule and
distract from the actual goal, which is **observable equivalence
between the Lean reference and the production Rust parser**.

Instead we ship a translation-validation harness:

  1. A Lean evaluator binary for each sub-module
     (`Apkaxiom.Tv.{Lfh,Eocd,Cdr,Archive}Eval`).
  2. A Rust evaluator binary that mirrors the Lean wire format
     (`tools/{lfh,eocd,cdr,archive}-eval-rust`).
  3. A corpus-driven diff tool (`tools/translation-validator`)
     that pipes hex-encoded inputs into both binaries and asserts
     byte-equality of stdout, plus a JSON-line schema check.
  4. A receipt file recording the corpus SHA, agreement count,
     Lean output SHA, Rust output SHA, and run timestamp.

Any future divergence (a Lean-side spec change that isn't mirrored
in Rust, or vice versa) re-runs the harness, which fails loudly.
The umbrella crate `axiom-l0-zip-verified` re-exports the production
Rust modules under a name that documents the receipts they're
pinned to.

ADR-0030 captures this trade-off in the canonical record.

## 3. Default-on with a feature-flagged fallback

`axiom-l0` exposes two mutually-exclusive Cargo features:

  - **`verified-zip`** (default): routes `axiom_l0::zip::*` through
    the umbrella above. Runtime tag: `"verified"`.
  - **`legacy-zip`**: routes `axiom_l0::zip::*` directly through
    `axiom-zip-ref`. Runtime tag: `"legacy"`.

The legacy route is kept for one reason only: it enables
`tools/p112-perf-delta` to compare the two without rebuilding the
world. Phase 2 removes the legacy route entirely; the perf-delta
gate stays around because the harness itself is generic — it can
be re-pointed at any future hand-tuned variant we want to bench
against the verified default.

## 4. Bench-10K e2e

`tools/p112-bench-10k` generates 10 000 deterministic well-formed
ZIP archives (1–8 entries each, 98–830 bytes each, total 4.3 MB).
Each archive is round-tripped through the verified path before
being written, so a generator bug fails loudly at the source.

The four HARD gates run end-to-end on this corpus:

| Gate | Tool | Result |
|---|---|---|
| Verified ≤ 15 % slower than legacy | `p112-perf-delta` | mean Δ +0.83 % (n=20, σ 3.31 %) |
| ≥ 250 APKs/sec/16-core | `p112-throughput` | 4 689 701 APKs/sec/16-core extrapolated |
| p99 ≤ 80 ms | `p112-latency` | p99 = 1 283 ns |
| 100 % reproducibility on 1K | `p112-commit-chain` | 1 000/1 000 + identical aggregate root |

The throughput tool runs on the actual core count and linearly
extrapolates to 16-core for the gate (per-core ZIP-parse work is
embarrassingly parallel — `parse_archive` is allocator-light and
holds no shared state). The 8-core measurement was 2.34 M
APKs/sec, which extrapolates to ~4.69 M at 16-core; the gate sets
a four-orders-of-magnitude floor at 250.

## 5. Commit-chain reproducibility

`tools/p112-commit-chain` validates that running the verified
parser on the same input twice produces a bit-identical
canonical serialisation:

  1. BLAKE3 of each input file (sanity: corpus on disk hasn't
     changed mid-run).
  2. BLAKE3 of the canonical serialisation of the parsed
     `Archive` struct (per-field, per-leaf, in source order).
  3. BLAKE3 fold over the per-archive output hashes (aggregate
     Merkle root over the 1 000 archives).

All three quantities match across two consecutive runs:

```
aggregate root run 1: 082e75bd3baab21f7fac7abf45c516352b3b86d18a73f770dff57dc412d39641
aggregate root run 2: 082e75bd3baab21f7fac7abf45c516352b3b86d18a73f770dff57dc412d39641
```

This is the substrate the Phase-4 `.axc` artifacts rely on:
identical inputs ⇒ identical commit chains ⇒ identical artifact
hashes, end-to-end.

## 6. Hand-off

| Consumer | What lands |
|---|---|
| **P1.15** (IR emit) | `axiom_l0::zip::*` is the canonical L0 ZIP API; commit-chain leaves are stable across runs (P1.10 chain composes with the P1.12 verified path). |
| **P1.16** (signing-block extraction) | The TV-receipt pattern is reused: one Lean evaluator + one Rust evaluator + one diff per signing-block sub-module (v2/v3/v3.1/v4). |
| **P1.18** (E2E pipeline) | The verified `axiom-l0` is the L0 of the measured pipeline; throughput/latency baselines from this sub-phase set the lower bound. |
| **Phase 2** | `legacy-zip` feature is deletable; the perf-delta tool is generic and can be re-pointed at any future hand-tuned variant. |
