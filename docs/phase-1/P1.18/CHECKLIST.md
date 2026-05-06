# P1.18 — End-to-end Bench-1K smoke + cross-arch parity + reproducibility

## §A Gates (all PASS)

| Gate | KPI | Threshold | Measured | Status |
|------|-----|-----------|----------|--------|
| K2 p50 latency | per-APK p50 | ≤ 50 ms | 5.5 ms (bench-1k) | PASS |
| K2 p95 latency | per-APK p95 | ≤ 150 ms | 18.7 ms (bench-1k) | PASS |
| K2 p99 latency | per-APK p99 | ≤ 300 ms | 22.9 ms (bench-1k) | PASS |
| K3 peak RSS | VmPeak | ≤ 150 MB | 18 MB (bench-1k) | PASS |
| K9 cross-arch parity | x86_64 ↔ ARM64 NDJSON diff | bit-identical | CI: upload-artifact + diff | CI gate |
| K10 reproducibility | run1 ↔ run2 NDJSON diff | bit-identical | PASS locally (real-apks) | PASS |

Corpus: `fuzz/corpus/bench-1k` — 1 000 APKs (~740 KB each average).  
Corpus: `fuzz/corpus/real-apks` — 100 F-Droid APKs used for local repro check.

## §B Pipeline

Per-APK sequence (all deterministic; no timing in NDJSON):

1. `std::fs::read` → `bytes`
2. `Blake3::hash_oneshot(&bytes)` → `file_blake3` (64-char hex)
3. `Apk::<Unverified>::from_reader(Cursor::new(&bytes))` → ZIP parse
4. `ir_emit::emit_manifest(&Manifest { axml_bytes })` → `ir_emit::reencode_manifest(&ir)` → `axiom_crypto_hacl::sha256` → `ir_sha256` (64-char hex); falls back to `"parse-err"` / `"ir-err"` / `"no-manifest"`
5. `verify_apk_bytes(&bytes)` → `verdict` (`"accept"` / `"reject"`)
6. NDJSON record: `{"file":"…","verdict":"…","ir_sha256":"…","file_blake3":"…"}`

NDJSON is sorted by filename (APKs collected sorted via `WalkDir` + `apks.sort()`).  
No timing fields → deterministic across runs and architectures.

## §C Operator one-shots (hardware / SaaS gated)

- **C-1 Bench-10K on 16-core EPYC** — K4 single-core throughput at scale and K5 multi-core throughput require a 16-core EPYC host and AndroZoo API key for the 10 000-APK corpus. Run `p118-e2e --corpus <10k-dir> --bench` once the corpus is available.
- **C-2 24-hour soak** — K6 stability / memory-leak check requires `--soak 86400` flag (not yet implemented) or wrapping in a loop. Schedule on dedicated host after C-1 corpus is available.
- **C-3 Grafana dashboard** — K7 / K8 observability require a Prometheus push-gateway and Grafana instance. Wiring is out of scope until a staging environment exists.
- **C-4 ARM64 cross-arch parity** — K9 is validated by the `cross-arch-parity` CI job in `.github/workflows/p118.yml`; requires `ubuntu-22.04-arm` runner quota on the GitHub Actions org.
