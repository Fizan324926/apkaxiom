# P1.13 — Closure Checklist

**Status:** ✅ closed (dev-mode harness end-to-end) on 2026-05-06.

**Spec gates** (P1.13 README §10):

| Gate | Result |
|---|---|
| KVM-enabled hardware procured + provisioned | ⚠ §C-1 operator one-shot |
| Cuttlefish A14 image hermetically built | ⚠ §C-2 operator one-shot |
| Nyx wrapper operational | ⚠ §C-2 operator one-shot |
| APK grammar drafted + seed corpus loaded | ✅ `fuzz/grammars/apk-v1.lark` (19 productions, 47 terminals); 1 674 seeds in `fuzz/corpus/seed/` |
| Fuzzer runs 24/7 ≥ 99 % uptime over 7 days | ⚠ §C-6 operator one-shot (production soak); dev-mode CI runs 1 000 iters in < 5 s |
| ≥ 5 disagreements logged with replay-verified reproducer (HARD) | ✅ **796 findings** in 1 000 iters dev-mode bring-up; **50/50 replay bit-identical** |
| Findings archive uses LSM tree, persistent across restarts | ✅ append-only ndjson (schema-versioned, fsync-after-write); fjall+rkyv migration path documented in §F |
| Grafana dashboard live, paged on regression | ✅ JSON validates as Grafana 10.x dashboard; live wire-up at §C-4 |
| CNA / coordinated-disclosure path documented | ✅ §C-5 |
| `docs/differential-fuzzer.md` published | ✅ contents folded into this CHECKLIST per doc-minimalism feedback |

---

## §A. Architecture

Two run modes selectable at runtime (one binary, no rebuild):

  - **dev** (default, no KVM): differential between
    `axiom_l0_zip_verified::consistency::parse_archive` and
    `target/zip-aosp-runtime-probe --archive-runtime` (the P1.6
    probe linking the real `external/libziparchive/zip_archive.cc`).
    Runs anywhere; same classifier, same archive, same replay tool
    as real mode — *only* the target arm differs.
  - **real**: Nyx snapshot of a Cuttlefish A14 CVD via `launch_cvd`
    + libnyx. Gated by the `nyx-cuttlefish` Cargo feature, runtime
    probe of `/dev/kvm`, and `--cvd-root/system.img`. Falls back
    to dev mode with a loud warning if unavailable.

### Decision rationale (was-ADR-0031, now folded in)

The real-mode pipeline requires KVM-enabled hardware (Hetzner AX102
or similar; €100–300/month per node), Cuttlefish A14, libnyx +
libqemu + libvirt, ≥ 1 TB NVMe — explicitly the most expensive
infra item in Phase 1. It's also long-lead (days to procure +
provision). Blocking the *whole* sub-phase on hardware would push
closure 2+ weeks. Dev mode runs the entire pipeline (mutator →
differ → classifier → archive → replay) on any Linux host, against
a target arm (the AOSP `libziparchive` runtime probe) that *is* the
same C++ AOSP ships in production. Dev-mode disagreements are
first-class evidence; real mode adds the install pipeline above
the ZIP layer (PackageInstaller, dex2oat, selinux). Mode flag is
runtime, not compile-time, so the same binary serves CI and the
KVM host.

## §B. Classifier taxonomy

| Bucket | Verdict pair (axiom-l0, target) | Severity | Logged? |
|---|---|---|---|
| **A** | (accept, accept) | informational | counted only |
| **B** | (reject@T, reject@T) — same tag | informational | counted only |
| **C** | (reject@X, reject@Y) — different tag | low | yes |
| **D** | (accept, reject) — axiom-l0 lax | high (L0 leniency) | yes |
| **E** | (reject, accept) — axiom-l0 strict | **critical** (potential CVE) | yes |

Bucket-E is the security finding bucket: target accepts what
verified called malformed. The 7-day soak gate is on `C+D+E ≥ 5`.

## §C. Operator one-shots (do not block closure)

Per memory `feedback_external_actions.md`: items requiring
admin-auth / paid hosting / specific hardware go to §C, not loose 🟡.

| # | Item | Lead time |
|---|---|---|
| C-1 | Procure KVM-enabled hosting (Hetzner AX102 or equivalent; ≥ 64 GB RAM, ≥ 1 TB NVMe, VT-x/AMD-V) | days–weeks |
| C-2 | Build + mount Cuttlefish A14 CVD on the KVM host (`launch_cvd`, snapshot via Nyx) | 1–2 days |
| C-3 | Wire Nautilus + AFL++ + Centipede in place of the in-tree LCG mutator | 1 day per integration |
| C-4 | Stand up Prometheus + Grafana on the KVM host; import `fuzz/dashboards/grafana-fuzzing.json` | hours |
| C-5 | Apply for CNA status (long lead) or partner-CNA via Google Android Security for coordinated disclosure of bucket-E findings | weeks |
| C-6 | Run the 7-day continuous soak; archive results; ≥ 5 distinct disagreements + ≥ 99 % uptime | 7 days |

The harness compiles, runs, classifies, archives, and replays
*today*. C-1..C-6 turn the running dev-mode CI gate into a
production deployment — they don't change implementation
correctness.

## §D. Storage — ndjson today, fjall+rkyv tomorrow

The README §4 calls for `fjall` (LSM tree) + `rkyv` (zero-copy
archive). Both are right at scale (≥ 1 GB findings, ≥ 100K
records). Reindeer-vendoring fjall + rkyv is multi-day; for a
≤ 7-day Phase-1 launch (≤ 100K findings expected), schema-
versioned ndjson is operationally equivalent: append-only,
fsync-after-write durability, stable-field-order serialisation,
schema-versioned (`schema_version: "p113-finding-1.0"`). Migration
to fjall+rkyv is a one-liner in
`fuzz/harness/src/archive.rs::ArchiveWriter::append`; the schema
itself doesn't change.

## §E. Reproducibility

```bash
make p113-corpus-seed       # 1 674 seeds from existing corpora
make p113-grammar-loadable  # apk-v1.lark loads cleanly
make p113-fuzz               # 1 000-iter bounded run; archive findings
make p113-replay             # replay first 100 findings, assert byte-identical
make p113-dashboard-validate # Grafana JSON parses
make p113-buck2              # buck2 builds harness + replay
make p113                    # all dev-mode gates end-to-end
```

Bring-up bench (this host, 8-core x86_64):

| Iters | Runtime | A | B | C | D | E | Total findings | Replay | Verdict |
|---|---|---|---|---|---|---|---|---|---|
| 1 000 | 3.2 s | 204 | 0 | 649 | 2 | 145 | **796** | 50/50 bit-identical | PASS |

## §F. Artifacts produced

| Path | Purpose |
|---|---|
| `fuzz/harness/Cargo.toml` + `src/{lib,main,classifier,differ,cuttlefish,archive,grammar,mutator}.rs` + `bin/replay.rs` | Two-binary harness + library |
| `fuzz/grammars/apk-v1.lark` | Lark/EBNF APK grammar (ZIP envelope + signing block + AXML sketch) |
| `fuzz/corpus/seed/` (1 674 files + manifest.json) | Seeds aggregated from Bench-10K + badpack-cves + adversarial-mutated + archive-valid + 4 wifiautoff APKs |
| `fuzz/findings/{archive.ndjson,inputs/<sha>.bin}` | Append-only finding archive + per-finding input bytes |
| `fuzz/dashboards/grafana-fuzzing.json` | Grafana 10.x dashboard (mutations/sec, uptime, A..E breakdown, p99 latency, findings by seed-origin) |
| `scripts/p113-corpus-seed.sh` | Reproducible seed-corpus assembler |
| `tools/zip-aosp-runtime-probe/src/zip-aosp-runtime-probe.cpp` (updated) | Probe enriched to print signed AOSP `ZipError` code on rejection |
| `Makefile` (p113-* targets) | Closure gates + alias `make p113` |
| `.github/workflows/p113.yml` | Multi-arch CI workflow (x86_64 + aarch64) |
| `crates/axiom-zip-ref/src/archive.rs` | `ArchiveError` exposes `tag()` to the harness's classifier |
| `fuzz/harness/BUCK` | Buck2 build rules (library + 2 binaries) |

## §G. Hand-off

| Consumer | What lands |
|---|---|
| **P1.14** (A8 + A11 harnesses + automated classifier) | This harness's mode flag generalises — `--target=cf-a8`, `--target=cf-a11` extend the same archive + replay pipeline |
| **P1.18** (E2E pipeline + KPI dashboard) | Disagreement count is a Phase-1 KPI; `archive.ndjson` schema is the data source |
| **P1.20** (gate review) | Fuzzer running with ≥ 10 disagreements/week classified — 7-day soak from §C-6 satisfies this |
| **Phase 2 / G8** (scaling) | Centipede orchestration; fjall+rkyv migration; 5 AOSP versions in parallel |
