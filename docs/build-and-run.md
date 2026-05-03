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

| Goal | Canonical command | Underlying tool |
|---|---|---|
| Build everything | `make build` | `buck2 build //:all` |
| Build via Cargo (IDE / fast iter) | `make build-cargo` | `cargo build --workspace` |
| Run all tests | `make test` | `buck2 test //crates/...:` |
| Run cargo unit tests | `make test-cargo` | `cargo test --workspace` |
| Local determinism check | `make repro-check` | `scripts/repro-check.sh` |
| Bake reference hashes | `make hash-snapshot` | `scripts/hash-snapshot.sh` |
| Verify against committed reference | `make verify-hashes` | `scripts/verify-hashes.sh` |
| Re-vendor third-party Rust deps | `make third-party` | `reindeer vendor && reindeer buckify` |
| Bazel sub-workspace probe | `make bazel-info` | `cd external/aosp && bazel info` |
| Lint (clippy + fmt --check) | `make lint` | cargo |
| Auto-format Rust + Nix | `make fmt` | cargo + nixpkgs-fmt |
| Validate the flake | `make nix-check` | `nix flake check` |
| Bump flake pins | `make nix-update` | `nix flake update` (review the diff!) |

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
error. See `docs/ADR-0004-nix-flake.md` for the rationale.

The committed reference hashes live in
`docs/reproducibility-hashes.<platform>.txt` (one file per platform). Bumps
to these references go through ADR review per `docs/PHASE_GATES.md`.

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
│   ├── build-and-run.md                ← this file
│   ├── ADR-0002-buck2.md               ← Buck2 + Reindeer rationale
│   ├── ADR-0004-nix-flake.md           ← Nix flake rationale
│   ├── reproducibility-hashes.*.txt    ← per-platform reference hashes
│   └── phase-{1..6}/                   ← sub-phase specs
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

The PR-gate workflow lives at `.github/workflows/ci.yml`. The matrix:

| Job | Runs on | Purpose |
|---|---|---|
| `build (linux-x86_64 / 1)` | ubuntu-24.04 | Build + test + repro-check + verify-hashes |
| `build (linux-x86_64 / 2)` | ubuntu-24.04 | Second runner — feeds cross-runner determinism |
| `build (linux-aarch64 / 1, 2)` | ubuntu-24.04-arm | Same on ARM |
| `build (darwin-arm64 / 1, 2)` | macos-14 | Same on macOS |
| `cross-runner-determinism` | ubuntu-24.04 | Diffs runners 1 vs 2 per platform |
| `lint` | ubuntu-24.04 | clippy + fmt --check |
| `bazel-probe` | ubuntu-24.04 | `bazel info` against `external/aosp/` |

Hard-cap timeout: **25 minutes per build job** (PHASE_GATES.md K10).

---

## Hand-off

This sub-phase (P1.1) hands off to:
- **P1.2** — Lean toolchain pin slots are pre-cut in `flake.nix`.
- **P1.3, P1.7, P1.8, P1.10, P1.15** — Buck2 + Reindeer machinery is ready
  for real Rust crates with real third-party deps.
- **P1.13, P1.14** — `external/aosp/` is a working Bazel sub-workspace.
- **P1.17** — CI workflow is the scaffold; the soundness gate slots in.
- **P1.18** — Pyroscope/Prometheus emission hooks attach to the existing CI.

For the full list see `docs/phase-1/P1.1/README.md` §11.
