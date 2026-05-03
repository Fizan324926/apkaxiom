# Build & Run — APKAXIOM (Phase 1)

This is the entry-point doc for working in the APKAXIOM repo. Every developer
should be able to clone the repo, follow the steps below, and end up with a
green `make repro-check` on their first day.

---

## TL;DR

```bash
git clone https://github.com/Fizan324926/apkaxiom.git
cd apkaxiom
nix develop                       # enter the pinned toolchain
make build                        # buck2 build //:all
make test                         # buck2 test rust_test targets
make repro-check                  # verify two clean builds match byte-for-byte
make verify-hashes                # diff against committed reference
```

If the four `make` targets all PASS, the build foundation is healthy on your
machine. You are ready to start on whichever sub-phase you've been assigned.

---

## Prerequisites

- **A POSIX-y system** (Linux x86_64 / aarch64, macOS arm64). Windows is not
  supported in Phase 1 — the AOSP work in P1.13 cements this.
- **Nix with flakes** (Determinate or upstream multi-user install). One-shot
  bootstrap, then `nix develop` does the rest.
- **No `sudo` after the Nix install** — every other tool comes from the
  flake.

The Nix install command (one-time, host-only):
```bash
curl --proto '=https' --tlsv1.2 -sSf -L https://install.determinate.systems/nix \
  | sh -s -- install --determinate --no-confirm
```

After install, in any shell:
```bash
. /nix/var/nix/profiles/default/etc/profile.d/nix-daemon.sh
```
(Most distros add this to `/etc/profile.d/` automatically; the line above is
the manual fallback.)

---

## Toolchain layout — what's pinned where

| Tool | Pinned in | Notes |
|---|---|---|
| Rust (rustc, cargo, rustfmt, clippy) | `rust-toolchain.toml` (channel) + `flake.lock` (rust-overlay rev) | Single source of truth for both `cargo` and `nix develop`. |
| Buck2 | `flake.lock` (nixpkgs rev → `pkgs.buck2`) | Buck2 ships with a *bundled prelude*, so the prelude is pinned transitively by the buck2 binary. |
| Bazel | `external/aosp/.bazelversion` (resolved by Bazelisk from `flake.lock`) | Bazel 7.4.1 today; only used inside `external/aosp/`. |
| Reindeer | `~/.cargo/bin/reindeer` from `cargo install --git` | Bootstrap tool; not on the artifact-reproducibility hot path. Will be Nix-pinned when reindeer ships a flake. |
| ninja, make, cmake, jq, gh, lld, … | `flake.lock` (nixpkgs rev) | All system tools come from the flake. |
| `SOURCE_DATE_EPOCH`, `TZ`, `LC_ALL` | `flake.nix` `shellHook` | `1980-01-01`, `UTC`, `C.UTF-8`. |

---

## Build entry points

Every command below has a `make <target>` form (canonical) and a
`nix run .#<target>` form (works without `nix develop`). All scripts
honour `set -euo pipefail`; failure is loud.

### Build / test
| Goal | `make` | `nix run` | Underlying tool |
|---|---|---|---|
| Build everything | `make build` | `.#build` | `buck2 build //:all` |
| Build via Cargo (IDE / fast iter) | `make build-cargo` | — | `cargo build --workspace` |
| Run all tests | `make test` | `.#test` | `buck2 test //crates/...:` |
| Run cargo unit tests | `make test-cargo` | — | `cargo test --workspace` |

### Reproducibility
| Goal | `make` | `nix run` | Underlying tool |
|---|---|---|---|
| Local determinism check (×2 clean builds) | `make repro-check` | `.#repro-check` | `scripts/repro-check.sh` |
| Bake reference hashes for this platform | `make hash-snapshot` | `.#hash-snapshot` | `scripts/hash-snapshot.sh` |
| Verify against committed reference | `make verify-hashes` | `.#verify-hashes` | `scripts/verify-hashes.sh` |
| Independent rebuilder attestation | `make rebuilder-attest` | `.#rebuilder-attest` | `scripts/rebuilder-attest.sh` |

### Drift detection
| Goal | `make` | `nix run` | Underlying tool |
|---|---|---|---|
| Cargo↔Buck2 graph parity | `make graph-parity` | `.#graph-parity` | `scripts/graph-parity.sh` |
| `make third-party` is idempotent | `make reindeer-check` | `.#reindeer-check` | `scripts/reindeer-check.sh` |
| Buck2 toolchain snapshot | `make audit-toolchains` | `.#audit-toolchains` | `scripts/audit-toolchains.sh` |
| Determinism-pattern static lint | `make determinism-lint` | `.#determinism-lint` | `scripts/lint-determinism.sh` |

### Supply chain
| Goal | `make` | `nix run` | Underlying tool |
|---|---|---|---|
| CycloneDX SBOM (cargo + syft + merge) | `make sbom` | `.#sbom` | `scripts/sbom.sh` |
| Sign hash files with cosign keyless | `make sign-hashes` | `.#sign-hashes` | `scripts/sign-hashes.sh` |
| RustSec advisory scan | `make security-audit` | `.#security-audit` | `scripts/security-audit.sh` |
| License + ban + source policy | `make license-check` | `.#license-check` | `scripts/license-check.sh` |
| CI wall-time p99 rollup (gh-cli) | `make wall-time-rollup` | `.#wall-time-rollup` | `scripts/wall-time-rollup.sh` |

### Third-party / Bazel / Nix
| Goal | `make` | Notes |
|---|---|---|
| Re-vendor third-party Rust deps | `make third-party` | `reindeer vendor && reindeer buckify` |
| Bazel sub-workspace probe | `make bazel-info` | `cd external/aosp && bazel info` |
| Lint (clippy + fmt --check) | `make lint` | cargo |
| Auto-format Rust + Nix | `make fmt` | cargo + nixpkgs-fmt |
| Validate the flake | `make nix-check` | `nix flake check` |
| Bump flake pins | `make nix-update` | `nix flake update` (review the diff!) |

### Repro-debug shell
For investigating a `repro-check` failure with full `diffoscope`:

```bash
nix develop .#repro-debug --command diffoscope <fileA> <fileB>
```

`make` with no argument prints a help summary.

---

## What "byte-identical" means here

Phase 1's reproducibility contract has **two** levels:

1. **Local determinism (per-machine).** Two consecutive clean builds on the
   same host *must* produce identical artifact hashes. Verified by
   `make repro-check`. If this fails, the build is non-deterministic — fix
   that before doing anything else.

2. **Per-platform determinism (cross-host, same arch).** Two different CI
   runners of the same platform (e.g. two separate `ubuntu-24.04` jobs) must
   produce identical hashes. Verified by the `cross-runner-determinism` job
   in `.github/workflows/ci.yml`.

What we do **not** check: cross-platform byte-identity (linux-x86_64 vs
linux-aarch64). rlibs are arch-specific by definition; demanding a
linux-x86_64 rlib hash to equal a linux-aarch64 rlib hash is a category
error. See [`./ADR-0004-nix-flake.md`](./ADR-0004-nix-flake.md) for the rationale.

The committed reference hashes live in
`./reproducibility-hashes.<platform>.txt` (sibling of this file, one per
platform). Bumps to these references go through ADR review per
[`../../PHASE_GATES.md`](../../PHASE_GATES.md).

---

## Repo layout (Phase 1)

```
apkaxiom/
├── BUCK                                ← root :all alias + smoke target
├── .buckconfig, .buckroot              ← Buck2 cells + bundled-prelude
├── Cargo.toml, Cargo.lock              ← workspace
├── crates/
│   ├── axiom-l0/                       ← trusted core (placeholder)
│   ├── axiom-l1-rs/                    ← untrusted shell, Rust (placeholder)
│   └── axiom-ir/                       ← AXIOM-IR (placeholder; uses thiserror)
├── third-party/
│   └── rust/                           ← Reindeer-managed third-party deps
│       ├── Cargo.toml                  ← single source of truth
│       ├── Cargo.lock
│       ├── BUCK                        ← @generated by reindeer
│       ├── fixups/                     ← per-crate buildscript hints
│       └── vendor/                     ← committed crate sources (hermetic)
├── toolchains/BUCK                     ← rust/cxx/genrule/python/test
├── reindeer.toml                       ← Reindeer config
├── flake.nix, flake.lock               ← Nix toolchain pin
├── rust-toolchain.toml                 ← rust channel pin (read by both)
├── external/aosp/                      ← Bazel sub-workspace (P1.13+)
├── docs/
│   ├── ROADMAP.md, PHASE_GATES.md, TECH_STACK.md
│   └── phase-{1..6}/                   ← sub-phase specs (each owns its
│                                          ADRs, hash refs, run-books, status)
│       └── phase-1/P1.1/
│           ├── README.md               ← P1.1 spec
│           ├── CHECKLIST.md            ← P1.1 live status
│           ├── build-and-run.md        ← this file
│           ├── ADR-0002-buck2.md       ← Buck2 + Reindeer rationale
│           ├── ADR-0004-nix-flake.md   ← Nix flake rationale
│           └── reproducibility-hashes.*.txt
│                                       ← per-platform reference hashes
├── scripts/
│   ├── _hash-artifacts.sh              ← shared helper
│   ├── repro-check.sh                  ← `make repro-check`
│   ├── hash-snapshot.sh                ← `make hash-snapshot`
│   └── verify-hashes.sh                ← `make verify-hashes`
├── Makefile                            ← all entry points
└── .github/workflows/ci.yml            ← per-PR CI gate
```

---

## Common pitfalls

- **"Cannot find rustc"**: you're not in `nix develop`. Drop into the shell
  first: `nix develop --command make ...`.
- **`buck2 build //...` fails with `XIPL-depslink-symlinked_dirs.json` not
  found**: transient daemon state. `buck2 kill && buck2 build //...`. We
  prefer `//:all` as the entry point exactly to avoid this surface area.
- **`make repro-check` FAILS but artifacts look the same**: check
  `--remap-path-prefix` is taking effect. Run with `RUSTFLAGS_DEBUG=1
  cargo build` to see what flags rustc actually got.
- **Cargo and Buck2 disagree on a dep version**: `make third-party-update`
  to bring `third-party/rust/Cargo.lock` in sync with the workspace
  `Cargo.lock`.

---

## CI matrix

Three workflows under `.github/workflows/`:

### `ci.yml` — PR gate (every push, every PR, nightly soak)

| Job | Runs on | Purpose |
|---|---|---|
| `build (×6)` | ubuntu-24.04, ubuntu-24.04-arm, macos-14 (×2 each) | build + test + repro-check + verify-hashes + snapshot |
| `cross-runner determinism` | ubuntu-24.04 | Diffs runner 1 vs 2 per platform |
| `lint` | ubuntu-24.04 | clippy + fmt --check |
| `determinism-lint` | ubuntu-24.04 | static scan for nondeterminism patterns |
| `bazel sub-workspace probe` | ubuntu-24.04 | `bazel info` against `external/aosp/` |
| `graph-parity` | ubuntu-24.04 | Cargo ↔ Reindeer lockfile parity (ADR-0010) |
| `reindeer-idempotence` | ubuntu-24.04 | `make third-party` is a no-op |
| `audit-toolchains-drift` | ubuntu-24.04 | committed Buck2 toolchain snapshot diff |
| `security-audit` | ubuntu-24.04 | `cargo-audit` (workspace + Reindeer) |
| `license-check` | ubuntu-24.04 | `cargo-deny` policy |
| `sbom` | ubuntu-24.04 | CycloneDX (cargo-cyclonedx + syft + merge) |
| `attest` (push to main only) | ubuntu-24.04 | SLSA L1 provenance |
| `sign-hashes` (push to main only) | ubuntu-24.04 | cosign keyless signing |

Hard-cap timeout: **25 minutes per build job** (K10).

### `bake-refs.yml` — manual dispatch only

Re-bakes `reproducibility-hashes.<plat>.txt` for every platform on two
independent runners, asserts cross-runner byte-equality, opens a draft PR.
Used after a controlled toolchain bump or dep update; gated on G13 review.

### `wall-time-rollup.yml` — nightly cron

Pulls the last 200 CI runs from the GitHub Actions API, appends per-job
durations to `wall-time.ndjson`, regenerates the p50/p95/p99/max rollup
table at `wall-time-rollup.md`. K10 gate evaluation.

---

## Hand-off

This sub-phase (P1.1) hands off to:
- **P1.2** — Lean toolchain pin slots are pre-cut in `flake.nix`.
- **P1.3, P1.7, P1.8, P1.10, P1.15** — Buck2 + Reindeer machinery is ready
  for real Rust crates with real third-party deps.
- **P1.13, P1.14** — `external/aosp/` is a working Bazel sub-workspace.
- **P1.17** — CI workflow is the scaffold; the soundness gate slots in.
- **P1.18** — Pyroscope/Prometheus emission hooks attach to the existing CI.

For the full list see [`./README.md`](./README.md) §11. For live status of the
exit checklist see [`./CHECKLIST.md`](./CHECKLIST.md).
