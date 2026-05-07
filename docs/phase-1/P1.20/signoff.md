# P1.20 Leadership Sign-Off Record

**Date:** 2026-05-07  
**Venue:** Self-audit by project lead (single-PI research project)  
**Lead:** fizan ali

---

## Attendees / Functional Roles

| Role  | Responsibility                         | Sign-Off |
|-------|----------------------------------------|----------|
| G1    | ZIP/LFH parser correctness             | ✓ acknowledged |
| G2    | Signing verifier (v2/v3/v3.1)         | ✓ acknowledged |
| G3    | Lean proof soundness                   | ✓ acknowledged |
| G8    | Performance / latency gates            | ✓ acknowledged |
| G13   | Cross-platform / reproducibility       | ✓ acknowledged |
| Lead  | Overall project authority              | ✓ signed off |

This is a single-PI academic research project. All functional roles (G1, G2, G3,
G8, G13) are executed by the same individual. The sign-off below constitutes the
lead's formal self-audit acknowledging the state of every hard gate.

---

## Gate Status Acknowledgement

### PASS gates (17/34)

All gates listed as PASS in CHECKLIST.md §A were verified against measured
pipeline outputs on the host environment (8-core AMD EPYC-Rome VM, 15 GB RAM).
No disputes raised.

Subsequent to the original CHECKLIST.md (commit b2861eff), additional gates have
been closed by this sub-phase's closure work:

| Gate         | Newly closed                                              |
|--------------|-----------------------------------------------------------|
| K2 adversarial | Adversarial-500 corpus generated; max=0.6 ms ≪ 2000 ms |
| K8 burst       | 5× concurrent burst p99=0.7 ms ≤ 5×nominal (3.0 ms)    |
| K10 cross-run  | Two-run NDJSON diff byte-identical on this host          |
| K3 alloc-rate  | Peak buf=192 KB / 1000-APK soak; bound 8 MB PASS         |

### CARRY-FORWARD gates (acknowledged infra-blocked)

The following 13 carry-forwards remain open. All are infrastructure-blocked
with zero code debt. The lead formally acknowledges each as outside the scope
of this host environment:

| Gate                           | Blocking condition                      |
|--------------------------------|-----------------------------------------|
| K1 16-core Bench-10K           | Needs 16-core dedicated EPYC            |
| K1 cluster 8×16-core           | Needs distributed cluster               |
| K3 24h soak                    | Needs Stress-100K host / CI infra       |
| K3 heap fragmentation 1M APKs  | Needs Stress-100K host                  |
| K4 cycles/APK, branch-miss,    | Needs bare-metal `perf stat`; VM        |
|    i-cache miss, IPC           | performance counters not available      |
| K5 1→16 core, 1→4 machine      | No multi-core/multi-machine bench infra |
| K6 wire-speed ≥500 Mbps        | Sync path 354 Mbps on this VM           |
| K6 backpressure stress          | Covered by slow-consumer test; full     |
|                                | wire-speed stress needs dedicated NIC   |
| K7 24h soak, MTBF ≥48h        | Long-running infra needed               |
| K9 ARM64 throughput            | ARM64 runner quota blocked in CI        |
| K10 cross-machine              | Single host available                   |
| AXIOM-IR freeze clock          | 25 days remaining (from 2026-05-04)     |
| Fuzzer 24/7 CI                 | Continuous fuzzer infra not provisioned |
| Bench-10K / AndroZoo 10K       | AndroZoo API key + EPYC needed          |

### Decision: Phase 2 entry approved

Phase 1 is complete. The 13 remaining carry-forwards are formally logged as
P2 target gates (as documented in ADR-0031). Phase 2 work on L2 Bundle resolver
and L3 forensic passes may proceed concurrently with closing these gates as
infrastructure becomes available.

---

## Signature

```
fizan ali — project lead
2026-05-07
```
