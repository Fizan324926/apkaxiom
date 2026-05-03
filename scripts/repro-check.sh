#!/usr/bin/env bash
# Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
#
# repro-check.sh — verify byte-identical artifacts across two clean builds.
#
# This is the *local* leg of the Phase 1 reproducibility contract: a single
# machine, two clean builds, must produce hash-identical output.
# Cross-machine (per-platform) reproducibility is verified by the CI matrix
# (see .github/workflows/ci.yml + scripts/verify-hashes.sh).
#
# Strategy:
#  1. `buck2 clean` — wipe all derived state.
#  2. Build + hash → snapshot A. Save build B's tree for forensics.
#  3. `buck2 clean` again.
#  4. Build + hash → snapshot B.
#  5. Diff A vs B. Identical → exit 0. Otherwise call repro-budget.sh
#     for an actionable failure report (per ADR-0009).
#
# Why two *clean* builds? An incremental build always reuses cached
# artifacts and can hide nondeterminism. Two clean builds force every
# action to re-execute end-to-end.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT
A="$WORKDIR/hashes-A.txt"
B="$WORKDIR/hashes-B.txt"
BUILD_B_SNAPSHOT="$WORKDIR/build-B-tree"

echo "=== build A: buck2 clean ==="
buck2 clean
echo "=== build A: build + hash ==="
bash scripts/_hash-artifacts.sh > "$A"
echo "snapshot A:"
cat "$A"

echo
echo "=== build B: buck2 clean ==="
buck2 clean
echo "=== build B: build + hash ==="
bash scripts/_hash-artifacts.sh > "$B"
echo "snapshot B:"
cat "$B"

# Stash a copy of build-B's relevant artifacts for the budget reporter.
# We only copy the small set of files participating in the corpus, so the
# tmpdir stays modest even on a real build.
mkdir -p "$BUILD_B_SNAPSHOT"
find buck-out -type f \
  \( -name 'lib*.rmeta' -o -name 'lib*.rlib' \
     -o -path '*__hello_world__*' -name 'out.txt' \
     -o -path '*__*-test__/axiom_*' \) \
  -exec cp --parents -t "$BUILD_B_SNAPSHOT" {} \; 2>/dev/null || true

echo
echo "=== diff ==="
if diff -u "$A" "$B"; then
  echo
  echo "PASS: artifacts byte-identical across two clean builds on $(uname -sm)."
  exit 0
fi

echo
echo "FAIL: artifacts diverged between builds."

# Hand off to the budget reporter for actionable detail.
bash scripts/repro-budget.sh "$A" "$B" "$BUILD_B_SNAPSHOT" || true

exit 1
