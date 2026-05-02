# P3.14 — BSH-256 Rust Implementation + DiskANN Similarity Index (1M-vector scale)

> BSH-256 in production. DiskANN-backed similarity index over 1 million APKs. Sub-linear similarity search. Reproducibility 100% bit-identical.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §10.1](../../../README.md#layer-5) · [../../TECH_STACK.md §11 (DiskANN)](../../TECH_STACK.md#storage)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P3.14 |
| Owner(s) | G6 |
| Duration | Weeks 10–15 |
| Critical-path | yes |
| Hard prerequisites | P3.13 (RFC frozen) |

## 2. Goal & Scope

A production-grade BSH-256 implementation in Rust + a DiskANN-backed similarity index that scales to 1 million vectors. Sub-linear ANN search. Full obfuscation-stability eval on ProGuard/R8/DexGuard repackaging pairs.

### In scope
- `crates/axiom-l5-bsh` — BSH-256 implementation per the frozen RFC
- DiskANN-backed `crates/axiom-l5-similarity` index
- MinHash + LSH layer for orthogonal coarse-grained filtering
- Reproducibility: 100% bit-identical BSH on repeated runs
- ProGuard/R8/DexGuard stability eval on Repack-2K
- 1M-vector scale benchmark (synthetic + AndroZoo subset)

### Out of scope
- Bisimulation (P3.15)
- Layer 5 unified surface (P3.17)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P3.13** | BSH-256 RFC (frozen) |
| **P2.5/P2.6/P2.8** | Verified AXML/ARSC/DEX parsers (BSH inputs come from these) |
| **P1.10** | HACL\* BLAKE3 |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **DiskANN** | latest | Disk-resident ANN index |
| **HACL\* BLAKE3** | from P1.10 | Hash primitive |
| **MinHash + LSH** (Rust crate) | latest | Coarse filter |
| **Apache Arrow + Parquet** | latest | Persistent vector storage |
| **Lance** | 0.18+ | Versioned ML embedding store |
| **rkyv** | latest | Zero-copy archived BSH index |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **DiskANN** | ANN library | **Free** OSS (MIT) | https://github.com/microsoft/DiskANN | Microsoft Research |
| **HNSW (hnswlib)** | comparison baseline | **Free** OSS | https://github.com/nmslib/hnswlib | We benchmark against |
| **FAISS** | alt comparison | **Free** OSS (MIT) | https://github.com/facebookresearch/faiss | Reference |
| **Lance** | embedding store | **Free** OSS (Apache 2.0) | https://github.com/lancedb/lance | LanceDB |
| **Apache Arrow + Parquet** | columnar | **Free** OSS | https://arrow.apache.org | Already in stack |
| **AndroZoo** | corpus | **Free academic** | already provisioned | 1M-vector eval |

**No new API keys.**

## 6. System Inventory — Have vs Need

### Already present
- ✅ HACL\* BLAKE3, Apache Arrow, rkyv

### Missing — must install
- ❌ **DiskANN** — clone + build
- ❌ **Lance** — Cargo dep

### Install commands

```bash
# DiskANN (C++ library; we wrap via FFI)
git clone https://github.com/microsoft/DiskANN
cd DiskANN && mkdir build && cd build && cmake .. && make -j$(nproc)
sudo make install

# Lance — Cargo dep
# crates/axiom-l5-similarity/Cargo.toml: lance = "0.18"
```

## 7. Features & Functions Delivered (Comprehensive)

### BSH-256 implementation
- `pub fn compute_bsh(behavior_set: &BehaviorSet) -> Bsh256`
- Per-input canonicalization (per RFC frozen in P3.13)
- HACL\* BLAKE3 with personalization
- Reproducibility: 100% bit-identical across runs (HARD)

### Similarity index
- `pub struct SimilarityIndex { diskann: DiskAnnIndex, lance: LanceTable, lsh_filter: MinHashLsh }`
- `pub fn insert(bsh: Bsh256, apk_id: ApkId, metadata: Metadata)`
- `pub fn search(query: Bsh256, k: usize) -> Vec<(ApkId, Distance)>` — sub-linear
- `pub fn neighbors_within(query: Bsh256, threshold: Distance) -> Vec<(ApkId, Distance)>`
- Disk-resident index with periodic compaction

### Layered filter
- MinHash + LSH banding for fast `O(1)`-ish bucket lookup
- DiskANN inside the bucket for high-precision retrieval
- Falls back to brute-force for very small indices

### Persistent storage
- Lance-backed table for versioned BSH archive
- Parquet exports for analytical queries via DuckDB
- Periodic compaction + index rebuild scheduled

### Stability eval (ProGuard/R8/DexGuard)
- Repack-2K corpus run through each obfuscator
- BSH stability rate: % of repackaged versions that hash to the same BSH as original
- HARD ≥ 90% stability per obfuscator

### Public Rust API
- `pub fn compute_bsh(...)` — single-APK
- `pub fn compute_bsh_streaming(...)` — events from L0 stream
- `pub fn similarity_search(query, k)` — top-k
- `pub fn similarity_within(query, threshold)` — radius search

### Performance instrumentation
- Pyroscope continuous profile of similarity service
- Prometheus metrics: insert/search/neighbor latency

### Documentation
- `docs/bsh-impl.md` — implementation notes, perf characteristics
- `docs/similarity-index.md` — DiskANN setup, sharding, recovery

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| BSH compute throughput | ≥ 1,000 APKs/sec | ≥ 3,000 APKs/sec |
| BSH compute p99 | ≤ 30 ms | ≤ 10 ms |
| Reproducibility (per-APK BSH bit-identical across runs) | 100 % | 100 % |
| BSH collision rate across 50K APKs | < 0.1 % | < 0.01 % |
| BSH stability across ProGuard/R8/DexGuard | ≥ 90 % | ≥ 98 % |
| LSH lookup p99 (1M index) | ≤ 200 ms | ≤ 50 ms |
| Index build throughput | ≥ 5,000 APKs/sec | ≥ 20,000 APKs/sec |
| 1M-vector index size | ≤ 8 GB | ≤ 4 GB |
| Similarity throughput on 1M index | ≥ 1,000 queries/sec | ≥ 5,000 queries/sec |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   ├── axiom-l5-bsh/
│   │   ├── Cargo.toml
│   │   ├── BUCK
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── canonical.rs              # per-input canonicalization
│   │       ├── compute.rs                # BLAKE3 personalization
│   │       └── streaming.rs
│   └── axiom-l5-similarity/
│       ├── Cargo.toml
│       ├── BUCK
│       ├── build.rs                       # DiskANN FFI
│       └── src/
│           ├── lib.rs
│           ├── diskann_ffi.rs
│           ├── lsh.rs                     # MinHash + LSH
│           ├── lance_store.rs
│           └── compaction.rs
├── corpus/
│   └── repack-2k-stability/               # Repack-2K + obfuscator outputs
└── docs/
    ├── bsh-impl.md                        # NEW
    └── similarity-index.md                # NEW
```

## 10. Standalone Output

```bash
buck2 build //crates/axiom-l5-bsh //crates/axiom-l5-similarity --release
buck2 run //bench:bsh-throughput
# "BSH compute: 1700 APKs/sec/core; p99=18ms"
buck2 run //bench:similarity-1m
# "1M-index lookup p99: 95ms; throughput: 1800 queries/sec"
```

## 11. End-to-End Test

```bash
buck2 test //crates/axiom-l5-bsh:full-eval
buck2 test //crates/axiom-l5-similarity:1m-scale
# - BSH throughput ≥ 1K APKs/sec (HARD)
# - p99 ≤ 30 ms (HARD)
# - Reproducibility 100% (HARD)
# - Collision < 0.1% on 50K (HARD)
# - Stability ≥ 90% on Repack-2K (HARD)
# - 1M-index lookup p99 ≤ 200 ms (HARD)
# - Index size ≤ 8 GB (HARD)
```

## 12. Exit Checklist

- [ ] BSH-256 implementation per frozen RFC
- [ ] DiskANN + MinHash LSH layered index
- [ ] Lance-backed persistent storage
- [ ] BSH throughput ≥ 1K APKs/sec/16-core (HARD)
- [ ] BSH p99 ≤ 30 ms (HARD)
- [ ] Reproducibility 100 % bit-identical (HARD)
- [ ] Collision rate < 0.1 % on 50K APKs (HARD)
- [ ] Stability ≥ 90 % on ProGuard/R8/DexGuard (HARD)
- [ ] 1M-index lookup p99 ≤ 200 ms (HARD)
- [ ] 1M-index size ≤ 8 GB (HARD)
- [ ] Documentation published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P3.15** | BSH used as similarity oracle by bisimulation |
| **P3.17** | Layer 5 unified surface |
| **P3.18** | E2E pipeline measures BSH KPIs |
| **Phase 4 / G7** | BSH-256 commits in `.axc` certs |
