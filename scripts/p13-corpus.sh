#!/usr/bin/env bash
# Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
#
# p13-corpus.sh — populate $P13_APK_CORPUS (default: /tmp/p13-apk-corpus)
# with up to N public F-Droid APKs, drawn from F-Droid's public package
# index. Idempotent: re-running re-uses any APKs already on disk and
# only fetches what is missing.
#
# This is the corpus that `scripts/p13-audit.sh` measures the upstream
# parser against. F-Droid is open-source and license-clean; AndroZoo
# is reserved for malware-class corpora later in P1.13/P1.18.
#
# Usage:
#   bash scripts/p13-corpus.sh           # default 100 APKs
#   bash scripts/p13-corpus.sh 50        # any size
#   P13_APK_CORPUS=/data/apks bash scripts/p13-corpus.sh

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

CORPUS_DIR="${P13_APK_CORPUS:-/tmp/p13-apk-corpus}"
TARGET="${1:-100}"
INDEX_URL="https://f-droid.org/repo/index-v1.json"
INDEX_TMP="/tmp/p13-fdroid-index.json"

mkdir -p "$CORPUS_DIR"

# Count what we already have.
have=$(find "$CORPUS_DIR" -maxdepth 1 -name '*.apk' | wc -l)
if (( have >= TARGET )); then
  echo "PASS: corpus already has $have APKs (≥ $TARGET); nothing to fetch."
  exit 0
fi

# Download the F-Droid v1 index once per script invocation. ~80 MB,
# cached in /tmp so iterative runs are fast.
if [[ ! -f "$INDEX_TMP" ]] || [[ "$(stat -c %s "$INDEX_TMP" 2>/dev/null || echo 0)" -lt 1000000 ]]; then
  echo "Fetching F-Droid index (~80 MB) …"
  curl -fsSL "$INDEX_URL" -o "$INDEX_TMP.tmp"
  mv "$INDEX_TMP.tmp" "$INDEX_TMP"
fi

# Pick the smallest N APKs we don't already have, sorted by .size to
# keep total bandwidth modest. F-Droid stores them at
# https://f-droid.org/repo/<apkName>.
#
# `apkName` is the actual published filename (e.g. "org.fdroid.fdroid_1019050.apk"),
# different from `packageName`. The first entry per `package` is
# typically the latest release.
echo "Selecting $((TARGET - have)) candidates by smallest size …"
candidates=$(jq -r '
  [
    .packages
    | to_entries[]
    | .value[0]                      # latest version per package
    | select(.size != null and .size > 50000 and .size < 30000000)
    | {apkName, size}
  ]
  | sort_by(.size)
  | .[].apkName
' "$INDEX_TMP")

fetched=0
need=$((TARGET - have))
for apkName in $candidates; do
  (( fetched >= need )) && break
  out="$CORPUS_DIR/$apkName"
  if [[ -f "$out" ]]; then
    continue
  fi
  url="https://f-droid.org/repo/$apkName"
  if curl -fsSL --max-time 60 "$url" -o "$out.tmp" 2>/dev/null; then
    mv "$out.tmp" "$out"
    fetched=$((fetched + 1))
    if (( fetched % 10 == 0 )); then
      echo "  … fetched $fetched/$need"
    fi
  else
    rm -f "$out.tmp"
  fi
done

total=$(find "$CORPUS_DIR" -maxdepth 1 -name '*.apk' | wc -l)
total_size=$(du -sh "$CORPUS_DIR" 2>/dev/null | awk '{print $1}')
echo
echo "PASS: corpus at $CORPUS_DIR has $total APK(s), $total_size total."
