# ADR-0004 — Nix flake as the toolchain pinning mechanism

**Status:** Accepted (P1.1)
**Date:** 2026-05-02
**Owner:** G13 — Platform Infrastructure
**Supersedes:** none
**Related:** ADR-0002 (Buck2)

---

## Context

Reproducibility is the spine of APKAXIOM's soundness story. A theorem proved against a Lean toolchain that does not byte-match the next developer's Lean toolchain is folklore, not a theorem. Phase 1 therefore needs a mechanism that:

1. Pins **every** tool that touches a build artifact — rustc, cargo, buck2, bazel, ninja, gcc, clang, Lean (P1.2), mathlib4 (P3.x), Halo2 toolchain (P4.x), etc.
2. Pins them by **content hash**, not by tag (tags can be retagged).
3. Survives a host distro upgrade. Pinning to "system rustc" is a non-starter.
4. Works on Linux x86_64, Linux aarch64, and macOS arm64 from one config.
5. Lives in-repo so a fresh checkout reproduces an old build.

## Decision

We adopt **Nix flakes** (Determinate Nix flavor; upstream Nix accepted) as the single source of toolchain truth.

- `flake.nix` declares the toolchain; `flake.lock` records the exact commits of `nixpkgs`, `rust-overlay`, and `flake-utils`.
- `rust-toolchain.toml` (committed) declares the Rust release and is read both by `rustup` (outside Nix) and by `pkgs.rust-bin.fromRustupToolchainFile` inside the flake. This guarantees `cargo build` (no Nix) and `nix develop --command cargo build` resolve to the same Rust.
- `nix develop` is the canonical entry point; the `Makefile` warns about non-Nix runs but does not enforce them, since IDE workflows benefit from a host rustc.
- `nix flake check` is a CI gate: every PR must produce a flake that builds the `toolchain-probe` derivation cleanly on every supported system.

### Reproducibility envelope

Inside `nix develop` (and exported from the `Makefile` for runs outside it):

| Variable | Value | Purpose |
|---|---|---|
| `SOURCE_DATE_EPOCH` | `315532800` (1980-01-01) | Stable archive timestamps (zip, ar, tar). |
| `TZ` | `UTC` | Eliminates host-timezone drift in any tool that touches `localtime()`. |
| `LC_ALL` / `LANG` | `C.UTF-8` | Stable collation, locale-dependent error messages. |
| `RUSTFLAGS` | includes `--remap-path-prefix=$PWD=.` | Strips host-paths from debuginfo + panic strings. |

These are the minimum table-stakes for byte-reproducible Rust. The list will grow as we encounter new sources of nondeterminism in Phase 2+ (Lean, Halo2, TFLite).

## What "byte-identical" means in practice

The P1.1 spec asked for cross-platform byte-identity:
```
diff hashes-ubuntu-24.04.txt hashes-ubuntu-24.04-arm.txt
diff hashes-ubuntu-24.04.txt hashes-macos-14.txt
```

This is a category error: `.rlib` and `.rmeta` files contain architecture-specific code and metadata. A linux-x86_64 rlib *cannot* hash-match a linux-aarch64 rlib, full stop.

We therefore reformulate the contract as two separate, both-meaningful invariants:

1. **Local determinism:** two clean builds on the same host produce identical hashes. (`make repro-check`).
2. **Per-platform determinism:** two independent runners of the same platform produce identical hashes. (CI's `cross-runner-determinism` job, comparing `snapshot-<platform>-1.txt` against `snapshot-<platform>-2.txt`).

Both invariants are necessary and sufficient for the practical reproducibility property we actually want: "given the same `git` SHA + the same `flake.lock`, any team member on the same platform reproduces the build."

The committed reference hashes live at `./reproducibility-hashes.<platform>.txt` (sibling of this ADR; one file per platform). Bumps must be reviewed under [`../../PHASE_GATES.md`](../../PHASE_GATES.md).

## Why Nix flakes over alternatives

| Alternative | Why we rejected it |
|---|---|
| **Docker images** | Too coarse. Caches are fragile, image diffs are unreadable, no per-tool pinning. |
| **`mise` / `asdf`** | Pin only "what version", not "what content". Different mirrors of the same version can differ. |
| **`rustup-only`** | Pins Rust but not anything else. AOSP toolchain alone is more than rustup can hold. |
| **`devcontainer` + apt** | Apt repositories rotate. Reproducibility lifetime measured in weeks at best. |
| **Bazel toolchain registry** | Bazel-only. Buck2 cannot consume it. |
| **`Earthly` / `BuildJet`** | Operationally heavier, with a smaller community for the kinds of toolchains we will pin (Lean, Halo2). |

Nix is the only mechanism where the *exact bytes* of the rustc binary, the libgcc you link against, and the OpenSSL you depend on are all determined by a hash in a single lockfile.

## Consequences

- **Onboarding cost.** New contributors need to install Nix once. The Determinate installer reduces friction to a single command, no questions asked.
- **CI time.** First `nix develop` on a cold runner is slow (downloads from `cache.nixos.org`). `magic-nix-cache-action` mitigates this.
- **Bumping pins is a deliberate action.** `nix flake update` produces a `flake.lock` diff that must be reviewed; toolchain updates are no longer "whatever happened on the runner".
- **Lean / Halo2 / mathlib4 will fit naturally.** P1.2's first task is to add `lean4` and `mathlib4` flake inputs; the slot is already cut in `flake.nix`.

## Trade-offs

- **Nix has a learning curve.** We accept this; no alternative offers comparable rigor.
- **macOS support is rougher than Linux.** Determinate Nix has improved this materially; we will revisit if it becomes a bottleneck.
- **Some tools we want are not in nixpkgs.** Today: Reindeer. We bootstrap via `cargo install --git`; once Reindeer ships a flake we will pin it via the flake instead.
- **Determinate Nix is a fork.** It tracks upstream closely and was selected for its non-interactive installer (Phase 1 CI requires non-interactive bootstrap). If upstream Nix releases a non-interactive installer, we will revisit the choice with a follow-up ADR.

## References

- Nix flakes: https://nixos.wiki/wiki/Flakes
- Determinate Nix: https://docs.determinate.systems/
- rust-overlay: https://github.com/oxalica/rust-overlay
- Reproducible builds: https://reproducible-builds.org/
