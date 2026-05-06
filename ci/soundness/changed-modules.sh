#!/usr/bin/env bash
# ci/soundness/changed-modules.sh — emit a 'lake build <modules>' command
# covering only Lean modules touched in the diff vs BASE.
# Usage: changed-modules.sh [BASE_REF]
# Prints a full 'lake build ...' command ready to eval or copy-paste.

set -euo pipefail

BASE="${1:-origin/main}"

changed_lean=$(git diff --name-only "$BASE" HEAD -- 'theorems/**' 2>/dev/null \
               | grep '\.lean$' || true)

if [[ -z "$changed_lean" ]]; then
  echo "lake build Apkaxiom"
  exit 0
fi

modules=()
while IFS= read -r path; do
  module=$(echo "$path" \
    | sed 's|^theorems/||' \
    | sed 's|\.lean$||' \
    | tr '/' '.')
  modules+=("$module")
done <<< "$changed_lean"

echo "lake build ${modules[*]}"
