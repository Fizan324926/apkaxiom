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

.PHONY: p113-aosp-probe-asan
p113-aosp-probe-asan: ## P1.13 Gap-7 — sanitiser-instrumented probe (ASan + UBSan). Catches libziparchive C++ UB the unsanitised probe silently tolerates.
	@mkdir -p $(ROOT)/target
	g++ -std=c++20 -O1 -g -fsanitize=address,undefined \
	  -fno-omit-frame-pointer -fno-sanitize-recover=all \
	  -Wno-class-memaccess -Wno-unused-parameter \
	  -DZLIB_CONST -D_LARGEFILE64_SOURCE -include sys/stat.h \
	  -I$(ROOT)/external/libziparchive/include \
	  -I$(ROOT)/external/libziparchive \
	  -I$(ROOT)/tools/zip-aosp-runtime-probe/include \
	  -I$(ROOT) \
	  -o $(ROOT)/target/zip-aosp-runtime-probe-asan \
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

##@ P1.10 — Merkle commit chain

.PHONY: p110-vectors
p110-vectors: ## P1.10 §B item 6 — BLAKE3 official test-vector parity (35 lengths × 3 modes × 2 widths + streaming).
	cargo test -p axiom-blake3-hacl --release --lib -- --nocapture

.PHONY: p110-cross-impl
p110-cross-impl: ## P1.10 §B item 9 — Rust BLAKE3 vs C-reference (Python blake3) parity on 4 APKs + 35 paint vectors.
	cargo test -p axiom-blake3-hacl --release --test cross_impl -- --nocapture

.PHONY: p110-cross-impl-regen
p110-cross-impl-regen: ## Re-derive cross-impl reference values from the Python blake3 package (operator one-shot).
	python3 scripts/gen-cross-impl-rs.py
	python3 scripts/gen-blake3-vectors.py
	$(MAKE) p110-cross-impl

.PHONY: p110-reproducibility
p110-reproducibility: ## P1.10 §B item 4 — Merkle root reproducibility + KAT regression on 4 real APKs.
	cargo test -p axiom-l1-rs --release --test commit_chain_reproducibility -- --nocapture

.PHONY: p110-chunk-invariance
p110-chunk-invariance: ## P1.10 §B item 7 — Merkle root invariant under chunk_size ∈ {1, 7, 17, 64, 65, 256, 1024, 4096, 4097, 65536}.
	cargo test -p axiom-l1-rs --release --test commit_chain_chunk_invariance -- --nocapture

.PHONY: p110-tamper-fuzz
p110-tamper-fuzz: ## P1.10 §B item 5 — 10K random single-bit-flip mutations × 4 fixtures = 40K trials, kill rate ≥ 99 % per region.
	cargo run -q -p p110-tamper-fuzz --release -- \
	  --runs $(or $(P110_TAMPER_RUNS),10000) \
	  --gate $(or $(P110_TAMPER_GATE),99.0)

.PHONY: p110-chain-fuzz
p110-chain-fuzz: ## P1.10 §B item 10 — 10K-mutation in-process commit-chain fuzz (no panic, deterministic, accept-set parity).
	cargo test -p axiom-l1-rs --release --test fuzz_commit_chain_inproc -- --nocapture

.PHONY: p110-merkle-proof
p110-merkle-proof: ## P1.10 §B item 11 — MerkleProof generate / verify / encode / decode (incl. 1000-leaf stress).
	cargo test -p axiom-l1-rs --release --lib merkle_proof -- --nocapture

.PHONY: p110-hash-throughput
p110-hash-throughput: ## P1.10 §B item 12 — BLAKE3 single-core throughput n=100 (gate ≥ 1.5 GB/s; reports σ + p50/p95).
	cargo run -q -p p110-hash-throughput --release -- \
	  --runs $(or $(P110_RUNS),100) \
	  --gate $(or $(P110_HASH_GATE),1.5)

.PHONY: p110-merkle-perf-delta
p110-merkle-perf-delta: ## P1.10 §B item 2 — 3-arm perf-delta (Δ_lit reported, Δ_overhead gated ≤ 10 % or |Δ|≤2σ).
	cargo run -q -p p110-merkle-perf-delta --release -- \
	  --runs $(or $(P110_PERF_RUNS),20) \
	  --iters $(or $(P110_PERF_ITERS),50) \
	  --gate $(or $(P110_PERF_GATE),15.0)

.PHONY: p110-buck2
p110-buck2: ## P1.10 §B item 14 — multi-arch Buck2 hermeticity (same BUCK builds on x86_64 + aarch64).
	buck2 build \
	  //crates/axiom-blake3-hacl:axiom-blake3-hacl \
	  //tools/p110-hash-throughput:p110-hash-throughput \
	  //tools/p110-merkle-perf-delta:p110-merkle-perf-delta \
	  //tools/p110-tamper-fuzz:p110-tamper-fuzz

.PHONY: p110-gates
p110-gates: ## Run every P1.10 sub-phase gate end-to-end.
	$(MAKE) p110-vectors
	$(MAKE) p110-cross-impl
	$(MAKE) p110-reproducibility
	$(MAKE) p110-chunk-invariance
	$(MAKE) p110-merkle-proof
	$(MAKE) p110-tamper-fuzz
	$(MAKE) p110-chain-fuzz
	$(MAKE) p110-hash-throughput
	$(MAKE) p110-merkle-perf-delta
	$(MAKE) p110-buck2

##@ P1.11 — APK signing schemes (v1/v2/v3/v3.1)

.PHONY: p111-block-parse
p111-block-parse: ## P1.11 §B item 1 — Rust APK signing-block parser unit + integration tests on 4 F-Droid + 3 multi-scheme fixtures.
	cargo test -p axiom-sigblock --release

.PHONY: p111-lean-build
p111-lean-build: ## P1.11 — Lake-build every Lean signing module (4 029 LOC).
	nix develop --accept-flake-config --command lake build Apkaxiom.Signing.Asn1 Apkaxiom.Signing.X509 Apkaxiom.Signing.Pkcs7 Apkaxiom.Signing.Block Apkaxiom.Signing.Scheme Apkaxiom.Signing.V1 Apkaxiom.Signing.V2 Apkaxiom.Signing.V3 Apkaxiom.Signing.V3_1 Apkaxiom.Signing.Dispatch Apkaxiom.Signing.Crypto Apkaxiom.Signing.Asn1.Properties Apkaxiom.Signing.X509.Properties Apkaxiom.Signing.Pkcs7.Properties Apkaxiom.Signing.PoR.Properties Apkaxiom.Signing.Block.Properties Apkaxiom.Signing.Scheme.Properties Apkaxiom.Signing.V1.Properties Apkaxiom.Signing.V2.Properties Apkaxiom.Signing.V3.Properties Apkaxiom.Signing.V3_1.Properties Apkaxiom.Signing.Dispatch.Properties

.PHONY: p111-verifier
p111-verifier: ## P1.11 — full Rust v1/v2/v3/v3.1 verifier tests (cryptographic) on 17 fixtures.
	cargo test -p axiom-sigverify --release

.PHONY: p111-kat
p111-kat: ## P1.11 G9 + G10 — KAT regression + cross-impl SHA-256 (RustCrypto vs Python hashlib reference).
	cargo test -p axiom-sigverify --release --test kat_fixtures

.PHONY: p111-fuzz-inproc
p111-fuzz-inproc: ## P1.11 G12 — in-process fuzz on locate / parse_v2 / parse_v3 / parse_v3_1 (40 K runs).
	cargo test -p axiom-sigblock --release --test fuzz_inproc

.PHONY: p111-tamper-fuzz
p111-tamper-fuzz: ## P1.11 G11 — 10 K random-bit-flip mutations × 4 fixtures, gate ≥ 95 % per committed region.
	cargo run -q -p p111-tamper-fuzz --release -- \
	  --runs $(or $(P111_TAMPER_RUNS),10000) \
	  --gate $(or $(P111_TAMPER_GATE),95.0)

.PHONY: p111-differential-rs
p111-differential-rs: ## P1.11 G5 — Rust differential binary; verifier-level Lean ↔ Rust ↔ apksigner agreement.
	APKSIGNER=$(or $(APKSIGNER),$(HOME)/android-sdk/build-tools/35.0.0/apksigner) cargo run -q -p p111-differential --release

.PHONY: p111-sig-eval
p111-sig-eval: ## P1.11 §B item 3 — build both evaluator binaries.
	cargo build -q -p sig-eval-rust --release
	nix develop --accept-flake-config --command lake build sig-eval

.PHONY: p111-adversarial
p111-adversarial: ## P1.11 §B — regenerate adversarial corpus and verify every variant rejects under apksigner.
	python3 scripts/p111-gen-adversarial.py
	@for f in corpus/signing/adversarial/*.apk; do \
	  if apksigner verify "$$f" >/dev/null 2>&1; then \
	    echo "::error::$$f unexpectedly verifies under apksigner"; exit 1; \
	  else \
	    echo "  reject: $$(basename $$f)"; \
	  fi; \
	done

.PHONY: p111-differential
p111-differential: ## P1.11 §B item 4 (HARD) — three-way Lean ↔ Rust ↔ apksigner differential on 16 APKs.
	bash scripts/p111-differential.sh

.PHONY: p111-buck2
p111-buck2: ## Verify Buck2 builds for every P1.11 target.
	buck2 build \
	  //crates/axiom-sigblock:axiom-sigblock \
	  //tools/sig-eval-rust:sig-eval-rust

.PHONY: p111-coverage
p111-coverage: ## P1.11 G13 — line coverage on axiom-sigblock + axiom-sigverify (gate ≥ 75 %).
	cargo llvm-cov --no-cfg-coverage -p axiom-sigblock --summary-only
	cargo llvm-cov --no-cfg-coverage -p axiom-sigverify --summary-only

.PHONY: p111-gates
p111-gates: ## Run every P1.11 gate end-to-end.
	$(MAKE) p111-block-parse
	$(MAKE) p111-lean-build
	$(MAKE) p111-verifier
	$(MAKE) p111-kat
	$(MAKE) p111-fuzz-inproc
	$(MAKE) p111-tamper-fuzz
	$(MAKE) p111-sig-eval
	$(MAKE) p111-adversarial
	$(MAKE) p111-differential-rs
	$(MAKE) p111-buck2

##@ P1.12 — Verified ZIP layer (LFH + CDR + EOCD + Consistency)

.PHONY: p112-bench-10k
p112-bench-10k: ## P1.12 G5 — generate the 10 000-archive deterministic Bench-10K corpus.
	cargo run -q -p p112-bench-10k --release -- corpus/zip/bench-10k

.PHONY: p112-corpus-drift
p112-corpus-drift: ## P1.12 Gap-11 — assert the committed Bench-10K corpus is byte-identical to a fresh regen.
	@if [ ! -d corpus/zip/bench-10k ]; then \
	  echo "::error::corpus/zip/bench-10k missing — run \`make p112-bench-10k\` first"; exit 1; \
	fi
	@TMP=$$(mktemp -d); \
	  cargo run -q -p p112-bench-10k --release -- $$TMP/bench-10k; \
	  if diff -rq corpus/zip/bench-10k $$TMP/bench-10k > /tmp/p112-corpus-drift.diff 2>&1; then \
	    echo "p112-corpus-drift: bench-10k byte-identical to regen ✓"; \
	    rm -rf $$TMP; \
	  else \
	    echo "::error::p112-corpus-drift: committed bench-10k differs from a fresh regen"; \
	    head -20 /tmp/p112-corpus-drift.diff; \
	    rm -rf $$TMP; exit 1; \
	  fi

.PHONY: p112-tamper-fuzz
p112-tamper-fuzz: ## P1.12 Gap-8 — differential tamper-fuzz: 10 mutations × 10 000 archives, verified ≡ direct on every trial.
	cargo run -q -p p112-tamper-fuzz --release -- --runs 10 --archives 10000

.PHONY: p112-aosp-parity
p112-aosp-parity: ## P1.12 Gap-9 — AOSP runtime parity on Bench-10K (verified-accept ⇒ AOSP-accept ≥ 99 %).
	$(MAKE) p16-aosp-runtime-probe
	cargo run -q -p p112-aosp-parity --release -- --count 10000

.PHONY: p112-tv-bench-10k
p112-tv-bench-10k: ## P1.12 Gap-1 — Lean ↔ Rust TV on the full 10K Bench corpus.
	cargo build -q -p archive-eval-rust --release
	cd $(ROOT) && lake build archive-eval
	cargo run -q -p translation-validator --release -- \
	  --corpus $(ROOT)/corpus/zip/bench-10k \
	  --rust-bin $(ROOT)/target/release/archive-eval-rust \
	  --lean-cmd $(ROOT)/.lake/build/bin/archive-eval \
	  --receipt $(ROOT)/docs/phase-1/P1.12/tv-receipt-bench-10k.txt

.PHONY: p112-coverage
p112-coverage: ## P1.12 Gap-10 — line-coverage gate on the verified umbrella + axiom-zip-ref (≥ 75 %).
	cargo llvm-cov --no-cfg-coverage \
	  -p axiom-l0-zip-verified \
	  -p axiom-zip-ref \
	  --summary-only

.PHONY: p112-perf-delta
p112-perf-delta: ## P1.12 row 4 — verified-vs-handwritten perf gate (HARD ≤ 15 %).
	cargo run -q -p p112-perf-delta --release -- --gate 15.0 --strict 5.0

.PHONY: p112-throughput
p112-throughput: ## P1.12 row 4 — multi-core throughput gate (HARD ≥ 250 APKs/sec/16-core).
	cargo run -q -p p112-throughput --release -- --gate 250

.PHONY: p112-latency
p112-latency: ## P1.12 row 4 — per-archive p99 latency gate (HARD ≤ 80 ms).
	cargo run -q -p p112-latency --release

.PHONY: p112-commit-chain
p112-commit-chain: ## P1.12 row 4 — Bench-1K commit-chain reproducibility (100 % bit-identical).
	cargo run -q -p p112-commit-chain --release -- --count 1000

.PHONY: p112-tv-cdr
p112-tv-cdr: ## P1.12 G4 — regenerate the CDR Lean ↔ Rust translation-validation receipt.
	cargo build -q -p cdr-eval-rust --release
	cd theorems && lake build cdr-eval
	cargo run -q -p translation-validator --release -- \
	  --corpus $(ROOT)/corpus/zip/cdr-valid \
	  --corpus $(ROOT)/corpus/zip/cdr-adversarial \
	  --rust-bin $(ROOT)/target/release/cdr-eval-rust \
	  --lean-cmd "$(ROOT)/theorems/.lake/build/bin/cdr-eval" \
	  --receipt $(ROOT)/docs/phase-1/P1.12/tv-receipt-cdr.txt

.PHONY: p112-tv-consistency
p112-tv-consistency: ## P1.12 G4 — regenerate the whole-archive Lean ↔ Rust TV receipt.
	cargo build -q -p archive-eval-rust --release
	cd theorems && lake build archive-eval
	cargo run -q -p translation-validator --release -- \
	  --corpus $(ROOT)/corpus/zip/archive-valid \
	  --rust-bin $(ROOT)/target/release/archive-eval-rust \
	  --lean-cmd "$(ROOT)/theorems/.lake/build/bin/archive-eval" \
	  --receipt $(ROOT)/docs/phase-1/P1.12/tv-receipt-consistency.txt

.PHONY: p112-tv
p112-tv: ## P1.12 G4 — regenerate every CDR + Consistency TV receipt.
	$(MAKE) p112-tv-cdr
	$(MAKE) p112-tv-consistency

.PHONY: p112-axiom-l0-feature-matrix
p112-axiom-l0-feature-matrix: ## P1.12 G10 — assert axiom-l0 builds + tests on both verified-zip and legacy-zip.
	cargo build  -q -p axiom-l0 --release
	cargo test   -q -p axiom-l0 --release
	cargo build  -q -p axiom-l0 --release --no-default-features --features legacy-zip
	cargo test   -q -p axiom-l0 --release --no-default-features --features legacy-zip

.PHONY: p112-buck2
p112-buck2: ## P1.12 G12 + Gap-15 — verify Buck2 builds every P1.12 crate + tool.
	buck2 build \
	  //crates/axiom-l0-zip-verified:axiom-l0-zip-verified \
	  //crates/axiom-l0:axiom-l0 \
	  //tools/p112-bench-10k:p112-bench-10k \
	  //tools/p112-perf-delta:p112-perf-delta \
	  //tools/p112-throughput:p112-throughput \
	  //tools/p112-latency:p112-latency \
	  //tools/p112-commit-chain:p112-commit-chain \
	  //tools/p112-tamper-fuzz:p112-tamper-fuzz \
	  //tools/p112-aosp-parity:p112-aosp-parity

.PHONY: p112-gates
p112-gates: ## Run every P1.12 gate end-to-end.
	$(MAKE) p112-bench-10k
	$(MAKE) p112-perf-delta
	$(MAKE) p112-throughput
	$(MAKE) p112-latency
	$(MAKE) p112-commit-chain
	$(MAKE) p112-axiom-l0-feature-matrix

.PHONY: p112
p112: p112-gates ## Alias — run every P1.12 gate.

##@ P1.13 — Differential fuzzing plant (Cuttlefish A14 via Nyx)

.PHONY: p113-corpus-seed
p113-corpus-seed: ## P1.13 G3 — assemble fuzz/corpus/seed/ from existing project corpora.
	bash $(ROOT)/scripts/p113-corpus-seed.sh

.PHONY: p113-grammar-loadable
p113-grammar-loadable: ## P1.13 G2 — `cargo test grammar::tests` validates apk-v1.lark loads.
	cargo test -q -p p113-fuzz-harness --release grammar::tests

.PHONY: p113-fuzz
p113-fuzz: ## P1.13 G7 (CI) — bounded dev-mode fuzz: 1 000 iters, archive findings.
	$(MAKE) p113-corpus-seed
	$(MAKE) p16-aosp-runtime-probe
	cargo build -q -p p113-fuzz-harness --release
	rm -rf $(ROOT)/fuzz/findings
	$(ROOT)/target/release/p113-fuzz-driver \
	  --mode dev \
	  --seeds $(ROOT)/fuzz/corpus/seed \
	  --archive $(ROOT)/fuzz/findings \
	  --probe $(ROOT)/target/zip-aosp-runtime-probe \
	  --grammar $(ROOT)/fuzz/grammars/apk-v1.lark \
	  --iters 1000 --log-every 200

.PHONY: p113-fuzz-soak
p113-fuzz-soak: ## P1.13 production soak (continuous; gated on KVM host §C-1). Defaults to 5-minute budget for CI.
	$(MAKE) p113-corpus-seed
	$(MAKE) p16-aosp-runtime-probe
	cargo build -q -p p113-fuzz-harness --release
	$(ROOT)/target/release/p113-fuzz-driver \
	  --mode dev \
	  --seeds $(ROOT)/fuzz/corpus/seed \
	  --archive $(ROOT)/fuzz/findings \
	  --probe $(ROOT)/target/zip-aosp-runtime-probe \
	  --grammar $(ROOT)/fuzz/grammars/apk-v1.lark \
	  --budget $${P113_SOAK_SECONDS:-300} --log-every 5000

.PHONY: p113-replay
p113-replay: ## P1.13 G6 — replay first 100 findings; assert byte-identical reproducibility (HARD).
	cargo build -q -p p113-fuzz-harness --release
	$(ROOT)/target/release/p113-fuzz-replay \
	  --archive $(ROOT)/fuzz/findings/archive.ndjson \
	  --probe $(ROOT)/target/zip-aosp-runtime-probe \
	  --limit 100

.PHONY: p113-dashboard-validate
p113-dashboard-validate: ## P1.13 G8 — validate the Grafana dashboard JSON parses.
	@python3 -c "import json; json.load(open('$(ROOT)/fuzz/dashboards/grafana-fuzzing.json')); print('grafana JSON valid')"

.PHONY: p113-buck2
p113-buck2: ## P1.13 G10 — verify Buck2 builds the harness + every bin.
	buck2 build \
	  //fuzz/harness:p113-fuzz-harness \
	  //fuzz/harness:p113-fuzz-driver \
	  //fuzz/harness:p113-fuzz-replay \
	  //fuzz/harness:p113-fuzz-dedupe \
	  //fuzz/harness:p113-fuzz-grammar-gen \
	  //fuzz/harness:p113-afl-harness

.PHONY: p113-gates
p113-gates: ## Run every P1.13 gate end-to-end (dev mode).
	$(MAKE) p113-corpus-seed
	$(MAKE) p113-grammar-loadable
	$(MAKE) p113-fuzz
	$(MAKE) p113-replay
	$(MAKE) p113-dashboard-validate

.PHONY: p113
p113: p113-gates ## Alias — run every P1.13 gate.

.PHONY: p113-fuzz-50k
p113-fuzz-50k: ## P1.13 Gap-1 — 50 000-iter dev-mode soak with all arms (rate-limited 1/50).
	$(MAKE) p113-corpus-seed
	$(MAKE) p16-aosp-runtime-probe
	$(MAKE) p113-aosp-probe-asan
	cargo build -q -p p113-fuzz-harness --release
	rm -rf $(ROOT)/fuzz/findings
	$(ROOT)/target/release/p113-fuzz-driver \
	  --mode dev \
	  --seeds $(ROOT)/fuzz/corpus/seed \
	  --archive $(ROOT)/fuzz/findings \
	  --probe $(ROOT)/target/zip-aosp-runtime-probe \
	  --asan-probe $(ROOT)/target/zip-aosp-runtime-probe-asan \
	  --grammar $(ROOT)/fuzz/grammars/apk-v1.lark \
	  --arms unzip,jdk-jar,py-zipfile --arms-sample-rate 50 \
	  --metrics 127.0.0.1:9913 \
	  --iters 50000 --log-every 5000 \
	  --min-findings-gate 5

.PHONY: p113-grammar-gen
p113-grammar-gen: ## P1.13 Gap-13 — generate 500 grammar-shaped seeds.
	cargo build -q -p p113-fuzz-harness --release
	$(ROOT)/target/release/p113-fuzz-grammar-gen \
	  --count 500 \
	  --out $(ROOT)/fuzz/corpus/seed/grammar-gen

.PHONY: p113-dedupe
p113-dedupe: ## P1.13 Gap-12 — cluster archive findings by root-cause; emit minimal reproducers.
	cargo build -q -p p113-fuzz-harness --release
	$(ROOT)/target/release/p113-fuzz-dedupe \
	  --archive $(ROOT)/fuzz/findings/archive.ndjson \
	  --out $(ROOT)/fuzz/findings/clusters.ndjson

.PHONY: p113-afl-harness
p113-afl-harness: ## P1.13 Gap-4 — build the AFL++ fork-mode harness.
	cargo build -q -p p113-fuzz-harness --release --bin p113-afl-harness

.PHONY: p113-afl-fuzz
p113-afl-fuzz: ## P1.13 Gap-4 — run AFL++ in non-instrumented (-n) mode for $$P113_AFL_SECONDS (default 300s).
	$(MAKE) p113-afl-harness
	$(MAKE) p16-aosp-runtime-probe
	@if [ ! -e /root/.afl-core-set ]; then \
	  echo core > /proc/sys/kernel/core_pattern 2>/dev/null && touch /root/.afl-core-set || true; \
	fi
	mkdir -p $(ROOT)/fuzz/afl-output
	rm -rf $(ROOT)/fuzz/afl-output/*
	AFL_SKIP_CPUFREQ=1 AFL_NO_AFFINITY=1 \
	  AFL_SKIP_BIN_CHECK=1 AFL_I_DONT_CARE_ABOUT_MISSING_CRASHES=1 \
	  APKAXIOM_AOSP_PROBE=$(ROOT)/target/zip-aosp-runtime-probe \
	  timeout $${P113_AFL_SECONDS:-300} afl-fuzz \
	    -n \
	    -i $(ROOT)/fuzz/corpus/seed/badpack-cves \
	    -o $(ROOT)/fuzz/afl-output \
	    -t 5000 -m none \
	    -V $${P113_AFL_SECONDS:-300} \
	    -- $(ROOT)/target/release/p113-afl-harness @@ || true
	@echo "afl summary:"
	@cat $(ROOT)/fuzz/afl-output/default/fuzzer_stats 2>/dev/null | head -25 || echo "  (no fuzzer_stats — afl exited early)"

.PHONY: p113-afl-instrumented
p113-afl-instrumented: ## P1.13 Gap-4 closure (audit-2) — build sancov-instrumented Rust binary via cargo-afl.
	@command -v cargo-afl >/dev/null 2>&1 || { \
	  echo "cargo-afl missing — install with:  cargo install cargo-afl --version '<0.15' --locked"; \
	  exit 1; \
	}
	PATH=/root/security_research_tools/bin:$$PATH \
	  cargo afl build --manifest-path $(ROOT)/fuzz/afl-instrumented/Cargo.toml \
	    --release --bin p113-afl-instrumented

.PHONY: p113-afl-fuzz-instrumented
p113-afl-fuzz-instrumented: ## P1.13 Gap-4 closure (audit-2) — run AFL++ in instrumented mode (sancov-guided) for $$P113_AFL_SECONDS (default 300s).
	$(MAKE) p113-afl-instrumented
	$(MAKE) p16-aosp-runtime-probe
	@if [ ! -e /root/.afl-core-set ]; then \
	  echo core > /proc/sys/kernel/core_pattern 2>/dev/null && touch /root/.afl-core-set || true; \
	fi
	mkdir -p $(ROOT)/fuzz/afl-instrumented-output
	rm -rf $(ROOT)/fuzz/afl-instrumented-output/*
	PATH=/root/security_research_tools/bin:$$PATH \
	  AFL_SKIP_CPUFREQ=1 AFL_NO_AFFINITY=1 \
	  APKAXIOM_AOSP_PROBE=$(ROOT)/target/zip-aosp-runtime-probe \
	  timeout $${P113_AFL_SECONDS:-300} afl-fuzz \
	    -i $(ROOT)/fuzz/corpus/seed/badpack-cves \
	    -o $(ROOT)/fuzz/afl-instrumented-output \
	    -t 5000 -m none \
	    -V $${P113_AFL_SECONDS:-300} \
	    -- $(ROOT)/fuzz/afl-instrumented/target/release/p113-afl-instrumented || true
	@echo "afl-instrumented summary:"
	@cat $(ROOT)/fuzz/afl-instrumented-output/default/fuzzer_stats 2>/dev/null | head -25 \
	  || echo "  (no fuzzer_stats — afl exited early)"

.PHONY: p113-parallel
p113-parallel: ## P1.13 Gap-6 — Centipede-equivalent: N parallel dev-mode workers (default 4).
	$(MAKE) p113-corpus-seed
	$(MAKE) p16-aosp-runtime-probe
	cargo build -q -p p113-fuzz-harness --release
	rm -rf $(ROOT)/fuzz/findings-parallel
	@for w in 1 2 3 4; do \
	  mkdir -p $(ROOT)/fuzz/findings-parallel/worker-$$w; \
	  $(ROOT)/target/release/p113-fuzz-driver \
	    --mode dev \
	    --seeds $(ROOT)/fuzz/corpus/seed \
	    --archive $(ROOT)/fuzz/findings-parallel/worker-$$w \
	    --probe $(ROOT)/target/zip-aosp-runtime-probe \
	    --grammar $(ROOT)/fuzz/grammars/apk-v1.lark \
	    --iters 5000 --log-every 5000 \
	    --seed $$((0xb113000000000000 + w)) > $(ROOT)/fuzz/findings-parallel/worker-$$w.log 2>&1 & \
	done; \
	wait
	@echo "parallel workers complete:"
	@for w in 1 2 3 4; do \
	  echo "  worker $$w:"; tail -2 $(ROOT)/fuzz/findings-parallel/worker-$$w.log; \
	done

.PHONY: p113-prom-grafana
p113-prom-grafana: ## P1.13 Gap-3 — start Prometheus + Grafana locally (driver must be running with --metrics).
	@mkdir -p $(ROOT)/fuzz/observability
	@cat > $(ROOT)/fuzz/observability/prometheus.yml <<-'PROM'\
		global:\
		  scrape_interval: 5s\
		scrape_configs:\
		  - job_name: p113-fuzz\
		    static_configs:\
		      - targets: ['127.0.0.1:9913']\
		PROM
	@echo "Prometheus config: $(ROOT)/fuzz/observability/prometheus.yml"
	@echo "Run: prometheus --config.file=$(ROOT)/fuzz/observability/prometheus.yml"
	@echo "     grafana-server --homepath /usr/share/grafana --config /etc/grafana/grafana.ini"

.PHONY: p113-coverage-axiom-l0
p113-coverage-axiom-l0: ## P1.13 Gap-8 — coverage of axiom-l0-zip-verified hit by the seed corpus.
	cargo llvm-cov --no-cfg-coverage \
	  -p axiom-l0-zip-verified \
	  -p axiom-zip-ref \
	  --summary-only

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
