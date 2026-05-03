#!/usr/bin/env bash
# Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
#
# repro-budget.sh — actionable failure-mode reporter for repro-check.
#
# Called by `repro-check.sh` after the diff has shown that snapshot A and
# snapshot B disagree. The job is not to fix the divergence but to
# *localise* it: which artifact diverged, where in that artifact, and
# what the host probably did wrong.
#
# Output is one Markdown section per divergent artifact, written to stderr,
# capped at the first 8 divergences (a build with more than 8 divergent
# artifacts is broken in a way that does not benefit from per-file detail).
#
# Inputs (from repro-check.sh):
#   $1 = path to snapshot A (sha256-and-name lines)
#   $2 = path to snapshot B (sha256-and-name lines)
#   $3 = optional: path to a temp dir holding the actual artifacts of
#        build B for byte-level inspection. If omitted, only the snapshot
#        diff is reported.

set -euo pipefail

A="${1:?usage: repro-budget.sh <snapshot-A> <snapshot-B> [build-B-dir]}"
B="${2:?usage: repro-budget.sh <snapshot-A> <snapshot-B> [build-B-dir]}"
BUILD_B="${3:-}"

cd "$(git rev-parse --show-toplevel)"

# Strip header/comment lines + the CORPUS_ROOT line for easier diff.
strip_meta() { grep -vE '^(#|$)' "$1" | grep -vE 'CORPUS_ROOT' | sort -u; }
ABODY="$(strip_meta "$A")"
BBODY="$(strip_meta "$B")"

if [[ "$ABODY" == "$BBODY" ]]; then
  echo "repro-budget: snapshots match line-for-line; CORPUS_ROOT mismatch implies a hashing-script bug, not a build divergence." >&2
  exit 0
fi

ROOT_A="$(grep 'CORPUS_ROOT' "$A" | awk '{print $1}')"
ROOT_B="$(grep 'CORPUS_ROOT' "$B" | awk '{print $1}')"

cat >&2 <<EOF

# Reproducibility budget report

  Build A CORPUS_ROOT: $ROOT_A
  Build B CORPUS_ROOT: $ROOT_B

The two clean builds emitted different artifact hashes. The first 8
divergences follow. Each entry names the artifact, prints the side-by-side
hashes, and (if the build-B tree is provided) shows the offset of the first
byte that disagrees.

Common causes, in priority order:
  1. SOURCE_DATE_EPOCH / TZ / LC_ALL drift between the two builds.
  2. \`--remap-path-prefix\` not in effect (debuginfo / panic strings carry
     absolute host paths).
  3. Non-deterministic codegen (codegen-units > 1, RUSTC_BOOTSTRAP set).
  4. Iteration order in build.rs / proc-macro (HashMap without sort).
  5. Embedded build timestamps from a dep's build script.

EOF

declare -A HASH_A
while IFS= read -r line; do
  [[ -z "$line" ]] && continue
  hash="${line%% *}"
  rest="${line#* }"
  rest="${rest## }"
  HASH_A["$rest"]="$hash"
done <<< "$ABODY"

count=0
while IFS= read -r line; do
  [[ -z "$line" ]] && continue
  hash_b="${line%% *}"
  rest="${line#* }"
  rest="${rest## }"
  hash_a="${HASH_A[$rest]:-<missing>}"
  if [[ "$hash_a" == "$hash_b" ]]; then
    continue
  fi
  count=$((count + 1))
  if [[ $count -gt 8 ]]; then
    echo >&2
    echo "  …(more divergences elided; total mismatches in body: see diff above)" >&2
    break
  fi

  printf '\n## %d. `%s`\n' "$count" "$rest" >&2
  printf '    A: %s\n' "$hash_a" >&2
  printf '    B: %s\n' "$hash_b" >&2

  if [[ -n "$BUILD_B" && -d "$BUILD_B" ]]; then
    ARTIFACT_B="$(find "$BUILD_B" -type f -path "*${rest}" 2>/dev/null | head -1 || true)"
    ARTIFACT_A="$(find buck-out -type f -path "*${rest}" 2>/dev/null | head -1 || true)"
    if [[ -n "$ARTIFACT_A" && -n "$ARTIFACT_B" ]]; then
      first_diff=$(cmp -l "$ARTIFACT_A" "$ARTIFACT_B" 2>/dev/null | head -1 || true)
      if [[ -n "$first_diff" ]]; then
        offset=$(awk '{print $1}' <<< "$first_diff")
        # cmp gives 1-indexed offsets; convert to 0-indexed for hexdump.
        offset=$((offset - 1))
        printf '    First divergent byte at offset 0x%x.\n' "$offset" >&2
        # Window of 32 bytes around the divergence.
        start=$(( offset > 16 ? offset - 16 : 0 ))
        printf '    Hex (build A, ±16 bytes):\n' >&2
        xxd -s "$start" -l 64 "$ARTIFACT_A" | sed 's/^/      /' >&2
        printf '    Hex (build B, ±16 bytes):\n' >&2
        xxd -s "$start" -l 64 "$ARTIFACT_B" | sed 's/^/      /' >&2
      fi
      # If the artifact is an `ar`-style archive (rlib), drill in.
      if file "$ARTIFACT_A" 2>/dev/null | grep -qE 'ar archive|current ar archive'; then
        printf '    Archive members (rlib):\n' >&2
        diff <(ar t "$ARTIFACT_A" 2>/dev/null | sort) \
             <(ar t "$ARTIFACT_B" 2>/dev/null | sort) | sed 's/^/      /' >&2 || true
      fi
    fi
  fi
done <<< "$BBODY"

echo >&2
echo "Re-run with the \`repro-debug\` shell for full diffoscope:" >&2
echo "  nix develop .#repro-debug --command diffoscope <A> <B>" >&2
