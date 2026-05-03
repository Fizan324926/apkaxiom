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
hash-snapshot: ## Emit docs/phase-1/P1.1/reproducibility-hashes.$(PLATFORM).txt for this build.
	bash $(ROOT)/scripts/hash-snapshot.sh

.PHONY: verify-hashes
verify-hashes: ## Diff this machine's hashes against the committed reference.
	bash $(ROOT)/scripts/verify-hashes.sh

.PHONY: graph-parity
graph-parity: ## Assert Cargo and Reindeer lockfiles agree on shared crates.
	bash $(ROOT)/scripts/graph-parity.sh

.PHONY: audit-toolchains
audit-toolchains: ## Snapshot Buck2 toolchain graph to docs/phase-1/P1.1/audit-toolchains.{txt,json}.
	bash $(ROOT)/scripts/audit-toolchains.sh

.PHONY: reindeer-check
reindeer-check: ## Assert `make third-party` is idempotent against the committed tree.
	bash $(ROOT)/scripts/reindeer-check.sh

.PHONY: rebuilder-attest
rebuilder-attest: ## Run independent rebuild + emit signed attestation JSON.
	bash $(ROOT)/scripts/rebuilder-attest.sh

##@ Supply chain

.PHONY: sbom
sbom: ## Emit CycloneDX SBOM (cargo-cyclonedx + syft).
	bash $(ROOT)/scripts/sbom.sh

.PHONY: sign-hashes
sign-hashes: ## Sign every reference-hash file with cosign keyless.
	bash $(ROOT)/scripts/sign-hashes.sh

.PHONY: security-audit
security-audit: ## cargo-audit on workspace + Reindeer lockfiles.
	bash $(ROOT)/scripts/security-audit.sh

.PHONY: license-check
license-check: ## cargo-deny: license/sources/bans/advisories.
	bash $(ROOT)/scripts/license-check.sh

.PHONY: determinism-lint
determinism-lint: ## Static lints for nondeterminism patterns in first-party Rust.
	bash $(ROOT)/scripts/lint-determinism.sh

.PHONY: wall-time-rollup
wall-time-rollup: ## Append CI wall-time samples + regenerate p99 rollup.
	bash $(ROOT)/scripts/wall-time-rollup.sh

##@ Lean (P1.2)

.PHONY: lean-build
lean-build: ## Lake-build the Apkaxiom theorems (incl. mathlib probe).
	lake build Apkaxiom

.PHONY: lean-extract
lean-extract: ## Re-run lean-to-rust extractor on theorems/Apkaxiom/Hello.lean.
	buck2 run //tools/lean-to-rust -- \
	  theorems/Apkaxiom/Hello.lean \
	  crates/axiom-extract-hello/src/lib.rs

.PHONY: translation-validate
translation-validate: ## Run Lean↔Rust operational-equivalence harness.
	buck2 run //tools/translation-validator

.PHONY: lean-update
lean-update: ## Bump lake-manifest.json (privileged — requires G13 review and an explicit reason).
	@echo "*** lake update is privileged — analogous to nix flake update."
	@echo "*** Each manifest bump goes through a code-review on the resulting"
	@echo "*** lake-manifest.json + reproducibility-hashes diff."
	lake update

##@ P1.3 audit

.PHONY: p13-audit
p13-audit: ## Re-derive P1.3 upstream-apk-info audit data.
	bash $(ROOT)/scripts/p13-audit.sh

.PHONY: p13-diagram
p13-diagram: ## Re-render the v1.0 architecture diagram (graphviz → svg).
	dot -Tsvg \
	  $(ROOT)/docs/phase-1/P1.3/diagrams/axiom-l1-rs-architecture.dot \
	  -o $(ROOT)/docs/phase-1/P1.3/diagrams/axiom-l1-rs-architecture.svg

##@ P1.4 AXIOM-IR v0.1

.PHONY: p14-ir
p14-ir: ## Re-derive P1.4 AXIOM-IR corpus + ir-data JSON via tools/ir-corpus.
	bash $(ROOT)/scripts/p14-ir-corpus.sh

.PHONY: p14-diagram
p14-diagram: ## Re-render P1.4 type-lattice + lowering-flow diagrams.
	dot -Tsvg \
	  $(ROOT)/docs/phase-1/P1.4/diagrams/axiom-ir-types.dot \
	  -o $(ROOT)/docs/phase-1/P1.4/diagrams/axiom-ir-types.svg
	dot -Tsvg \
	  $(ROOT)/docs/phase-1/P1.4/diagrams/axiom-ir-flow.dot \
	  -o $(ROOT)/docs/phase-1/P1.4/diagrams/axiom-ir-flow.svg

.PHONY: p14-schema-hash
p14-schema-hash: ## Recompute SHA-256 of schema/axiom_ir_v0_1.capnp and pin to ir-data/.
	@sha256sum $(ROOT)/schema/axiom_ir_v0_1.capnp \
	  | awk '{print $$1}' \
	  > $(ROOT)/docs/phase-1/P1.4/ir-data/schema-capnp-hash.txt
	@cat $(ROOT)/docs/phase-1/P1.4/ir-data/schema-capnp-hash.txt

.PHONY: p14-schema-check
p14-schema-check: ## Verify capnp schema SHA-pin (and run capnp compile if installed).
	cargo run -q -p ir-schema-check -- $(ROOT)

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
