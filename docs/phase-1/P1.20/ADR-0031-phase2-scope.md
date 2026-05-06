# ADR-0031 — Phase 2 Scope: Bundle Era

**Status:** Accepted.
**Sub-phase:** P1.20 — Phase 1 hard-gate review.
**Date:** 2026-05-06.

---

## Context

Phase 1 built the parser foundation: streaming ZIP (L0), Lean-extracted APK parser (L1),
BLAKE3 Merkle commit chain, AXIOM-IR v0.1 manifest dialect, full signing-block verifier
(v2/v3/v3.1, RSA/ECDSA/DSA), differential fuzzer plant, and the end-to-end evaluation
harness. The Lean mechanization covers LFH + CDR + EOCD completeness theorems, APK
Signing Block formal spec, and a three-way translation validation pipeline
(Lean ↔ hand-Rust ↔ extracted-Rust, 1 499/1 499 TV receipt).

Phase 1 produced measured data:
- Bench-1K: p50=4.5 ms, p95=15.9 ms, p99=18.4 ms, peak RSS=18 MB, 175 APKs/sec
- BLAKE3 throughput: 1.601 GB/s (HACL*-verified)
- Signing verifier: 1 000/1 000 (100%) verdict agreement with apksigner on bench-1k
- Soundness: 0 sorry in theorems/, 0 proof drift incidents

17 of 34 Phase 1 hard gates carry forward (all infrastructure-blocked; see CHECKLIST.md §D).

---

## Decision

Phase 2 scope is **Bundle Era**: add the App Bundle resolver (L2) and the three
forensic passes (L3) on top of the Phase 1 foundation, while closing the
infrastructure-blocked Phase 1 carry-forwards opportunistically.

### In-scope for Phase 2

**L2 — App Bundle resolver (P2.1–P2.4)**
- `axiom-l2` crate: parses `BundleConfig.pb`, split-APK manifests, and on-demand
  feature modules; produces a `BundiorSet` struct with deterministic per-split digests.
- Lean formalization: `BundleConfig.proto` → `BundleConfigSpec.lean` covering split
  naming invariants and feature-module dependency ordering.
- Translation validator: Lean ↔ `axiom-l2` agreement on APKAXIOM-Bundles-5K.
- KPI gate: ≥99.9% agreement vs AOSP installer on Bundles-5K.

**L3 — Forensic passes (P2.5–P2.9)**
- Shadow Stack pass: reconstructs the install-time cert chain; FP rate <10% on
  benign Bench-10K.
- AXML Provenance pass: classifies manifest attributes as original vs injected vs
  rewritten; misidentification rate <5%.
- Negative-Space pass: detects declared-but-absent components; FP rate <20%.
- All three passes produce structured NDJSON receipts compatible with
  `p119-eval-compare` format.

**Phase 1 carry-forward closure (opportunistic, P2.10)**
- Obtain AndroZoo API key and run Bench-10K eval (closes K1 16-core, Bench-10K eval,
  K9 ARM64 throughput parity, K3/K7 24 h soak).
- Build Adversarial-500 corpus (closes K2 adversarial worst-case).
- Run `perf stat` on bare-metal EPYC (closes all K4 metrics).
- AXIOM-IR freeze clock: 4-week countdown auto-closes ~4 weeks after P1.15 merge with
  no IR-breaking changes.

**Continuous infrastructure (P2.11)**
- Promote differential fuzzer to 24/7 CI-as-a-service (closes K12).
- Wire Prometheus push-gateway to `p119-eval-compare --prometheus-push`.
- ARM64 runner quota on GitHub Actions org (closes K9 CI).

### Out of scope for Phase 2

- DEX disassembly, bytecode analysis (Phase 3).
- Certificate chain forensics beyond the shadow-stack pass (Phase 4).
- ML-based behavioural classification (Phase 5).
- v1.0 production ship / APKAXIOM-Eval-50K (Phase 6).

---

## Options considered

**Option A (chosen): L2+L3 Bundle Era.** Matches the PHASE_GATES §6 definition exactly.
Keeps L0/L1 as stable foundation; adds L2 resolver and L3 forensic passes incrementally.
Natural boundary: bundles shipped in Android 10+ are now the dominant distribution format.

**Option B: Phase 1 carry-forward sprint first, then bundles.** Risk: carry-forwards are
infrastructure-blocked (not code-blocked), so a sprint would stall waiting on hardware
procurement. Better to proceed with new code work and close carry-forwards in parallel.

**Option C: Skip bundles, start DEX.** Premature — no bundle resolver means forensic
passes can't see feature modules. DEX work without L2 produces incomplete shadow stacks.

---

## Consequences

1. Phase 2 gate (PHASE_GATES §6) requires: ≥150 APKs/sec L0–L3, p99 ≤800 ms,
   bundle correctness ≥99.9% vs AOSP, forensic FP <12%, K12 fuzzer green 30 days.
2. Phase 1 carry-forward debt (17 items) owned by G13 infra team; reviewed at
   P2.10 sprint gate.
3. AXIOM-IR v0.1 remains frozen; Phase 2 may propose AXIOM-IR v0.2 for bundle dialect
   extensions via a new ADR (next free: ADR-0032).
4. The CAV 2026 paper draft (`papers/phase1-cav.tex`) should be submitted once
   the AndroZoo Bench-10K numbers are in (C-2 above). No code change required —
   just fill in the §C numbers and submit.
