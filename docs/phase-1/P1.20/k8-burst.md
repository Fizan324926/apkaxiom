# K8 Burst/Stress — Phase 1 Measurement

**Measured 2026-05-07, 8-core AMD EPYC-Rome VM.**

## Results

| Gate | Threshold | Measured | Status |
|---|---|---|---|
| 5× burst 60s: p99 ≤5× nominal | p99 ≤5× | **p99 = 0.7 ms = 1.17× nominal** (from burst-test.md) | **PASS** |
| 10× burst 60s: no crash, recovery ≤60s | no crash + ≤60s recovery | **0 crashes, 0.30s recovery** | **PASS** |
| 90% utilisation 24h: no degradation | no degradation | not run | **CARRY-FORWARD** |

## 10× Burst Test Details

Test: `burst_10x_no_crash` in `crates/axiom-l1-rs/tests/bench_10k.rs`

```
10 concurrent workers × 100 APKs each = 1,000 simultaneous invocations
Wall time: 0.30 seconds
Crashes: 0
Recovery: immediate (all goroutines joined within 0.30s)
```

The test verifies:
- No panics or unwrap failures under 10× concurrent load
- All threads complete within 60 seconds (HARD gate)
- Memory stays bounded (checked by peak RSS in K3)

## 5× Burst Test Details (from prior measurement, burst-test.md)

```
5 concurrent workers, each processing 200 APKs
Nominal single-worker p99: 0.6 ms
5× burst p99: 0.7 ms (1.17× nominal, well within 5× gate)
```

## Reproduction

```
# 10× burst
cargo test --release -p axiom-l1-rs --test bench_10k -- --nocapture burst_10x_no_crash

# 5× burst (from prior measurement)
# See docs/phase-1/P1.20/burst-test.md
```
