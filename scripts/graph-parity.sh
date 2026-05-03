#!/usr/bin/env bash
# Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
#
# graph-parity.sh — assert that the Cargo workspace graph and the
# Reindeer-managed Buck2 graph agree on every shared third-party crate
# version. Per ADR-0010.
#
# Why this exists:
#   - `cargo build` resolves third-party deps from the *workspace* `Cargo.lock`.
#   - `buck2 build` resolves them from `third-party/rust/Cargo.lock` (Reindeer).
#   - These two lockfiles are independent — nothing automatically keeps them
#     in sync.
#   - Drift is silent: cargo-only IDE workflows still pass while
#     `buck2 build //:all` links against a stale rlib.
#
# Algorithm:
#   1. `cargo metadata --format-version=1 --locked` → flatten to
#      `<crate>=<version>` lines.
#   2. Same for the Reindeer manifest at `third-party/rust/Cargo.lock`.
#   3. For every crate that appears in *both*, the version must match.
#      Crates that appear in only one are fine (workspace-only or
#      vendor-only is a normal state).
#
# Exits 0 on parity, 1 on mismatch with a per-crate diff.
#
# Output: `target/graph-parity.json` (machine-readable) + a Markdown
# summary on stdout.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

mkdir -p target

WS_LOCK="Cargo.lock"
TP_LOCK="third-party/rust/Cargo.lock"

if [[ ! -f "$WS_LOCK" || ! -f "$TP_LOCK" ]]; then
  echo "FAIL: missing lockfile(s); workspace=$WS_LOCK third-party=$TP_LOCK" >&2
  exit 2
fi

# Helper: flatten a Cargo.lock to "name version" lines via cargo metadata
# (workspace) or by parsing the lockfile directly (Reindeer manifest).
ws_pairs=$(cargo metadata --format-version=1 --locked --offline 2>/dev/null \
  | jq -r '.packages[] | "\(.name) \(.version)"' \
  | sort -u)

# For the Reindeer manifest we parse the lockfile directly to avoid needing
# a Cargo.toml that fully resolves under `cargo metadata`.
tp_pairs=$(awk '
  BEGIN { name=""; ver="" }
  /^\[\[package\]\]/ { name=""; ver="" }
  /^name = / { gsub(/"/,"",$3); name=$3 }
  /^version = / { gsub(/"/,"",$3); ver=$3; if (name && ver) print name, ver }
' "$TP_LOCK" | sort -u)

# Compute the set of names that appear in both.
ws_names=$(awk '{print $1}' <<<"$ws_pairs" | sort -u)
tp_names=$(awk '{print $1}' <<<"$tp_pairs" | sort -u)
shared=$(comm -12 <(echo "$ws_names") <(echo "$tp_names"))

mismatches=()
matches=0
while IFS= read -r name; do
  [[ -z "$name" ]] && continue
  ws_v=$(awk -v n="$name" '$1==n {print $2; exit}' <<<"$ws_pairs")
  tp_v=$(awk -v n="$name" '$1==n {print $2; exit}' <<<"$tp_pairs")
  if [[ "$ws_v" == "$tp_v" ]]; then
    matches=$((matches + 1))
  else
    mismatches+=("$name|$ws_v|$tp_v")
  fi
done <<< "$shared"

# Machine-readable JSON for downstream consumption (CI annotations etc.).
{
  printf '{\n'
  printf '  "workspace_lockfile": "%s",\n' "$WS_LOCK"
  printf '  "third_party_lockfile": "%s",\n' "$TP_LOCK"
  printf '  "shared_crate_count": %d,\n' "$(wc -l <<<"$shared" | tr -d ' ')"
  printf '  "matching_crate_count": %d,\n' "$matches"
  printf '  "mismatches": [\n'
  for i in "${!mismatches[@]}"; do
    IFS='|' read -r n ws tp <<< "${mismatches[$i]}"
    sep=","
    [[ $i -eq $((${#mismatches[@]} - 1)) ]] && sep=""
    printf '    {"crate":"%s","workspace":"%s","third_party":"%s"}%s\n' \
      "$n" "$ws" "$tp" "$sep"
  done
  printf '  ]\n}\n'
} > target/graph-parity.json

if [[ ${#mismatches[@]} -eq 0 ]]; then
  printf '## graph-parity: PASS\n\n'
  printf '%d shared crates, all versions match.\n' "$matches"
  exit 0
fi

printf '## graph-parity: FAIL\n\n'
printf '%d shared crates, %d mismatches:\n\n' \
  "$(wc -l <<<"$shared" | tr -d ' ')" "${#mismatches[@]}"
printf '| crate | workspace | third-party |\n'
printf '|-------|-----------|-------------|\n'
for m in "${mismatches[@]}"; do
  IFS='|' read -r n ws tp <<< "$m"
  printf '| `%s` | %s | %s |\n' "$n" "$ws" "$tp"
done
printf '\nResolve with:\n'
printf '  make third-party-update\n'
printf '\nIf the workspace dep was bumped intentionally, that command\n'
printf 'will sync %s to match. If not, investigate why a vendored crate\n' "$TP_LOCK"
printf 'lags or leads the workspace.\n'
exit 1
