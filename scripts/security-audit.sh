#!/usr/bin/env bash
# Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
#
# security-audit.sh — cargo-audit against the workspace + Reindeer
# manifest. Per ADR-0008 (supply-chain).
#
# `cargo audit` checks against the RustSec advisory database. We run it
# twice: once on the workspace `Cargo.lock`, once on the Reindeer manifest
# `third-party/rust/Cargo.lock`. Both must come back clean.
#
# Output:
#   target/security-audit.workspace.json
#   target/security-audit.third-party.json

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

mkdir -p target

run_audit() {
  local label="$1" lock="$2" out="$3"
  echo "=== cargo-audit on $label ($lock) ==="
  if cargo audit --json --file "$lock" > "$out" 2>&1; then
    local n
    n=$(jq -r '.vulnerabilities.found // false' "$out")
    if [[ "$n" == "false" ]]; then
      echo "  PASS: no advisories"
      return 0
    fi
    echo "  FAIL: vulnerabilities found in $label"
    jq -r '.vulnerabilities.list[] | "    - \(.advisory.id) \(.advisory.title) (\(.package.name) \(.package.version))"' "$out"
    return 1
  else
    # cargo-audit exits non-zero when issues are present; still capture JSON.
    echo "  FAIL: cargo-audit returned non-zero"
    jq -r '.vulnerabilities.list[]?
      | "    - \(.advisory.id) \(.advisory.title) (\(.package.name) \(.package.version))"' \
      "$out" 2>/dev/null || true
    return 1
  fi
}

rc=0
run_audit "workspace" "Cargo.lock" "target/security-audit.workspace.json" || rc=1
run_audit "third-party" "third-party/rust/Cargo.lock" \
  "target/security-audit.third-party.json" || rc=1

if [[ $rc -eq 0 ]]; then
  echo
  echo "PASS: no advisories in either lockfile."
fi
exit "$rc"
