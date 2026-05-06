#!/usr/bin/env python3
# Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
#
# P1.15 ground-truth semantic-decode check.
#
# Uses androguard as an independent reference decoder and compares its output
# against AXIOM-IR's manifest decoder on the same APK corpus.
#
# Hard gates (exits non-zero on failure):
#   - package name match rate  ≥ 98 %
#   - min_sdk match rate       ≥ 95 %  (where at least one decoder is non-zero)
#
# Informational (printed but not gated):
#   - average |our_components − ref_components| per APK
#   - average |our_uses_permissions − ref_uses_permissions| per APK
#
# Requires:  pip install androguard
# Usage:     python3 scripts/p115-ground-truth-check.py [--corpus DIR] [--limit N]

import argparse
import json
import logging
import os
import subprocess
import sys
from pathlib import Path

# Suppress androguard's chatty DEBUG/INFO output before importing it.
logging.disable(logging.DEBUG)
for _lg in ("androguard", "androguard.core", "androguard.core.apk",
            "androguard.core.axml", "androguard.core.api_specific_resources"):
    logging.getLogger(_lg).setLevel(logging.CRITICAL)

try:
    from androguard.core.apk import APK as _APK
except ImportError:
    sys.exit("ERROR: androguard not installed — run: pip install androguard")


def androguard_parse(path: Path):
    """Return reference fact dict, or None on parse failure."""
    try:
        a = _APK(str(path))
        comps = (
            len(a.get_activities())
            + len(a.get_services())
            + len(a.get_receivers())
            + len(a.get_providers())
        )
        raw_min = a.get_min_sdk_version()
        raw_tgt = a.get_target_sdk_version()
        return {
            "package": a.get_package() or "",
            "min_sdk": int(raw_min) if raw_min else 0,
            "target_sdk": int(raw_tgt) if raw_tgt else 0,
            "components": comps,
            "uses_permissions": len(a.get_permissions()),
        }
    except Exception:
        return None


def axiom_parse_all(corpus_dir: Path, binary: str) -> dict:
    """Run p115-semantic-check --json; return {filename -> fact_dict}."""
    r = subprocess.run(
        [binary, "--corpus", str(corpus_dir), "--json"],
        capture_output=True,
        text=True,
    )
    result = {}
    for line in r.stdout.splitlines():
        line = line.strip()
        if not line or not line.startswith("{"):
            continue
        try:
            d = json.loads(line)
            result[d["apk"]] = d
        except Exception:
            pass
    return result


def locate_binary(override=None) -> str:
    if override and os.path.isfile(override):
        return override
    candidates = [
        "target/release/p115-semantic-check",
        "./target/release/p115-semantic-check",
    ]
    for c in candidates:
        if os.path.isfile(c):
            return c
    sys.exit(
        "ERROR: p115-semantic-check binary not found.\n"
        "Run: cargo build -p p115-roundtrip --release"
    )


def main() -> None:
    ap = argparse.ArgumentParser(
        description="P1.15 ground-truth comparison: AXIOM-IR vs androguard"
    )
    ap.add_argument(
        "--corpus",
        default="fuzz/corpus/real-apks",
        help="Directory containing .apk files",
    )
    ap.add_argument(
        "--limit",
        type=int,
        default=None,
        help="Max APKs to compare (default: all)",
    )
    ap.add_argument("--binary", default=None, help="Path to p115-semantic-check binary")
    args = ap.parse_args()

    binary = locate_binary(args.binary)
    corpus = Path(args.corpus)
    apks = sorted(corpus.glob("*.apk"))
    if not apks:
        sys.exit(f"ERROR: no .apk files found in {corpus}")
    if args.limit:
        apks = apks[: args.limit]

    print(f"ground-truth-check: {len(apks)} APKs in {corpus}")
    print(f"  reference:  androguard {_APK.__module__.split('.')[0]}")
    print(f"  subject:    {binary}")
    print()

    print("  [1/2] running AXIOM-IR decoder (bulk)...")
    axiom_all = axiom_parse_all(corpus, binary)

    print(f"  [2/2] running androguard reference on {len(apks)} APKs...")
    pkg_match = pkg_mismatch = pkg_skip = 0
    sdk_match = sdk_mismatch = sdk_skip = 0
    comp_delta = perm_delta = 0
    mismatches: list[tuple] = []

    for apk_path in apks:
        name = apk_path.name
        our = axiom_all.get(name)
        ref = androguard_parse(apk_path)

        if our is None or ref is None:
            pkg_skip += 1
            sdk_skip += 1
            continue

        # Package name — must match exactly.
        if our["package"] == ref["package"]:
            pkg_match += 1
        else:
            pkg_mismatch += 1
            mismatches.append((name, "package", our["package"], ref["package"]))

        # min_sdk — compare where at least one decoder decoded it.
        if our["min_sdk"] > 0 or ref["min_sdk"] > 0:
            if our["min_sdk"] == ref["min_sdk"]:
                sdk_match += 1
            else:
                sdk_mismatch += 1
                mismatches.append((name, "min_sdk", our["min_sdk"], ref["min_sdk"]))
        else:
            sdk_skip += 1

        comp_delta += abs(our.get("components", 0) - ref.get("components", 0))
        perm_delta += abs(our.get("uses_permissions", 0) - ref.get("uses_permissions", 0))

    # ── Summary ──────────────────────────────────────────────────────────────
    print()
    print("=" * 65)
    pkg_total = pkg_match + pkg_mismatch
    sdk_total = sdk_match + sdk_mismatch

    pkg_rate = pkg_match / pkg_total if pkg_total else 0.0
    sdk_rate = sdk_match / sdk_total if sdk_total else 1.0

    print(
        f"package name  : {pkg_match}/{pkg_total} match "
        f"({pkg_rate*100:.1f}%)  mismatch={pkg_mismatch}  skip={pkg_skip}"
    )
    print(
        f"min_sdk       : {sdk_match}/{sdk_total} match "
        f"({sdk_rate*100:.1f}%)  mismatch={sdk_mismatch}  skip={sdk_skip}"
    )
    if pkg_total:
        print(f"comp delta    : avg {comp_delta/pkg_total:.2f} per APK (informational)")
        print(f"perm delta    : avg {perm_delta/pkg_total:.2f} per APK (informational)")

    if mismatches:
        print(f"\nMismatches (first 25 shown):")
        for apk, field, ours, ref_val in mismatches[:25]:
            print(f"  {apk}: {field}: ours={ours!r}  ref={ref_val!r}")

    # ── Gates ────────────────────────────────────────────────────────────────
    print()
    print("=" * 65)
    all_pass = True

    pkg_ok = pkg_rate >= 0.98
    sdk_ok = sdk_rate >= 0.95

    if pkg_ok:
        print(f"GATE PASS — package: {pkg_match}/{pkg_total} ({pkg_rate*100:.1f}%) ≥ 98%")
    else:
        print(f"GATE FAIL — package: {pkg_match}/{pkg_total} ({pkg_rate*100:.1f}%) < 98%")
        all_pass = False

    if sdk_ok:
        print(f"GATE PASS — min_sdk: {sdk_match}/{sdk_total} ({sdk_rate*100:.1f}%) ≥ 95%")
    else:
        print(f"GATE FAIL — min_sdk: {sdk_match}/{sdk_total} ({sdk_rate*100:.1f}%) < 95%")
        all_pass = False

    sys.exit(0 if all_pass else 1)


if __name__ == "__main__":
    main()
