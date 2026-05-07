# K4 CPU Efficiency — Phase 1 Measurement

**Measured 2026-05-07, 8-core AMD EPYC-Rome VM, x86_64.**
**Method: `resource.getrusage(RUSAGE_CHILDREN)` (user-space CPU time), `perf` not in PATH on this VM.**

## Measured Values

| Metric | Method | Measured | HARD Gate | Status |
|---|---|---|---|---|
| Cycles per APK | Estimated from user CPU time @ 2.5 GHz | **~384K cycles/APK** | ≤1B | **PASS** |
| Branch-miss rate | `perf stat` required — not available on VM | — | <3% | **CARRY-FORWARD** |
| L1 i-cache miss rate | `perf stat` required — not available on VM | — | <5% | **CARRY-FORWARD** |
| IPC | `perf stat` required — not available on VM | — | ≥1.8 | **CARRY-FORWARD** |

## Cycle Estimate Derivation

```
n_apks      = 4,168
user_cpu_s  = 0.640 s  (RUSAGE_CHILDREN.ru_utime)
clock_freq  = 2.5 GHz  (conservative for AMD EPYC-Rome)

cycles_total  = 0.640 × 2.5 × 10⁹ = 1.60 × 10⁹
cycles_per_apk = 1.60 × 10⁹ / 4,168 = 383,877 ≈ 384K
```

The HARD gate is ≤1 billion cycles/APK. Our estimate is **2,604× below the gate**.

CPU efficiency note: user/wall ratio = 0.640 / 1.54 = 41.6%. The remaining 58% is I/O
(reading APK files from disk). The pipeline itself is fast; the bottleneck is storage bandwidth.

## To Close the Remaining Three Metrics

Run on a host with `perf stat` available (bare-metal Linux, non-VM):
```
perf stat -e cycles,instructions,branches,branch-misses,L1-icache-load-misses \
  <pipeline-binary> <apk-corpus>
IPC = instructions / cycles
branch-miss rate = branch-misses / branches
L1 i-cache miss rate = L1-icache-load-misses / instructions
```

These three carry forward to Phase 2 as infra-blocked items (C-4 in §C).
