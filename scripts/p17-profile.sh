#!/usr/bin/env bash
# Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
#
# P1.7 §C closure — continuous-profiling capture.
#
# Runs the `p17-bench-1k` harness under `perf record`, generates a
# flamegraph SVG via the perl `flamegraph.pl` pipeline, and emits
# folded-stacks output (Pyroscope-compatible). Output goes to
# `docs/phase-1/P1.7/profiles/` and is git-ignored (regenerable).
#
# Pyroscope-compatible: the `<dst>.folded` file produced here is
# the exact format `pprof` and Pyroscope's `pyroscope/folded`
# ingest API consume, so when the operator one-shot lights up the
# self-host stack (per CHECKLIST §C) the same artifacts feed
# dashboards directly.

set -uo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
OUT_DIR="$ROOT/docs/phase-1/P1.7/profiles"
ARCHIVES="${P17_PROFILE_ARCHIVES:-1000}"

mkdir -p "$OUT_DIR"

# Find a perf binary that matches our kernel. The host /usr/bin/perf
# may be linux-tools for an older kernel; nix's perf (from the
# `perf-linux-…` derivation pulled by flamegraph/cargo-flamegraph
# transitively) is what we want for userspace profiling.
PERF_BIN="${PERF_BIN:-}"
if [ -z "$PERF_BIN" ]; then
  # Prefer nix's perf-linux-* over the host /usr/bin/perf wrapper.
  # The host wrapper redirects to /usr/lib/linux-tools/<exact-kernel>
  # which fails when the running kernel doesn't match what's
  # installed (common in dev shells / containers).
  for cand in /nix/store/*-perf-linux-*/bin/perf; do
    if [ -x "$cand" ]; then
      PERF_BIN="$cand"
      break
    fi
  done
  if [ -z "$PERF_BIN" ]; then
    PERF_BIN="$(command -v perf 2>/dev/null || true)"
  fi
fi
if [ -z "$PERF_BIN" ] || [ ! -x "$PERF_BIN" ]; then
  echo "::error::perf not on PATH (run inside \`nix develop\`)" >&2
  exit 2
fi
echo "Using perf: $PERF_BIN"
if ! command -v flamegraph.pl >/dev/null 2>&1; then
  echo "::error::flamegraph.pl not on PATH (nix dev shell should have it)" >&2
  exit 2
fi

# Make sure the bench is built (release).
cargo build -q -p p17-bench-1k --release

PERF_DATA="$OUT_DIR/p17-bench.perf.data"
FOLDED="$OUT_DIR/p17-bench.folded"
SVG="$OUT_DIR/p17-bench.svg"

echo "Running perf record on p17-bench-1k --archives $ARCHIVES …"
# `--call-graph dwarf` is more accurate than fp on Rust binaries
# (Rust release builds elide frame pointers).
"$PERF_BIN" record -F 999 --call-graph dwarf -o "$PERF_DATA" \
  -- "$ROOT/target/release/p17-bench-1k" --archives "$ARCHIVES" \
  >"$OUT_DIR/perf-record.stdout" 2>"$OUT_DIR/perf-record.stderr"
record_rc=$?
if [ $record_rc -ne 0 ]; then
  echo "::error::perf record exited $record_rc; see $OUT_DIR/perf-record.stderr"
  cat "$OUT_DIR/perf-record.stderr" >&2
  exit 1
fi
if [ ! -s "$PERF_DATA" ]; then
  echo "::error::perf record produced no data at $PERF_DATA"
  cat "$OUT_DIR/perf-record.stderr" >&2
  exit 1
fi

echo "Generating folded-stacks output …"
# `stackcollapse-perf.pl` ships in the FlameGraph package next to
# `flamegraph.pl`; resolve it from the same prefix.
FG_PREFIX="$(dirname "$(command -v flamegraph.pl)")"
COLLAPSE="$FG_PREFIX/stackcollapse-perf.pl"
if [ ! -x "$COLLAPSE" ]; then
  echo "::error::stackcollapse-perf.pl not found alongside flamegraph.pl ($FG_PREFIX)" >&2
  exit 2
fi
"$PERF_BIN" script -i "$PERF_DATA" 2>"$OUT_DIR/perf-script.stderr" \
  | "$COLLAPSE" > "$FOLDED"

echo "Generating flamegraph SVG …"
flamegraph.pl "$FOLDED" > "$SVG" 2>"$OUT_DIR/flamegraph.stderr" \
  || echo "::warn::flamegraph.pl unavailable; folded stacks at $FOLDED"

# Brief summary.
samples=$(wc -l < "$FOLDED")
top10=$(sort -t' ' -k2 -nr "$FOLDED" 2>/dev/null | head -10)

echo
echo "Profile capture complete:"
echo "  perf data:       $PERF_DATA ($(du -h "$PERF_DATA" 2>/dev/null | cut -f1))"
echo "  folded stacks:   $FOLDED ($samples samples)"
[ -f "$SVG" ] && echo "  flamegraph SVG:  $SVG"
echo
echo "Top stacks (Pyroscope-compatible folded format):"
echo "$top10"
