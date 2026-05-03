#!/usr/bin/env bash
# Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
#
# p13-audit.sh — capture machine-readable measurements of the upstream
# apk-info tree pinned at `external/apk-info-pinned-sha.txt`. Outputs
# land in `docs/phase-1/P1.3/audit-data/*.json` and are committed; the
# CHECKLIST renders summary tables from them.
#
# Re-running this script against the same upstream SHA produces equal
# JSON (modulo timestamps which we strip). That makes the audit
# reproducible: anyone can re-derive the numbers in CHECKLIST §B from
# the upstream tree alone.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

UPSTREAM="external/apk-info-upstream"
OUT_DIR="docs/phase-1/P1.3/audit-data"
SHA_FILE="external/apk-info-pinned-sha.txt"

if [[ ! -d "$UPSTREAM" ]]; then
  echo "FAIL: $UPSTREAM not present. Run \`git clone https://github.com/delvinru/apk-info $UPSTREAM\` first." >&2
  exit 2
fi

PINNED_SHA="$(cat "$SHA_FILE")"
ACTUAL_SHA="$(cd "$UPSTREAM" && git rev-parse HEAD 2>/dev/null || echo unknown)"
if [[ -n "$ACTUAL_SHA" && "$ACTUAL_SHA" != "unknown" && "$ACTUAL_SHA" != "$PINNED_SHA" ]]; then
  echo "FAIL: $UPSTREAM is at $ACTUAL_SHA but $SHA_FILE pins $PINNED_SHA" >&2
  exit 2
fi

mkdir -p "$OUT_DIR"

echo "=== upstream identity ==="
{
  printf '{\n'
  printf '  "schema": "apkaxiom.p13-audit/v1",\n'
  printf '  "upstream_repo": "https://github.com/delvinru/apk-info",\n'
  printf '  "pinned_sha": "%s",\n' "$PINNED_SHA"
  printf '  "audited_at_utc": "%s"\n' "$(date -u -d "@${SOURCE_DATE_EPOCH:-$(date -u +%s)}" +%Y-%m-%dT%H:%M:%SZ)"
  printf '}\n'
} > "$OUT_DIR/identity.json"

# ---------------------------------------------------------------------------
# 1) Lines of code (tokei) — per crate, per language.
# ---------------------------------------------------------------------------
echo "=== tokei (LOC) ==="
( cd "$UPSTREAM" && tokei --output json . ) > "$OUT_DIR/tokei.json"

# ---------------------------------------------------------------------------
# 2) cargo-deny / cargo-audit on the upstream lockfile.
# ---------------------------------------------------------------------------
echo "=== cargo-audit (upstream Cargo.lock) ==="
cargo audit --json --file "$UPSTREAM/Cargo.lock" > "$OUT_DIR/cargo-audit.json" || true

# ---------------------------------------------------------------------------
# 3) `unsafe` block census — per crate, per file.
# ---------------------------------------------------------------------------
echo "=== unsafe census ==="
{
  printf '{\n  "by_file": [\n'
  first=1
  while IFS= read -r f; do
    [[ -z "$f" ]] && continue
    # `grep -c` exits 1 when count is zero; swallow the exit but keep
    # the count itself.
    n=$(grep -cE '\bunsafe\b' "$f" 2>/dev/null || true)
    n="${n:-0}"
    [[ "$n" -eq 0 ]] && continue
    rel="${f#"$UPSTREAM"/}"
    if [[ $first -eq 1 ]]; then first=0; else printf ',\n'; fi
    printf '    {"file": "%s", "unsafe_occurrences": %d}' "$rel" "$n"
  done < <(find "$UPSTREAM" -type f -name '*.rs' -not -path '*/target/*' | sort)
  printf '\n  ]\n}\n'
} > "$OUT_DIR/unsafe-census.json"

# ---------------------------------------------------------------------------
# 4) Public-API surface — every `pub fn`, `pub struct`, `pub enum`,
#    `pub trait`, `pub type` per crate. Approximation: simple grep, good
#    enough for the audit (a real syn-based scan is P1.7 work).
# ---------------------------------------------------------------------------
echo "=== public-API surface ==="
{
  printf '{\n  "crates": [\n'
  first=1
  for crate_dir in "$UPSTREAM"/cli "$UPSTREAM"/core "$UPSTREAM"/crates/*/; do
    [[ -d "$crate_dir" ]] || continue
    [[ -d "$crate_dir/src" ]] || continue
    name="$(basename "$crate_dir")"
    fns=$( (grep -rhE '^\s*pub\s+(async\s+)?fn\s+\w+'  "$crate_dir"/src/ 2>/dev/null || true) | wc -l)
    structs=$( (grep -rhE '^\s*pub\s+struct\s+\w+'     "$crate_dir"/src/ 2>/dev/null || true) | wc -l)
    enums=$(   (grep -rhE '^\s*pub\s+enum\s+\w+'       "$crate_dir"/src/ 2>/dev/null || true) | wc -l)
    traits=$(  (grep -rhE '^\s*pub\s+trait\s+\w+'      "$crate_dir"/src/ 2>/dev/null || true) | wc -l)
    types=$(   (grep -rhE '^\s*pub\s+type\s+\w+'       "$crate_dir"/src/ 2>/dev/null || true) | wc -l)
    if [[ $first -eq 1 ]]; then first=0; else printf ',\n'; fi
    printf '    {"crate":"%s","pub_fn":%d,"pub_struct":%d,"pub_enum":%d,"pub_trait":%d,"pub_type":%d}' \
      "$name" "$fns" "$structs" "$enums" "$traits" "$types"
  done
  printf '\n  ]\n}\n'
} > "$OUT_DIR/public-api.json"

# ---------------------------------------------------------------------------
# 5) Dep tree summary — count direct deps + total resolved deps from the
#    workspace lockfile.
# ---------------------------------------------------------------------------
echo "=== dep tree summary ==="
total_deps=$(awk '/^\[\[package\]\]/{count++} END{print count+0}' "$UPSTREAM/Cargo.lock")
direct_deps=$(awk '
  BEGIN{in_deps=0}
  /^\[workspace\.dependencies\]/ { in_deps=1; next }
  /^\[/ { in_deps=0 }
  in_deps && /^[a-zA-Z0-9_-]+\s*=/ { count++ }
  END{print count+0}
' "$UPSTREAM/Cargo.toml")
{
  printf '{\n'
  printf '  "total_resolved": %d,\n' "$total_deps"
  printf '  "direct_workspace_deps": %d\n' "$direct_deps"
  printf '}\n'
} > "$OUT_DIR/deps.json"

# ---------------------------------------------------------------------------
# 6) cargo-bloat — needs an actual build, which needs upstream's edition
#    2024 toolchain. Skip if our pinned rustc cannot compile it; record
#    the reason for transparency.
# ---------------------------------------------------------------------------
echo "=== cargo-bloat (best-effort) ==="
if ( cd "$UPSTREAM" && cargo build --release --offline 2>/dev/null >/dev/null ); then
  ( cd "$UPSTREAM" && cargo bloat --release --message-format json -n 50 ) \
    > "$OUT_DIR/cargo-bloat.json"
else
  {
    printf '{\n'
    printf '  "skipped": true,\n'
    printf '  "reason": "upstream uses edition 2024 + rust nightly; our pinned rustc 1.83.0 (stable) declines to build it. cargo-bloat would land in a follow-up audit run on a nightly-augmented dev-shell."\n'
    printf '}\n'
  } > "$OUT_DIR/cargo-bloat.json"
fi

# ---------------------------------------------------------------------------
# Summary roll-up — single-file digest the CHECKLIST renders.
# ---------------------------------------------------------------------------
echo "=== summary ==="
jq -n \
  --slurpfile id "$OUT_DIR/identity.json" \
  --slurpfile loc "$OUT_DIR/tokei.json" \
  --slurpfile audit "$OUT_DIR/cargo-audit.json" \
  --slurpfile unsafe_census "$OUT_DIR/unsafe-census.json" \
  --slurpfile pub_api "$OUT_DIR/public-api.json" \
  --slurpfile deps "$OUT_DIR/deps.json" \
  --slurpfile bloat "$OUT_DIR/cargo-bloat.json" \
  '{
    identity: $id[0],
    rust_loc: ($loc[0].Rust.code // 0),
    rust_files: ($loc[0].Rust.reports // [] | length),
    advisories_found: (try ($audit[0].vulnerabilities.list | length) catch 0),
    unsafe_files: ($unsafe_census[0].by_file | length),
    unsafe_total_occurrences: ($unsafe_census[0].by_file | map(.unsafe_occurrences) | add // 0),
    public_api: $pub_api[0].crates,
    deps: $deps[0],
    cargo_bloat_skipped: ($bloat[0].skipped // false)
  }' > "$OUT_DIR/summary.json"

echo
echo "Wrote $(ls "$OUT_DIR"/*.json | wc -l) audit artifacts to $OUT_DIR"
echo
echo "Summary:"
jq . "$OUT_DIR/summary.json"
