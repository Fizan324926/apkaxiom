#!/usr/bin/env bash
# Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
#
# P1.6 production fuzz campaign. Drives `tools/zip-fuzz` (a stable-Rust
# panic-catching parser harness) with mutation streams from radamsa
# (the spec-named black-box fuzzer). One 60-second campaign per parser
# target.
#
# Pass condition: zero panics across the campaign. Any panic exits
# the script non-zero, prints the captured input length, and aborts
# the rest of the run.

set -uo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
FUZZ_BIN="$ROOT/target/release/zip-fuzz"
DURATION="${P16_FUZZ_SECONDS:-60}"

if ! command -v radamsa >/dev/null 2>&1; then
  echo "::error::radamsa not on PATH (run inside \`nix develop\`)" >&2
  exit 2
fi

if [ ! -x "$FUZZ_BIN" ]; then
  echo "Building zip-fuzz …" >&2
  cargo build -q -p zip-fuzz --release
fi

run_target() {
  local tgt="$1"
  local src
  case "$tgt" in
    archive) src="$ROOT/corpus/zip/archive-valid" ;;
    stream)  src="$ROOT/corpus/zip/archive-valid" ;;
    *)       src="$ROOT/corpus/zip/$tgt-valid" ;;
  esac
  if [ ! -d "$src" ]; then
    echo "::error::fuzz seed dir missing: $src" >&2
    return 2
  fi

  echo "=== fuzzing $tgt for ${DURATION}s via radamsa ==="
  local iters=0
  local panics=0
  local end=$(( $(date +%s) + DURATION ))
  while [ "$(date +%s)" -lt "$end" ]; do
    # 100 mutations per radamsa invocation to amortise its startup
    # cost; --iters 1 in zip-fuzz processes the whole stdin buffer
    # once per radamsa burst.
    if ! radamsa --seed "$RANDOM" -n 100 "$src"/*.bin \
         | "$FUZZ_BIN" --target "$tgt" --iters 1 \
         > /dev/null 2>&1; then
      panics=$((panics + 1))
      echo "::error::p16-fuzz: panic in $tgt at iteration $iters" >&2
      return 1
    fi
    iters=$((iters + 1))
  done
  echo "  $tgt: $iters radamsa bursts (≈ $((iters * 100)) mutations), 0 panics"
  return 0
}

for tgt in lfh eocd cdr archive stream; do
  if ! run_target "$tgt"; then
    exit 1
  fi
done

echo
echo "p16-fuzz: 5 targets × ${DURATION}s, 0 panics across all campaigns"
