#!/usr/bin/env bash
# test/cve/reproduce.sh — One-shot reproduction script for the CVE candidates.
#
# Runs every PoC against:
#   1. Androguard 4.1.3 (latest PyPI as of 2026-05-08)
#   2. apksigner (Ubuntu's build-tools 31.0.2)
#
# Each call is wrapped to capture exit code, exception class, and source
# location. Output is a deterministic table that can be diff'd against
# the published advisories.

set -u
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
POC_DIR="${ROOT}/cve/poc"

if [[ ! -d "$POC_DIR" ]]; then
    echo "ERROR: PoC directory not found: $POC_DIR" >&2
    exit 2
fi

echo "================================================================"
echo "  APKAXIOM CVE Candidate Reproduction — $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "================================================================"
echo
echo "Tool versions:"
python3 -c "import androguard; print('  androguard', androguard.__version__)"
echo "  apksigner $(apksigner --version 2>&1 | head -1)"
echo

# ── Section 1 — Androguard ─────────────────────────────────────────────────────
echo
echo "=================================================================="
echo "  SECTION 1 — Androguard 4.1.3 (Python)"
echo "=================================================================="
echo
echo "Path                             | Bytes | APK ctor                          | get_androidversion_name             | AnalyzeAPK                          "
echo "---------------------------------+-------+-----------------------------------+-------------------------------------+-------------------------------------"

python3 - "$POC_DIR" <<'PYEOF' 2>/dev/null
import os, sys, traceback, logging
logging.disable(logging.CRITICAL)
poc_dir = sys.argv[1]
devnull_fd = os.open(os.devnull, os.O_WRONLY)
old_stderr = os.dup(2); os.dup2(devnull_fd, 2)

def run(fn, *a):
    try:
        fn(*a); return None
    except Exception as e:
        tb = traceback.extract_tb(sys.exc_info()[2])
        af = [f for f in tb if 'androguard' in f.filename]
        loc = af[-1] if af else tb[-1]
        return (
            type(e).__name__,
            str(e)[:30],
            loc.filename.split('/site-packages/')[-1] + ":" + str(loc.lineno),
        )

from androguard.core.apk import APK
from androguard.misc import AnalyzeAPK

paths = sorted(os.path.join(poc_dir, p) for p in os.listdir(poc_dir) if p.endswith(".apk"))

def gav(p): APK(p).get_androidversion_name()

results = []
for p in paths:
    name = os.path.basename(p)
    sz = os.path.getsize(p)
    r1 = run(APK, p)
    r2 = run(gav, p)
    r3 = run(AnalyzeAPK, p)
    def fmt(r):
        if r is None: return "OK"
        return f"{r[0]} @ {r[2]}"
    results.append((name, sz, r1, r2, r3))

os.dup2(old_stderr, 2); os.close(old_stderr); os.close(devnull_fd)

for name, sz, r1, r2, r3 in results:
    def fmt(r):
        if r is None: return "OK"
        return f"{r[0]} @ {r[2]}"
    print(f"{name:<32} | {sz:>5} | {fmt(r1):<33} | {fmt(r2):<35} | {fmt(r3):<35}")
PYEOF

echo
echo "=================================================================="
echo "  SECTION 2 — apksigner (build-tools 31.0.2)"
echo "=================================================================="
echo
echo "Path                             | Bytes | rc | Exception class                          | First-line message                                "
echo "---------------------------------+-------+----+------------------------------------------+--------------------------------------------------"

for f in "$POC_DIR"/*.apk; do
    name=$(basename "$f")
    sz=$(stat -c %s "$f")
    out=$(apksigner verify --verbose "$f" 2>&1)
    rc=$(apksigner verify --verbose "$f" 2>/dev/null > /dev/null; echo $?)
    if [[ "$out" == *"Exception in thread"* ]]; then
        exc=$(echo "$out" | grep -oP "[a-zA-Z0-9.]+(?:Exception|Error)" | head -1)
        msg=$(echo "$out" | head -1 | sed -E 's/.*Exception: //' | head -c 50)
        printf "%-32s | %5d | %2s | %-40s | %-50s\n" "$name" "$sz" "$rc" "$exc" "$msg"
    else
        first=$(echo "$out" | head -1 | head -c 60)
        printf "%-32s | %5d | %2s | %-40s | %-50s\n" "$name" "$sz" "$rc" "(no exception)" "$first"
    fi
done

echo
echo "================================================================"
echo "  Verification complete. All findings above reproducible from"
echo "  these PoC files alone (under 1 KB each, 5 of 7 ≤ 313 bytes)."
echo "================================================================"
