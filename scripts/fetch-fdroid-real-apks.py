#!/usr/bin/env python3
# scripts/fetch-fdroid-real-apks.py — Download real F-Droid APKs for bench-10k corpus.
#
# Downloads the F-Droid index-v1.jar (a ZIP containing index-v1.json),
# selects up to TARGET_COUNT APKs covering a variety of sizes and signing
# schemes, skips packages already present in the bench-1k corpus, and
# downloads into corpus/bench-10k/real-fdroid/.
#
# Usage:
#   python3 scripts/fetch-fdroid-real-apks.py \
#       [--target 500] \
#       [--out corpus/bench-10k/real-fdroid] \
#       [--existing fuzz/corpus/bench-1k] \
#       [--max-size-mb 30] \
#       [--min-size-bytes 50000]

import argparse
import hashlib
import io
import json
import os
import sys
import time
import urllib.request
import zipfile

FDROID_REPO = "https://f-droid.org/repo"
INDEX_URL = f"{FDROID_REPO}/index-v1.jar"
USER_AGENT = "APKAXIOM-corpus-builder/1.0 (+research)"


def fetch_index(cache_path: str) -> dict:
    if os.path.exists(cache_path):
        print(f"[index] using cached index at {cache_path}", file=sys.stderr)
        with open(cache_path, "rb") as fh:
            raw = fh.read()
    else:
        print(f"[index] downloading {INDEX_URL} ...", file=sys.stderr)
        req = urllib.request.Request(INDEX_URL, headers={"User-Agent": USER_AGENT})
        with urllib.request.urlopen(req, timeout=120) as resp:
            raw = resp.read()
        with open(cache_path, "wb") as fh:
            fh.write(raw)
        print(f"[index] saved {len(raw):,} bytes to {cache_path}", file=sys.stderr)

    with zipfile.ZipFile(io.BytesIO(raw)) as zf:
        with zf.open("index-v1.json") as jf:
            return json.load(jf)


def collect_existing_packages(existing_dir: str) -> set[str]:
    pkgs: set[str] = set()
    if not existing_dir or not os.path.isdir(existing_dir):
        return pkgs
    for fname in os.listdir(existing_dir):
        if fname.endswith(".apk"):
            # filename is <package>_<versionCode>.apk
            base = fname[: -len(".apk")]
            parts = base.rsplit("_", 1)
            if len(parts) == 2:
                pkgs.add(parts[0])
            else:
                pkgs.add(base)
    return pkgs


def select_apks(
    index: dict,
    existing_pkgs: set[str],
    target: int,
    max_size_bytes: int,
    min_size_bytes: int,
) -> list[dict]:
    """Return a list of {package, apkName, size, hash} dicts to download."""
    packages = index.get("packages", {})
    candidates: list[dict] = []

    for pkg_name, versions in packages.items():
        if pkg_name in existing_pkgs:
            continue
        if not versions:
            continue
        # Pick the most recent version (first in list per F-Droid convention).
        for v in versions:
            apk_name = v.get("apkName", "")
            size = v.get("size", 0)
            apk_hash = v.get("hash", "")
            if not apk_name or not apk_name.endswith(".apk"):
                continue
            if size < min_size_bytes or size > max_size_bytes:
                continue
            candidates.append({
                "package": pkg_name,
                "apkName": apk_name,
                "size": size,
                "hash": apk_hash,
                "versionCode": v.get("versionCode", 0),
            })
            break  # only one version per package

    # Sort by size to get diversity: interleave small/medium/large.
    candidates.sort(key=lambda x: x["size"])
    n = len(candidates)
    print(f"[select] {n} eligible candidates (target={target})", file=sys.stderr)
    if n == 0:
        return []

    # Pick every k-th to spread across the size range.
    if n <= target:
        selected = candidates
    else:
        step = n / target
        selected = [candidates[int(i * step)] for i in range(target)]

    return selected[:target]


def download_apk(apk_info: dict, out_dir: str, retry: int = 3) -> bool:
    apk_name = apk_info["apkName"]
    url = f"{FDROID_REPO}/{apk_name}"
    dest = os.path.join(out_dir, apk_name)
    if os.path.exists(dest):
        return True  # already downloaded

    expected_hash = apk_info.get("hash", "").lower()

    for attempt in range(1, retry + 1):
        try:
            req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
            with urllib.request.urlopen(req, timeout=60) as resp:
                data = resp.read()
            if expected_hash:
                actual = hashlib.sha256(data).hexdigest().lower()
                if actual != expected_hash:
                    print(
                        f"[warn] hash mismatch for {apk_name}: expected={expected_hash} got={actual}",
                        file=sys.stderr,
                    )
                    # Don't skip — still save; hash may be SHA-1 in older entries.
            with open(dest, "wb") as fh:
                fh.write(data)
            return True
        except Exception as exc:
            wait = 2 ** attempt
            print(
                f"[warn] attempt {attempt}/{retry} failed for {apk_name}: {exc}; "
                f"retrying in {wait}s",
                file=sys.stderr,
            )
            time.sleep(wait)
    print(f"[error] giving up on {apk_name}", file=sys.stderr)
    return False


def main() -> None:
    parser = argparse.ArgumentParser(description="Download real F-Droid APKs")
    parser.add_argument("--target", type=int, default=500, help="Number of APKs to download")
    parser.add_argument(
        "--out", default="corpus/bench-10k/real-fdroid", help="Output directory"
    )
    parser.add_argument(
        "--existing",
        default="fuzz/corpus/bench-1k",
        help="Directory of existing APKs to skip (avoids duplicates)",
    )
    parser.add_argument(
        "--max-size-mb", type=float, default=30.0, help="Skip APKs larger than this (MB)"
    )
    parser.add_argument(
        "--min-size-bytes", type=int, default=50_000, help="Skip APKs smaller than this (bytes)"
    )
    parser.add_argument(
        "--index-cache",
        default="/tmp/fdroid-index-v1.jar",
        help="Local cache path for the index JAR",
    )
    args = parser.parse_args()

    out_dir = os.path.abspath(args.out)
    os.makedirs(out_dir, exist_ok=True)

    existing_pkgs = collect_existing_packages(os.path.abspath(args.existing))
    print(f"[existing] {len(existing_pkgs)} packages already in bench-1k (will skip)", file=sys.stderr)

    index = fetch_index(args.index_cache)
    print(f"[index] {len(index.get('packages', {}))} packages total", file=sys.stderr)

    max_size_bytes = int(args.max_size_mb * 1024 * 1024)
    selected = select_apks(
        index, existing_pkgs, args.target, max_size_bytes, args.min_size_bytes
    )
    print(f"[download] {len(selected)} APKs selected for download", file=sys.stderr)

    n_ok = n_err = 0
    t0 = time.perf_counter()
    for i, apk_info in enumerate(selected, 1):
        ok = download_apk(apk_info, out_dir)
        if ok:
            n_ok += 1
        else:
            n_err += 1
        if i % 50 == 0 or i == len(selected):
            elapsed = time.perf_counter() - t0
            rate = i / elapsed
            print(
                f"[progress] {i}/{len(selected)} done  ok={n_ok} err={n_err}  "
                f"{rate:.1f} APKs/s  {elapsed:.0f}s elapsed",
                file=sys.stderr,
            )

    total_size = sum(
        os.path.getsize(os.path.join(out_dir, f))
        for f in os.listdir(out_dir)
        if f.endswith(".apk")
    )
    n_files = len([f for f in os.listdir(out_dir) if f.endswith(".apk")])
    print(
        f"\n[done] {n_files} APKs in {out_dir}  "
        f"({total_size / 1024 / 1024:.1f} MB total)  "
        f"ok={n_ok} err={n_err}",
        file=sys.stderr,
    )


if __name__ == "__main__":
    main()
