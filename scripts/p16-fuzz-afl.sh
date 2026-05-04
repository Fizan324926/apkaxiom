#!/usr/bin/env bash
# Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
#
# P1.6 production fuzz — AFL++ leg.
#
# Drives `tools/zip-fuzz` under afl-fuzz with our valid corpus as the
# seed set. Uses AFL_SKIP_BIN_CHECK=1 because zip-fuzz is a stable-Rust
# binary built without afl-clang-fast instrumentation; AFL still
# exercises its full mutation engine (havoc / splice / cmplog) and
# its forkserver, just without compile-time coverage edges. For
# coverage-guided Rust fuzzing we keep cargo-fuzz / honggfuzz / radamsa
# in the rotation (`make p16-fuzz`); this script is the AFL++ leg.
#
# Pass condition: zero crashes across the campaign. Any AFL crash
# discovery is captured into the output dir for triage.

set -uo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
FUZZ_BIN="$ROOT/target/release/zip-fuzz"
DURATION="${P16_AFL_SECONDS:-60}"
WORKDIR="${P16_AFL_WORKDIR:-/tmp/apkaxiom-afl}"

if ! command -v afl-fuzz >/dev/null 2>&1; then
  echo "::error::afl-fuzz not on PATH (run inside \`nix develop\`)" >&2
  exit 2
fi

if [ ! -x "$FUZZ_BIN" ]; then
  echo "Building zip-fuzz …" >&2
  cargo build -q -p zip-fuzz --release
fi

# AFL refuses to run without these knobs on a system with the
# auto-cpufreq governor; we accept whatever's set, no policy bumps.
export AFL_SKIP_CPUFREQ=1
export AFL_NO_AFFINITY=1
# Allow the non-instrumented binary.
export AFL_SKIP_BIN_CHECK=1
# Don't open the GUI noise.
export AFL_NO_UI=1
# AFL refuses to run when /proc/sys/kernel/core_pattern starts with
# `|` (a pipe to a userspace handler). On dev-shell hosts where we
# cannot rewrite core_pattern as root, this knob downgrades the
# refusal to a warning. The AFL stats still record genuine crashes —
# they're just routed through SIGSEGV / SIGABRT rather than
# coredumpctl.
export AFL_I_DONT_CARE_ABOUT_MISSING_CRASHES=1
# Some kernels also gate /proc/sys/kernel/sched_child_runs_first; we
# don't need it.
export AFL_SKIP_INIT_TIMING=1

run_target() {
  local tgt="$1"
  local seed_dir
  case "$tgt" in
    archive) seed_dir="$ROOT/corpus/zip/archive-valid" ;;
    *)       seed_dir="$ROOT/corpus/zip/$tgt-valid" ;;
  esac
  if [ ! -d "$seed_dir" ]; then
    echo "::error::seed dir missing: $seed_dir" >&2
    return 2
  fi
  local out_dir="$WORKDIR/$tgt"
  rm -rf "$out_dir"
  mkdir -p "$out_dir"

  echo "=== AFL++ $tgt for ${DURATION}s (QEMU mode) ==="
  # AFL takes the seeds from -i and writes findings to -o. Use -V to
  # cap wall-time. Read from stdin (default mode). QEMU mode (-Q)
  # provides binary instrumentation without requiring afl-clang-fast
  # at compile time — the Rust target binary is opaque to AFL but
  # QEMU traces basic blocks for coverage feedback. ~5x slower than
  # native instrumentation but still drives a real coverage-guided
  # campaign.
  if ! timeout $((DURATION + 30)) afl-fuzz \
       -i "$seed_dir" \
       -o "$out_dir" \
       -V "$DURATION" \
       -Q \
       -- "$FUZZ_BIN" --target "$tgt" --iters 1 \
       > "$out_dir/afl.log" 2>&1; then
    rc=$?
    # AFL+ exits 0 on V-cap completion; 124 from timeout means we
    # overshot which is fine; non-zero otherwise is a real fault.
    if [ "$rc" -ne 124 ]; then
      echo "::warn::afl-fuzz exited $rc (campaign may have aborted early)"
      tail -20 "$out_dir/afl.log" >&2
    fi
  fi

  # Crash count from AFL++ output dir (default queue layout is
  # default/crashes/ in single-fuzzer mode).
  local crashes_dir="$out_dir/default/crashes"
  local crashes=0
  if [ -d "$crashes_dir" ]; then
    crashes=$(find "$crashes_dir" -type f ! -name 'README*' | wc -l)
  fi
  if [ "$crashes" -gt 0 ]; then
    echo "::error::AFL++ $tgt found $crashes crash(es); see $crashes_dir" >&2
    return 1
  fi
  # Stats summary. Absence of fuzzer_stats is a real failure
  # (campaign aborted before AFL got past init).
  local stats="$out_dir/default/fuzzer_stats"
  if [ -f "$stats" ]; then
    local execs total_paths
    execs=$(awk -F': *' '/^execs_done/{print $2}' "$stats" | xargs)
    total_paths=$(awk -F': *' '/^corpus_count/{print $2}' "$stats" | xargs)
    echo "  $tgt: ${execs:-?} execs, ${total_paths:-?} paths discovered, 0 crashes"
  else
    echo "::error::p16-fuzz-afl: $tgt produced no fuzzer_stats (init failed)" >&2
    tail -30 "$out_dir/afl.log" >&2
    return 1
  fi
  return 0
}

for tgt in lfh eocd cdr archive; do
  if ! run_target "$tgt"; then
    exit 1
  fi
done

echo
echo "p16-fuzz-afl: 4 targets × ${DURATION}s, 0 crashes across all campaigns"
echo "AFL++ output: $WORKDIR/<target>/default/{queue,crashes,hangs}"
