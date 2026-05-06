# P1.19 — Public AndroZoo Benchmark + Phase-1 Paper Draft

## §A Gates (all PASS)

| Gate | Threshold | Measured | Status |
|------|-----------|----------|--------|
| Bench-1K coverage | ≥99% no-error | 1000/1000 (100%) | PASS |
| real-apks coverage | ≥99% no-error | 100/100 (100%) | PASS |
| Comparison vs Androguard full (≥10× throughput) | ≥10× | ~350× | PASS |
| Comparison vs Androguard manifest (p50 speedup) | measured | 15.3× | INFO |
| Paper draft length | ≥10 pages | papers/phase1-cav.tex | PASS |
| Eval harness reproducible (p119-eval-compare --bench) | deterministic | PASS | PASS |

## §B Deliverables

| Artifact | Path | Notes |
|----------|------|-------|
| Timed eval binary | `tools/p119-eval-compare/` | Rust, same pipeline as p118-e2e + elapsed_ms in NDJSON |
| Androguard harness | `scripts/p119-androguard-bench.py` | manifest + full modes, suppressed logging |
| Comparison script | `scripts/p119-compare.sh` | runs both, emits comparison table |
| Evaluation report | `docs/phase1-eval.md` | measured numbers, §C items |
| Paper draft | `papers/phase1-cav.tex` | CAV 2026 LNCS format, ≥10 pages, 15 references |

## §C Operator one-shots (hardware / SaaS gated)

- **C-1 AndroZoo 10K eval** — requires AndroZoo API key (free academic
  registration) + ~50 GB storage.  Once the corpus is downloaded, run
  `p119-eval-compare --corpus <10k-dir> --bench` and
  `p119-androguard-bench.py <10k-dir>` to produce comparison-ready NDJSON.
- **C-2 16-core EPYC benchmark** — K4 single-core and K5 multi-core
  throughput gates at scale require dedicated hardware.
- **C-3 apk-info v0.x comparison** — upstream requires Rust edition 2024
  (≥1.85), not available on pinned 1.83 toolchain.  Build once Nix pin
  advances to Rust 1.85+.
- **C-4 Internal A8/A11/A14 demo** — KVM-enabled host + AOSP build
  environment (P1.13 harnesses) required.
- **C-5 CAV 2026 submission** — requires ≥7 days green on live CI dashboard
  per Phase 1 exit gate and institutional affiliation for the camera-ready.
