# K8 Burst Stress Test — 5× Concurrent Pipeline Invocations

**Date:** 2026-05-07  
**Host:** AMD EPYC-Rome (8 vCPU, 15 GB RAM), Linux 6.8.0, x86_64  
**Corpus:** 500-APK adversarial corpus (corpus/adversarial-500/)

---

## Method

Five independent `p118-e2e` processes were launched concurrently via bash `&`
and `wait`. Each process processed the full 500-APK adversarial corpus.
This models the "5× burst" scenario from K8.

```bash
for i in 1 2 3 4 5; do
  /usr/bin/time -v p118-e2e --corpus corpus/adversarial-500 \
    --json-out /tmp/burst_run_${i}.ndjson &
done
wait
```

---

## Results

### Per-process latency under 5× concurrent load

| Run | p50 (ms) | p95 (ms) | p99 (ms) | max (ms) | throughput (APKs/s) |
|-----|----------|----------|----------|----------|---------------------|
| 1   | 0.1      | 0.5      | 0.5      | 0.5      | 6 753               |
| 2   | 0.1      | 0.5      | 0.5      | 0.7      | 6 768               |
| 3   | 0.1      | 0.5      | 0.7      | 0.8      | 6 475               |
| 4   | 0.1      | 0.5      | 0.5      | 0.6      | 6 516               |
| 5   | 0.1      | 0.5      | 0.6      | 0.8      | 6 049               |
| **max** | **0.1** | **0.5** | **0.7** | **0.8** | — |

### Nominal baseline (single process, same corpus)

| p50   | p95   | p99   | max   |
|-------|-------|-------|-------|
| 0.1ms | 0.5ms | 0.6ms | 0.6ms |

### K8 gate evaluation

| Gate criterion                        | Threshold       | Measured                        | Status   |
|---------------------------------------|-----------------|---------------------------------|----------|
| 5× burst p99 ≤ 5× nominal p99        | ≤ 3.0 ms        | **0.7 ms** (1.17× nominal)      | **PASS** |
| 5× burst: no crashes                  | 0 crashes       | 0 crashes, 0 errors             | **PASS** |
| Peak RSS under 5× load (per process)  | ≤ 150 MB        | **3.3 MB** per process          | **PASS** |

### Resource usage under 5× concurrent load

| Metric                    | Per-process  | ×5 aggregate |
|---------------------------|--------------|--------------|
| Peak RSS (kB)             | 3 328        | ~16 640 kB   |
| User time (s)             | ~0.07        | ~0.35 s total|
| Verdicts                  | 200A/300R    | 1000A/1500R  |
| Errors                    | 0            | 0            |

---

## Notes

- The 5× concurrent load showed only 1.17× p99 degradation vs nominal, well
  within the 5× allowance. The low latency reflects the pipeline's single-APK
  memory footprint and the absence of shared mutable state across workers.
- The K8 "10× burst, 90% utilisation 24h" variants remain CARRY-FORWARD as
  they require sustained CI burst infrastructure.
