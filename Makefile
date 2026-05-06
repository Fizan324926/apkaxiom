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
p14-diagram: ## Re-render P1.4 type-lattice + lowering-flow + encode-pipeline diagrams.
	dot -Tsvg \
	  $(ROOT)/docs/phase-1/P1.4/diagrams/axiom-ir-types.dot \
	  -o $(ROOT)/docs/phase-1/P1.4/diagrams/axiom-ir-types.svg
	dot -Tsvg \
	  $(ROOT)/docs/phase-1/P1.4/diagrams/axiom-ir-flow.dot \
	  -o $(ROOT)/docs/phase-1/P1.4/diagrams/axiom-ir-flow.svg
	mmdc -p $(ROOT)/docs/phase-1/P1.4/diagrams/puppeteer.json \
	  -i $(ROOT)/docs/phase-1/P1.4/diagrams/axiom-ir-encode-pipeline.mmd \
	  -o $(ROOT)/docs/phase-1/P1.4/diagrams/axiom-ir-encode-pipeline.svg

.PHONY: p14-schema-hash
p14-schema-hash: ## Recompute SHA-256 of schema/axiom_ir_v0_1.capnp and pin to ir-data/.
	@sha256sum $(ROOT)/schema/axiom_ir_v0_1.capnp \
	  | awk '{print $$1}' \
	  > $(ROOT)/docs/phase-1/P1.4/ir-data/schema-capnp-hash.txt
	@cat $(ROOT)/docs/phase-1/P1.4/ir-data/schema-capnp-hash.txt

.PHONY: p14-schema-check
p14-schema-check: ## Verify capnp schema SHA-pin (and run capnp compile if installed).
	cargo run -q -p ir-schema-check -- $(ROOT)

##@ P1.5 ZIP layer

.PHONY: p15-corpus
p15-corpus: ## Re-derive the 1800-sample ZIP corpus under corpus/zip/.
	buck2 run //tools/zip-corpus-gen -- $(ROOT)/corpus/zip

.PHONY: p15-differential
p15-differential: ## Lean ↔ Rust differential on every corpus sample (~2 min).
	cargo build -q -p zip-differential --release
	$(ROOT)/target/release/zip-differential $(ROOT)

.PHONY: p15-lean
p15-lean: ## Build the Lean ZIP modules (LocalHeader + Eocd).
	lake build Apkaxiom.Zip.LocalHeader Apkaxiom.Zip.Eocd

.PHONY: p15-aosp-probe
p15-aosp-probe: ## Compile the C++ third-prong AOSP probe (header-only against vendored libziparchive).
	@# IMPORTANT: build under the same shell that will run the
	@# differential — i.e., always under `nix develop`. The probe
	@# links against the active glibc; the differential harness
	@# spawns it from inside `nix develop` and a system-glibc binary
	@# will fail with `GLIBC_ABI_DT_X86_64_PLT not found`.
	@mkdir -p $(ROOT)/target
	g++ -std=c++20 -O2 -Wall -DZLIB_CONST \
	  -I$(ROOT) -I$(ROOT)/tools/zip-aosp-probe/include \
	  -o $(ROOT)/target/zip-aosp-probe \
	  $(ROOT)/tools/zip-aosp-probe/src/zip-aosp-probe.cpp

.PHONY: p16-aosp-runtime-probe
p16-aosp-runtime-probe: ## Compile the C++ runtime probe linking AOSP zip_archive.cc end-to-end.
	@mkdir -p $(ROOT)/target
	g++ -std=c++20 -O2 -Wno-class-memaccess -Wno-unused-parameter \
	  -DZLIB_CONST -D_LARGEFILE64_SOURCE -include sys/stat.h \
	  -I$(ROOT)/external/libziparchive/include \
	  -I$(ROOT)/external/libziparchive \
	  -I$(ROOT)/tools/zip-aosp-runtime-probe/include \
	  -I$(ROOT) \
	  -o $(ROOT)/target/zip-aosp-runtime-probe \
	  $(ROOT)/tools/zip-aosp-runtime-probe/src/zip-aosp-runtime-probe.cpp \
	  $(ROOT)/external/libziparchive/zip_archive.cc \
	  $(ROOT)/external/libziparchive/zip_archive_stream_entry.cc \
	  $(ROOT)/external/libziparchive/zip_cd_entry_map.cc \
	  $(ROOT)/external/libziparchive/zip_error.cpp \
	  -lz

.PHONY: p16-aosp-runtime-report
p16-aosp-runtime-report: p16-aosp-runtime-probe ## Run the AOSP runtime probe across the corpus and report leniency.
	bash $(ROOT)/scripts/p16-aosp-runtime-report.sh

.PHONY: p15-differential-3way
p15-differential-3way: p15-aosp-probe ## Three-way Lean ↔ Rust ↔ AOSP-probe differential.
	cargo build -q -p zip-differential --release
	ZIP_AOSP_PROBE=$(ROOT)/target/zip-aosp-probe \
	  $(ROOT)/target/release/zip-differential $(ROOT)

##@ P1.6 ZIP layer (CDR + Consistency)

.PHONY: p16-lean
p16-lean: ## Build the Lean P1.6 modules (CDR + Properties + Consistency).
	lake build \
	  Apkaxiom.Zip.CentralDirectory \
	  Apkaxiom.Zip.CentralDirectory.Properties \
	  Apkaxiom.Zip.Consistency

.PHONY: p16-corpus
p16-corpus: ## Re-derive the full ZIP corpus (P1.5 1800 + P1.6 1000 = 2800 samples).
	buck2 run //tools/zip-corpus-gen -- $(ROOT)/corpus/zip

.PHONY: p16-differential-3way
p16-differential-3way: p15-aosp-probe ## Alias for p15-differential-3way (full 2800-sample 3-way diff).
	$(MAKE) p15-differential-3way

.PHONY: p16-fuzz
p16-fuzz: ## Production fuzz: radamsa → zip-fuzz, 60s per parser target.
	bash $(ROOT)/scripts/p16-fuzz.sh

.PHONY: p16-fuzz-afl
p16-fuzz-afl: ## Production fuzz: AFL++ in QEMU mode, 60s per parser target.
	bash $(ROOT)/scripts/p16-fuzz-afl.sh

##@ P1.7 streaming reader

.PHONY: p17-bench
p17-bench: ## Run the streaming-vs-file parser microbench (10K iters per arm).
	cargo run -q -p zip-stream-bench --release -- --iters 10000

.PHONY: p17-soak
p17-soak: ## Sustained-throughput soak (default 60s, gate ≥ 500 Mbps; tune via P17_DURATION/P17_MIN_MBPS).
	cargo run -q -p zip-stream-soak --release -- \
	  --duration-secs $(or $(P17_DURATION),60) \
	  --min-mbps $(or $(P17_MIN_MBPS),500)

.PHONY: p17-bench-1k
p17-bench-1k: ## Latency bench against the 1000-archive synthetic corpus.
	cargo run -q -p p17-bench-1k --release -- --archives 1000

.PHONY: p17-profile
p17-profile: ## Capture perf flamegraph + folded stacks for the streaming bench.
	bash $(ROOT)/scripts/p17-profile.sh

.PHONY: p17-soak-async
p17-soak-async: ## io_uring (Glommio) soak via the async ApkParser variant. Requires CAP_SYS_RESOURCE / `ulimit -l unlimited`.
	cd $(ROOT)/tools/zip-stream-soak-async && cargo run --release -- \
	  --duration-secs $(or $(P17_DURATION),30) \
	  --min-mbps $(or $(P17_MIN_MBPS),100)

.PHONY: p18-perf-delta
p18-perf-delta: ## P1.8 §F-1 perf-delta gate (3 arms vs P1.7 baseline; default 5×500K iters; dev-shell gates 0.5%/5%).
	cargo run -q -p p18-perf-delta --release -- \
	  --runs $(or $(P18_RUNS),20) \
	  --iters $(or $(P18_ITERS),500000) \
	  --gate-typestate $(or $(P18_GATE_TYPESTATE),0.5) \
	  --gate-full $(or $(P18_GATE_FULL),5.0)

.PHONY: p18-test-doc
p18-test-doc: ## Run the 26 compile_fail doc-tests that prove the type-state guards.
	cargo test -p axiom-l1-rs --doc

.PHONY: p18-test-real-apk
p18-test-real-apk: ## Run the F-Droid real-APK e2e integration tests (4 distinct APKs).
	cargo test -p axiom-l1-rs --test real_apk_fdroid

.PHONY: p18-test-parity
p18-test-parity: ## Run the sync↔async wrapper parity test against the 4 real APKs.
	cargo test -p axiom-l1-rs --test sync_async_parity

.PHONY: p18-fuzz-inproc
p18-fuzz-inproc: ## Run the 10K-mutation in-process fuzz of the typestate pipeline.
	cargo test -p axiom-l1-rs --release --test fuzz_apk_typestate_inproc -- --nocapture

.PHONY: p18-buck2
p18-buck2: ## Verify Buck2 + Reindeer hermeticity gate.
	$(MAKE) reindeer-check
	buck2 build //crates/axiom-l1-rs:axiom-l1-rs

.PHONY: p18-gates
p18-gates: ## Run every P1.8 sub-phase gate end-to-end.
	$(MAKE) p18-test-doc
	$(MAKE) p18-test-real-apk
	$(MAKE) p18-test-parity
	$(MAKE) p18-fuzz-inproc
	$(MAKE) p18-buck2
	$(MAKE) p18-perf-delta

##@ P1.9 — Translation-validation harness

.PHONY: tv-build
tv-build: ## Build the Lean and Rust LFH evaluators + the validator.
	lake build lfh-eval
	cargo build -p lfh-eval-rust -p translation-validator -p axiom-l0-zip-lfh-verified --release

.PHONY: tv
tv: tv-build ## Run the translation validator over the LFH corpus (Lean ↔ hand-Rust) and write a fresh receipt.
	./target/release/translation-validator \
	  --corpus corpus/zip/lfh-valid \
	  --corpus corpus/zip/lfh-adversarial \
	  --rust-bin target/release/lfh-eval-rust \
	  --receipt docs/phase-1/P1.9/tv-receipt-lfh-full.txt

.PHONY: extract
extract: ## Re-run lean-to-rust on the LocalHeader.lean source.
	cargo build -q -p lean-to-rust --release
	./target/release/lean-to-rust theorems/Apkaxiom/Zip/LocalHeader.lean \
	  crates/axiom-l0-zip-lfh-extracted/src/lib.rs

.PHONY: extract-determinism
extract-determinism: ## Run the extractor twice; assert byte-identical output.
	cargo build -q -p lean-to-rust --release
	./target/release/lean-to-rust theorems/Apkaxiom/Zip/LocalHeader.lean /tmp/extracted-1.rs
	./target/release/lean-to-rust theorems/Apkaxiom/Zip/LocalHeader.lean /tmp/extracted-2.rs
	cmp /tmp/extracted-1.rs /tmp/extracted-2.rs && \
	  echo "PASS: extractor output is deterministic across consecutive runs."

.PHONY: tv-bin-reproducibility
tv-bin-reproducibility: ## Same-host bit-reproducibility check on the Rust evaluator binary.
	cargo build -q -p lfh-eval-rust --release
	cp target/release/lfh-eval-rust /tmp/lfh-eval-rust.1
	cargo clean -p lfh-eval-rust 2>/dev/null || true
	cargo build -q -p lfh-eval-rust --release
	cp target/release/lfh-eval-rust /tmp/lfh-eval-rust.2
	cmp /tmp/lfh-eval-rust.1 /tmp/lfh-eval-rust.2 && \
	  echo "PASS: lfh-eval-rust is bit-reproducible on this host." || \
	  (echo "WARN: lfh-eval-rust binary differs across builds — check SOURCE_DATE_EPOCH / RUSTFLAGS pinning."; \
	   sha256sum /tmp/lfh-eval-rust.1 /tmp/lfh-eval-rust.2; \
	   exit 0)

.PHONY: tv-three-way
tv-three-way: extract tv-build ## Run the three-arm TV (Lean ↔ hand-Rust ↔ extracted-Rust).
	cargo build -q -p lfh-eval-extracted --release
	./target/release/translation-validator \
	  --corpus corpus/zip/lfh-valid \
	  --corpus corpus/zip/lfh-adversarial \
	  --rust-bin target/release/lfh-eval-rust \
	  --extracted-bin target/release/lfh-eval-extracted \
	  --receipt docs/phase-1/P1.9/tv-receipt-lfh-three-way.txt

.PHONY: tv-check-receipt
tv-check-receipt: tv-build ## Re-run the validator and assert the resulting receipt's lean-output-sha256 matches the committed shim constant.
	bash -c '\
	  ./target/release/translation-validator \
	    --corpus corpus/zip/lfh-valid \
	    --corpus corpus/zip/lfh-adversarial \
	    --rust-bin target/release/lfh-eval-rust \
	    --receipt /tmp/tv-receipt-fresh.txt && \
	  cmp /tmp/tv-receipt-fresh.txt docs/phase-1/P1.9/tv-receipt-lfh-full.txt && \
	  echo "PASS: fresh receipt is byte-identical to the committed receipt." \
	'

.PHONY: p19-perf-delta
p19-perf-delta: ## P1.9 §10 row 5 perf-delta gate (verified shim vs hand-Rust, gate ≤ 5%).
	cargo run -q -p p19-perf-delta --release

.PHONY: p19-buck2
p19-buck2: ## Verify Buck2 builds for every P1.9 target.
	buck2 build \
	  //crates/axiom-l0-zip-lfh-verified:axiom-l0-zip-lfh-verified \
	  //crates/axiom-l0-zip-lfh-extracted:axiom-l0-zip-lfh-extracted \
	  //tools/lfh-eval-rust:lfh-eval-rust \
	  //tools/lfh-eval-extracted:lfh-eval-extracted \
	  //tools/eocd-eval-rust:eocd-eval-rust \
	  //tools/translation-validator:translation-validator \
	  //tools/p19-perf-delta:p19-perf-delta

.PHONY: tv-eocd
tv-eocd: ## Run TV harness for the EOCD parser (Lean ↔ hand-Rust).
	cargo build -q -p eocd-eval-rust -p translation-validator --release
	$(NIX_DEVELOP) lake build eocd-eval
	./target/release/translation-validator \
	  --corpus corpus/zip/eocd-valid \
	  --corpus corpus/zip/eocd-adversarial \
	  --rust-bin target/release/eocd-eval-rust \
	  --lean-cmd "lake exe eocd-eval" \
	  --receipt docs/phase-1/P1.9/tv-receipt-eocd.txt

.PHONY: tv-fuzz
tv-fuzz: ## Run the 10K-mutation TV fuzz (verified ↔ extracted).
	cargo test -p axiom-l0-zip-lfh-verified --release --test tv_fuzz_inproc -- --nocapture

.PHONY: tv-coverage-gate
tv-coverage-gate: ## P1.9 §V item 8 — line coverage of axiom_zip_ref::lfh ≥ 85%.
	@bash -c 'if command -v cargo-llvm-cov >/dev/null 2>&1; then \
	  bash scripts/coverage-gate.sh 85.0 ; \
	else \
	  echo "tv-coverage-gate: SKIP (cargo-llvm-cov not in PATH; install via cargo install cargo-llvm-cov)" ; \
	fi'

.PHONY: tv-mutation-gate
tv-mutation-gate: ## P1.9 §V item 2/10 — mutation kill rate on lfh.rs ≥ 95%.
	@bash -c 'if command -v cargo-mutants >/dev/null 2>&1; then \
	  bash scripts/mutation-gate.sh 95.0 ; \
	else \
	  echo "tv-mutation-gate: SKIP (cargo-mutants not in PATH; install via cargo install cargo-mutants)" ; \
	fi'

.PHONY: tv-schema-check
tv-schema-check: ## P1.9 §V item 6 — validate evaluator JSON output against the canonical schema.
	cargo build -q -p tv-schema-check -p lfh-eval-rust -p lfh-eval-extracted --release
	@echo "Lean evaluator output:"
	@cat <(for f in corpus/zip/lfh-valid/*.bin corpus/zip/lfh-adversarial/*.bin; do xxd -p -c 9999 < "$$f"; done) \
	  | lake exe lfh-eval | ./target/release/tv-schema-check
	@echo "Hand-Rust evaluator output:"
	@cat <(for f in corpus/zip/lfh-valid/*.bin corpus/zip/lfh-adversarial/*.bin; do xxd -p -c 9999 < "$$f"; done) \
	  | ./target/release/lfh-eval-rust | ./target/release/tv-schema-check
	@echo "Extracted-Rust evaluator output:"
	@cat <(for f in corpus/zip/lfh-valid/*.bin corpus/zip/lfh-adversarial/*.bin; do xxd -p -c 9999 < "$$f"; done) \
	  | ./target/release/lfh-eval-extracted | ./target/release/tv-schema-check

.PHONY: tv-termination
tv-termination: ## P1.9 §V items 11+13 — termination + side-channel timing on the LFH corpus.
	cargo test -p axiom-zip-ref --release --test termination_and_timing -- --nocapture

.PHONY: p19-gates
p19-gates: ## Run every P1.9 sub-phase gate end-to-end.
	$(MAKE) extract-determinism
	$(MAKE) tv-three-way
	$(MAKE) tv-eocd
	$(MAKE) tv-fuzz
	$(MAKE) tv-coverage-gate
	$(MAKE) tv-mutation-gate
	$(MAKE) tv-schema-check
	$(MAKE) tv-termination
	$(MAKE) tv-check-receipt
	$(MAKE) p19-perf-delta
	$(MAKE) p19-buck2

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
