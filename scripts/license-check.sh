#!/usr/bin/env bash
# Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
#
# license-check.sh — cargo-deny against the workspace. Enforces the
# license-allowlist, dependency-source allowlist, and ban-list defined in
# `deny.toml`. Per ADR-0008 (supply-chain).
#
# Categories:
#   - licenses       — every dep must carry a license in the allowlist
#   - bans           — explicitly banned crates / features / version ranges
#   - sources        — only crates.io and the vendored tree are allowed
#   - advisories     — overlap with cargo-audit; cheap to keep on
#
# The first failing category exits the script non-zero; all categories
# are still reported in the JSON output.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

mkdir -p target

if [[ ! -f deny.toml ]]; then
  echo "FAIL: deny.toml is missing — see ADR-0008 for the policy template" >&2
  exit 2
fi

echo "=== cargo-deny check (all) ==="
if cargo deny --format json check 2>&1 | tee target/license-check.jsonl; then
  echo "PASS: cargo-deny clean."
  exit 0
fi

echo "FAIL: cargo-deny reported issues. See target/license-check.jsonl." >&2
exit 1
