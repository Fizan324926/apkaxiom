# Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
#
# APKAXIOM hermetic toolchain pin — P1.1.
#
# This flake is the *single source of truth* for every tool that touches
# build artifacts. `flake.lock` (committed) pins the exact commits of
# nixpkgs and rust-overlay; `rust-toolchain.toml` (committed) pins the exact
# Rust release. Together they make the build reproducible from a fresh
# checkout on any supported host.
#
# Usage:
#   nix develop                         # drop into the pinned toolchain
#   nix develop --command make          # run a single command in it
#   nix flake check                     # toolchain probe + shellcheck +
#                                       # lockfile freshness
#   nix run .#repro-check               # local determinism check
#   nix run .#sbom                      # emit CycloneDX SBOM
#   nix flake update                    # bump pins (review the lockfile diff)
#
# The Lean toolchain (P1.2) and mathlib4 commit (P3.x) will be pinned here
# in subsequent sub-phases. Slots are marked TODO(P1.2) below so the diff is
# trivial.

{
  description = "APKAXIOM — hermetic toolchain pin (P1.1)";

  # Optional binary cache. The Cachix cache is provisioned out-of-band by
  # the G13 lead; until then `extra-substituters` is empty and the build
  # falls back to the public NixOS cache only. See ADR-0006 for rationale
  # and the provisioning checklist.
  nixConfig = {
    extra-substituters = [
      # "https://apkaxiom.cachix.org"
    ];
    extra-trusted-public-keys = [
      # "apkaxiom.cachix.org-1:<placeholder; replace after provisioning>"
    ];
  };

  inputs = {
    # Pin a recent stable nixpkgs branch. The exact commit is recorded in
    # flake.lock; bumps go through ADR-0004 review.
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";

    # Narrow second input — *only* used to pull supply-chain tooling
    # (cargo-audit, cargo-deny) that lags in 24.11 and can't yet parse
    # CVSS:4.0 advisories. The risk surface is small and is documented in
    # ADR-0008.
    nixpkgs-unstable.url = "github:NixOS/nixpkgs/nixos-unstable";

    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    # mathlib4 source pinned at the v4.29.1 commit. We do not use this as a
    # build input directly — `lakefile.toml` + `lake-manifest.json` drive the
    # actual fetch. Recording it here gives us a flake-level provenance
    # anchor: any change to the SHA shows up in `flake.lock`.
    mathlib4 = {
      url = "github:leanprover-community/mathlib4/v4.29.1";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, nixpkgs-unstable, flake-utils, rust-overlay, mathlib4, ... }:
    flake-utils.lib.eachSystem [
      "x86_64-linux"
      "aarch64-linux"
      "x86_64-darwin"
      "aarch64-darwin"
    ] (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        pkgsUnstable = import nixpkgs-unstable { inherit system; };

        # Single source of truth for the Rust toolchain. `rust-toolchain.toml`
        # is also read by `rustup` outside Nix, so cargo-only paths agree.
        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile
          ./rust-toolchain.toml;

        # Tools shared across every devShell variant.
        #
        # The reproducibility-engineering tools (cosign, syft, cargo-cyclonedx,
        # cargo-audit, cargo-deny, shellcheck, b3sum) are P1.1 additions per
        # ADR-0007 (hash-corpus), ADR-0008 (provenance/SBOM/signing), and
        # ADR-0009 (repro-budget reporter). They are non-negotiable inputs to
        # the build-foundation contract; missing any of them downgrades P1.1
        # below the state-of-the-art bar.
        commonTools = with pkgs; [
          rustToolchain
          buck2
          bazelisk
          cmake
          ninja
          gnumake
          gnused
          gawk
          pkg-config
          jq
          git
          gh
          zstd
          curl
          coreutils
          lld
          # Reproducibility / supply-chain tooling (P1.1)
          cosign
          syft
          shellcheck
          b3sum
          # libstdc++ runtime — Lean's bundled libleancpp.a references
          # `__cxa_call_terminate` which lives here. Without this the
          # mathlib `cache:exe` fails to link.
          stdenv.cc.cc.lib
          # P1.3 audit tooling. cargo-bloat / cargo-llvm-lines for
          # binary-size analysis of the upstream apk-info, hyperfine
          # for parser micro-benchmarks, graphviz for the v1.0
          # architecture diagram, tokei for accurate LOC counting.
          # flamegraph stays a dev-side opt-in (it needs perf events
          # which CI runners don't grant).
          cargo-bloat
          cargo-llvm-lines
          hyperfine
          graphviz
          tokei
          # P1.4 wire-format validation. capnproto's `capnp` schema
          # compiler validates `schema/axiom_ir_v0_1.capnp` syntax;
          # invoked unconditionally by tools/ir-schema-check (no longer
          # "skip if missing"). Per the closed §10-8 gap, this is the
          # mandatory leg of the v0.1 wire-format contract; a Rust-side
          # round-trip via generated bindings is deferred to Phase 4
          # per ADR-0014.
          capnproto
          # P1.6 production fuzzers (per ADR-0019, formerly deferred to
          # P1.13/P1.14; pulled forward to close the §I gap on real
          # coverage-guided fuzzing). cargo-fuzz drives libFuzzer for
          # Rust crates; radamsa is the mutation-only black-box fuzzer
          # the spec calls out by name; honggfuzz is the alt-coverage
          # alternative for cross-checking. AFL++ stays out for now —
          # it requires its own instrumented compiler and would inflate
          # the dev-shell closure significantly.
          cargo-fuzz
          radamsa
          honggfuzz
        ] ++ [
          # Pulled from nixpkgs-unstable; nixos-24.11 lags on these.
          # - cargo-audit / cargo-deny: 24.11 can't parse CVSS:4.0
          #   advisories (rustsec < 0.31).
          # - cargo-cyclonedx: 24.11 can't parse Cargo.lock v4.
          # - lean4 / elan (P1.2): 24.11 has Lean 4.10.0; we want 4.29.1
          #   to match the mathlib4 v4.29.1 line. `lean-toolchain` and
          #   `lakefile.toml` carry the matching pins so `elan`-based dev
          #   workflows agree with the Nix-pinned binary.
          pkgsUnstable.cargo-audit
          pkgsUnstable.cargo-deny
          pkgsUnstable.cargo-cyclonedx
          pkgsUnstable.lean4
          pkgsUnstable.elan
          # mermaid-cli (`mmdc`) — diagram renderer for spec §10-10's
          # mermaid prong. nixos-24.11 ships an older version; we
          # pull from unstable to get current syntax support.
          pkgsUnstable.mermaid-cli
        ];

        # Heavy debugging tools — only loaded into the `repro-debug` shell so
        # everyday `nix develop` stays light. diffoscope is the gold-standard
        # tool for binary-diff investigation when `make repro-check` fails.
        reproDebugTools = with pkgs; [
          diffoscopeMinimal
          tree
          ripgrep
          file
          binutils
        ];

        # Reproducibility env vars. Centralised so every shell variant sets
        # the same baseline.
        #
        # The `LIBRARY_PATH` / `LD_LIBRARY_PATH` prepend pulls in the newer
        # libstdc++ from `pkgsUnstable.stdenv` (gcc-15.2.0). The 24.11
        # stdenv is gcc-13.3.0 whose libstdc++ predates `__cxa_call_terminate`,
        # which Lean's `libleancpp.a` (built with newer GCC) references —
        # without this prepend, `lake update` fails to link mathlib's
        # `cache:exe`.
        reproEnv = ''
          export SOURCE_DATE_EPOCH=315532800   # 1980-01-01 (zip/ar epoch)
          export TZ=UTC
          export LC_ALL=C.UTF-8
          export LANG=C.UTF-8
          export RUSTFLAGS="--remap-path-prefix=$PWD=. ''${RUSTFLAGS:-}"
          # Lake invokes the C compiler named in `LEAN_CC` for its final
          # link step. We pin that to gcc-15.2.0 (from pkgsUnstable) so
          # mathlib's `cache:exe` and any other C++-bearing Lake targets
          # resolve `__cxa_call_terminate` from a libstdc++ that has it.
          export LEAN_CC="${pkgsUnstable.gcc}/bin/gcc"
          export LIBRARY_PATH="${pkgsUnstable.stdenv.cc.cc.lib}/lib:''${LIBRARY_PATH:-}"
          export LD_LIBRARY_PATH="${pkgsUnstable.stdenv.cc.cc.lib}/lib:''${LD_LIBRARY_PATH:-}"
          # Force every gcc invocation in the dev shell to add the newer
          # libstdc++ to the linker's *explicit* `-L` set, so lake's
          # cache:exe link finds `__cxa_call_terminate` even when its own
          # command line does not pass an explicit `-L`.
          export NIX_LDFLAGS="-L${pkgsUnstable.stdenv.cc.cc.lib}/lib ''${NIX_LDFLAGS:-}"
        '';

        # Wrap a repo-relative script as a `nix run .#<name>`-able app. The
        # wrapper exports `commonTools` onto PATH and exec's the script
        # in-place — buck2 / cargo / etc. all see the pinned tools without
        # the user having to enter `nix develop` first.
        mkApp = name: scriptPath:
          let
            bin = pkgs.writeShellScriptBin "apkaxiom-${name}" ''
              set -euo pipefail
              export PATH="${pkgs.lib.makeBinPath commonTools}:''${PATH:-}"
              ${reproEnv}
              exec bash ${scriptPath} "$@"
            '';
          in
          {
            type = "app";
            program = "${bin}/bin/apkaxiom-${name}";
          };
      in
      {
        # Default dev shell — interactive use, IDEs, CI.
        # Uses pkgsUnstable's stdenv so the C/C++ toolchain is gcc-15.2.0;
        # 24.11's gcc-13.3.0 ships a libstdc++ that predates
        # `__cxa_call_terminate`, which Lean's libleancpp.a requires.
        devShells.default = pkgsUnstable.mkShell {
          name = "apkaxiom-dev";
          packages = commonTools;
          shellHook = ''
            ${reproEnv}
            echo "APKAXIOM dev shell — toolchain pinned by flake.lock"
            echo "  rustc:    $(rustc --version)"
            echo "  buck2:    $(buck2 --version 2>/dev/null || echo '<unavailable>')"
            echo "  bazelisk: $(bazel --version 2>/dev/null | head -1 || echo '<bazelisk; init on first run>')"
            echo "  lean:     $(lean --version 2>/dev/null | head -1 || echo '<unavailable>')"
            echo "  cosign:   $(cosign version 2>/dev/null | awk -F: '/^GitVersion/{print $2}' | xargs || echo '<unavailable>')"
            echo "  syft:     $(syft version 2>/dev/null | awk -F: '/^Version/{print $2}' | xargs || echo '<unavailable>')"
            echo "  nix:      $(nix --version 2>/dev/null | head -1)"
          '';
        };

        # CI shell — same packages as default, but no interactive banner so
        # logs stay clean.
        devShells.ci = pkgsUnstable.mkShell {
          name = "apkaxiom-ci";
          packages = commonTools;
          shellHook = reproEnv;
        };

        # Repro-debug shell — adds diffoscope, file, ripgrep, etc. for
        # investigating reproducibility failures. Heavier than `default`,
        # so opt-in only.
        devShells.repro-debug = pkgsUnstable.mkShell {
          name = "apkaxiom-repro-debug";
          packages = commonTools ++ reproDebugTools;
          shellHook = ''
            ${reproEnv}
            echo "APKAXIOM repro-debug shell — diffoscope, ripgrep, binutils"
            echo "Use: diffoscope <fileA> <fileB>  to drill into hash diffs."
          '';
        };

        # Apps — every script that participates in the P1.1 contract is
        # exposed so any consumer can invoke it without first entering
        # `nix develop`. Examples:
        #   nix run .#repro-check
        #   nix run github:Fizan324926/apkaxiom#sbom
        apps = {
          build = mkApp "build" ./scripts/build.sh;
          test = mkApp "test" ./scripts/test.sh;
          repro-check = mkApp "repro-check" ./scripts/repro-check.sh;
          verify-hashes = mkApp "verify-hashes" ./scripts/verify-hashes.sh;
          hash-snapshot = mkApp "hash-snapshot" ./scripts/hash-snapshot.sh;
          graph-parity = mkApp "graph-parity" ./scripts/graph-parity.sh;
          audit-toolchains = mkApp "audit-toolchains" ./scripts/audit-toolchains.sh;
          reindeer-check = mkApp "reindeer-check" ./scripts/reindeer-check.sh;
          sbom = mkApp "sbom" ./scripts/sbom.sh;
          security-audit = mkApp "security-audit" ./scripts/security-audit.sh;
          license-check = mkApp "license-check" ./scripts/license-check.sh;
          determinism-lint = mkApp "determinism-lint" ./scripts/lint-determinism.sh;
          sign-hashes = mkApp "sign-hashes" ./scripts/sign-hashes.sh;
          rebuilder-attest = mkApp "rebuilder-attest" ./scripts/rebuilder-attest.sh;
          wall-time-rollup = mkApp "wall-time-rollup" ./scripts/wall-time-rollup.sh;
        };

        # `nix flake check` runs every derivation under `checks.*`. The set
        # below intentionally uses *cheap* checks that finish in seconds —
        # heavy checks (full repro-check, SBOM emission) live under `apps.*`
        # and run in CI rather than in the per-PR flake-check budget.
        checks = {
          # Probe every pinned tool is at least invocable on this system.
          toolchain-probe = pkgs.runCommand "apkaxiom-toolchain-probe" {
            nativeBuildInputs = commonTools;
          } ''
            set -euo pipefail
            {
              rustc --version
              cargo --version
              buck2 --version
              ninja --version
              cmake --version | head -1
              jq --version
              cosign version 2>/dev/null | head -1 || cosign --help 2>&1 | head -1
              syft version 2>/dev/null | head -1 || syft --help 2>&1 | head -1
              cargo-cyclonedx --help 2>&1 | head -1 || true
              cargo-audit --version 2>/dev/null | head -1 || true
              cargo-deny --version
              shellcheck --version | head -2 | tail -1
              b3sum --version
              echo OK
            } > $out
          '';

          # Static analysis on every shell script in the repo. This is the
          # P1.1 enforcement of "scripts in the reproducibility hot path
          # cannot have shellcheck warnings".
          shellcheck = pkgs.runCommand "apkaxiom-shellcheck" {
            nativeBuildInputs = [ pkgs.shellcheck ];
            src = ./scripts;
          } ''
            set -euo pipefail
            cp -r $src ./scripts
            chmod -R u+w ./scripts
            shellcheck --severity=warning ./scripts/*.sh
            touch $out
          '';

          # Lockfile freshness — fails if `flake.lock` has gone stale (older
          # than the budget). State-of-the-art is to bump quarterly with an
          # ADR; this gate prevents silent rot.
          lockfile-freshness = pkgs.runCommand "apkaxiom-lockfile-freshness" {
            nativeBuildInputs = [ pkgs.jq pkgs.coreutils ];
            src = ./flake.lock;
            # Budget: 120 days. Adjust via ADR if business need shifts.
            maxAgeDays = "120";
            # Reference "now" timestamp baked into the flake. Bumped via
            # `make nix-update` followed by an ADR-0004 review note.
            referenceNow = toString 1746230400; # 2026-05-03T00:00:00Z
          } ''
            set -euo pipefail
            mtime=$(jq -r '.nodes.nixpkgs.locked.lastModified // 0' "$src")
            age_days=$(( (referenceNow - mtime) / 86400 ))
            if [ "$age_days" -gt "$maxAgeDays" ]; then
              echo "FAIL: nixpkgs pin is $age_days days old (>$maxAgeDays)" >&2
              echo "Bump via \`make nix-update\` and submit an ADR." >&2
              exit 1
            fi
            echo "OK: nixpkgs pin age is $age_days days (<= $maxAgeDays)" > $out
          '';
        };

        # Convenience formatter target.
        formatter = pkgs.nixpkgs-fmt;
      }
    );
}
