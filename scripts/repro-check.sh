#!/usr/bin/env bash
# Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
#
# repro-check.sh — verify byte-identical artifacts across two clean builds.
#
# This is the *local* leg of the Phase 1 reproducibility contract: a single
# machine, two clean builds, must produce hash-identical output. Cross-machine
# (per-platform) reproducibility is verified by the CI matrix (see
# .github/workflows/ci.yml + scripts/verify-hashes.sh).
#
# Strategy:
#  1. `buck2 clean` — wipe all derived state.
#  2. Build + hash → snapshot A.
#  3. `buck2 clean` again.
#  4. Build + hash → snapshot B.
#  5. Diff A vs B. Identical → exit 0. Otherwise exit 1.
#
# Why two *clean* builds? An incremental build always reuses cached artifacts,
# so it can hide nondeterminism. Two clean builds force every action to
# re-execute end-to-end.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT
A="$WORKDIR/hashes-A.txt"
B="$WORKDIR/hashes-B.txt"

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

echo
echo "=== diff ==="
if diff -u "$A" "$B"; then
  echo
  echo "PASS: artifacts byte-identical across two clean builds on $(uname -sm)."
  exit 0
else
  echo
  echo "FAIL: artifacts diverged between builds. Investigate any of:"
  echo "  - SOURCE_DATE_EPOCH / TZ / LC_ALL drift"
  echo "  - non-deterministic codegen (codegen-units, RUSTC_BOOTSTRAP)"
  echo "  - host-path leakage (--remap-path-prefix)"
  echo "  - unsorted iteration order in build.rs"
  exit 1
fi
