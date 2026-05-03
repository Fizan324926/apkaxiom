#!/usr/bin/env bash
# Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
#
# verify-hashes.sh — diff this build's per-platform hashes against the
# committed reference at `docs/reproducibility-hashes.<platform>.txt`.
#
# Exits 0 iff:
#  - reference file exists for this platform, AND
#  - every reference hash matches a freshly-built artifact's hash
#
# To regenerate the reference if a legitimate input changed (toolchain bump,
# dep bump, source change): run `make hash-snapshot`, commit the new file
# behind an ADR per PHASE_GATES.md.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

UNAME_S=$(uname -s | tr '[:upper:]' '[:lower:]')
UNAME_M=$(uname -m)
PLATFORM="$UNAME_S-$UNAME_M"
REF="docs/reproducibility-hashes.$PLATFORM.txt"

if [[ ! -f "$REF" ]]; then
  echo "FAIL: no reference hashes for platform '$PLATFORM' at $REF" >&2
  echo "  → run \`make hash-snapshot\` to bake one (and gate via ADR review)." >&2
  exit 2
fi

# Strip header comments + blank lines, sort, compare.
expected="$(grep -vE '^(#|$)' "$REF" | sort -u)"

buck2 clean
actual="$(bash scripts/_hash-artifacts.sh)"

if [[ "$expected" == "$actual" ]]; then
  echo "PASS: reference reproducible on $PLATFORM"
  exit 0
fi

echo "FAIL: artifacts diverge from reference on $PLATFORM"
diff -u <(echo "$expected") <(echo "$actual") || true
echo
echo "If the divergence is expected (toolchain/dep bump), regenerate the"
echo "reference with: make hash-snapshot   (then commit + ADR)."
exit 1
