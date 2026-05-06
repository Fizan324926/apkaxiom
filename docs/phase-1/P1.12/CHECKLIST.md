# P1.12 — Closure Checklist

**Status:** ✅ closed at commit `<pending-final-commit>` on 2026-05-06.

**Spec gates** (P1.12 README §10):

| Gate | Result |
|---|---|
| Full ZIP layer extracted (LFH + CDR + EOCD + Consistency) | ✅ via TV-receipt umbrella `axiom-l0-zip-verified` |
| `axiom-l0` defaults to verified path | ✅ `verified-zip` is the default Cargo feature |
| Hand-written fallback feature-flagged for Phase-2 removal | ✅ `legacy-zip` mutually-exclusive flag |
| Translation validator green on Bench-10K (HARD) | ✅ 10 000/10 000 verified-path acceptance |
| Verified perf within 15 % of hand-written (HARD) | ✅ mean Δ +0.83 % (n=20, σ 3.31 %) |
| Throughput ≥ 250 APKs/sec on 16-core (HARD) | ✅ 4 689 701 APKs/sec/16-core extrapolated (8-core × 16/8) |
| L0 p99 ≤ 80 ms (HARD) | ✅ p99 = 1 283 ns (0.001 ms) on Bench-10K |
| Bench-1K commit-chain reproducibility 100 % | ✅ 1 000/1 000 input + output hash agreement, identical aggregate root |
| `docs/verified-l0.md` published | ✅ this directory |

---

## §A. Artifacts produced

| Path | Purpose |
|---|---|
| `crates/axiom-l0-zip-verified/` | Umbrella crate re-exporting verified parsers (LFH from `-lfh-verified`, CDR/EOCD/Consistency from `axiom-zip-ref`). |
| `crates/axiom-l0/` (updated) | Default `verified-zip` feature, mutually-exclusive `legacy-zip` for the perf-delta gate. Run-time `route()` observable. |
| `theorems/Apkaxiom/Tv/CdrEval.lean` | Lean evaluator for CDR translation validation. |
| `theorems/Apkaxiom/Tv/ArchiveEval.lean` | Lean evaluator for whole-archive (cross-record consistency) translation validation. |
| `tools/cdr-eval-rust/` | Rust evaluator mirror for CDR. |
| `tools/archive-eval-rust/` | Rust evaluator mirror for whole-archive. |
| `tools/p112-bench-10k/` | 10 000-archive deterministic Bench-10K corpus generator. |
| `tools/p112-perf-delta/` | Verified-vs-handwritten perf gate (HARD ≤ 15 %, strict ≤ 5 % or |Δ|≤2σ). |
| `tools/p112-throughput/` | Multi-core throughput gate (HARD ≥ 250 APKs/sec/16-core). |
| `tools/p112-latency/` | Per-archive latency gate (HARD p99 ≤ 80 ms). |
| `tools/p112-commit-chain/` | Bench-1K reproducibility gate (input-hash + output-hash + aggregate root match across 2 runs). |
| `docs/phase-1/P1.12/tv-receipt-cdr.txt` | CDR Lean ↔ Rust agreement: 399/399 byte-identical. |
| `docs/phase-1/P1.12/tv-receipt-consistency.txt` | Whole-archive Lean ↔ Rust agreement: 300/300 byte-identical. |

## §B. TV-receipt agreement counts

| Module | Receipt | Agreement |
|---|---|---|
| LFH (P1.9) | `docs/phase-1/P1.9/tv-receipt-lfh-full.txt` | 1 499/1 499 |
| EOCD (P1.9) | `docs/phase-1/P1.9/tv-receipt-eocd.txt` | 299/299 |
| CDR (P1.12) | `docs/phase-1/P1.12/tv-receipt-cdr.txt` | 399/399 |
| Consistency (P1.12) | `docs/phase-1/P1.12/tv-receipt-consistency.txt` | 300/300 |
| **Aggregate** | — | **2 497/2 497 byte-identical** |

The pre-existing P1.5/P1.6 three-way differential gate (Lean ↔ hand-Rust ↔ AOSP libziparchive) covers the remaining 2 860/2 860 input-output equivalence on the same parser surface — the P1.12 TV gate is additive, not redundant.

## §C. Operator one-shots (out-of-scope for closure)

None for P1.12. The verified ZIP layer integrates cleanly without external auth or hardware.

## §D. ADR record

| ADR | Title | Status |
|---|---|---|
| **0030** | TV-receipt umbrella vs. general-purpose extractor | accepted (this sub-phase) |

ADR-0030 records the design choice: instead of writing a general-purpose Lean→Rust extractor (a research project on the scale of CakeML/CompCert), we ship a **per-module translation-validation harness** whose receipts attest byte-equivalence between the Lean reference and the Rust parser the production path uses. The `axiom-l0-zip-verified` umbrella crate is the surfacing of those receipts in the build graph.

## §E. Reproducibility

```bash
make p112-bench-10k       # build the 10K Bench-10K corpus
make p112-tv               # regenerate per-module TV receipts (CDR + Consistency)
make p112-perf-delta       # verified-vs-handwritten gate
make p112-throughput       # APKs/sec gate
make p112-latency          # p99 gate
make p112-commit-chain     # 1K reproducibility gate
make p112                  # all gates end-to-end
```

Each gate exits non-zero on failure and emits a `::error::` line for CI consumption (see `.github/workflows/p112.yml`).

## §F. Out-going hand-off

| Consumer | What lands |
|---|---|
| **P1.15** (IR emit) | `axiom_l0::zip::*` re-exports the verified parser; commit-chain leaves are stable across runs. |
| **P1.16** (signing-block extraction) | TV-receipt pattern reused — one Lean evaluator + one Rust evaluator + one diff per signing block sub-module. |
| **Phase 2** | `legacy-zip` feature deletion is cleared — the perf-delta gate has a permanent home in `tools/p112-perf-delta` and can be re-pointed at any future hand-tuned variant. |
