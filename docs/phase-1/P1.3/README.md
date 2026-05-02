# P1.3 — apk-info v0.x Audit & v1.0 Architecture Spec

> Read every line of upstream `apk-info`. Decide what stays, what's rewritten, what migrates. Spec `axiom-l1-rs` v1.0 before a single line is touched.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../README.md](../../../README.md) · [../../README.md §22 (apk-info integration)](../../README.md#apkinfo-integration)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P1.3 |
| Owner(s) | G2 (Parser Engineering & AOSP Archaeology) |
| Duration | Weeks 1–3 (parallel with P1.1) |
| Critical-path | **no** — runs in parallel; only blocks P1.7 onward |
| Hard prerequisites | none — design phase |

## 2. Goal & Scope

A 30-page audit report on the upstream `apk-info` codebase ([github.com/delvinru/apk-info](https://github.com/delvinru/apk-info)) and a v1.0 architecture spec for `axiom-l1-rs` (the APKAXIOM successor) approved by G1, G2, G3 leads. **No code is written in this sub-phase.** The output is two documents and two ADRs.

The audit answers: which modules are sound enough to keep, which need rewriting, which type-state guards are missing, where the unverified Rust diverges from AOSP semantics, and where the streaming refactor must intervene.

### In scope
- Full code-walk of upstream `apk-info` v0.x (every crate, every public API).
- Performance baseline measurement of `apk-info` v0.x against Bench-1K when available, or against synthetic inputs in the meantime.
- v1.0 spec covering: streaming reader, per-Android-version trait, type-state phantom types, BLAKE3 Merkle commit hooks, AXIOM-IR-v0.1 emitter.
- Migration path from `apk-info` v0.x → `axiom-l1-rs` v1.0 with timeline.
- ADR-0005 (`axiom-l1-rs` as engineering beachhead — not a rewrite).
- ADR-0007 (versioning policy: `apk-info` continues as upstream-compatible crate; `axiom-l1-rs` is the APKAXIOM-internal name).

### Out of scope
- Any code changes to `apk-info` v0.x.
- Implementing the v1.0 spec (P1.7, P1.8, P1.10, P1.15 do this).
- Performance regression tests against Androguard (P1.7 baseline).

## 3. Hard Dependencies on Prior Sub-Phases

None — this sub-phase begins on day 1 in parallel with P1.1.

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **git** | 2.43+ | Clone upstream + history walk |
| **gh** | 2.89+ | Open issues / PRs upstream if needed |
| **rustc / cargo** | 1.95 | Build `apk-info` v0.x for measurement |
| **cargo-bloat** / **cargo-llvm-lines** | latest | Size analysis of v0.x crates |
| **cargo-flamegraph** | latest | Where time is spent in v0.x |
| **hyperfine** | 1.18+ | Micro-benchmarks |
| **apktool** | 2.9+ (HAVE on host) | Reference APK decoder for cross-check |
| **markdown lint / prettier** | latest | Doc consistency |
| **graphviz** (`dot`) | 2.42+ | Architecture diagrams in spec |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL / Account | Notes |
|---|---|---|---|---|
| **GitHub access to delvinru/apk-info** | public source | **Free** | https://github.com/delvinru/apk-info | Apache 2.0; clone freely |
| **GitHub Discussions / Issues with upstream** | community | **Free** | Same repo | Engage upstream maintainer (Alexey / delvinru) early about migration plans — courtesy + technical input |
| **AndroZoo** | APK corpus | **Free** for academic | https://androzoo.uni.lu | API key issued after academic-email signup; needed for Bench-1K curation in P1.18 — *request key now to avoid blocking later* |
| **MalwareBazaar** (abuse.ch) | malware feed | **Free** | https://bazaar.abuse.ch | Used for sourcing C2 (repackaged-malware) test inputs; account creation free |
| **VirusTotal** | ground-truth labels | Free tier; **Paid** ($$$/yr) for production | https://www.virustotal.com/gui/join-us | Free tier sufficient for v0.x performance reference work |
| **F-Droid archive** | open-source APKs | **Free** | https://f-droid.org/archive/ | Reference clean APK corpus |
| **arXiv account** | preprint | **Free** | https://arxiv.org/user/ | Useful for citing recent BadPack-class papers in the audit doc |

**No API keys yet** unless AndroZoo signup is pursued early (recommended).

**Account-level decisions made here:**
- Whether to fork `apk-info` to `Fizan324926/apk-info-axiom-l1` (recommended) or vendor source-only into our repo.
- Communication channel with upstream maintainer (GitHub issue thread vs. private email).

## 6. System Inventory — Have vs Need

### Already present (verified)
- ✅ git, gh, rustc, cargo, make, cmake
- ✅ apktool (already installed!)
- ✅ python3, jq

### Missing — must install
- ❌ **cargo-bloat** — `cargo install cargo-bloat`
- ❌ **cargo-llvm-lines** — `cargo install cargo-llvm-lines`
- ❌ **cargo-flamegraph** — `cargo install flamegraph`
- ❌ **hyperfine** — `sudo apt-get install -y hyperfine`
- ❌ **graphviz** — `sudo apt-get install -y graphviz`
- ❌ **prettier** (markdown) — `npm install -g prettier`

### Install commands

```bash
# Rust analysis tools
cargo install cargo-bloat cargo-llvm-lines flamegraph

# System tools
sudo apt-get update
sudo apt-get install -y hyperfine graphviz linux-tools-common linux-tools-generic

# Markdown tooling
npm install -g prettier @prettier/plugin-markdown markdownlint-cli2

# Bring upstream apk-info
git clone https://github.com/delvinru/apk-info.git external/apk-info-upstream
cd external/apk-info-upstream && git rev-parse HEAD > ../apk-info-pinned-sha.txt
```

## 7. Working Directory & Files Produced

```
apkaxiom/
├── docs/
│   ├── apk-info-audit.md              # NEW — ~30 page audit report
│   ├── axiom-l1-rs-spec.md            # NEW — v1.0 spec
│   ├── ADR-0005-axiom-l1-beachhead.md # NEW — apk-info as beachhead, not rewrite
│   ├── ADR-0007-versioning-policy.md  # NEW
│   └── apk-info-perf-baseline.md      # NEW — measured v0.x numbers
├── external/
│   ├── apk-info-upstream/             # NEW — read-only vendored clone
│   └── apk-info-pinned-sha.txt        # NEW — SHA pinned for audit
└── diagrams/
    ├── axiom-l1-rs-architecture.dot   # NEW — graphviz source
    └── axiom-l1-rs-architecture.svg   # NEW — rendered output
```

### What `apk-info-audit.md` covers (30 pages)
1. Module inventory — every crate, every public type, every public function.
2. Soundness review — where v0.x makes assumptions not backed by Lean (i.e., everything; this sub-phase classifies the gaps by severity).
3. Performance baseline — measured throughput, latency, memory on synthetic inputs and on a handful of public APKs.
4. Memory-safety audit — `cargo audit`, `cargo-bloat`, `unsafe` block census.
5. AXIOM-IR readiness — what data structures already align with the manifest dialect, what needs transformation.
6. Migration cost — per-module estimate of lines-changed and risk.
7. Recommendation per module — *keep / refactor / rewrite / delete*.

### What `axiom-l1-rs-spec.md` covers
1. Public API surface — every type-state state, every public method, every error variant.
2. Streaming reader trait — `ApkParser::from_reader<R: Read>`.
3. Per-Android-version dispatch — trait `AndroidVersionParser`.
4. BLAKE3 Merkle commit hooks — emission contract per parse step.
5. AXIOM-IR-v0.1 emitter — manifest-dialect output specification.
6. Compatibility commitments — what stays compatible with `apk-info` v0.x for ecosystem continuity.

## 8. Standalone Output

The two reports + two ADRs themselves. They are reviewable, citable, and self-contained — anyone (including upstream maintainers) can read them without needing to read APKAXIOM source.

## 9. End-to-End Test

This sub-phase is design-only. The "test" is **lead sign-off**: G1, G2, G3 leads must each leave a documented review on the audit + spec. Sign-offs are checked into `docs/sign-offs/P1.3.md`.

```bash
# Verification: every required reviewer left a "✅ approved" comment
grep -c "^✅ approved by G" docs/sign-offs/P1.3.md
# expected: ≥ 3
```

## 10. Exit Checklist

- [ ] `apk-info` v0.x audit ≥ 30 pages, with per-module recommendations
- [ ] `axiom-l1-rs` v1.0 spec frozen and reviewed by G1, G2, G3 leads
- [ ] Migration path documented with per-sub-phase ownership
- [ ] ADR-0005 (beachhead, not rewrite) merged
- [ ] ADR-0007 (versioning) merged
- [ ] Upstream maintainer engaged with courtesy notification + offer of cross-review
- [ ] AndroZoo academic-access request submitted (don't wait until P1.18)
- [ ] Performance baseline measured on hyperfine + flamegraph for ≥ 100 sample APKs
- [ ] Architecture diagrams rendered (graphviz → svg) and embedded in spec

## 11. Hand-Off

| Consumed by | What they need |
|---|---|
| **P1.4** | AXIOM-IR-v0.1 manifest-dialect output specification (the spec is the contract) |
| **P1.7** | Streaming reader trait API surface |
| **P1.8** | Type-state phantom-type design |
| **P1.10** | Merkle commit emission contract |
| **P1.15** | AXIOM-IR emitter contract |
| **P1.18** | AndroZoo access (provisioned in this sub-phase to avoid Phase-1 end-of-cycle blocking) |
| **G2 (ongoing)** | Per-module migration roadmap |
