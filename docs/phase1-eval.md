# Phase 1 Evaluation Report

Measured performance of the APKAXIOM pipeline on real-world APK corpora.

## Corpora

| Corpus | APKs | Source | Mean size |
|--------|------|--------|-----------|
| real-apks | 100 | F-Droid | ~650 KB |
| bench-1k | 1 000 | F-Droid | ~740 KB |

Both corpora consist of real Android application packages from the F-Droid
open-source repository.  They cover a range of SDK targets (API 21–34),
signing schemes (v2, v3, v3.1), and compression methods (deflate + store).

## APKAXIOM Pipeline Numbers

Pipeline per APK: BLAKE3 one-shot → ZIP parse → AXML IR emit + reencode +
SHA-256 → `verify_apk_bytes` (RSA/ECDSA/DSA signature verification).

### bench-1k (1 000 APKs)

| Metric | Value | Gate | Status |
|--------|-------|------|--------|
| p50 latency | 4.5 ms | ≤ 50 ms | PASS |
| p95 latency | 15.9 ms | ≤ 150 ms | PASS |
| p99 latency | 18.4 ms | ≤ 300 ms | PASS |
| Peak RSS | 18 MB | ≤ 150 MB | PASS |
| Throughput | 175 APKs/sec | — | INFO |
| Coverage | 1 000/1 000 (100%) | ≥ 99% | PASS |

### real-apks (100 APKs)

| Metric | Value |
|--------|-------|
| p50 latency | 0.9 ms |
| p95 latency | 1.6 ms |
| p99 latency | 2.2 ms |
| Peak RSS | 4 MB |
| Throughput | 1 038 APKs/sec |
| Coverage | 100/100 (100%) |

## Comparison vs Androguard

Androguard 4.1.3, measured on real-apks (100 APKs), single core.

| Tool | Mode | p50 | p95 | Throughput | p50 speedup |
|------|------|-----|-----|------------|-------------|
| APKAXIOM | full pipeline | 0.9 ms | 1.6 ms | 1 038 APKs/s | 1.0× |
| Androguard | manifest only | 13.3 ms | 21.6 ms | 70 APKs/s | 15.3× |
| Androguard | full analysis | ~304 ms | ~7 351 ms | 0.5 APKs/s | ~350× |

**Manifest-only mode**: `APK(p).get_package()` + `get_permissions()` — parses
AXML and certificate metadata, no DEX analysis.

**Full-analysis mode**: `AnalyzeAPK(p)` — full DEX disassembly + control-flow
analysis.  APKAXIOM does not disassemble DEX; this comparison reflects the
total pipeline cost for a typical security-analysis workflow.

**HARD gate (≥10× vs full Androguard)**: PASS at 350×.

## Coverage

100% of both corpora processed without error.  Seven bench-1k APKs have GP
bit 3 set with `usz=0` in the LFH; APKAXIOM handles these correctly via the
Data Descriptor read path.

Signing scheme breakdown on bench-1k:
- v3 + v2: majority
- v2 only: ~12%
- v3 only: ~3%
- DSA-SHA256 only: 3 APKs (honest deviation — RustCrypto `dsa 0.6.3`)

## Reproducibility

Two consecutive bench-1k runs produce bit-identical NDJSON receipts (K10).
x86\_64 and ARM64 receipts are verified identical in CI (K9).

## §C Operator One-Shots

- **C-1 AndroZoo 10K eval** — requires AndroZoo API key + ~50 GB storage.
  Run `p119-eval-compare --corpus <10k-dir> --bench` once available.
- **C-2 16-core EPYC benchmark** — K4/K5 multi-core throughput at scale.
- **C-3 apk-info v0.x comparison** — apk-info upstream requires Rust
  edition 2024 (≥1.85); not available on pinned 1.83 toolchain.
- **C-4 A8/A11/A14 internal demo** — KVM + AOSP build environment required
  (P1.13 harnesses).
