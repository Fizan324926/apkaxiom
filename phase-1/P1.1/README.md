# P1.1 — Hermetic Build Foundation

> Buck2 + Reindeer + Nix + Bazel-for-AOSP. Every build byte-identical on three machines. Fail-closed CI before a single line of Lean is written.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../README.md](../../README.md) · [../../TECH_STACK.md](../../TECH_STACK.md) · [../../PHASE_GATES.md](../../PHASE_GATES.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P1.1 |
| Owner(s) | G13 (Platform Infrastructure) |
| Duration | Weeks 1–3 |
| Critical-path | **yes** — every other sub-phase depends on this |
| Hard prerequisites | none — foundation work |

## 2. Goal & Scope

The repository must produce **byte-identical build artifacts** on three independent machines from the same `git` SHA. Without this, no soundness claim downstream is meaningful: a Lean theorem you re-verified on machine A but not on machine B is not a theorem, it is folklore.

This sub-phase stands up the substrate: Buck2 as the primary build system (Reindeer for Cargo→Buck2 conversion), a Bazel sub-workspace reserved for AOSP harness compilation only, and Nix flakes pinning every toolchain version (Lean, Rust, Buck2 itself, Bazel, mathlib4 commit, GCC, clang, etc.).

### In scope
- Buck2 workspace at repo root, building all current crates (`axiom-l0`, `axiom-l1-rs`, `axiom-ir` — initially empty skeletons).
- Reindeer for Cargo→Buck2 conversion of Rust dependencies.
- Bazel sub-workspace under `external/aosp/` ready to receive AOSP harnesses (Phase P1.13).
- Nix flake `flake.nix` pinning all toolchains.
- Reproducibility test harness `make repro-check` that diffs Buck2 outputs across 3 machines.
- Per-PR CI workflow: build + reproducibility check + (placeholder) perf gate.

### Out of scope (explicit)
- Lean toolchain integration (P1.2).
- Real perf benchmarks (P1.7+).
- Soundness regression CI gate (P1.17 owns this).
- Continuous fuzzing infrastructure (P1.13).

## 3. Hard Dependencies on Prior Sub-Phases

None. This is the bedrock.

## 4. Required Tools, Libraries, and Languages

| Tool | Pinned version | Purpose |
|---|---|---|
| **Buck2** | rev pinned via Nix flake (track upstream `meta/buck2`) | Primary build system |
| **Reindeer** | latest stable | Cargo → Buck2 conversion |
| **Bazel (Bazelisk)** | 7.x | Sub-workspace for AOSP harnesses only |
| **Nix** (with flakes) | 2.20+ | Toolchain pin source of truth |
| **Rust** | 1.95+ stable, pinned via rustup-via-Nix | The systems language |
| **GCC** / **clang** | GCC 13+ / clang 18+ | C/C++ deps for AOSP |
| **make / cmake / ninja** | latest stable | Build glue |
| **gh** (GitHub CLI) | 2.89+ | CI orchestration |
| **OpenTelemetry collector** | 0.100+ | Build event tracing (Pyroscope feeds in P1.18) |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL / Where to sign up | Notes |
|---|---|---|---|---|
| **GitHub organization** | code hosting + CI | **Paid** ($4/user/mo Team or $21/user/mo Enterprise; free for OSS) | https://github.com/organizations/new | Required as central code-hosting platform; the APKAXIOM org owns the repos |
| **GitHub Actions** | CI runtime | **Free** for public repos; **paid** ($0.008/min Linux at base tier) for private | Bundled with GitHub | Used for PR-gate CI |
| **Buildkite** | self-hosted-agent CI | **Paid** ($15/user/mo or ~$1500–3000/mo at our scale) | https://buildkite.com/pricing | Recommended for Buck2 RBE; agents run on our hardware |
| **GitHub Container Registry (GHCR)** | OCI image registry | **Free** for public + within plan limits private | https://ghcr.io | Hosts our reproducible-build container images |
| **Cachix** (Nix binary cache) | Nix cache | **Free** tier 5 GB; **paid** $5+/mo for more | https://www.cachix.org/ | Speeds Nix flake builds across machines |
| **Buck2 Remote Build Execution (RBE)** | distributed build | self-hosted, **free**; managed via BuildBuddy ~$200/mo+ | https://www.buildbuddy.io/pricing/ | Optional in P1.1; required at scale by P1.18 |
| **Sigstore (cosign)** | signing | **Free** OSS public good | https://www.sigstore.dev/ | Used for signing build artifacts |
| **Trustix** (Tweag) | build verification network | **Free** OSS | https://github.com/nix-community/trustix | Cross-check rebuilds against independent rebuilders |

**No API keys at this sub-phase** beyond GitHub OAuth tokens (issued automatically). Cachix and BuildBuddy require account creation; both have free tiers sufficient for Phase 1.

## 6. System Inventory — Have vs Need on Development Host

### Already present (verified at M0 system check)
- ✅ git 2.43.0, gh 2.89.0
- ✅ make 4.3, cmake 3.28
- ✅ gcc 13.3, clang 18.1
- ✅ rustc / cargo / rustup 1.95
- ✅ pkg-config, jq, curl, wget, tar, unzip
- ✅ Linux 6.8, Ubuntu 24.04

### Missing — must install
- ❌ **Buck2** (build system)
- ❌ **Reindeer** (Cargo→Buck2)
- ❌ **Bazel / Bazelisk** (AOSP sub-workspace)
- ❌ **Nix** (with flakes enabled)
- ❌ **Ninja** (faster `make` for cmake/Bazel)

### Install commands (Ubuntu 24.04 reference)

```bash
# 1) Nix (multi-user, with flakes)
sh <(curl -L https://nixos.org/nix/install) --daemon
mkdir -p ~/.config/nix
echo 'experimental-features = nix-command flakes' >> ~/.config/nix/nix.conf

# 2) Ninja
sudo apt-get install -y ninja-build

# 3) Bazelisk (Bazel launcher; pins per-repo)
curl -L https://github.com/bazelbuild/bazelisk/releases/latest/download/bazelisk-linux-amd64 \
  -o /usr/local/bin/bazel && chmod +x /usr/local/bin/bazel

# 4) Buck2 (via Nix or prebuilt binary)
# Prebuilt:
curl -L https://github.com/facebook/buck2/releases/latest/download/buck2-x86_64-unknown-linux-gnu.zst \
  | zstd -d > /usr/local/bin/buck2 && chmod +x /usr/local/bin/buck2

# 5) Reindeer (Buck2's Cargo bridge)
cargo install --locked --git https://github.com/facebookincubator/reindeer reindeer

# 6) Cachix client (optional but recommended)
nix-env -iA cachix -f https://cachix.org/api/v1/install
```

After install, **all subsequent operations go through Nix-pinned versions** via the repo's `flake.nix`, not host-installed versions. Host installs above are bootstrap only.

## 7. Working Directory & Files Produced

```
apkaxiom/
├── flake.nix                          # NEW — pins Lean, Rust, Buck2, Bazel, mathlib4
├── flake.lock                         # NEW — frozen versions
├── BUCK                               # NEW — root Buck2 module
├── third-party/
│   ├── BUCK
│   └── rust/
│       ├── BUCK                       # NEW — Reindeer-generated Rust deps
│       └── third-party.toml           # NEW — Cargo-style dep declaration
├── crates/
│   ├── axiom-l0/
│   │   ├── BUCK                       # NEW — empty skeleton
│   │   ├── Cargo.toml
│   │   └── src/lib.rs                 # `pub fn placeholder() {}`
│   ├── axiom-l1-rs/
│   │   ├── BUCK
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   └── axiom-ir/
│       ├── BUCK
│       ├── Cargo.toml
│       └── src/lib.rs
├── external/aosp/
│   ├── WORKSPACE                      # NEW — Bazel sub-workspace
│   └── README.md                      # "AOSP harnesses live here, P1.13+"
├── Makefile                           # NEW — wraps `buck2 build`, `make repro-check`, etc.
├── docs/
│   ├── build-and-run.md               # NEW
│   ├── ADR-0002-buck2.md              # NEW
│   └── ADR-0004-nix-flake.md          # NEW
└── .github/
    └── workflows/
        └── ci.yml                     # NEW — per-PR build + repro check
```

## 8. Standalone Output (the deliverable)

A **reproducible-build harness** that any external party can clone and verify:

```bash
git clone https://github.com/Fizan324926/apkaxiom.git
cd apkaxiom
nix develop                # drops you into the pinned toolchain shell
make repro-check           # builds with Buck2, hashes outputs, diffs against ref hashes
# expected: "✓ All artifacts byte-identical to reference"
```

The reference SHA-256 hashes are committed in `docs/reproducibility-hashes.txt` and updated only when build inputs intentionally change (with ADR review).

## 9. End-to-End Test

A demo PR that adds a no-op file change builds reproducibly across 3 reference machines (1× x86_64 Linux, 1× ARM64 Linux, 1× x86_64 macOS dev workstation). Hash equality is required for the PR to merge.

```yaml
# .github/workflows/ci.yml fragment
jobs:
  reproducibility:
    strategy:
      matrix:
        os: [ubuntu-24.04, ubuntu-24.04-arm, macos-14]
    steps:
      - uses: actions/checkout@v4
      - uses: nixbuild/nix-quick-install-action@v27
      - run: nix develop --command make repro-check
      - run: sha256sum target/build/axiom-l0/* > hashes-${{ matrix.os }}.txt
  diff-hashes:
    needs: reproducibility
    steps:
      - run: diff hashes-ubuntu-24.04.txt hashes-ubuntu-24.04-arm.txt
      - run: diff hashes-ubuntu-24.04.txt hashes-macos-14.txt
```

## 10. Exit Checklist

- [ ] `flake.nix` pins Lean (placeholder for P1.2), Rust 1.95, Buck2 rev, Bazel 7.x, mathlib4 commit
- [ ] `flake.lock` committed and reviewed
- [ ] Buck2 workspace builds all 3 placeholder crates
- [ ] Reindeer integration produces `third-party/rust/BUCK`
- [ ] Bazel sub-workspace at `external/aosp/` initializes (`bazel info`)
- [ ] `make repro-check` runs in <25 min
- [ ] 30 consecutive PRs land with byte-identical CI builds across 3 OSes
- [ ] CI build wall time ≤ 25 min p99 (HARD per PHASE_GATES.md §5 K10)
- [ ] ADR-0002 (Buck2) and ADR-0004 (Nix flake) merged
- [ ] `docs/build-and-run.md` published with copy-pasteable instructions
- [ ] Cachix binary cache live, hit rate ≥90% on warm runs
- [ ] Reference hashes in `docs/reproducibility-hashes.txt`

## 11. Hand-Off

| Consumed by | What they need |
|---|---|
| **P1.2** | `flake.nix` slot for Lean toolchain pin |
| **P1.3, P1.7, P1.8, P1.10, P1.15** | Buck2 + Cargo via Reindeer for Rust crates |
| **P1.5, P1.6, P1.11** | Buck2 build of placeholder Lean modules (real Lean integration in P1.2) |
| **P1.13, P1.14** | Bazel sub-workspace at `external/aosp/` ready for AOSP harnesses |
| **P1.17** | CI workflow scaffolding (soundness gate added on top) |
| **P1.18** | Pyroscope/Prometheus emission hooks in CI |
