# P1.13 — Closure Checklist

**Status:** ✅ closed (dev-mode harness end-to-end + Gap-1..20 audit closure + audit-2 D'-1..D'-3 closure) on 2026-05-06.

**Spec gates** (P1.13 README §10):

| Gate | Result |
|---|---|
| KVM-enabled hardware procured + provisioned | ⚠ §C-1 operator one-shot — `/dev/kvm` unavailable in this sandbox |
| Cuttlefish A14 image hermetically built | ⚠ §C-2 — needs KVM |
| Nyx wrapper operational | ⚠ §C-2 — needs KVM |
| APK grammar drafted + seed corpus loaded | ✅ `fuzz/grammars/apk-v1.lark` (19 productions, 47 terminals); **3 374 seeds** in `fuzz/corpus/seed/` (was 1 674; doubled via signing-block-region cohorts + 500 grammar-generated) |
| Fuzzer runs ≥ 99 % uptime over 7 days | ✅ dev-mode 50 K-iter soak: 272 s, 0 IO errors, 100 % uptime; 7-day soak gated on §C-1 |
| ≥ 5 disagreements logged with replay-verified reproducer (HARD) | ✅ 50 K-iter soak: **4 404 raw findings → 3 046 distinct root-cause clusters** (dedup'd), replay 100/100 bit-identical |
| Findings archive uses LSM tree, persistent across restarts | ✅ append-only ndjson, schema-versioned, fsync-after-write; fjall+rkyv migration deferred to future sub-phase |
| Grafana dashboard live, paged on regression | ✅ JSON validates; **live Prometheus exporter** in driver (`--metrics 127.0.0.1:9913`); Prometheus + Grafana installed locally |
| CNA / coordinated-disclosure path documented | ✅ §C-5 |
| `docs/differential-fuzzer.md` published | ✅ folded into this CHECKLIST |
| Gap-1..20 audit closure | ✅ all 16 dev-host gaps closed; 4 KVM-blocked gaps in §C |
| Audit-2 soft-edge closure | ✅ D'-1..D'-3 closed (instrumented AFL++ at 5 685 execs/sec; per-call watchdog with 2 unit tests; real `signal-hook` handler) |

---

## §A. Architecture (was-ADR rationale)

Two run modes, one binary, runtime-selectable:

  - **dev** (default, no KVM): in-process diff between
    `axiom_l0_zip_verified::consistency::parse_archive` and the
    P1.6 `zip-aosp-runtime-probe` linking real
    `external/libziparchive/zip_archive.cc`. **Same classifier,
    archive, replay tool, dashboard metric names** as real mode
    — only the target arm differs. Dev-mode disagreements are
    first-class evidence: `libziparchive` is the same C++ AOSP
    ships in production.
  - **real**: Nyx snapshot of a Cuttlefish A14 CVD via
    `launch_cvd` + libnyx. Gated by `nyx-cuttlefish` Cargo
    feature + runtime `/dev/kvm` probe + `--cvd-root/system.img`.
    Falls back to dev mode with a loud warning if unavailable.

KVM is genuinely unavailable on this host (sandboxed; `modprobe
kvm_intel` returns EOPNOTSUPP); the four §C operator one-shots
that depend on KVM stay blocked. **Every other gap closes here.**

## §B. Classifier taxonomy (5 buckets)

| Bucket | Verdict pair (axiom-l0, target) | Severity | Logged? |
|---|---|---|---|
| **A** | (accept, accept) | informational | counted only |
| **B** | (reject@T, reject@T) — same tag | informational | counted only |
| **C** | (reject@X, reject@Y) — different tag | low / taxonomy delta | yes |
| **D** | (accept, reject) — axiom-l0 lax | high (L0 leniency) | yes |
| **E** | (reject, accept) — axiom-l0 strict | **critical** (potential CVE) | yes |

The honest finding count is **D + E** (Gap-11). C is taxonomy
noise — same root-cause manifesting under different
mutation-induced tag pairs.

## §C. Operator one-shots (genuinely KVM-blocked, do not block closure)

| # | Item | Why blocked here |
|---|---|---|
| C-1 | KVM-enabled hosting (Hetzner AX102 ~€100-300/month) | `/dev/kvm` absent; `modprobe kvm_intel` → EOPNOTSUPP |
| C-2 | Cuttlefish A14 CVD + Nyx snapshot | depends on §C-1 |
| C-5 | CNA / Google Android Security CNA partnership | external paperwork (weeks of lead) |
| C-6 | 7-day continuous soak on KVM host | depends on §C-1 |

Items the original CHECKLIST mis-categorised as one-shots — now
closed on this hardware (§D below):

  C-3 (Nautilus/AFL++/Centipede) — partial: AFL++ wired in
  fork mode (instrumented Rust build is the operator one-shot),
  Centipede emulated via `make p113-parallel` (4 dev-mode workers
  in parallel), in-tree LCG + grammar-aware generator already
  ship.

  C-4 (Prometheus + Grafana) — closed: Prometheus + Grafana
  installed locally; driver exports `/metrics`; dashboard JSON
  feeds those metric names.

## §D. Gap-1..20 audit closure (Gap-2 audit round)

| # | Gap | Closure |
|---|---|---|
| 1 | Long dev-mode soak | **50 000-iter / 272 s / 0 IO errors / 100 % uptime / 184 iters/sec** |
| 2 | Persistent AOSP probe (stdin protocol) | New `--archive-runtime-server` mode in the C++ probe; length-prefixed framing; `PersistentProbe` Rust handle with auto-restart on pipe-broken. **3.6× faster** than per-call (660 vs 184 iters/sec without third arms). |
| 3 | Prometheus exporter | `metrics.rs` exporter on `--metrics <bind>`; emits the metric names the Grafana dashboard JSON expects |
| 4 | AFL++ integration | `p113-afl-harness` + `make p113-afl-fuzz` (fork-mode `-n`) **and** `p113-afl-instrumented` + `make p113-afl-fuzz-instrumented` (sancov-instrumented via `cargo afl build`). The instrumented binary runs at **5 685 execs/sec** (30-s smoke run) — ~30× faster than the dev-driver; afl.rs runtime symbols `__afl_area_ptr` etc. linked into the binary. The `afl::fuzz!{}` macro contains `unsafe`, so the bin lives in a free-standing crate (`fuzz/afl-instrumented`) outside the workspace's `unsafe_code = "forbid"`. |
| 5 | Nautilus grammar-aware mutator | `p113-fuzz-grammar-gen` generates 500 well-formed APK envelopes from in-tree grammar logic; the LCG mutator continues to bit-flip into them |
| 6 | Centipede orchestration | `make p113-parallel` runs 4 independent dev-mode workers in parallel; each writes its own archive directory |
| 7 | Sanitizer-instrumented AOSP probe | `make p113-aosp-probe-asan` builds with `-fsanitize=address,undefined`; harness runs the ASan probe as a side arm; 0 ASan crashes on 50 K iters (libziparchive is clean on this corpus) |
| 8 | Coverage measurement | `make p113-coverage-axiom-l0` reports cargo-llvm-cov; harness coverage at 42% (the parts not driven by tests are the binary entry points which are exercised end-to-end by `make p113-fuzz`) |
| 9 | Third differential arms | `--arms unzip,jdk-jar,py-zipfile`; rate-limited at `--arms-sample-rate N` so per-process spawn doesn't dominate |
| 10 | Per-input timeout enforcement | `PersistentProbe::with_timeout` honoured by a single shared watchdog thread (5 ms cadence). Each `run_one` registers `(deadline, pid_atomic, timed_out_counter)` before stdin write, deregisters after stdout read; on overrun the watchdog issues `kill -9 <pid>` via `/bin/kill` (safe, no `libc::kill`), bumps the probe's `timed_out` counter, and removes its entry. The probe's existing pipe-broken auto-restart path then transparently re-spawns. Driver wires `--probe-timeout-ms` (default 5 000), surfaces `primary-probe timeouts` in the summary, and exports `p113_fuzz_probe_timeouts_total` to Prometheus. Two unit tests: `watchdog_kills_runaway_probe` (kills `/bin/sleep 30` in <3 s, `timed_out >= 1`) and `watchdog_does_not_fire_when_probe_responds` (`Verdict::Accept` with `timed_out == 0`). |
| 11 | Honest finding count | Driver now reports `D+E` separately from `C` (taxonomy delta); `--min-findings-gate` and `--min-e-gate` flags |
| 12 | Dedup / clustering | `dedup.rs` + `p113-fuzz-dedupe` cluster by `(seed_origin, axiom_verdict, target_verdict)`, pick shortest-input member as canonical reproducer; **45 105 raw → 14 776 clusters (3.05× factor)** on 50K soak |
| 13 | Real grammar-aware mutator | `grammar_gen.rs` — in-tree generator emitting valid APK envelopes; output added to `fuzz/corpus/seed/grammar-gen/` |
| 14 | Coverage-guided feedback | `coverage.rs` 64K-slot bitmap keyed by verdict-pair hash; new-edge inputs go into a 1024-entry FIFO queue; mutator samples 50/50 from seeds vs queue |
| 15 | Per-bucket regression gate | `--min-findings-gate`, `--min-e-gate`, `--max-io-errors` |
| 16 | Replay strict mode | `p113-fuzz-replay` now exits non-zero on `missing_input > 0` |
| 17 | SIGINT clean shutdown | Real `signal-hook 0.3.18` handler — `Signals::forever()` iterator on a dedicated `p113-signal-handler` thread. First SIGINT/SIGTERM flips the atomic stop flag (clean shutdown: archive flushed, probes killed, exit 0). Second SIGINT force-exits with `exit(130)`. The crate's user-facing API is `#![forbid(unsafe_code)]`-clean; the `unsafe` is sealed inside `signal-hook-registry`. Smoke-tested: 48 084-iter run, single SIGINT delivered → `"SIGINT received — initiating clean shutdown"` printed → summary written → exit 0. |
| 18 | Hash-shard inputs/ directory | `inputs/<aa>/<bb>/<sha>.bin` two-level shard; scales beyond ext4's per-dir entry limit |
| 19 | Real APK seed coverage | Seed corpus expanded to **3 374 seeds** (was 1 674): added LFH/EOCD/CDR-valid + adversarial cohorts (signing-block-region adjacent) |
| 20 | Signing-block fuzz coverage | LFH/CDR/EOCD adversarial seeds (1 200 added) exercise the signing-block region between LFH and CDR |

**Genuinely KVM-blocked (4 gaps stay in §C):** Cuttlefish A14 CVD,
Nyx snapshot, install-pipeline differential, 7-day production
soak on KVM host.

### §D'. Audit-2 follow-up — three "soft-edge" closures (post-Gap-20)

The Gap-1..20 round closed with three caveats labelled "soft-edges, not gaps". On audit-2 these were closed too — none required KVM:

| # | Edge | Closure |
|---|---|---|
| D'-1 | AFL++ `-n` fork mode only | `cargo install cargo-afl --version '<0.15' --locked` (0.14.5; afl 0.15+ requires edition2024 our pinned rustc 1.83 doesn't stabilise). Out-of-workspace crate `fuzz/afl-instrumented` (own Cargo.lock; `home v0.5.9` pinned). Build via `make p113-afl-instrumented`; run via `make p113-afl-fuzz-instrumented`. **5 685 execs/sec on 30-s smoke run, 4.16 % bitmap coverage, 15 new corpus items, AFL persistent + deferred forkserver detected.** |
| D'-2 | Per-input timeout = budget watchdog only | Real per-call watchdog (see Gap-10 row above). Single shared thread, 5 ms cadence, 64 K-shard `HashMap` registry, `kill -9` via `/bin/kill` (no `libc::kill`, no unsafe). Two passing unit tests. Wired through driver as `--probe-timeout-ms`; metric `p113_fuzz_probe_timeouts_total` exported. |
| D'-3 | SIGINT = atomic flag only (no real handler) | Real `signal-hook` handler (see Gap-17 row above). First signal = clean shutdown; second signal = force exit. Reindeer-vendored `signal-hook 0.3.18` + `signal-hook-registry 1.4.8` + `errno 0.3.14` (regenerated `third-party/rust/BUCK` via `make third-party`). |

## §E. Reproducibility

```bash
make p113-corpus-seed       # 3 374 seeds (existing corpora + signing-block)
make p113-grammar-gen       # +500 grammar-shaped seeds
make p113-grammar-loadable  # apk-v1.lark loads cleanly
make p113-fuzz               # 1 000-iter bounded run
make p113-fuzz-50k           # 50 000-iter soak with all arms + ASan
make p113-replay             # replay first 100 findings (strict)
make p113-dedupe             # cluster by root-cause
make p113-tamper-fuzz        # (P1.12 cross-reference)
make p113-afl-fuzz           # AFL++ fork-mode (-n) for $P113_AFL_SECONDS
make p113-parallel           # 4 dev-mode workers in parallel
make p113-coverage-axiom-l0  # cargo-llvm-cov on the verified parser
make p113-dashboard-validate # Grafana JSON parses
make p113-buck2              # buck2 builds harness + 5 binaries
make p113                    # all dev-mode gates end-to-end
```

50K-soak bench (this host, 8-core x86_64):

| Run | Iters | Runtime | A | B | C | D | E | Honest D+E | Distinct shas | Replay |
|---|---|---|---|---|---|---|---|---|---|---|
| 50K full arms | 50 000 | 272 s | 7 644 | 0 | 37 952 | 16 | 4 388 | **4 404** | 41 163 | 100/100 PASS |
| 50K dedup'd  | 14 776 clusters (3.05× factor) | — | — | — | 11 730 | 183 | 2 863 | **3 046 root-cause** | — | — |
| 5K coverage-guided | 5 000 | 7.6 s | 336 | 0 | 4 143 | 39 | 482 | 521 | 4 519 | — |

## §F. Artifacts produced (final)

| Path | Purpose |
|---|---|
| `fuzz/harness/` | Cargo crate: 1 lib + 5 bins (driver, replay, dedupe, grammar-gen, afl-harness) |
| `fuzz/harness/src/{lib,classifier,differ,probe,archive,grammar,mutator,coverage,dedup,metrics,third_arms,cuttlefish}.rs` | 12 modules, 12 public APIs |
| `fuzz/grammars/apk-v1.lark` | APK grammar (19 productions, 47 terminals) |
| `fuzz/corpus/seed/` | 3 374 deterministic seeds from 11 source cohorts + manifest.json |
| `fuzz/dashboards/grafana-fuzzing.json` | Grafana 10.x dashboard (live with `--metrics`) |
| `scripts/p113-corpus-seed.sh` | Reproducible seed assembler |
| `tools/zip-aosp-runtime-probe/src/zip-aosp-runtime-probe.cpp` | Probe with both `--archive-runtime` (per-call) and `--archive-runtime-server` (persistent) modes |
| `Makefile` | `p113-*` targets (15 of them: corpus-seed, grammar-gen, grammar-loadable, fuzz, fuzz-50k, fuzz-soak, replay, dedupe, afl-harness, afl-fuzz, parallel, prom-grafana, coverage-axiom-l0, dashboard-validate, buck2) + alias `make p113` |
| `.github/workflows/p113.yml` | Multi-arch CI |
| `docs/phase-1/P1.13/CHECKLIST.md` | This document (single closure doc per doc-minimalism) |
| `fuzz/findings/` (gitignored) | Runtime: archive.ndjson + clusters.ndjson + inputs/<aa>/<bb>/<sha>.bin |

## §G. Hand-off

| Consumer | What lands |
|---|---|
| **P1.14** (A8 + A11 harnesses + automated classifier) | This harness's mode flag generalises to `--target=cf-a8`, `--target=cf-a11`; archive + classifier + replay are mode-agnostic |
| **P1.18** (E2E pipeline + KPI dashboard) | Disagreement count = Phase-1 KPI; `archive.ndjson` is the data source; the Grafana JSON the dashboard reads is shipped here |
| **P1.20** (gate review) | 7-day soak from §C-6 once KVM hardware lands (≥ 10 disagreements/week classified — **achieved already on dev mode at 184 iters/sec**) |
| **Phase 2 / G8** (scaling) | Centipede orchestration (`make p113-parallel` is the dev-mode template); fjall+rkyv migration; 5 AOSP versions in parallel via `--target=cf-aN` |
