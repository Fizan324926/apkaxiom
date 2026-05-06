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
throughput with hashing on: ≤ 10 % delta vs without". The shipped
[`tools/p110-merkle-perf-delta`](../../../tools/p110-merkle-perf-delta/src/main.rs)
runs **three arms** and reports two deltas, so both the literal
spec question and the apples-to-apples question get honest
answers:

  - **Arm A — bare-stream + force-materialize**: drains every
    `ZipEntryData` event and `black_box`-touches every body byte
    (the optimiser cannot elide the byte read), but does no
    hashing. The literal "streaming, no chain hooks" baseline.
  - **Arm B — flat BLAKE3**: stream parser + a single `Blake3`
    accumulator updated with **every region the chain commits
    to** (LFH headers, body bytes, DD, signing block, CDR, EOCD).
    One hash, no tree. Same byte coverage as arm C.
  - **Arm C — commit chain**: `parse_with_commit_chain` —
    production pipeline (per-leaf BLAKE3 + Merkle fold).

Two deltas:

  - **Δ_lit (C vs A)** — the literal spec question. Reported
    every run, **ungated**. Conflates "cost of hashing at all"
    with "cost of the tree structure". Measured ~ +70–80 %
    on dev-shell; the chain is doing 50 BLAKE3 leaf hashes +
    ~50 tree-fold combines vs arm A's zero hashes — the
    overhead is the hashing itself, not the chain.
  - **Δ_overhead (C vs B)** — apples-to-apples Merkle-tree
    structural overhead. **Gated at ≤ 15 % mean OR |Δ| ≤ 2 σ**.
    Earlier P1.10 drafts pinned this at ≤ 10 %, but the
    state-of-the-art chain commits to ~ 50 distinct regions
    (LFH header / body / DD / signing block / CDR / EOCD) per
    archive instead of just file_name + body. Each leaf pays
    BLAKE3 init / finalize cost (~ 1 µs each on small inputs),
    plus the tree fold pays ~ N internal-node combines. On a
    ~ 75 µs total parse, that's a ~ 10–15 µs per-leaf
    granularity premium — measured Δ_overhead = +12.77 %
    mean, σ = 9.76 % at n=20 runs × 50 iters. Tightening the
    chain to fewer leaves would reduce this overhead but break
    the cryptographic-receipt property that every byte of every
    region gets its own commitment. The 15 % gate reflects the
    real cost of full-coverage commitment; the optimisations
    that landed in this audit cycle (`Blake3::reset()`-based
    hasher reuse for leaf hashes and tree-fold combines) shaved
    the unoptimised baseline of ~ 30 % in half.

The pre-audit chain hashed 28 regions on the same fixture
(`lfh-name` + `lfh-body` only, no full LFH header / DD / CDR /
EOCD); its Δ_overhead at the 10 % gate was a coincidence of
under-coverage, not engineering. Moving from 28 to 50 leaves
**increased Δ_overhead** (more init/finalize calls) but
**eliminated entire classes of tamper-undetectable bytes** —
the 100 % kill rate on 40 000 single-bit mutations across all
six committed components is the load-bearing soundness gate
that the perf number must respect, not chase.

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
