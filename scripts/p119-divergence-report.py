#!/usr/bin/env python3
# scripts/p119-divergence-report.py — Per-APK divergence analysis for APKAXIOM vs Androguard.
#
# Reads the NDJSON outputs of p119-eval-compare (APKAXIOM) and
# p119-androguard-bench.py (Androguard) and produces:
#   - A NDJSON file of divergent APKs (one side ok / other side error)
#   - A summary to stderr
#
# "Divergence" means one tool parsed the APK successfully and the other failed.
# Note: APKAXIOM verdict "reject" (bad signature) is NOT a parse error —
# it means the APK was structurally well-formed but the signing block failed
# verification.  Only "parse-err" / "ir-err" in the ir_sha256 field signals a
# structural failure on the APKAXIOM side.
#
# Usage:
#   python3 scripts/p119-divergence-report.py \
#       --apkaxiom /tmp/p119-apkaxiom-fdroid.ndjson \
#       --androguard /tmp/p119-androguard-fdroid.ndjson \
#       [--output /tmp/p119-divergences.ndjson]

import argparse
import json
import sys


def load_ndjson(path: str) -> dict[str, dict]:
    """Return {filename: record} mapping."""
    records: dict[str, dict] = {}
    with open(path, encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            rec = json.loads(line)
            records[rec["file"]] = rec
    return records


def apkaxiom_ok(rec: dict) -> bool:
    """True if APKAXIOM produced a clean parse (ir_sha256 is a hex hash)."""
    ir = rec.get("ir_sha256", "")
    return ir not in ("parse-err", "ir-err", "no-manifest", "")


def main() -> None:
    parser = argparse.ArgumentParser(description="APKAXIOM vs Androguard divergence report")
    parser.add_argument("--apkaxiom", required=True, help="NDJSON from p119-eval-compare")
    parser.add_argument("--androguard", required=True, help="NDJSON from p119-androguard-bench.py")
    parser.add_argument("--output", "-o", help="NDJSON output for divergent APKs")
    args = parser.parse_args()

    ax_records = load_ndjson(args.apkaxiom)
    ag_records = load_ndjson(args.androguard)

    common = set(ax_records) & set(ag_records)
    only_ax = set(ax_records) - set(ag_records)
    only_ag = set(ag_records) - set(ax_records)

    divergences: list[dict] = []
    n_both_ok = 0
    n_both_fail = 0
    n_ax_only_ok = 0
    n_ag_only_ok = 0

    for fname in sorted(common):
        ax = ax_records[fname]
        ag = ag_records[fname]
        ax_good = apkaxiom_ok(ax)
        ag_good = ag.get("ok", False)

        if ax_good and ag_good:
            n_both_ok += 1
        elif not ax_good and not ag_good:
            n_both_fail += 1
        elif ax_good and not ag_good:
            n_ax_only_ok += 1
            divergences.append({
                "file": fname,
                "type": "ax_ok_ag_fail",
                "ax_elapsed_ms": ax.get("elapsed_ms"),
                "ax_verdict": ax.get("verdict"),
                "ax_ir_sha256": ax.get("ir_sha256"),
                "ag_elapsed_ms": ag.get("elapsed_ms"),
            })
        else:  # not ax_good and ag_good
            n_ag_only_ok += 1
            divergences.append({
                "file": fname,
                "type": "ag_ok_ax_fail",
                "ax_elapsed_ms": ax.get("elapsed_ms"),
                "ax_ir_sha256": ax.get("ir_sha256"),
                "ag_elapsed_ms": ag.get("elapsed_ms"),
            })

    n_total = len(common)
    print(
        f"\nDivergence report: {n_total} APKs in common  "
        f"({len(only_ax)} ax-only  {len(only_ag)} ag-only)",
        file=sys.stderr,
    )
    print(f"  both-ok:      {n_both_ok}", file=sys.stderr)
    print(f"  both-fail:    {n_both_fail}", file=sys.stderr)
    print(f"  ax-ok ag-fail: {n_ax_only_ok}  (APKAXIOM parsed; Androguard failed)", file=sys.stderr)
    print(f"  ag-ok ax-fail: {n_ag_only_ok}  (Androguard parsed; APKAXIOM failed)", file=sys.stderr)
    print(f"  total divergences: {len(divergences)}", file=sys.stderr)

    if args.output:
        with open(args.output, "w", encoding="utf-8") as fh:
            for d in divergences:
                fh.write(json.dumps(d) + "\n")
        print(f"\njson-out: {args.output}", file=sys.stderr)

    # Print the divergent APKs to stdout for quick inspection.
    if divergences:
        print(f"\nDivergent APKs ({len(divergences)}):")
        for d in divergences[:20]:
            print(f"  [{d['type']}] {d['file']}")
        if len(divergences) > 20:
            print(f"  ... and {len(divergences) - 20} more (see --output file)")
    else:
        print("\nNo divergences found.")


if __name__ == "__main__":
    main()
