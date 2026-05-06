#!/usr/bin/env bash
# Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
#
# P1.13 G3 — assemble fuzz/corpus/seed/ from the existing
# project corpora. Reproducible: same source corpora ⇒ byte-
# identical seed set. The drift gate `make p113-grammar-drift`
# regenerates and asserts no change.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SEED_DIR="$ROOT/fuzz/corpus/seed"
MANIFEST="$SEED_DIR/manifest.json"

mkdir -p "$SEED_DIR"
rm -rf "$SEED_DIR"/*

declare -A SOURCES=(
  ["bench-1k"]="$ROOT/corpus/zip/bench-10k"
  ["badpack-cves"]="$ROOT/corpus/zip/badpack-cves"
  ["adversarial-mutated"]="$ROOT/corpus/zip/adversarial-mutated"
  ["archive-valid"]="$ROOT/corpus/zip/archive-valid"
  ["wifiautoff-apks"]="$ROOT/corpus/signing"
)

# How many we take from each source. Bench-1K is just the first
# 1 000; the others ship in their entirety.
declare -A LIMITS=(
  ["bench-1k"]=1000
  ["badpack-cves"]=999
  ["adversarial-mutated"]=999
  ["archive-valid"]=999
  ["wifiautoff-apks"]=999
)

# Header.
cat > "$MANIFEST" <<EOF
{
  "schema_version": "p113-seed-1.0",
  "phase": "P1.13",
  "sources": {
EOF

first_src=1
total=0
for src_name in "$(printf '%s\n' "${!SOURCES[@]}" | sort)"; do
  for src_name in $(printf '%s\n' "${!SOURCES[@]}" | sort); do
    src_dir="${SOURCES[$src_name]}"
    limit="${LIMITS[$src_name]}"
    if [[ ! -d "$src_dir" ]]; then
      echo "WARN: source $src_dir missing — skipping" >&2
      continue
    fi
    out_dir="$SEED_DIR/$src_name"
    mkdir -p "$out_dir"
    count=0
    # Find .bin / .apk files, in sorted order.
    while IFS= read -r f; do
      if [[ $count -ge $limit ]]; then break; fi
      base="$(basename "$f")"
      cp "$f" "$out_dir/$base"
      count=$((count+1))
    done < <(find "$src_dir" -type f \( -name "*.bin" -o -name "*.apk" \) | sort)
    total=$((total+count))
    if [[ $first_src -eq 0 ]]; then echo "    ," >> "$MANIFEST"; fi
    first_src=0
    cat >> "$MANIFEST" <<EOF
    "$src_name": {
      "source_dir": "$(realpath --relative-to="$ROOT" "$src_dir")",
      "count": $count,
      "limit": $limit
    }
EOF
  done
  break
done

cat >> "$MANIFEST" <<EOF
  },
  "total_seeds": $total
}
EOF

echo "p113-corpus-seed: wrote $total seeds to $SEED_DIR"
