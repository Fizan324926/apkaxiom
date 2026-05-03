#!/usr/bin/env bash
# Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
#
# theorems/lean-build.sh — invoked by `//theorems:hello` (Buck2 genrule).
# Drives `lake build Apkaxiom` over the repo source tree, then emits a
# sha256 manifest of every produced `.olean` to $1.
#
# Buck2 sandboxes the genrule's *outputs*, not its inputs — `lake build`
# happens in the repo working tree (where Lake expects to be), and the
# `.lake/build/` artefacts persist there. We deliberately do NOT capture
# the artefacts themselves into Buck's cache because they are large
# (hundreds of MB once mathlib lands) and Lake already manages
# incremental rebuilds far better than Buck2 can.

set -euo pipefail

OUT="${1:?usage: lean-build.sh <output-manifest>}"
# Resolve $OUT to an absolute path *before* changing directory.
case "$OUT" in
  /*) ;;                          # already absolute
  *)  OUT="$PWD/$OUT" ;;
esac

cd "$(git rev-parse --show-toplevel)"

# Produce the .olean tree (incremental — Lake handles caching itself).
lake build Apkaxiom >&2

# Emit a sorted, deduped manifest of every *first-party* .olean we
# produced. Mathlib + Batteries + other vendored Lean deps live under
# `.lake/packages/*/` and have their own upstream reproducibility story;
# including them here would bloat our manifest with churn that is not
# under our control.
{
  while IFS= read -r path; do
    [[ -z "$path" || ! -f "$path" ]] && continue
    sha256sum "$path" | awk -v rel="${path#./}" \
      '{printf "%s  %s\n", $1, rel}'
  done < <(find .lake/build/lib -type f -name '*.olean' \
            -path '*/Apkaxiom*' 2>/dev/null | sort -u)
} > "$OUT"

# Final manifest line: the BLAKE3 of the body itself (matches the P1.1
# CORPUS_ROOT pattern in `_hash-artifacts.sh`).
ROOT="$(b3sum --no-names "$OUT")"
printf '%s  CORPUS_ROOT[blake3]\n' "$ROOT" >> "$OUT"

# Mirror to stderr for CI logs.
{
  echo "olean-manifest:"
  cat "$OUT"
} >&2
