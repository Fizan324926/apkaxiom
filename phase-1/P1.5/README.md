# P1.5 — Lean ZIP Layer: Local File Headers + EOCD

> First real Lean theorem on actual ZIP semantics. ~600 LOC formalizing local file headers + the end-of-central-directory record. Cross-checked against AOSP `libziparchive` on 1,000+ inputs.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../README.md §6 (Layer 1)](../../README.md#layer-1)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P1.5 |
| Owner(s) | G1 (Formal Methods Core) |
| Duration | Weeks 3–7 |
| Critical-path | **yes** — every later Lean theorem builds on this |
| Hard prerequisites | P1.2 (Lean toolchain), P1.4 (IR type-system primitives) |

## 2. Goal & Scope

The ZIP local file header (LFH) and the end-of-central-directory record (EOCD) are formalized in Lean 4. A theorem states that parsing an LFH-prefixed byte sequence yields the typed structure that AOSP `libziparchive` would produce on the same input. Adversarial inputs (BadPack-class, malformed offsets) are part of the test corpus from day one.

### In scope
- `theorems/Apkaxiom/Zip/LocalHeader.lean` (~600 LOC).
- `theorems/Apkaxiom/Zip/Eocd.lean` (~400 LOC).
- Soundness theorem `parseLfh_sound : ∀ bs, parseLfh bs = ok h → libziparchive_parseLfh bs = ok h`.
- Property-based corpus generator (≥ 1,000 hand-fuzzed LFHs + 100 valid + adversarial EOCDs).
- AOSP `libziparchive` cross-check harness (compiles libziparchive, feeds same bytes, diffs results).

### Out of scope
- Central directory record (P1.6).
- Cross-record consistency (P1.6).
- APK Signing Block (P1.11).
- Rust extraction (P1.9, P1.12).

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P1.1** | Bazel sub-workspace at `external/aosp/` for libziparchive build; Buck2 for Lean rule |
| **P1.2** | Working Lake build; mathlib4 cache; "hello" theorem proves the toolchain works |
| **P1.4** | IR type-system primitives (Lean reflection of byte arrays, results) |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Lean 4** | pinned via P1.2 | Theorem prover |
| **mathlib4** | pinned commit | `ByteArray`, `Std.Data`, `Result` |
| **AOSP source — `system/core/libziparchive`** | commit pinned per Android API level | Reference implementation we cross-check against |
| **AOSP repo tool** | latest | Fetches AOSP source layered manifests |
| **Bazel** | 7.x | Compiles libziparchive into a small standalone binary in our sub-workspace |
| **C++ toolchain** | clang 18+ (HAVE) | Builds libziparchive |
| **Rust** | 1.95 | Differential-test harness driver (calls Lean evaluator + libziparchive binary, compares) |
| **Hypothesis** (Python) or **proptest** (Rust) | latest | Property-based input generation |
| **AFL++ / libFuzzer** | latest | Adversarial corpus generation (used here as a *generator*, not a fuzzer in the runtime sense) |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL / Account | Notes |
|---|---|---|---|---|
| **AOSP source** | full Android source tree | **Free** OSS (Apache 2.0) | https://source.android.com | Pinned commits; we sync only the components we need |
| **Google `repo` tool** | manifest-driven multi-repo client | **Free** OSS | https://gerrit.googlesource.com/git-repo | Required to fetch AOSP correctly |
| **AOSP build prerequisites** | various OS packages | **Free** | https://source.android.com/setup/build/initializing | Documented per Ubuntu version |
| **Android Open Source Project Mailing List** | community | **Free** | https://groups.google.com/g/android-platform | Forum for libziparchive questions |
| **mathlib4 PR review** | community | **Free** | leanprover-community/mathlib4 | If we contribute upstream-useful primitives, ~1 month review |
| **Cuttlefish on KVM** | Android emulator | **Free** OSS | https://source.android.com/docs/devices/cuttlefish | Not used in P1.5; pre-mention because P1.13 needs this and KVM access is an environment requirement |

**Hardware requirement note:** AOSP libziparchive is a small subset, but the reference build pulls in NDK C++ stdlib. Disk: ~8 GB for AOSP partial sync. Memory: ≥ 8 GB to compile.

**No API keys.** AOSP is anonymous public.

## 6. System Inventory — Have vs Need

### Already present
- ✅ git, gh, make, cmake
- ✅ clang 18, gcc 13
- ✅ Rust, cargo
- ✅ python3 3.12
- ✅ Lean / Lake (from P1.2)
- ✅ Bazel (from P1.1)
- ✅ Linux 6.8

### Missing — must install
- ❌ **`repo` tool** (Google's multi-repo manifest client)
- ❌ **AOSP build deps** (libstdc++-dev, lib32z1, etc.)
- ❌ **proptest** (Rust property-based testing) — added as crate dep
- ❌ **Hypothesis** (Python) — for input-generation scripts
- ❌ **ninja** (already installed in P1.1)

### Install commands

```bash
# 1) Google repo tool
mkdir -p ~/.bin
PATH="$HOME/.bin:$PATH"
curl https://storage.googleapis.com/git-repo-downloads/repo > ~/.bin/repo
chmod +x ~/.bin/repo

# 2) AOSP minimum build prerequisites (Ubuntu 24.04)
sudo apt-get install -y \
  git-core gnupg flex bison build-essential zip curl zlib1g-dev \
  libc6-dev-i386 libncurses5 lib32ncurses-dev x11proto-core-dev \
  libx11-dev lib32z1-dev libgl1-mesa-dev libxml2-utils xsltproc \
  unzip fontconfig

# 3) Sync only the components we need
mkdir -p external/aosp/sync && cd external/aosp/sync
~/.bin/repo init -u https://android.googlesource.com/platform/manifest -b android-14.0.0_r12
# Then a custom local manifest restricting to /system/core/libziparchive + tools/apksig
cat > .repo/local_manifests/apkaxiom.xml <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<manifest>
  <!-- Sync only what we formalize -->
  <project path="system/core/libziparchive" name="platform/system/core" />
  <project path="tools/apksig" name="platform/tools/apksig" />
</manifest>
EOF
~/.bin/repo sync -j$(nproc)

# 4) Property-based testing
# (added to Cargo.toml — proptest = "1.5")

# 5) Python Hypothesis for generators
pip3 install hypothesis
```

Disk: ~ 5–8 GB for the partial sync.

## 7. Working Directory & Files Produced

```
apkaxiom/
├── theorems/
│   └── Apkaxiom/
│       └── Zip/
│           ├── LocalHeader.lean        # NEW — ~600 LOC
│           └── Eocd.lean                # NEW — ~400 LOC
├── tests/
│   └── differential/
│       ├── Cargo.toml                   # NEW — Rust harness driver
│       ├── BUCK
│       └── src/main.rs                  # NEW — runs Lean evaluator + libziparchive, diffs
├── corpus/
│   └── zip/
│       ├── lfh-valid/                   # NEW — 1000+ valid LFH samples
│       ├── lfh-adversarial/             # NEW — 500+ adversarial (BadPack, oversize, malformed)
│       ├── eocd-valid/                  # NEW — 100+ valid EOCDs
│       └── eocd-adversarial/            # NEW — 200+ adversarial
├── tools/
│   └── corpus-gen/                      # NEW — Hypothesis-driven generator
│       └── gen_lfh.py
├── external/aosp/
│   └── sync/                            # vendored AOSP partial sync (gitignored)
│       └── system/core/libziparchive/
└── docs/
    └── lean-zip-layer.md                # NEW — design notes, invariants, edge cases
```

## 8. Standalone Output

The Lean modules and the differential-test harness, plus the corpus. Anyone can run:

```bash
nix develop
buck2 build //theorems:zip-lfh //theorems:zip-eocd
buck2 test //tests/differential:zip-vs-libziparchive
# Output: "1500/1500 inputs Lean ↔ libziparchive agreed"
```

The corpus itself is reusable infrastructure for every later sub-phase.

## 9. End-to-End Test

The differential harness runs every input through:
1. Lean reference evaluator (via `lake exe ...`).
2. AOSP A14 `libziparchive` (compiled into a small driver binary via Bazel sub-workspace).

Outputs are diffed for: parse success/failure verdict, recovered structure (when success), error category (when failure). **Required: 100% agreement on all 1,500+ inputs.**

```yaml
# .github/workflows/zip-differential.yml
jobs:
  zip-differential:
    steps:
      - uses: actions/checkout@v4
      - uses: nixbuild/nix-quick-install-action@v27
      - run: nix develop --command buck2 build //external/aosp:libziparchive-bin
      - run: nix develop --command buck2 test //tests/differential:zip-vs-libziparchive
```

## 10. Exit Checklist

- [ ] `LocalHeader.lean` theorem stated and proved (≥ 600 LOC)
- [ ] `Eocd.lean` theorem stated and proved (≥ 400 LOC)
- [ ] Cumulative Lean LOC ≥ 1,000
- [ ] Theorems re-verify on CI in ≤ 15 min (HARD per PHASE_GATES.md §5)
- [ ] Corpus: ≥ 1,000 valid LFHs + 500 adversarial; ≥ 100 valid EOCDs + 200 adversarial
- [ ] Differential harness: 100% Lean ↔ libziparchive agreement on all inputs (HARD)
- [ ] AOSP `libziparchive` builds reproducibly under our Bazel sub-workspace
- [ ] `docs/lean-zip-layer.md` published with design notes
- [ ] Property-based test runs nightly, no new disagreements found

## 11. Hand-Off

| Consumed by | What they need |
|---|---|
| **P1.6** | Theorems + corpus extended to central directory; reuses LFH theorem |
| **P1.9** | LFH parser is the first real extraction target |
| **P1.12** | Extracted Rust ZIP layer replaces hand-written |
| **P1.13, P1.14** | Adversarial corpus seeds the differential fuzzer |
| **P1.18** | LFH coverage rate is one of the K11 KPIs measured |
