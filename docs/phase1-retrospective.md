# Phase 1 Retrospective — Parser Foundation

Phase 1 built the formal-verification and performance foundation for APKAXIOM.
This document records what shipped, the honest delta from the original plan,
and the carry-forward debt entering Phase 2.

---

## What shipped (20 sub-phases)

### Foundation (P1.1–P1.4)

- **P1.1** — Hermetic build: Buck2 + Nix flake + Reindeer; reproducible on all
  reference machines. ADRs 0006–0011 (provenance, SBOM signing, repro budget,
  graph parity, rebuilder federation).
- **P1.2** — Lean pin: mathlib4 vendored, `axiom-l1-rs` placeholder theorem
  re-verifies in <10 min on CI. Lake build script wired into Buck2.
- **P1.3** — apk-info v0.x audit: 5 operator one-shots in §F; ADR-id collision
  resolved (future ADRs start at 0012). Upstream pinned at commit 759b39ce.
- **P1.4** — IR spec: AXIOM-IR v0.1 cap'n proto schema frozen; freeze-hash and
  capnp-hash pinned. ADRs 0013–0015.

### Lean ZIP layer (P1.5–P1.6)

- **P1.5** — LFH completeness: 1 800/1 800 Lean↔Rust↔AOSP three-way diff;
  libziparchive vendored (1.8 MB). ADRs 0016+0017.
- **P1.6** — CDR + EOCD completeness: 41 universal completeness theorems via
  `show + rfl` and `bv_decide`; 2 860/2 860 three-way diff. ADRs 0018+0019.

### apk-info v1.0 (P1.7–P1.8, P1.10, P1.15)

- **P1.7** — Streaming reader: sync 354 Mbps 60-min soak; io_uring 21.5 Gbps
  30-min soak. ADRs 0020+0021.
- **P1.8** — Type-state machine: `Apk<Unverified>` / `Apk<Verified>` phantom
  types; Jarv1Carrier + ApkSigBlock split; 4 F-Droid APK fixtures; 77 tests.
  ADRs 0022–0024.
- **P1.10** — BLAKE3 Merkle: 1.601 GB/s (HACL*-verified); 35 official KAT
  vectors × 3 modes; 40 K mutations × 4 fixtures = 100% tamper kill.
  `+12.66%` overhead (under 15% gate).
- **P1.15** — AXIOM-IR emitter: `emit_manifest` + `reencode_manifest`;
  AXML binary parse + re-encode round-trip; IR determinism gate.

### Lean Signing Block (P1.11)

- **P1.11** — APK Signing Block formal spec in Lean 4: 4 029 LOC; v2/v3/v3.1
  rotation lineage; 17-APK Lean↔Rust↔apksigner agreement; tamper-fuzz 100%
  on committed regions; coverage 88%. ADR-0029.

### Extraction pipeline (P1.9, P1.12)

- **P1.9** — Three-way translation validator: Lean ↔ hand-Rust ↔ extracted-Rust
  on 1 499/1 499 LFH inputs; 299/299 EOCD; 10 K mutation fuzz 0 divergences;
  mutation kill 100% (28/28 viable). ADRs 0025–0027.
- **P1.12** — Extracted ZIP verifier: 11 gates green; TV-receipt umbrella;
  real-APK throughput 15 M/16-core (projected); AOSP runtime parity 10 K/10 K;
  coverage 97.4%. ADR-0030.

### Differential fuzzer (P1.13, P1.14)

- **P1.13** — Dev-mode harness: AFL++ at 5 685 execs/sec; per-call watchdog;
  SIGINT handler; 50 K soak 4 404 honest findings, 100/100 replay bit-identical.
- **P1.14** — Cross-version differential plant: 100/100 real F-Droid APKs in
  Bucket A (0 false-positive XV); xgboost 100% on test split.

### Signing verifier (P1.16)

- **P1.16** — Full production verifier: 1 000/1 000 verdict agreement with
  apksigner on bench-1k; 204 APKs/sec; libcrux ECDSA-P256 on hot path;
  RustCrypto DSA-SHA256 for the 3/1 000 DSA-only APKs in the corpus.

### Soundness regression CI (P1.17)

- **P1.17** — Fail-closed soundness gate: sorry-audit + lake-verify + TV +
  signing tests in a single CI job; `make soundness`; deliberate-break runbook;
  mathlib upgrade runbook.

### End-to-end evaluation (P1.18–P1.19)

- **P1.18** — Bench-1K smoke harness: `p118-e2e` binary (BLAKE3+ZIP+IR+verify);
  K2 p99=18.4 ms, K3 RSS=18 MB, K10 repro, K9 cross-arch CI gate.
- **P1.19** — AndroZoo comparison + paper: 350× faster than Androguard full
  analysis (HARD ≥10× gate); 15.3× faster p50 vs manifest-only; 653-line LNCS
  CAV 2026 draft; `docs/phase1-eval.md`.

---

## Delta from original plan

| Item | Planned | Actual | Delta |
|------|---------|--------|-------|
| Lean LOC | ≥2 000 | ≥4 029 (P1.11 alone) | **+2×** |
| Three-way TV receipts | LFH only | LFH + EOCD + 10 K fuzz | **broader** |
| Signing verifier algorithms | RSA-PKCS1 + RSA-PSS | + ECDSA-P256 (libcrux) + DSA-SHA256 | **+2 algs** |
| BLAKE3 throughput gate | ≥1.5 GB/s | 1.601 GB/s | **PASS** |
| Bench-1K p99 | ≤300 ms | 18.4 ms | **16× under gate** |
| Peak RSS | ≤150 MB | 18 MB | **8× under gate** |
| apk-info comparison | planned vs apk-info + Androguard | Androguard done; apk-info blocked (edition 2024) | **partial** |
| AndroZoo 10K eval | planned | not run — API key needed | **carry-forward** |
| K4 CPU efficiency | planned | not measured (perf stat / bare metal) | **carry-forward** |
| K8 stress/burst | planned | not measured (no load infra) | **carry-forward** |
| 24 h soak | planned | 50 K run only (P1.13); full soak not run | **carry-forward** |

**Notable unplanned wins:** GP bit-3 `usz=0` corpus finding (any parser using
declared size to cap decompression truncates the manifest on a measurable class
of F-Droid APKs); DSA-SHA256 discovery in bench-1k (3/1 000 APKs use DSA-only
signing — not in the original 100-APK test set); AFL++ at 5 685 execs/sec on
the dev host without KVM.

---

## Phase 1 carry-forward debt

17 hard-gate rows carry into Phase 2. All are infrastructure-blocked:

| Debt item | Blocker | P2 priority |
|-----------|---------|-------------|
| K1 16-core throughput on Bench-10K | AndroZoo API key + 16-core EPYC | P2.10 |
| K2 adversarial worst-case (Adversarial-500) | corpus not built | P2.10 |
| K3 24 h soak, alloc rate, fragmentation | Stress-100K host | P2.10 |
| K4 all CPU efficiency metrics | bare-metal `perf stat` | P2.10 |
| K5 multi-core, multi-machine scaling | no cluster | P2.10 |
| K6 wire-speed ≥500 Mbps (sync path 354 Mbps) | this host; io_uring path passes | P2.11 |
| K7 crash rate (1 M APKs), MTBF | Stress-100K host | P2.10 |
| K8 all stress/burst metrics | no load infra | P2.10 |
| K9 ARM64 throughput parity | GitHub Actions ARM64 runner quota | P2.11 |
| K10 cross-machine byte-identity (3 machines) | second + third machines | P2.11 |
| AXIOM-IR frozen ≥4 weeks | 4-week clock; auto-closes with no IR changes | auto |
| Differential fuzzer 24/7 ≥10/week | CI-as-a-service infra | P2.11 |
| apk-info v0.x comparison | Rust edition 2024 (≥1.85) not in Nix pin | P2.10 |
| Bench-10K / AndroZoo 10K eval | AndroZoo API key | P2.10 |
| Sign-off from G1/G2/G3/G8/G13 | personnel | §C |
| Public benchmark dashboard | Grafana/Prometheus infra | P2.11 |
| CAV 2026 submission | Bench-10K numbers + institutional affiliation | P2.10 |

---

## Phase 2 entry state

- All Phase 1 code is on `main`, CI green, no open `sorry` in `theorems/`.
- Lean LOC: ≥4 029 (P1.11 alone exceeds the ≥2 000 Phase 1 target).
- ADRs sealed: 0006–0030; next free ADR is 0032 (0031 = Phase 2 scope).
- TV receipt: 1 499/1 499 LFH + 299/299 EOCD + 10 K mutation fuzz clean.
- Bench-1K all K2+K3 gates PASS; K10 reproducibility PASS; K11 soundness PASS.
- Phase 2 scope defined in ADR-0031.
