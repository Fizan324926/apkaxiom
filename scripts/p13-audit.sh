#!/usr/bin/env bash
# Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
#
# p13-audit.sh — derive every committed P1.3 audit measurement from
# the upstream apk-info tree pinned at `external/apk-info-pinned-sha.txt`,
# plus the F-Droid corpus pulled by `scripts/p13-corpus.sh`. Outputs
# land in `docs/phase-1/P1.3/audit-data/*.json`. Idempotent: a second
# run produces equal JSON (modulo intentional non-determinism in
# hyperfine timings — see `audit-data/perf-summary.json` which records
# distribution shape rather than per-run absolute values).
#
# Pipeline:
#   1. Identity + SHA pin verification.
#   2. tokei                 → loc + per-language breakdown.
#   3. cargo-audit           → vulnerabilities + unmaintained warnings.
#   4. unsafe-census         → AST-based count via tools/unsafe-census.
#   5. public-API surface    → grep over `pub <kind>` (kept as in P1.3
#                              first pass; deepens to a syn-based
#                              scanner in P1.7 when the AXIOM-IR is real).
#   6. dep-tree summary      → resolved + direct deps.
#   7. cargo-bloat           → release-build size, per-crate. Uses
#                              rustup 1.89 to satisfy upstream's
#                              edition 2024 requirement.
#   8. perf + correctness    → hyperfine over the F-Droid corpus +
#                              parse-success rate per APK + per-error
#                              tally. (Skipped if the corpus is
#                              missing; run scripts/p13-corpus.sh first.)
#   9. summary roll-up       → one-file digest the CHECKLIST renders.
#  10. upstream-tree-sha     → recursive sha256 of the vendored tree
#                              (drift gate input; see ci.yml).

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

UPSTREAM="external/apk-info-upstream"
OUT_DIR="docs/phase-1/P1.3/audit-data"
SHA_FILE="external/apk-info-pinned-sha.txt"
CORPUS_DIR="${P13_APK_CORPUS:-/tmp/p13-apk-corpus}"

if [[ ! -d "$UPSTREAM" ]]; then
  echo "FAIL: $UPSTREAM not present. Run \`git clone https://github.com/delvinru/apk-info $UPSTREAM\` first." >&2
  exit 2
fi

PINNED_SHA="$(cat "$SHA_FILE")"
mkdir -p "$OUT_DIR"

# ---------------------------------------------------------------------------
# 1) Identity + recursive sha256 of the vendored upstream tree.
# ---------------------------------------------------------------------------
echo "=== upstream identity ==="
{
  printf '{\n'
  printf '  "schema": "apkaxiom.p13-audit/v1",\n'
  printf '  "upstream_repo": "https://github.com/delvinru/apk-info",\n'
  printf '  "pinned_sha": "%s"\n' "$PINNED_SHA"
  printf '}\n'
} > "$OUT_DIR/identity.json"

# Recursive sha256 of every file under $UPSTREAM (sorted, with relative
# paths). Committed to the repo; CI's `apk-info-upstream-pin` job
# diffs the reproduced value against the committed one.
( cd "$UPSTREAM" && find . -type f -not -path './target/*' -not -path './.lake/*' -not -path './.git/*' -print0 \
    | sort -z | xargs -0 sha256sum ) | sha256sum | awk '{print $1}' \
  > "$OUT_DIR/upstream-tree-sha256.txt"

# ---------------------------------------------------------------------------
# 2) tokei — LOC + per-language breakdown.
# ---------------------------------------------------------------------------
echo "=== tokei (LOC) ==="
# tokei's Markdown blob-detection is non-deterministic across runs
# (the order of `.children.Bash` / `.children.Python` etc. varies).
# We project to a deterministic subset: per-language code/comments/blanks
# + sorted file list. The subset is sufficient for the audit (we only
# read .Rust.* in the CHECKLIST).
( cd "$UPSTREAM" && tokei --output json . ) | jq '
  to_entries
  | map(
      {
        (.key): {
          code: .value.code,
          comments: .value.comments,
          blanks: .value.blanks,
          inaccurate: .value.inaccurate,
          reports: ((.value.reports // [])
            | map({name, stats: {code: .stats.code, comments: .stats.comments, blanks: .stats.blanks}})
            | sort_by(.name))
        }
      }
    )
  | add
' > "$OUT_DIR/tokei.json"

# ---------------------------------------------------------------------------
# 3) cargo-audit on the upstream Cargo.lock. Captures BOTH
#    .vulnerabilities.list (CVE-class) AND .warnings.* (unmaintained,
#    yanked, notice). The summary roll-up below counts both.
# ---------------------------------------------------------------------------
echo "=== cargo-audit (upstream Cargo.lock) ==="
cargo audit --json --file "$UPSTREAM/Cargo.lock" > "$OUT_DIR/cargo-audit.json" || true

# ---------------------------------------------------------------------------
# 4) AST-based `unsafe` census via tools/unsafe-census.
# ---------------------------------------------------------------------------
echo "=== unsafe census (syn AST) ==="
buck2 build //tools/unsafe-census:unsafe-census >&2
buck2 run //tools/unsafe-census -- "$UPSTREAM" 2>/dev/null \
  > "$OUT_DIR/unsafe-census.json"

# ---------------------------------------------------------------------------
# 5) Public-API surface — grep approximation. Replaced by a syn scan
#    in P1.7 when AXIOM-IR is real and the spec calls for a frozen
#    surface diff.
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
# 6) Dep-tree summary.
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
# 7) cargo-bloat — release build size, top-50 functions per crate.
#    Upstream is edition 2024 + resolver 3, requires rustc ≥ 1.85.
#    Our pinned rustc is 1.83 (P1.1-stable). Use rustup's 1.89 if
#    present; fall back to documenting the skip.
# ---------------------------------------------------------------------------
echo "=== cargo-bloat (release size) ==="
# Upstream uses edition 2024 + resolver 3, requires rustc ≥1.85.
# Our pinned rustc is 1.83 (P1.1-stable). Run cargo-bloat outside the
# nix dev-shell — invoke `nix develop --command true` then host-side
# rustup. We do this by exec'ing through `env -i` so the nix
# RUSTC/CARGO overrides don't leak.
RUSTUP_BIN=$(command -v rustup || echo "")
if [[ -n "$RUSTUP_BIN" ]] && rustup toolchain list 2>/dev/null | grep -q '^1\.89\.0-x86_64-unknown-linux-gnu'; then
  # Use a clean env to avoid nix's rustc-shim hijacking the cargo
  # build. PATH must include rustup + system tools.
  CARGO_ENV=( env -i HOME="$HOME" PATH="/root/.cargo/bin:/usr/bin:/bin" )
  if ( cd "$UPSTREAM" && "${CARGO_ENV[@]}" "$RUSTUP_BIN" run 1.89.0 cargo build --release 2>/dev/null >/dev/null ); then
    ( cd "$UPSTREAM" && "${CARGO_ENV[@]}" "$RUSTUP_BIN" run 1.89.0 cargo bloat --release -n 50 --message-format json 2>/dev/null ) \
      > "$OUT_DIR/cargo-bloat.json" || {
      printf '{\n  "skipped": true,\n  "reason": "cargo-bloat command failed"\n}\n' \
        > "$OUT_DIR/cargo-bloat.json"
    }
  else
    printf '{\n  "skipped": true,\n  "reason": "rustup 1.89 build failed"\n}\n' \
      > "$OUT_DIR/cargo-bloat.json"
  fi
else
  printf '{\n  "skipped": true,\n  "reason": "no host rustup with 1.89.0 toolchain found; upstream uses edition 2024 which requires rustc ≥1.85"\n}\n' \
    > "$OUT_DIR/cargo-bloat.json"
fi

# ---------------------------------------------------------------------------
# 8) Performance + correctness over the F-Droid corpus.
#    `scripts/p13-corpus.sh` populates $CORPUS_DIR/*.apk; we honour
#    whatever is there. Output:
#      perf-{show,axml}.json  — hyperfine raw output
#      perf-summary.json      — distribution shape (min/p50/p95/p99/max)
#                                + per-APK parse success/failure
#      correctness.json       — per-APK parse outcome + exit code +
#                                stderr fingerprint (first 200 chars)
# ---------------------------------------------------------------------------
echo "=== perf + correctness (F-Droid corpus) ==="
APK_BIN="$UPSTREAM/target/release/apk-info"
if [[ ! -x "$APK_BIN" && -n "$RUSTUP_BIN" ]]; then
  ( cd "$UPSTREAM" && env -i HOME="$HOME" PATH="/root/.cargo/bin:/usr/bin:/bin" "$RUSTUP_BIN" run 1.89.0 cargo build --release -p apk-info-cli 2>/dev/null >/dev/null ) || true
fi

# Portable test for "directory exists and contains at least one .apk".
HAS_CORPUS=0
if [[ -x "$APK_BIN" && -d "$CORPUS_DIR" ]]; then
  if find "$CORPUS_DIR" -maxdepth 1 -name '*.apk' -print -quit 2>/dev/null | grep -q .; then
    HAS_CORPUS=1
  fi
fi
if [[ "$HAS_CORPUS" -eq 1 ]]; then
  CORPUS_SIZE=$(find "$CORPUS_DIR" -maxdepth 1 -name '*.apk' | wc -l)
  echo "    corpus: $CORPUS_SIZE APKs in $CORPUS_DIR"

  # 8a) Per-APK correctness probe — runs `show` once per APK, captures
  # exit code + first 200 chars of stderr. Sampled, not exhaustive,
  # because hyperfine repeats the run for timing.
  {
    printf '{\n  "schema": "apkaxiom.p13-correctness/v1",\n'
    printf '  "tool": "apk-info show",\n'
    printf '  "corpus_size": %d,\n' "$CORPUS_SIZE"
    printf '  "results": [\n'
    first=1
    pass_count=0
    fail_count=0
    declare -A err_kinds
    for apk in "$CORPUS_DIR"/*.apk; do
      [[ -f "$apk" ]] || continue
      name=$(basename "$apk")
      stderr_capture=$( "$APK_BIN" show "$apk" 2>&1 >/dev/null || true )
      rc=$?
      if [[ "$rc" -eq 0 ]]; then
        pass_count=$((pass_count + 1))
        outcome=pass
      else
        fail_count=$((fail_count + 1))
        outcome=fail
        # Bucket the error: first 60 chars of the first stderr line.
        kind=$(printf '%s' "$stderr_capture" | head -1 | cut -c1-60)
        err_kinds[$kind]=$(( ${err_kinds[$kind]:-0} + 1 ))
      fi
      stderr_brief=$(printf '%s' "$stderr_capture" | head -c 200 | tr '\n' ' ' | sed 's/"/\\"/g')
      [[ $first -eq 1 ]] && first=0 || printf ',\n'
      printf '    {"apk":"%s","outcome":"%s","exit_code":%d,"stderr_brief":"%s"}' \
        "$name" "$outcome" "$rc" "$stderr_brief"
    done
    printf '\n  ],\n'
    printf '  "totals": {"pass": %d, "fail": %d},\n' "$pass_count" "$fail_count"
    printf '  "error_buckets": ['
    bk_first=1
    for k in "${!err_kinds[@]}"; do
      [[ $bk_first -eq 1 ]] && bk_first=0 || printf ','
      esc=$(printf '%s' "$k" | sed 's/"/\\"/g')
      printf '\n    {"kind":"%s","count":%d}' "$esc" "${err_kinds[$k]}"
    done
    printf '\n  ]\n}\n'
  } > "$OUT_DIR/correctness.json"

  # 8b) hyperfine — 5 runs × 1 warm-up per APK on `show` and `axml`.
  # On a 100-APK corpus this takes ~3-5 minutes; the script is fine
  # to run unattended.
  echo "    perf: apk-info show ($CORPUS_SIZE APKs)"
  hyperfine_args=(--warmup 1 --runs 5 --ignore-failure --export-json /tmp/p13-perf-show.json)
  for apk in "$CORPUS_DIR"/*.apk; do
    [[ -f "$apk" ]] || continue
    hyperfine_args+=("$APK_BIN show '$apk'")
  done
  hyperfine "${hyperfine_args[@]}" >/dev/null 2>&1 || true
  cp /tmp/p13-perf-show.json "$OUT_DIR/perf-show.json" 2>/dev/null || \
    printf '{"skipped": true, "reason": "hyperfine show failed"}\n' > "$OUT_DIR/perf-show.json"

  echo "    perf: apk-info axml ($CORPUS_SIZE APKs)"
  hyperfine_args=(--warmup 1 --runs 5 --ignore-failure --export-json /tmp/p13-perf-axml.json)
  for apk in "$CORPUS_DIR"/*.apk; do
    [[ -f "$apk" ]] || continue
    hyperfine_args+=("$APK_BIN axml '$apk'")
  done
  hyperfine "${hyperfine_args[@]}" >/dev/null 2>&1 || true
  cp /tmp/p13-perf-axml.json "$OUT_DIR/perf-axml.json" 2>/dev/null || \
    printf '{"skipped": true, "reason": "hyperfine axml failed"}\n' > "$OUT_DIR/perf-axml.json"

  # 8c) Distribution-shape summary — committed and stable across runs
  # in an order-of-magnitude sense; absolute values shift with host
  # noise so we record p50/p95/p99/max in milliseconds, sorted.
  for sub in show axml; do
    src="$OUT_DIR/perf-$sub.json"
    [[ -f "$src" ]] || continue
    jq --arg sub "$sub" '
      (.results // []) as $rs |
      ($rs | map(.median * 1000) | sort) as $sorted |
      ($sorted | length) as $n |
      def percentile(p):
        if $n == 0 then 0
        else $sorted[ ([(($n-1)*p) | floor, $n-1] | min) ]
        end;
      {
        subcommand: $sub,
        n: $n,
        median_ms: percentile(0.5),
        p95_ms: percentile(0.95),
        p99_ms: percentile(0.99),
        max_ms: ($sorted[-1] // 0)
      }
    ' "$src"
  done | jq -s '{schema: "apkaxiom.p13-perf-summary/v1", per_subcommand: .}' \
    > "$OUT_DIR/perf-summary.json"
else
  echo "    SKIPPED — corpus or binary missing"
  printf '{"skipped": true, "reason": "no corpus at %s, or apk-info-cli binary not built"}\n' \
    "$CORPUS_DIR" > "$OUT_DIR/correctness.json"
  printf '{"skipped": true, "reason": "no corpus"}\n' > "$OUT_DIR/perf-show.json"
  printf '{"skipped": true, "reason": "no corpus"}\n' > "$OUT_DIR/perf-axml.json"
  printf '{"skipped": true, "reason": "no corpus"}\n' > "$OUT_DIR/perf-summary.json"
fi

# ---------------------------------------------------------------------------
# 9) Summary roll-up.
# ---------------------------------------------------------------------------
echo "=== summary ==="
jq -n \
  --slurpfile id "$OUT_DIR/identity.json" \
  --slurpfile loc "$OUT_DIR/tokei.json" \
  --slurpfile audit "$OUT_DIR/cargo-audit.json" \
  --slurpfile uns "$OUT_DIR/unsafe-census.json" \
  --slurpfile pub_api "$OUT_DIR/public-api.json" \
  --slurpfile deps "$OUT_DIR/deps.json" \
  --slurpfile bloat "$OUT_DIR/cargo-bloat.json" \
  --slurpfile correctness "$OUT_DIR/correctness.json" \
  --slurpfile perf_sum "$OUT_DIR/perf-summary.json" \
  '{
    identity: $id[0],
    rust_loc: ($loc[0].Rust.code // 0),
    rust_files: ($loc[0].Rust.reports // [] | length),
    cargo_audit: {
      vulnerabilities: ($audit[0].vulnerabilities.list // [] | length),
      unmaintained:    ($audit[0].warnings.unmaintained // [] | length),
      yanked:          ($audit[0].warnings.yanked // [] | length),
      notices:         ($audit[0].warnings.notice // [] | length),
      total_advisories_and_warnings:
        ((($audit[0].vulnerabilities.list // []) | length)
       + (($audit[0].warnings.unmaintained // []) | length)
       + (($audit[0].warnings.yanked // []) | length)
       + (($audit[0].warnings.notice // []) | length))
    },
    unsafe_ast: $uns[0].totals,
    unsafe_ast_files: ($uns[0].by_file | length),
    unsafe_ast_parse_failures: ($uns[0].parse_failures | length),
    public_api: $pub_api[0].crates,
    deps: $deps[0],
    cargo_bloat_skipped: ($bloat[0].skipped // false),
    correctness: ($correctness[0] | (if .skipped then null else {pass: .totals.pass, fail: .totals.fail, error_buckets: .error_buckets} end)),
    perf: ($perf_sum[0] | (if .skipped then null else .per_subcommand end))
  }' > "$OUT_DIR/summary.json"

echo
echo "Wrote $(ls "$OUT_DIR"/*.json "$OUT_DIR"/upstream-tree-sha256.txt 2>/dev/null | wc -l) audit artifacts to $OUT_DIR"
echo
echo "Summary:"
jq . "$OUT_DIR/summary.json"
