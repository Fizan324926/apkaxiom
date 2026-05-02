# P1.7 — apk-info v1.0 Streaming Reader Trait

> Move from "load the file" to "process bytes as they arrive." Glommio thread-per-core io_uring runtime. Wire-speed inspection at ≥500 Mbps.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../README.md §22 (apk-info v1.0 path)](../../README.md#apkinfo-integration)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P1.7 |
| Owner(s) | G2 (Parser Engineering & AOSP Archaeology) |
| Duration | Weeks 3–7 |
| Critical-path | **yes** for the apk-info v1.0 chain (P1.7→P1.8→P1.10→P1.15) |
| Hard prerequisites | P1.3 (axiom-l1-rs spec) |

## 2. Goal & Scope

`axiom-l1-rs` exposes a streaming entry point: `ApkParser::from_reader<R: io::Read>` produces an event stream as bytes arrive without buffering the whole file. Time-to-first-Merkle-commit ≤ 5 ms p99. Sustained wire-speed inspection bandwidth ≥ 500 Mbps single-core.

The runtime is **Glommio** — Datadog's thread-per-core io_uring runtime. Tokio is rejected on the hot path (work-stealing overhead breaks our latency budgets).

### In scope
- `crates/axiom-l1-rs/src/stream.rs` — async streaming parser using Glommio.
- Event-stream API: `ParseEvent::{ZipEntryHeader, ZipEntryData, EocdSeen, ManifestStart, ManifestField, ManifestEnd, ResourceStart, ResourceEntry, ResourceEnd, ParseComplete}`.
- Backpressure correctness (never unbounded buffers).
- Bench harness `bench/stream-vs-file.rs` comparing streaming vs file-load.
- Wire-speed test harness (1 Gbps synthetic feeder) at `tests/wire-speed/`.

### Out of scope
- Type-state phantom guards (P1.8).
- Merkle commit hooks (P1.10).
- AXIOM-IR emission (P1.15).
- Verified Rust extraction (P1.12).

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P1.1** | Buck2 + Cargo, io_uring kernel ≥ 5.1 (verified — host has 6.8) |
| **P1.3** | Streaming reader API surface in `axiom-l1-rs-spec.md` |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Rust** | 1.95 | Implementation |
| **Glommio** | 0.9+ | Thread-per-core io_uring runtime |
| **liburing-dev** | 2.x | io_uring system library |
| **Linux kernel** | 6.x (HAVE 6.8) | io_uring full feature support |
| **futures** | 0.3+ | Async primitives |
| **byteorder** | 1.5+ | Endian-aware byte ops |
| **deku** or **scroll** | latest | Binary parser combinators |
| **hyperfine** | from P1.3 | Latency benchmarks |
| **iperf3** | latest | Network bandwidth feeder for wire-speed tests |
| **criterion.rs** | 0.5+ | Microbenchmarks |
| **flamegraph / perf** | from P1.3 | Profiling |
| **Pyroscope** (continuous profiling) | latest | Continuous profile capture (used heavily in this sub-phase) |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL / Account | Notes |
|---|---|---|---|---|
| **Glommio** | runtime | **Free** OSS (Apache 2.0) | https://github.com/DataDog/glommio | Datadog-maintained |
| **liburing** | system lib | **Free** OSS (MIT) | https://github.com/axboe/liburing | Linux kernel folks |
| **iperf3** | bandwidth tool | **Free** OSS | https://iperf.fr | Wire-speed feeding |
| **Pyroscope (Grafana Cloud Profiles)** | continuous profiling | Free OSS self-host; **paid** Grafana Cloud tier ($$$/mo) | https://pyroscope.io | We self-host |
| **Prometheus + Grafana** | metrics + dashboards | **Free** OSS | https://prometheus.io | Self-hosted |
| **OpenTelemetry collector** | tracing | **Free** OSS | https://opentelemetry.io | Self-hosted |
| **Hetzner / OVH / Latitude.sh dedicated server** | benchmark host | **Paid** (~ €40–200 / month per server) | https://www.hetzner.com | Reproducible benchmark hardware; necessary for KPI measurement (per PHASE_GATES.md App. B reference profile) |

**No API keys** unless using Grafana Cloud's hosted Pyroscope (paid tier; we self-host instead).

**Hardware requirement:** PHASE_GATES.md §5 measures throughput on a 16-core EPYC 9354 / Xeon Gold 6438M. A Hetzner AX102 / Helio Edge equivalent rents at ~€100/mo. Procure now to avoid blocking P1.18 KPI measurement.

## 6. System Inventory — Have vs Need

### Already present
- ✅ Rust 1.95
- ✅ Linux 6.8 with io_uring in kernel
- ✅ perf, strace
- ✅ make, cmake, ninja (from P1.1)

### Missing — must install
- ❌ **liburing-dev** — `sudo apt-get install -y liburing-dev`
- ❌ **iperf3** — `sudo apt-get install -y iperf3`
- ❌ **Pyroscope (self-hosted)** — Docker image
- ❌ **Prometheus + Grafana** — Docker images

### Install commands

```bash
# liburing
sudo apt-get install -y liburing-dev

# iperf3 for wire-speed feeding
sudo apt-get install -y iperf3

# Pyroscope (self-hosted)
docker run -d --name pyroscope -p 4040:4040 grafana/pyroscope:latest

# Prometheus + Grafana stack
mkdir -p monitoring && cd monitoring
cat > docker-compose.yml <<'EOF'
version: '3'
services:
  prometheus:
    image: prom/prometheus:latest
    ports: ["9090:9090"]
    volumes: ["./prometheus.yml:/etc/prometheus/prometheus.yml"]
  grafana:
    image: grafana/grafana:latest
    ports: ["3000:3000"]
    environment: { GF_SECURITY_ADMIN_PASSWORD: dev }
EOF
docker compose up -d
```

## 7. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-l1-rs/
│       ├── Cargo.toml                  # adds glommio = "0.9", deku = "0.18"
│       ├── BUCK
│       └── src/
│           ├── lib.rs
│           ├── stream.rs                # NEW — streaming parser
│           ├── event.rs                 # NEW — ParseEvent enum
│           └── parser/
│               ├── mod.rs
│               ├── zip.rs               # NEW — streaming ZIP parse
│               └── eocd.rs              # NEW — backward-scan for EOCD
├── bench/
│   ├── stream-vs-file.rs                # NEW — Criterion bench
│   └── wire-speed.rs                    # NEW — sustained 1 Gbps test
├── tests/
│   └── wire-speed/
│       ├── feeder.py                    # NEW — synthetic byte stream
│       └── README.md
└── docs/
    └── streaming-architecture.md        # NEW
```

## 8. Standalone Output

A streaming-capable apk-info v1.0 prerelease, benchable in isolation:

```bash
nix develop
buck2 build //crates/axiom-l1-rs --release
buck2 run //bench:stream-vs-file -- --apk-corpus corpus/bench-1k
# Output: streaming p99=4.2ms, file-load p99=4.5ms, both ≤5ms gate
```

## 9. End-to-End Test

A synthetic byte-stream feeder (1 Gbps constant rate) drives the streaming parser. Time-to-first-event measured. Sustained throughput measured for 60 minutes. No unbounded buffer growth.

```bash
# Wire-speed test
buck2 run //tests/wire-speed:soak -- --rate-mbps 1000 --duration 60m
# Required: 0 buffer growth, sustained throughput, no event-emit delay > 5 ms
```

KPIs measured (from PHASE_GATES.md §5):
- Time-to-first-Merkle-commit (placeholder until P1.10): ≤ 5 ms p99 (HARD)
- Streaming decision latency: ≤ 20 ms typical APK p99 (HARD)
- Wire-speed inspection bandwidth single-core: ≥ 500 Mbps (HARD)
- Streaming-vs-file-load throughput parity: within 5%

## 10. Exit Checklist

- [ ] `ApkParser::from_reader` lands and tests pass
- [ ] Glommio runtime integrated; `tokio` not used on this code path
- [ ] `ParseEvent` enum stable + serializable
- [ ] Backpressure correctness verified — adversarial slow-consumer test green
- [ ] Time-to-first-event ≤ 5 ms p99 on Bench-1K (when corpus available)
- [ ] Wire-speed test sustains ≥ 500 Mbps for 60 min
- [ ] Streaming-vs-file throughput parity within 5%
- [ ] Pyroscope captures profile every CI run
- [ ] `docs/streaming-architecture.md` published
- [ ] No regression vs apk-info v0.x parse-throughput baseline

## 11. Hand-Off

| Consumed by | What they need |
|---|---|
| **P1.8** | Streaming parser to wrap with type-state phantoms |
| **P1.10** | Streaming parse points for Merkle commit hooks |
| **P1.13** | Streaming parser as input to differential fuzzer harness |
| **P1.15** | Streaming events as the source of AXIOM-IR emission |
| **P1.18** | Wire-speed harness reused for KPI gates |
