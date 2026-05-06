#!/usr/bin/env bash
# scripts/p118-repro-check.sh — K10 reproducibility gate.
# Runs p118-e2e twice on the same corpus; diffs the NDJSON receipts.
# HARD gate: outputs must be bit-identical (PHASE_GATES.md §5 K10).

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CORPUS="${1:-${ROOT}/fuzz/corpus/real-apks}"
BIN="${ROOT}/target/release/p118-e2e"

if [[ ! -f "$BIN" ]]; then
    cargo build -q -p p118-e2e --release --manifest-path "$ROOT/Cargo.toml"
fi

RUN1="$(mktemp /tmp/p118-repro-run1.XXXXXX.ndjson)"
RUN2="$(mktemp /tmp/p118-repro-run2.XXXXXX.ndjson)"
trap 'rm -f "$RUN1" "$RUN2"' EXIT

echo "run 1..."
"$BIN" --corpus "$CORPUS" --json-out "$RUN1" 2>&1 | grep -v '^corpus:'

echo "run 2..."
"$BIN" --corpus "$CORPUS" --json-out "$RUN2" 2>&1 | grep -v '^corpus:'

if diff "$RUN1" "$RUN2" > /dev/null; then
    echo "PASS K10 reproducibility: outputs are bit-identical across both runs"
else
    echo "FAIL K10 reproducibility: outputs diverged between run 1 and run 2"
    diff "$RUN1" "$RUN2" | head -20
    exit 1
fi
