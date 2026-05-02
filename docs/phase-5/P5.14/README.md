# P5.14 — TFLite Model Parser + Structural Integrity Hash

> Parse TFLite Flatbuffer models out of APK assets, compute a canonical structural-integrity hash that ignores mutable weights but locks graph topology + tensor shapes + operator types.

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P5.14 |
| Owner(s) | G11 |
| Duration | Weeks 2–8 |
| Critical-path | yes (foundation for ML scanners) |
| Hard prerequisites | P5.1 |

## 2. Goal & Scope

A reliable, fast, reproducible TFLite parser + canonical hash. Used as: (a) tamper-detection anchor (hash compared to a signed reference); (b) input to downstream Neural Cleanse + STRIP scans (P5.15 / P5.16).

### In scope
- TFLite Flatbuffer schema parser (read-only)
- Asset enumeration (TFLite models inside APK assets/, raw/, lib/)
- Per-model summary: ops, tensors, shapes, quantization, signatures
- Canonical structural-integrity hash (BLAKE3 over canonical-encoded graph minus mutable weights)
- Round-trip serialization (parse → re-emit) for verification
- Per-model speed: ≤ 500 ms HARD (≤ 100 ms TARGET)
- Reproducibility: deterministic hash byte-for-byte across runs / arches

### Out of scope
- Backdoor detection (P5.15, P5.16)
- Adversarial robustness (P5.17)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P5.1** | TFLite runtime + flatbuffers schema vendored |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **TFLite C/C++ runtime** | 2.x latest | Reference parsing |
| **flatbuffers schema** | latest | Schema |
| **Rust** | 1.84+ | Implementation |
| **flatbuffers-rs** | latest | Rust flatbuffers |
| **BLAKE3 (HACL\*)** | (existing) | Hash |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **TFLite + flatbuffers** | lib | **Free** OSS (Apache 2.0) | https://www.tensorflow.org/lite | Already pinned |
| **TF-Hub / Kaggle Models** | corpus | **Free** | https://www.kaggle.com/models | Reference models |
| **MLPerf Mobile reference models** | corpus | **Free** | https://mlcommons.org/benchmarks/inference-mobile | Standard benchmarks |

**No new API keys** beyond Kaggle / TF-Hub.

## 6. System Inventory — Have vs Need

All present from P5.1.

## 7. Features & Functions Delivered (Comprehensive)

### Crate `axiom-tflite-parse`
- `parse(bytes) -> Result<Model>`
- `summarize(model) -> ModelSummary`
- Asset enumeration on APK to discover TFLite models
- Op + tensor + shape + quantization + signature extraction
- Canonical structural-integrity hash:
  - Sort op order by topological + tie-break by stable hash of op-name + input-tensor-shapes
  - Canonicalize tensor names (rename to t0, t1, ...)
  - Strip mutable buffer contents (weights), but keep buffer indices
  - Hash with BLAKE3
- Round-trip emitter (parse → re-emit byte-equivalent on canonicalized form)

### Reference corpus
- 100 reference TFLite models across MLPerf, TF-Hub, Kaggle Models
- Per-model fixture: parse output + canonical hash
- Used as regression for P5.15 / P5.16

### Tools
- `axiom-tflite-cli` — parse + summarize + hash
- `axiom-tflite-bench` — perf

### Performance
- ≤ 500 ms / model HARD (≤ 100 ms TARGET) on typical ≤ 50 MB model
- Streaming parse for large models

### Reproducibility
- Bytewise-identical hash across runs / arches
- Bytewise-identical canonical re-emit

### Soundness signal
- If parse fails or hash differs from signed reference, downstream scanners receive `model.untrusted = true`

### Documentation
- `docs/tflite-parse.md`

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Per-model parse + hash | ≤ 500 ms | ≤ 100 ms |
| Hash reproducibility across runs / arches | 100 % | 100 % |
| Round-trip canonical re-emit byte-identity | 100 % | 100 % |
| Reference-corpus coverage | ≥ 100 models | ≥ 200 models |
| Asset enumeration accuracy | 100 % | 100 % |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-tflite-parse/         # NEW
├── tools/
│   ├── axiom-tflite-cli
│   └── axiom-tflite-bench
├── corpora/
│   └── tflite-100/                 # NEW: reference corpus + fixtures
└── docs/
    └── tflite-parse.md             # NEW
```

## 10. Standalone Output

A reusable TFLite parser + canonical hasher.

## 11. End-to-End Test

```bash
buck2 test //crates/axiom-tflite-parse:...
buck2 run //tools:axiom-tflite-bench -- --corpus tflite-100
# Expect: ≤ 500 ms per model, 100 % reproducible
```

## 12. Exit Checklist

- [ ] Per-model parse + hash ≤ 500 ms (HARD)
- [ ] Hash reproducibility 100 %
- [ ] Round-trip byte-identity 100 %
- [ ] Reference corpus ≥ 100 models
- [ ] Asset enumeration accuracy 100 %
- [ ] Documentation `docs/tflite-parse.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P5.15** | Parsed model → Neural Cleanse |
| **P5.16** | Parsed model → STRIP |
| **P5.17** | Parsed model → adversarial robustness |
| **L6 cert** | Hash anchored in cert as model-integrity claim |
