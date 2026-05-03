#!/usr/bin/env bash
# Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
#
# audit-toolchains.sh — snapshot the active Buck2 toolchain graph + cell
# config + prelude provenance to a committed file. CI gates on the diff:
# any unintended drift in the toolchain graph trips the audit-toolchains
# job before a merge can land. Per ADR-0007.
#
# Outputs:
#   docs/phase-1/P1.1/audit-toolchains.txt     — text dump (committed)
#   docs/phase-1/P1.1/audit-toolchains.json    — machine-readable (committed)

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

OUT_DIR="docs/phase-1/P1.1"
TXT="$OUT_DIR/audit-toolchains.txt"
JSON="$OUT_DIR/audit-toolchains.json"

mkdir -p "$OUT_DIR"

# `buck2 audit` is non-deterministic about ordering; we sort everywhere we
# can to make the snapshot diff-stable.

{
  echo "# APKAXIOM Buck2 toolchain snapshot"
  echo "# Regenerate: make audit-toolchains"
  echo "# Drift in this file means the toolchain graph changed; review the"
  echo "# diff against the committed copy and either accept (with ADR) or"
  echo "# revert."
  echo
  echo "## Buck2 binary"
  echo "  version: $(buck2 --version | head -1)"
  echo
  echo "## Cells"
  (buck2 audit cell 2>/dev/null || true) | sort
  echo
  echo "## Configuration"
  (buck2 audit config --all-cells 2>/dev/null || true) | sort
  echo
  echo "## Toolchains registered under //toolchains:"
  for t in cxx genrule python_bootstrap rust remote_test_execution; do
    echo "### //toolchains:$t"
    out=$(buck2 cquery "//toolchains:$t" 2>/dev/null | sort | sed 's/^/  /' || true)
    if [[ -z "$out" ]]; then
      # Toolchain rules sometimes need a configuration to resolve; fall
      # back to an unconfigured-target query so the snapshot is still a
      # signal even when cquery declines.
      out=$(buck2 uquery "//toolchains:$t" 2>/dev/null | sort | sed 's/^/  /' || true)
    fi
    [[ -n "$out" ]] && echo "$out"
  done
  echo
  echo "## Resolved //:all dependency closure"
  (buck2 cquery "deps(//:all)" 2>/dev/null || true) | sort
  echo
  echo "## Workspace target list"
  (buck2 targets //... 2>/dev/null || true) | sort
} > "$TXT"

# JSON form: each toolchain's provider attributes.
{
  echo "{"
  echo "  \"buck2_version\": \"$(buck2 --version | head -1 | sed 's/"/\\"/g')\","
  echo "  \"cells\": $(buck2 audit cell 2>/dev/null | sort | jq -R . | jq -s .),"
  echo "  \"deps_all\": $(buck2 cquery 'deps(//:all)' 2>/dev/null | sort | jq -R . | jq -s .),"
  echo "  \"targets_all\": $(buck2 targets //... 2>/dev/null | sort | jq -R . | jq -s .)"
  echo "}"
} | jq . > "$JSON" 2>/dev/null || {
  # If jq fails (e.g. due to control chars), fall back to concatenated JSON
  # Lines so the file is at least committable.
  echo "FAIL: jq normalisation failed; emitting raw outputs" >&2
  cat > "$JSON" <<EOF
{"buck2_version": "$(buck2 --version | head -1)", "note": "raw fallback; jq failed"}
EOF
}

echo "Wrote: $TXT" >&2
echo "Wrote: $JSON" >&2
