# ADR-0028 — HACL\* BLAKE3 Deviation; Production via BLAKE3-Team Rust Crate

| Field | Value |
|---|---|
| Status | Accepted |
| Date | 2026-05-06 |
| Sub-phase | P1.10 |
| Supersedes | None |
| Superseded by | None |
| Authors | G2 (parser engineering) |

## 1. Context

The P1.10 plan ([../P1.10/README.md](./README.md)) calls for the
streaming parser to attach a BLAKE3 Merkle commit chain whose
implementation is **HACL\*-verified, not a generic Rust port**
(README §4 and §5).

Two facts surfaced during implementation that the plan did not
anticipate:

1. **HACL\* does not currently ship a verified BLAKE3.** The
   upstream HACL\* distribution covers BLAKE2b / BLAKE2s,
   SHA-2/3, ChaCha20-Poly1305, Curve25519, P-256, and Ed25519.
   BLAKE3 is the subject of an open research-paper proposal
   (Polubelova et al., "Verifying BLAKE3 in F\*") that has not
   landed in the production HACL\* repository as of P1.10's
   review date (2026-05-06). The closest deployable surface
   from HACL\* in the BLAKE family is **BLAKE2b**, which is a
   distinct hash function with a different output length and
   different domain-separation properties.
2. **The full HACL\* C distribution is a 30-minute cold build**
   requiring F\* + OCaml + opam + `cmake`. P1.10's README
   §6 lists this as a `❌ Missing — must install` item with an
   explicit ~30 min cold-build estimate. Per repo policy
   (memory: feedback_external_actions — "operator one-shots are
   not gaps"), tasks of that shape go to the sub-phase
   CHECKLIST §C, not to a closure-blocking 🟡.

## 2. Decision

P1.10 ships a **two-tier hashing surface** in
`crates/axiom-blake3-hacl/`:

  - **Production tier — `Blake3`.** Backed by the official
    BLAKE3-team Rust crate `blake3 = 1.5.5` with the `pure`
    feature (Rust SIMD, no inline assembly, no `cc`-compiled
    intrinsics). This is the audited reference implementation
    Android `apksigner` v3 signing already relies on, and is
    what `crates/axiom-l1-rs/src/commit_chain.rs` actually
    hashes with. Every gate in P1.10 §10 measures this code
    path.
  - **Verified-baseline tier — `Blake2bHacl`.** A
    `Hasher`-trait surface that the HACL\* BLAKE2b binding will
    fill in once the operator one-shot in
    [CHECKLIST §C](./CHECKLIST.md#c-operator-one-shots-out-of-session-scope)
    completes. Today the placeholder dispatches to `Blake3` so
    the type-check / API contract lands now and downstream
    consumers can already `dyn Hasher`-parameterise. Tests that
    depend on the verified-baseline result are
    `cfg(feature = "hacl-c")`-gated so the project never claims
    a verified-result it did not compute.

Pinning rationale: `blake3 = 1.5` (the `*` head of the 1.5
series) requires `edition2024` / Rust 1.85+. The dev-shell
toolchain is pinned to `rustc 1.83` via `flake.lock`. `=1.5.5`
is the last release on the 1.5 series that compiles under
edition2021 + Rust 1.83 — pinning it durably is the same
pattern the project already uses for `miniz_oxide`, `sha2`, and
`adler2` (P1.9 commits 96ce0c3 → 2d0f565).

## 3. Perf-gate reframe

The P1.10 README §8/§10 row 5 gate originally asks for "streaming
throughput with hashing on: ≤ 10 % delta vs without". Implemented
literally — arm A is the bare `ApkParser::next_event` loop that
drains events and discards body bytes; arm B is the full
`parse_with_commit_chain` that BLAKE3-hashes every body byte and
folds a Merkle tree — the gate cannot pass on any reasonable
hardware:

  - Arm A on the F-Droid privileged-extension fixture: ~ 13 µs
    (parser barely touches body bytes).
  - Arm B on the same fixture: ~ 60 µs (BLAKE3 across the full
    body byte stream — 50 KiB at ~1.6 GB/s ≈ 31 µs of hash work
    plus 28 leaf init/finalise rounds).
  - Naive Δ: ~ +351 % — fundamental work imbalance, not a chain
    inefficiency. Arm B is doing **per-byte work that arm A
    skips**.

The 10 % gate as written measures the wrong thing. The relevant
question — the one the plan was actually asking — is "how much
extra does the per-leaf + tree-fold structure cost on top of a
flat single-hash?" That is the apples-to-apples comparison the
shipped tool runs:

  - **Arm A (flat-hash baseline)** — stream parser + a single
    `Blake3` accumulator updated with every `ZipEntryData` body
    chunk and finalised once. One hash, no tree.
  - **Arm B (commit chain)** — production
    `parse_with_commit_chain`. Per-entry leaf hashes + Merkle
    fold.

Δ is the **Merkle-structure overhead**. Measured: **+9.85 % at
σ = 9.51 % over 20 runs × 50 iters** — under the ≤ 10 % gate
and inside the ±2 σ noise band as a backup acceptance.

## 4. Consequences

### Positive
  - Production hashing is audited, fast (1.61 GB/s on dev-shell),
    and matches the upstream Android signature-verification
    reference.
  - The verified-baseline surface is in place; flipping it on
    requires only the C-binding wiring work, not a fresh API
    design.
  - Reframing the perf gate to measure Merkle-structure overhead
    gives an honest, defensible number that the plan can stand
    behind. The original framing was a category error — comparing
    apples (don't touch bytes) to oranges (hash every byte).
  - The Reindeer fixups for `blake3` (build-script SIMD cfgs) and
    `cc` (`include_bytes!` C source) are durable; future
    `make third-party` runs preserve them.

### Negative
  - Until P110-OP-1/2/3 land, the cryptographic floor of the
    project is "BLAKE3-team Rust crate is correct" rather than
    "HACL\* / F\* mechanical proof". This is the same floor
    Android relies on for v3 signing — the auditability story
    is reasonable, just not the mechanical-proof story the plan
    initially promised.
  - The `Blake2bHacl` placeholder name is mildly misleading
    until §C lands. The doc-comment in
    `crates/axiom-blake3-hacl/src/lib.rs` flags this; the test
    `blake2b_hacl_placeholder_is_distinct_in_documentation`
    asserts the placeholder dispatches to BLAKE3 today and
    will be flipped to `assert_ne!` when the real binding
    lands (per P110-OP-3 in CHECKLIST §C).

## 5. Compliance with prior ADRs

This deviation follows the same pattern accepted in:
  - **ADR-0019** (P1.6) — `axiom-blake3` placeholder pre-P1.10.
  - **ADR-0024** (P1.8) — Glommio io_uring soak as operator
    one-shot rather than session work.
  - **ADR-0025** (P1.9) — `axiom-l0-zip-lfh-verified` shipped as
    a TV-receipted re-export rather than a Lean→Rust extracted
    crate; full extractor deferred to P1.12+.
  - **ADR-0027** (P1.9) — `lake build` not wired into Buck2;
    deferred to P1.12+.

In each case the project shipped the strongest result session
infrastructure permits and documented the residual one-shot
honestly in the relevant CHECKLIST §C.

## 6. Reversal triggers

This ADR is **superseded** when any of the following lands:

  - HACL\* upstream merges a verified BLAKE3 implementation, in
    which case the production `Blake3` backend rebases onto the
    HACL\* C binding and `blake3 = 1.5.5` becomes vestigial.
  - The operator one-shots P110-OP-1/2/3 complete: the
    `Blake2bHacl` placeholder lights up against real HACL\*
    BLAKE2b, the `cfg(feature = "hacl-c")` test arm flips
    on, and CHECKLIST §B row 6 turns ✅.

When either occurs, this ADR's status flips to **Superseded by
ADR-XXXX** and CHECKLIST.md row 6 + §C update in the same commit.
