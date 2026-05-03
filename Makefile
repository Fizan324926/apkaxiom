# Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
#
# APKAXIOM build entry points. Most targets are thin wrappers over `buck2`,
# `cargo`, `bazel`, and `nix`. Always invoke under `nix develop` for
# reproducible toolchains:
#
#   nix develop --command make <target>
#
# or, equivalently, drop into the shell once and run `make` directly.

SHELL := /usr/bin/env bash
.SHELLFLAGS := -euo pipefail -c
.DEFAULT_GOAL := help

# Resolve the repo root regardless of where make was invoked from.
ROOT := $(shell git rev-parse --show-toplevel 2>/dev/null || pwd)

# Reproducibility env exported to every recipe. flake.nix sets these inside
# `nix develop`; we re-export here so plain `make` is also deterministic.
export SOURCE_DATE_EPOCH ?= 315532800
export TZ ?= UTC
export LC_ALL ?= C.UTF-8
export LANG ?= C.UTF-8

# Detected (os, arch) tuple, used in artifact filenames.
UNAME_S := $(shell uname -s | tr '[:upper:]' '[:lower:]')
UNAME_M := $(shell uname -m)
PLATFORM := $(UNAME_S)-$(UNAME_M)

##@ Help

.PHONY: help
help: ## Show this help.
	@awk 'BEGIN { FS = ":.*?## " } \
	     /^##@ / { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) } \
	     /^[a-zA-Z0-9_.-]+:.*?## / { printf "  \033[36m%-22s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

##@ Build

.PHONY: build
build: ## Build everything (Buck2).
	buck2 build //:all

.PHONY: build-cargo
build-cargo: ## Build everything via Cargo (IDE / fast iteration path).
	cargo build --workspace --all-targets

.PHONY: test
test: ## Run all rust_test targets via Buck2.
	buck2 test \
	  //crates/axiom-l0:axiom-l0-test \
	  //crates/axiom-l1-rs:axiom-l1-rs-test \
	  //crates/axiom-ir:axiom-ir-test

.PHONY: test-cargo
test-cargo: ## Run all unit tests via Cargo.
	cargo test --workspace

.PHONY: clean
clean: ## Remove build artifacts.
	-buck2 clean 2>/dev/null
	-cargo clean 2>/dev/null
	-rm -rf $(ROOT)/result $(ROOT)/buck-out $(ROOT)/target

##@ Third-party (Reindeer)

.PHONY: third-party
third-party: ## Re-vendor third-party crates and regenerate third-party/rust/BUCK.
	reindeer vendor
	reindeer --third-party-dir=third-party/rust buckify

.PHONY: third-party-update
third-party-update: ## Update third-party Cargo.lock to latest compatible versions.
	reindeer update
	$(MAKE) third-party

##@ Reproducibility

.PHONY: repro-check
repro-check: ## Build //:all twice and verify byte-identical artifacts.
	bash $(ROOT)/scripts/repro-check.sh

.PHONY: hash-snapshot
hash-snapshot: ## Emit docs/reproducibility-hashes.$(PLATFORM).txt for this build.
	bash $(ROOT)/scripts/hash-snapshot.sh

.PHONY: verify-hashes
verify-hashes: ## Diff this machine's hashes against the committed reference.
	bash $(ROOT)/scripts/verify-hashes.sh

##@ Bazel sub-workspace

.PHONY: bazel-info
bazel-info: ## Probe the AOSP sub-workspace.
	cd $(ROOT)/external/aosp && bazel info

.PHONY: bazel-build
bazel-build: ## Build everything in the AOSP sub-workspace (no-op until P1.13).
	cd $(ROOT)/external/aosp && bazel build //... || true

##@ Lint / format

.PHONY: lint
lint: ## clippy + cargo fmt --check.
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings

.PHONY: fmt
fmt: ## Auto-format Rust + Nix.
	cargo fmt --all
	@command -v nixpkgs-fmt >/dev/null && nixpkgs-fmt flake.nix || true

##@ Nix

.PHONY: nix-check
nix-check: ## Validate the flake.
	nix flake check

.PHONY: nix-update
nix-update: ## Bump flake.lock pins (review the diff).
	nix flake update
