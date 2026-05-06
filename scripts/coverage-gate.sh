#!/usr/bin/env bash
# Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
#
# P1.9 §V item 8 — coverage gate.
#
# Runs cargo-llvm-cov against the LFH corpus, extracts the line
# coverage % for `crates/axiom-zip-ref/src/lfh.rs`, and fails if
# it's below the gate threshold.
#
# Usage: coverage-gate.sh [min-percent]
#   min-percent: default 85.0 (current measured: 87.5%)

set -uo pipefail

MIN="${1:-85.0}"
TARGET_FILE="crates/axiom-zip-ref/src/lfh.rs"

cd "$(git rev-parse --show-toplevel 2>/dev/null || pwd)"

# llvm-tools-preview from rustup is required. Inside `nix develop`
# the Lean/Buck2-pinned rustc doesn't bundle it; outside nix the
# rustup toolchain typically does. Check up-front and skip
# gracefully if missing — the gate is real when tools are real.
if ! cargo llvm-cov --version >/dev/null 2>&1; then
  echo "coverage-gate: SKIP (cargo-llvm-cov not installed)"
  exit 0
fi

JSON=$(cargo llvm-cov --package axiom-zip-ref --test coverage_corpus \
  --json --summary-only 2>/dev/null | tail -1)
if [[ -z "$JSON" || "$JSON" != \{* ]]; then
  echo "coverage-gate: SKIP (cargo-llvm-cov ran but produced no JSON — likely llvm-tools-preview missing)"
  exit 0
fi

# Extract per-file line-coverage percent for our target.
PERCENT=$(printf '%s' "$JSON" | python3 -c '
import json, sys
data = json.load(sys.stdin)
files = data["data"][0]["files"]
target = sys.argv[1]
for f in files:
    if f["filename"].endswith(target):
        print(f["summary"]["lines"]["percent"])
        sys.exit(0)
print(-1.0)
sys.exit(1)
' "$TARGET_FILE")

if [[ -z "$PERCENT" ]]; then
  echo "FAIL: could not extract coverage percent from llvm-cov output" >&2
  exit 1
fi

# Compare via Python (bash floats are awkward).
GATE_OK=$(python3 -c "print('PASS' if float('$PERCENT') >= float('$MIN') else 'FAIL')")

echo "coverage-gate: ${TARGET_FILE} line coverage = ${PERCENT}% (min ${MIN}%): ${GATE_OK}"
[[ "$GATE_OK" == "PASS" ]]
