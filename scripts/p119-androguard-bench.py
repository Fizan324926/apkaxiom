#!/usr/bin/env python3
# scripts/p119-androguard-bench.py — Androguard timing harness for comparison benchmarks.
#
# Runs Androguard on every APK in a corpus directory and emits comparison-ready
# NDJSON with per-APK elapsed time alongside parse outcome.
#
# Usage:
#   python3 scripts/p119-androguard-bench.py <corpus_dir> \
#       [--output <file>] [--mode manifest|full]
#
# manifest mode: APK() + get_package() + get_permissions()  (default)
# full mode:     AnalyzeAPK() — full DEX disassembly + control-flow analysis
#
# Output NDJSON record:
#   {"file":"foo.apk","tool":"androguard-manifest","elapsed_ms":12.3,"ok":true}

import argparse
import glob
import json
import logging
import os
import sys
import time

# Suppress all Androguard logging before import.
logging.disable(logging.CRITICAL)

try:
    from androguard.core.apk import APK
    from androguard.misc import AnalyzeAPK
except ImportError:
    print("ERROR: androguard not installed. Run: pip install androguard>=4.1", file=sys.stderr)
    sys.exit(1)


def collect_apks(corpus_dir: str) -> list[str]:
    apks = sorted(
        p for p in glob.glob(os.path.join(corpus_dir, "**", "*.apk"), recursive=True)
        if os.path.isfile(p)
    )
    return apks


def run_manifest(path: str) -> tuple[float, bool]:
    t0 = time.perf_counter()
    try:
        a = APK(path)
        _ = a.get_package()
        _ = a.get_permissions()
        return (time.perf_counter() - t0) * 1000.0, True
    except Exception:
        return (time.perf_counter() - t0) * 1000.0, False


def run_full(path: str) -> tuple[float, bool]:
    t0 = time.perf_counter()
    try:
        _a, _d, _dx = AnalyzeAPK(path)
        return (time.perf_counter() - t0) * 1000.0, True
    except Exception:
        return (time.perf_counter() - t0) * 1000.0, False


def percentile(sorted_vals: list[float], p: float) -> float:
    if not sorted_vals:
        return 0.0
    idx = int((p / 100.0) * (len(sorted_vals) - 1))
    return sorted_vals[min(idx, len(sorted_vals) - 1)]


def main() -> None:
    parser = argparse.ArgumentParser(description="Androguard timing harness")
    parser.add_argument("corpus_dir", help="Directory containing APK files")
    parser.add_argument("--output", "-o", help="NDJSON output file")
    parser.add_argument(
        "--mode",
        choices=["manifest", "full"],
        default="manifest",
        help="manifest: APK manifest + permissions only; full: DEX analysis",
    )
    args = parser.parse_args()

    tool_label = f"androguard-{args.mode}"
    apks = collect_apks(args.corpus_dir)
    if not apks:
        print(f"ERROR: no APK files found in {args.corpus_dir}", file=sys.stderr)
        sys.exit(1)

    print(f"corpus: {len(apks)} APKs in {args.corpus_dir}", file=sys.stderr)
    print(f"mode:   {args.mode}", file=sys.stderr)

    runner = run_manifest if args.mode == "manifest" else run_full
    records: list[str] = []
    times: list[float] = []
    n_ok = n_err = 0

    wall_t0 = time.perf_counter()
    for apk_path in apks:
        fname = os.path.basename(apk_path)
        elapsed_ms, ok = runner(apk_path)
        times.append(elapsed_ms)
        if ok:
            n_ok += 1
        else:
            n_err += 1
        record = {
            "file": fname,
            "tool": tool_label,
            "elapsed_ms": round(elapsed_ms, 3),
            "ok": ok,
        }
        records.append(json.dumps(record))

    wall_elapsed = time.perf_counter() - wall_t0
    throughput = len(apks) / wall_elapsed if wall_elapsed > 0 else 0.0

    if args.output:
        with open(args.output, "w", encoding="utf-8") as fh:
            fh.write("\n".join(records) + "\n")
        print(f"json-out: {args.output}", file=sys.stderr)

    # Summary stats to stderr.
    times_sorted = sorted(times)
    p50 = percentile(times_sorted, 50.0)
    p95 = percentile(times_sorted, 95.0)
    p99 = percentile(times_sorted, 99.0)

    print(
        f"\n{tool_label}: n={len(apks)} ({n_ok} ok / {n_err} err)", file=sys.stderr
    )
    print(
        f"  p50={p50:.1f}ms  p95={p95:.1f}ms  p99={p99:.1f}ms", file=sys.stderr
    )
    print(
        f"  throughput={throughput:.1f} APKs/sec  ({wall_elapsed:.2f}s wall)",
        file=sys.stderr,
    )


if __name__ == "__main__":
    main()
