#!/usr/bin/env bash
# P1.14 audit-2 — fetch a small (~7 MB) curated F-Droid APK
# corpus into fuzz/corpus/real-apks/. Idempotent: re-runs only
# refresh missing files. The 100 selected APKs cover a wide
# range of authors / categories / sizes (50 KB to 5 MB each).
set -euo pipefail
DIR="$(cd "$(dirname "$0")/.." && pwd)/fuzz/corpus/real-apks"
mkdir -p "$DIR"
cd "$DIR"

# 1) Fetch and extract the latest F-Droid index.
if [[ ! -f index-v1.json ]] || [[ -n "${P114_REFRESH_INDEX:-}" ]]; then
  echo "fetching F-Droid index-v1..."
  curl -sSf -o index-v1.jar https://f-droid.org/repo/index-v1.jar
  unzip -p index-v1.jar index-v1.json > index-v1.json
fi

# 2) Pick 100 small APKs deterministically from the index.
python3 - <<'PY'
import json, os
with open('index-v1.json') as f:
    d = json.load(f)
small = []
for pkg, versions in d['packages'].items():
    if not versions:
        continue
    v = versions[0]
    sz = v.get('size', 0)
    if 50_000 <= sz <= 5_000_000:
        small.append((pkg, v.get('apkName'), sz))
small.sort(key=lambda t: t[2])  # smallest first
out = small[:100]
with open('selection.json', 'w') as f:
    json.dump([{'pkg': p, 'apk': a, 'size': s} for (p, a, s) in out], f, indent=2)
print(f'selected {len(out)} APKs, {sum(s for _,_,s in out)/1024/1024:.1f} MB total')
PY

# 3) Download whichever APKs aren't present yet.
python3 - <<'PY'
import json, subprocess, sys, os
with open('selection.json') as f:
    sel = json.load(f)
ok = miss = 0
for e in sel:
    apk = e['apk']
    if os.path.exists(apk):
        ok += 1
        continue
    url = f'https://f-droid.org/repo/{apk}'
    r = subprocess.run(['curl', '-sSf', '--max-time', '20', '-o', apk, url],
                       capture_output=True)
    if r.returncode == 0:
        ok += 1
    else:
        miss += 1
        print(f'  MISS {apk}', file=sys.stderr)
print(f'on disk: {ok}; failed: {miss}')
PY

# 4) Validate every APK is a real ZIP. Move broken ones aside.
ok=0; broken=0
for f in *.apk; do
  if unzip -tq "$f" >/dev/null 2>&1; then
    ok=$((ok+1))
  else
    mv "$f" "$f.broken"
    broken=$((broken+1))
  fi
done
echo "valid APKs: $ok"
[[ $broken -eq 0 ]] || echo "broken (renamed .broken): $broken"

# 5) Cleanup index files (gitignored).
rm -f index-v1.jar index-v1.json selection.json
