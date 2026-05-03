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
#   nix flake check                     # verify the flake is well-formed
#   nix flake update                    # bump pins (review the lockfile diff)
#
# The Lean toolchain (P1.2) and mathlib4 commit (P3.x) will be pinned here
# in subsequent sub-phases. Slots are marked TODO(P1.2) below so the diff is
# trivial.

{
  description = "APKAXIOM — hermetic toolchain pin (P1.1)";

  inputs = {
    # Pin a recent stable nixpkgs branch. The exact commit is recorded in
    # flake.lock; bumps go through ADR-0004 review.
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";

    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    # TODO(P1.2): pin Lean 4 + mathlib4 here.
    #
    # lean4 = {
    #   url = "github:leanprover/lean4/v4.x.y";
    #   flake = false;  # build via override in P1.2
    # };
    # mathlib4 = {
    #   url = "github:leanprover-community/mathlib4/<sha>";
    #   flake = false;
    # };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, ... }:
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

        # Single source of truth for the Rust toolchain. `rust-toolchain.toml`
        # is also read by `rustup` outside Nix, so cargo-only paths agree.
        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile
          ./rust-toolchain.toml;

        # Tools shared across every devShell variant.
        commonTools = with pkgs; [
          rustToolchain
          buck2
          bazelisk
          cmake
          ninja
          gnumake
          pkg-config
          jq
          git
          gh
          zstd
          curl
          coreutils
          lld
          # Lean placeholder — real pin in P1.2.
          # lean4
        ];

        # Reproducibility env vars. Centralised so every shell variant sets
        # the same baseline.
        reproEnv = ''
          export SOURCE_DATE_EPOCH=315532800   # 1980-01-01 (zip/ar epoch)
          export TZ=UTC
          export LC_ALL=C.UTF-8
          export LANG=C.UTF-8
          # Cargo's path-prefix remap is also set in toolchains/BUCK, but
          # repeating it here keeps `cargo build` (outside Buck2) reproducible.
          export RUSTFLAGS="--remap-path-prefix=$PWD=. ''${RUSTFLAGS:-}"
        '';
      in
      {
        # Default dev shell — interactive use, IDEs, CI.
        devShells.default = pkgs.mkShell {
          name = "apkaxiom-dev";
          packages = commonTools;
          shellHook = ''
            ${reproEnv}
            echo "APKAXIOM dev shell — toolchain pinned by flake.lock"
            echo "  rustc:    $(rustc --version)"
            echo "  buck2:    $(buck2 --version 2>/dev/null || echo '<unavailable>')"
            echo "  bazelisk: $(bazel --version 2>/dev/null | head -1 || echo '<bazelisk; init on first run>')"
            echo "  nix:      $(nix --version 2>/dev/null | head -1)"
          '';
        };

        # CI shell — same packages as default, but no interactive banner so
        # logs stay clean.
        devShells.ci = pkgs.mkShell {
          name = "apkaxiom-ci";
          packages = commonTools;
          shellHook = reproEnv;
        };

        # `nix flake check` runs this. It's a sanity probe that every
        # toolchain we pin is at least invocable.
        checks.toolchain-probe = pkgs.runCommand "apkaxiom-toolchain-probe" {
          nativeBuildInputs = commonTools;
        } ''
          set -euo pipefail
          rustc --version > $out
          cargo --version >> $out
          buck2 --version >> $out
          ninja --version >> $out
          cmake --version | head -1 >> $out
          jq --version >> $out
          echo OK >> $out
        '';

        # Convenience formatter target.
        formatter = pkgs.nixpkgs-fmt;
      }
    );
}
