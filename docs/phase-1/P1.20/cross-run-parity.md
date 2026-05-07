# K10 Cross-Run NDJSON Byte-Identity Check

**Date:** 2026-05-07  
**Host:** AMD EPYC-Rome (8 vCPU, 15 GB RAM), Linux 6.8.0, x86_64  
**Corpus:** 13-APK signing corpus (corpus/signing/)

---

## Method

Two consecutive runs of `p118-e2e --json-out` on the same corpus, with the
output files compared via `diff`.

```bash
p118-e2e --corpus corpus/signing --json-out /tmp/run1.ndjson --bench
p118-e2e --corpus corpus/signing --json-out /tmp/run2.ndjson --bench
diff /tmp/run1.ndjson /tmp/run2.ndjson
```

---

## Results

```
BYTE_IDENTICAL: runs 1 and 2 match
```

Both NDJSON files are byte-for-byte identical. Timing fields are excluded from
the NDJSON output by design (the binary only writes `file`, `verdict`,
`ir_sha256`, and `file_blake3` — no elapsed_ms).

### Run 1 output

```
verdicts: 10 accept  3 reject  0 error  (13 total)
latency:  p50=0.4ms  p95=1.3ms  p99=1.3ms  max=1.3ms
throughput: 1696 APKs/sec
peak RSS: 4 MB
PASS K3 / K2 gates
```

### Run 2 output

```
verdicts: 10 accept  3 reject  0 error  (13 total)
latency:  p50=0.2ms  p95=1.2ms  p99=1.2ms  max=1.7ms
throughput: 1656 APKs/sec
peak RSS: 4 MB
PASS K3 / K2 gates
```

---

## Gate summary

| Gate                                      | Threshold | Measured           | Status           |
|-------------------------------------------|-----------|--------------------|------------------|
| Parser output reproducibility (same host) | 100%      | **100%** byte-identical | **PASS**    |
| Cross-machine rebuild byte-identity       | 100%      | Single machine only | CARRY-FORWARD   |

The cross-machine variant requires running on ≥3 hosts and diffing outputs —
left as infrastructure carry-forward (C-2).
