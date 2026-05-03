#!/usr/bin/env bash
# Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
#
# lint-determinism.sh — static lint for nondeterminism patterns in our
# first-party Rust code. Regex-based; cheap to run; intentionally
# conservative.
#
# What it catches:
#   1. `SystemTime::now()` / `Instant::now()` outside `#[cfg(test)]`.
#      Embeds wall-clock time in build-time codegen.
#   2. `process::id()` outside `#[cfg(test)]`. Embeds PID.
#   3. `rand::thread_rng()` / `random()` outside `#[cfg(test)]`. Non-seeded
#      RNG bakes entropy into output.
#   4. `HashMap::iter` without an explicit `BTreeMap`/`sort` adjacent.
#      Iteration order changes per-process due to randomized hash seed.
#   5. `read_dir(...)` without a sort. Filesystem iteration order varies.
#   6. `env!("HOME")` / `env::current_dir()` outside `#[cfg(test)]`. Path
#      leakage.
#
# Scope: only `crates/` (first-party). Vendored third-party crates are not
# scanned because we do not modify them; if a vendored crate leaks
# nondeterminism it surfaces in `make repro-check` and the fix is a fixup
# or upstream PR, not a lint suppression here.
#
# Exits 0 with the count of warnings on stderr; 1 if any *errors* are
# found. Today every match is a warning; promoting to error is per-rule
# and tracked in ADR-0009.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

WARN=0
ERROR=0

find_pattern() {
  local label="$1" pattern="$2" severity="$3"
  local hits
  hits=$(grep -rn -E "$pattern" crates/ --include='*.rs' \
          --exclude-dir='target' \
          --exclude-dir='tests' 2>/dev/null \
          | grep -v '^.*//' \
          | grep -v 'cfg(test)' \
          || true)
  if [[ -z "$hits" ]]; then
    return 0
  fi
  while IFS= read -r line; do
    [[ -z "$line" ]] && continue
    case "$severity" in
      error) ERROR=$((ERROR + 1)); printf 'error[%s]: %s\n' "$label" "$line" >&2 ;;
      warn)  WARN=$((WARN + 1));   printf 'warn[%s]:  %s\n' "$label" "$line" >&2 ;;
    esac
  done <<< "$hits"
}

find_pattern systemtime  '\bSystemTime::now\(\)'   warn
find_pattern instant     '\bInstant::now\(\)'      warn
find_pattern processid   '\bprocess::id\(\)'       warn
find_pattern threadrng   '\bthread_rng\(\)'        warn
find_pattern randomfn    '\brand::random\b'        warn
# HashMap iteration without sort — heuristic: HashMap declared and `.iter` used
# but no `BTreeMap` or `sort` in the same file.
hashmap_files=$(grep -lrE '\bHashMap\b' crates/ --include='*.rs' \
                 --exclude-dir='target' 2>/dev/null || true)
for f in $hashmap_files; do
  if grep -qE '\.(iter|keys|values|drain)\(' "$f" \
     && ! grep -qE '\bBTreeMap\b|\.sort\b|\.sorted\b' "$f"; then
    WARN=$((WARN + 1))
    printf 'warn[hashmap-iter-no-sort]: %s — HashMap iteration without an adjacent sort\n' "$f" >&2
  fi
done
find_pattern read_dir    '\bfs::read_dir\b'        warn
find_pattern envcurrdir  '\bcurrent_dir\(\)'       warn
find_pattern envvar      '\benv!\("HOME"\)'        warn

echo
if [[ $ERROR -gt 0 ]]; then
  echo "FAIL: $ERROR error(s), $WARN warning(s)" >&2
  exit 1
fi
if [[ $WARN -gt 0 ]]; then
  echo "OK with $WARN warning(s) — review them; promote to error in ADR-0009 if appropriate." >&2
else
  echo "PASS: no determinism lints triggered."
fi
