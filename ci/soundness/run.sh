#!/usr/bin/env bash
# ci/soundness/run.sh — fail-closed soundness regression runner.
# Usage: run.sh [sorry-audit | lake-verify | tv-validate | signing-tests | full]
# Default: full

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

case "${1:-full}" in

  sorry-audit)
    # Exclude:
    #   '^ *--'   single-line Lean comments
    #   '`sorry`' prose mentions inside block comments / docstrings
    hits=$(grep -rn '\bsorry\b' "$ROOT/theorems/" \
             --include='*.lean' \
             --exclude-dir='.lake' \
           | grep -v '^ *--' \
           | grep -v '`sorry`' \
           || true)
    if [[ -n "$hits" ]]; then
      echo "FAIL sorry-audit: bare sorry found in theorems/"
      echo "$hits"
      exit 1
    fi
    echo "PASS sorry-audit: no sorry in theorems/"
    ;;

  lake-verify)
    cd "$ROOT"
    lake build Apkaxiom 2>&1 | tee /tmp/lake-soundness.log
    if grep -i 'uses .sorry' /tmp/lake-soundness.log; then
      echo "FAIL lake-verify: sorry escaped grep — found in Lake output"
      exit 1
    fi
    echo "PASS lake-verify: all theorems check"
    ;;

  tv-validate)
    cd "$ROOT"
    make p19-gates
    echo "PASS tv-validate: translation validator green"
    ;;

  signing-tests)
    cd "$ROOT"
    make p116-tests
    echo "PASS signing-tests: P1.16 gate green"
    ;;

  full)
    T0=$(date +%s)
    echo "$T0" > /tmp/.soundness-start

    bash "$0" sorry-audit
    bash "$0" lake-verify
    bash "$0" tv-validate
    bash "$0" signing-tests

    T1=$(date +%s)
    ELAPSED=$((T1 - T0))
    echo "PASS soundness: ${ELAPSED}s"
    ;;

  report-timing)
    if [[ -f /tmp/.soundness-start ]]; then
      T0=$(cat /tmp/.soundness-start)
      T1=$(date +%s)
      echo "Wall time: $((T1 - T0)) seconds"
    else
      echo "Wall time: unknown (no start timestamp found)"
    fi
    ;;

  *)
    echo "Usage: run.sh [sorry-audit | lake-verify | tv-validate | signing-tests | full | report-timing]"
    exit 1
    ;;
esac
