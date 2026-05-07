#!/usr/bin/env bash
# APKAXIOM Bench-10K KPI harness wrapper.
# Usage: ./scripts/bench-10k-run.sh [threads]
# Delegates to the Rust integration test bench_10k_kpi_battery.
set -euo pipefail

THREADS="${1:-1}"
CORPUS_DIR="corpus/bench-10k"

if [[ ! -d "$CORPUS_DIR" ]]; then
    echo "ERROR: $CORPUS_DIR not found. Run: python3 scripts/gen-bench-10k.py" >&2
    exit 1
fi

N=$(find "$CORPUS_DIR" -name "*.apk" | wc -l)
echo "Bench-10K: $N APKs, $THREADS thread(s)"

BENCH_THREADS="$THREADS" cargo test --release -p axiom-l1-rs --test bench_10k \
    -- --nocapture bench_10k_kpi_battery 2>&1 | grep -v "^   Compiling\|^    Finished"
