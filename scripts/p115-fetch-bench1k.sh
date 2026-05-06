#!/usr/bin/env bash
# P1.15 — fetch a ~1 000-APK F-Droid corpus into
# fuzz/corpus/bench-1k/ for the Bench-1K round-trip gate.
# Idempotent: re-runs only download what is missing.
#
# Target: 1 000 unique APKs across authors / sizes / categories.
# Size range broadened vs. the 100-APK corpus to hit the count.
# All APKs are from the public F-Droid repository; no credentials
# required.
#
# Usage: bash scripts/p115-fetch-bench1k.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DIR="$ROOT/fuzz/corpus/bench-1k"
mkdir -p "$DIR"
cd "$DIR"

echo "p115-fetch-bench1k: target directory $DIR"

# 1) Fetch and extract the latest F-Droid index-v1.
if [[ ! -f index-v1.json ]] || [[ -n "${P115_REFRESH_INDEX:-}" ]]; then
  echo "fetching F-Droid index-v1..."
  curl -sSf -o index-v1.jar https://f-droid.org/repo/index-v1.jar
  unzip -p index-v1.jar index-v1.json > index-v1.json
  echo "index fetched."
fi

# 2) Select up to TARGET APKs deterministically from the index.
TARGET="${P115_APK_TARGET:-1000}"
python3 - "$TARGET" <<'PY'
import json, os, sys

target = int(sys.argv[1])

with open('index-v1.json') as f:
    d = json.load(f)

entries = []
for pkg, versions in d['packages'].items():
    if not versions:
        continue
    v = versions[0]
    sz = v.get('size', 0)
    apk_name = v.get('apkName', '')
    if not apk_name:
        continue
    # Broad size range — 10 KB to 20 MB — to collect 1 000 APKs.
    if 10_000 <= sz <= 20_000_000:
        entries.append((pkg, apk_name, sz))

# Sort by size so we download the smallest first (faster CI feedback).
entries.sort(key=lambda t: t[2])
out = entries[:target]

with open('selection.json', 'w') as f:
    json.dump([{'pkg': p, 'apk': a, 'size': s} for (p, a, s) in out], f, indent=2)

total_mb = sum(s for _, _, s in out) / 1024 / 1024
print(f'selected {len(out)} APKs (of {len(entries)} eligible), {total_mb:.1f} MB total')
PY

# 3) Download whichever APKs aren't present yet.
python3 - <<'PY'
import json, subprocess, sys, os

with open('selection.json') as f:
    sel = json.load(f)

ok = miss = skip = 0
for e in sel:
    apk = e['apk']
    if os.path.exists(apk):
        skip += 1
        continue
    url = f'https://f-droid.org/repo/{apk}'
    r = subprocess.run(
        ['curl', '-sSf', '--max-time', '30', '--retry', '2', '-o', apk, url],
        capture_output=True,
    )
    if r.returncode == 0:
        ok += 1
        if ok % 100 == 0:
            print(f'  downloaded {ok} new APKs so far...')
    else:
        miss += 1
        if miss <= 20:
            print(f'  MISS {apk}', file=sys.stderr)

print(f'already on disk: {skip}; downloaded: {ok}; failed: {miss}')
PY

# 4) Validate every APK is a real ZIP; move broken ones aside.
ok=0; broken=0
for f in *.apk; do
  if unzip -tq "$f" >/dev/null 2>&1; then
    ok=$((ok+1))
  else
    mv "$f" "$f.broken"
    broken=$((broken+1))
  fi
done
echo "valid APKs in bench-1k: $ok"
[[ $broken -eq 0 ]] || echo "broken (renamed .broken): $broken"

# 5) Cleanup index scratch files.
rm -f index-v1.jar index-v1.json selection.json

echo ""
echo "Done — run: make p115-roundtrip P115_CORPUS=$DIR"
echo "       run: make p115-semantic-check P115_CORPUS=$DIR"
