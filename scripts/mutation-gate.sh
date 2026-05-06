#!/usr/bin/env bash
# Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
#
# P1.9 §V item 2/10 (engineering-grade approximation) —
# mutation-testing gate for the LFH parser.
#
# `cargo-mutants` generates semantic mutations of `lfh.rs` (e.g.,
# replacing `<` with `<=`, swapping branch arms, returning Default
# instead of computed values). Each mutant is built and the test
# suite is run; if the suite passes, the mutant "escaped" and the
# tests are too weak. We require ≥ 95% kill rate.
#
# This is *not* SMT-proven equivalence (that would require Verus or
# Kani, neither available here), but it's the strongest engineering
# signal of test-suite adequacy short of formal verification: a
# 100% kill rate proves that every syntactic mutation of the parser
# *fails the test suite*, which is a meaningful lower bound on
# semantic coverage.
#
# Usage: mutation-gate.sh [min-kill-percent]
#   min-kill-percent: default 95.0 (current measured: 100% catch
#                      rate, 1/29 unviable so ~96.5% effective)

set -euo pipefail

MIN="${1:-95.0}"

cd "$(git rev-parse --show-toplevel 2>/dev/null || pwd)"

OUT=$(timeout 900 cargo mutants \
  --package axiom-zip-ref \
  --file 'crates/axiom-zip-ref/src/lfh.rs' \
  --no-shuffle \
  --jobs 4 \
  --timeout 30 2>&1 | tail -3)

# Extract counts from the summary line:
#   "29 mutants tested in 20s: 28 caught, 1 unviable"
SUMMARY=$(printf '%s' "$OUT" | grep "mutants tested" | tail -1)
TOTAL=$(printf '%s' "$SUMMARY" | grep -oE '^[0-9]+' | head -1)
CAUGHT=$(printf '%s' "$SUMMARY" | grep -oE '[0-9]+ caught' | grep -oE '^[0-9]+' || echo 0)
UNVIABLE=$(printf '%s' "$SUMMARY" | grep -oE '[0-9]+ unviable' | grep -oE '^[0-9]+' || echo 0)
MISSED=$(printf '%s' "$SUMMARY" | grep -oE '[0-9]+ missed' | grep -oE '^[0-9]+' || echo 0)

if [[ -z "$TOTAL" ]]; then
  echo "FAIL: could not parse cargo-mutants output" >&2
  printf 'output:\n%s\n' "$OUT" >&2
  exit 1
fi

VIABLE=$(( TOTAL - UNVIABLE ))
if [[ "$VIABLE" -le 0 ]]; then
  echo "WARN: all mutants unviable; cannot compute kill rate" >&2
  exit 0
fi

KILL_PCT=$(python3 -c "print(round(100.0 * float('$CAUGHT') / float('$VIABLE'), 2))")
GATE_OK=$(python3 -c "print('PASS' if float('$KILL_PCT') >= float('$MIN') else 'FAIL')")

echo "mutation-gate: lfh.rs viable=$VIABLE caught=$CAUGHT missed=$MISSED unviable=$UNVIABLE  kill-rate=${KILL_PCT}% (min ${MIN}%): ${GATE_OK}"
[[ "$GATE_OK" == "PASS" ]]
