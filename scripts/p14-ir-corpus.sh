#!/usr/bin/env bash
# Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
#
# p14-ir-corpus.sh — regenerate every committed P1.4 IR datum from the
# in-tree generator. Outputs land in `docs/phase-1/P1.4/ir-data/` and
# `docs/phase-1/P1.4/corpus/`. Idempotent: a second run produces equal
# output (every byte is hand-rolled / deterministic).
#
# Pipeline:
#   1. Build the `ir-corpus` binary via cargo.
#   2. Run it against the canonical paths.
#   3. Pin the SHA-256 of the Cap'n Proto schema text.
#   4. Run cargo test on `axiom-ir` (catches local regressions before
#      they reach CI).

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

IR_DATA="docs/phase-1/P1.4/ir-data"
CORPUS_DIR="docs/phase-1/P1.4/corpus"
SCHEMA="schema/axiom_ir_v0_1.capnp"

mkdir -p "$IR_DATA" "$CORPUS_DIR"

echo "=== build ir-corpus ==="
cargo build -q -p ir-corpus

echo "=== regenerate ir-data + corpus ==="
cargo run -q -p ir-corpus -- "$IR_DATA" "$CORPUS_DIR"

echo "=== pin Cap'n Proto schema hash ==="
sha256sum "$SCHEMA" | awk '{print $1}' > "$IR_DATA/schema-capnp-hash.txt"

echo "=== axiom-ir unit tests ==="
cargo test -q -p axiom-ir

echo "=== axiom-ir clippy ==="
cargo clippy -q -p axiom-ir --all-targets -- -D warnings

echo "=== summary ==="
cat "$IR_DATA/summary.json"
echo
echo "schema-hash:        $(cat "$IR_DATA/schema-hash.txt")"
echo "schema-capnp-hash:  $(cat "$IR_DATA/schema-capnp-hash.txt")"
