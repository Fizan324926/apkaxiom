# ADR-0030 — Translation-validation receipts vs. general-purpose Lean→Rust extractor

**Status:** accepted (P1.12, 2026-05-06).

**Context.** The P1.12 spec calls for "Rust extraction of full ZIP
layer" — replacing the hand-written Rust parser with code generated
from the Lean reference. A general-purpose Lean→Rust extractor is
a research project on the scale of CakeML or CompCert: handling
Lean's full type system, dependent types, tactic proofs, and
runtime-level inductive eliminators in the target language is
years of work, not weeks. Pursuing it would dominate the schedule
and shift focus away from what the project actually needs at the
trust boundary: **observable equivalence** between the Lean
reference and the production Rust parser.

**Decision.** Ship a per-module **translation-validation harness**
plus a re-export umbrella, instead of a code-generating extractor.
The umbrella crate `axiom-l0-zip-verified` re-exports the
production Rust parsers (`axiom-zip-ref` for CDR + EOCD +
Consistency, `axiom-l0-zip-lfh-verified` for LFH) under a name
that documents the per-module TV receipts they're pinned to. The
harness regenerates each receipt from a corpus, asserts byte-
equality between Lean evaluator output and Rust evaluator output,
and persists the corpus SHA, agreement count, Lean output SHA, and
Rust output SHA. Any divergence — Lean-side spec drift not mirrored
in Rust, Rust regression not caught upstream — fails the receipt
re-run loudly.

**Trade-offs.**

  * **Pros.** Engineering effort scales with the corpus size, not
    with the complexity of the Lean type system. New ZIP modules
    add one Lean evaluator + one Rust evaluator + one diff = small
    bounded work, not "extend the extractor". The trust story is
    auditable: a reader can run `make tv-zip` and re-verify each
    receipt independently. The perf story is identical — both
    routes share the same Rust monomorphisation.
  * **Cons.** The harness validates equivalence on the corpus, not
    on every byte-shape the parser will ever see. A divergence
    that the corpus doesn't exercise can slip through. Mitigation:
    the corpus is co-developed with the Lean theorems (every
    `ParseError` variant has a corresponding adversarial input;
    every well-formed branch has multiple inputs covering the
    field-set distribution); re-running the harness on each merge
    catches drift early. The P1.5/P1.6 three-way differential
    (Lean ↔ hand-Rust ↔ AOSP libziparchive, 2 860/2 860 inputs)
    serves as the broader-coverage cross-check.
  * **Reversible.** A future general-purpose extractor would
    replace `axiom-zip-ref` re-exports with extracted code; the
    umbrella crate's API stays the same; downstream consumers
    don't notice. The receipts at that point gate the extractor
    output instead of the hand-written reference.

**Alternatives considered.**

  1. **Write a general-purpose Lean→Rust extractor.** Rejected on
     scope: years of work, distracts from the actual L0 layer
     deliverable. Re-evaluate in Phase 2 when the parser surface
     stabilises.
  2. **Hand-translate Lean → Rust per module.** Rejected on
     duplicate-spec risk: two implementations of the same parser
     drift independently; the TV harness is the only honest
     reconciliation mechanism, so we'd need to build it anyway.
  3. **Use the Lean compiler's C backend + bindgen.** Rejected on
     trust surface: pulls in the Lean runtime as a dependency of
     the trusted core, ballooning what the verifier has to trust
     (and what the Reindeer third-party set has to vendor).

**Consequences.**

  * `axiom-l0-zip-verified` is the canonical default ZIP route in
    `axiom-l0` (default Cargo feature `verified-zip`).
  * The hand-written reference `axiom-zip-ref` becomes the
    source-of-truth Rust implementation that the receipts pin to;
    its own correctness story remains the P1.5/P1.6 three-way
    differential.
  * The per-module TV evaluator binaries (`Apkaxiom.Tv.*Eval`,
    `tools/*-eval-rust`) become regular maintained surfaces:
    every change to the Lean reference or the Rust parser must be
    accompanied by a receipt re-run.
  * Future verified surfaces (signing-block, manifest, resources)
    reuse the pattern. The per-module Lean evaluator and Rust
    evaluator are the standard artifact pair for any verified
    parser sub-module.

**References.**

  * [docs/phase-1/P1.12/CHECKLIST.md](CHECKLIST.md)
  * [docs/phase-1/P1.12/verified-l0.md](verified-l0.md)
  * [tv-receipt-cdr.txt](tv-receipt-cdr.txt) (399/399 byte-identical)
  * [tv-receipt-consistency.txt](tv-receipt-consistency.txt) (300/300 byte-identical)
  * [docs/phase-1/P1.9/tv-receipt-lfh-full.txt](../P1.9/tv-receipt-lfh-full.txt) (1 499/1 499)
  * [docs/phase-1/P1.9/tv-receipt-eocd.txt](../P1.9/tv-receipt-eocd.txt) (299/299)
