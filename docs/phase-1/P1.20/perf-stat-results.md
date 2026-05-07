# K4 Performance Measurement — p118-e2e on 8-core AMD EPYC-Rome VM

**Date:** 2026-05-07  
**Host:** AMD EPYC-Rome (8 vCPU, 15 GB RAM), Linux 6.8.0, x86_64  
**Tool:** `/usr/bin/time -v` (perf not available on this VM)

---

## Method

`perf stat` is not available on this VM (no hardware performance counter access).
`/usr/bin/time -v` provides wall/user/system time and peak RSS, from which
cycle/IPC approximations are derived using the measured CPU frequency.

```
CPU frequency (from /proc/cpuinfo): 2445.406 MHz
```

---

## Measured results

### Run on 13-APK signing corpus

```
Command: /usr/bin/time -v p118-e2e --corpus corpus/signing

User time (seconds):           0.00
System time (seconds):         0.00
Elapsed (wall clock) time:     0:00.01
% CPU:                         85%
Maximum resident set size:     2944 kB (≈ 2.9 MB)
Major page faults:             0
Minor page faults:             183
Voluntary context switches:    1
Involuntary context switches:  0

Throughput (self-reported):    2366 APKs/sec
Latency p50:                   0.1 ms  p95: 1.0 ms  p99: 1.0 ms
```

### Run on 500-APK adversarial corpus

```
Command: /usr/bin/time -v p118-e2e --corpus corpus/adversarial-500

User time (seconds):           0.06
System time (seconds):         0.01
Elapsed (wall clock) time:     0:00.07
% CPU:                         100%
Maximum resident set size:     3328 kB (≈ 3.3 MB)
Major page faults:             0

Throughput (self-reported):    6763 APKs/sec
Latency p50:                   0.1 ms  p95: 0.5 ms  p99: 0.6 ms
```

---

## Derived estimates (adversarial-500 run, 0.06s user time)

| Metric              | Calculation                                      | Value             |
|---------------------|--------------------------------------------------|-------------------|
| Total CPU cycles    | 0.06 s × 2 445 MHz                              | ~146.7 M cycles   |
| Cycles per APK      | 146.7 M / 500 APKs                              | **~293 K cycles** |
| Throughput-derived  | 500 APKs / 0.07 s wall                          | 7 143 APKs/sec    |
| Peak RSS per APK    | 3 328 kB / 500 APKs (shared binary)              | ≤ 7 KB marginal   |
| Minor faults        | 183 / 500 ≈ 0.37 per APK                        | (page re-use)     |

**IPC estimate:** Not computable without hardware instruction counters.  
**Branch-miss rate:** Not computable without hardware counters.  
**Gate K4 status:** CARRY-FORWARD — bare-metal `perf stat` required for authoritative
numbers. The derived cycle estimate (~293 K/APK) is well within the ≤1 B cycles/APK
threshold, but cannot be accepted as the authoritative gate measurement without
hardware counters.

---

## Gate summary

| Gate                      | Threshold  | Estimate        | Authoritative? |
|---------------------------|------------|-----------------|----------------|
| Cycles per APK            | ≤ 1 B      | ~293 K (approx) | No — VM only   |
| Branch-miss rate          | < 3%       | N/A             | No             |
| L1 i-cache miss rate      | < 5%       | N/A             | No             |
| IPC                       | ≥ 1.8      | N/A             | No             |

K4 remains CARRY-FORWARD pending access to bare-metal with `perf stat`.
The VM-derived cycle estimate gives strong confidence the threshold will be met
on actual hardware (293 K cycles/APK vs 1 B budget = 0.03% utilisation).
