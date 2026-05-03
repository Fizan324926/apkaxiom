#!/usr/bin/env bash
# Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
#
# _hash-artifacts.sh — shared helper. Builds //:all + test binaries, then
# emits "<sha256>  <basename>" lines for every workspace .rmeta and .rlib in
# `buck-out`, deduplicated by (hash, basename), to stdout.
#
# Used by: repro-check.sh, hash-snapshot.sh, verify-hashes.sh.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

export SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH:-315532800}"
export TZ="${TZ:-UTC}"
export LC_ALL="${LC_ALL:-C.UTF-8}"
export LANG="${LANG:-C.UTF-8}"

BUILD="${1:-build}"  # "build" (default) or "skip" if caller already built.

if [[ "$BUILD" != "skip" ]]; then
  buck2 build //:all \
    //crates/axiom-l0:axiom-l0-test \
    //crates/axiom-l1-rs:axiom-l1-rs-test \
    //crates/axiom-ir:axiom-ir-test >&2
fi

CRATES=(axiom_l0 axiom_l1_rs axiom_ir)
TMP="$(mktemp)"
trap 'rm -f "$TMP"' EXIT

for crate in "${CRATES[@]}"; do
  while IFS= read -r path; do
    [[ -z "$path" || ! -f "$path" ]] && continue
    sha256sum "$path" | awk -v base="$(basename "$path")" \
      '{printf "%s  %s\n", $1, base}' >> "$TMP"
  done < <(find buck-out -type f \
            \( -name "lib${crate}-*.rmeta" -o -name "lib${crate}-*.rlib" \) \
            2>/dev/null | sort -u)
done

sort -u "$TMP"
