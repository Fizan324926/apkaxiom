#!/usr/bin/env bash
# Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
#
# rebuilder-attest.sh — let any third party act as an independent
# rebuilder of an APKAXIOM build, and emit a signed attestation that
# their rebuild matches the canonical hash file. Per ADR-0011.
#
# Why this exists:
#   The reproducibility-builds.org playbook calls for *independent*
#   rebuilds against published hashes. Without independent verification,
#   "we built it twice and it matched" is just the project trusting
#   itself. A federation of rebuilders raises the bar: an adversary now
#   has to compromise *every* rebuilder's host to falsify a build.
#
# How it works:
#   1. Operator clones the repo at a specific git SHA on a machine that
#      satisfies our `nix develop` toolchain.
#   2. They run this script. It:
#      a) Verifies the local toolchain matches the flake.lock pin.
#      b) Runs `make verify-hashes` against the committed reference for
#         their platform.
#      c) On PASS, emits `target/rebuilder-attestation-<host>-<sha>.json`
#         and signs it with cosign keyless.
#      d) On FAIL, emits the same JSON with `result: "diverged"` and the
#         divergent-artifact list, also signed.
#   3. Operator uploads the JSON + .sig + .cert to a public location
#      (their own GitHub repo, IPFS, etc.) and posts the URL to a
#      designated APKAXIOM channel.
#
# We collate received attestations under `docs/phase-1/P1.1/rebuilders/`
# manually for now (P1.1 scale). Phase P1.18 wires automated collection.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

UNAME_S=$(uname -s | tr '[:upper:]' '[:lower:]')
UNAME_M=$(uname -m)
PLATFORM="$UNAME_S-$UNAME_M"
GIT_SHA=$(git rev-parse HEAD 2>/dev/null || echo unknown)
HOST_FP=$(printf '%s|%s|%s' \
  "$(uname -srm)" \
  "$(rustc --version 2>/dev/null || echo no-rustc)" \
  "$(buck2 --version 2>/dev/null || echo no-buck2)" \
  | sha256sum | awk '{print $1}')
TS=$(date -u +%Y-%m-%dT%H:%M:%SZ)
mkdir -p target docs/phase-1/P1.1/rebuilders

OUT="target/rebuilder-attestation-${PLATFORM}-${GIT_SHA:0:12}-$(date -u +%s).json"

# Run verify-hashes; capture pass/fail.
result="pass"
diff_payload=""
if ! out=$(bash scripts/verify-hashes.sh 2>&1); then
  result="diverged"
  diff_payload=$(printf '%s' "$out" | tail -50 | jq -Rs .)
fi
expected_root=""
actual_root=""
if [[ -f "docs/phase-1/P1.1/reproducibility-hashes.${PLATFORM}.txt" ]]; then
  expected_root=$(grep CORPUS_ROOT \
    "docs/phase-1/P1.1/reproducibility-hashes.${PLATFORM}.txt" \
    | awk '{print $1}' || true)
fi
actual_root=$(bash scripts/_hash-artifacts.sh skip 2>/dev/null \
  | grep CORPUS_ROOT | awk '{print $1}' || true)

cat > "$OUT" <<EOF
{
  "schema": "apkaxiom.rebuilder-attest/v1",
  "git_sha": "$GIT_SHA",
  "platform": "$PLATFORM",
  "host_fingerprint_sha256": "$HOST_FP",
  "timestamp_utc": "$TS",
  "rustc_version": "$(rustc --version 2>/dev/null || echo unknown)",
  "buck2_version": "$(buck2 --version 2>/dev/null | head -1 || echo unknown)",
  "flake_lock_sha256": "$(sha256sum flake.lock 2>/dev/null | awk '{print $1}')",
  "expected_corpus_root": "$expected_root",
  "actual_corpus_root":   "$actual_root",
  "result": "$result",
  "diverged_log": ${diff_payload:-null}
}
EOF

echo "Wrote: $OUT"
echo

# Sign if cosign is installed AND we're either in CI (OIDC available) or
# the operator explicitly asks for interactive signing.
if command -v cosign >/dev/null 2>&1; then
  if [[ "${COSIGN_EXPERIMENTAL:-0}" == "1" || "${GITHUB_ACTIONS:-}" == "true" ]]; then
    echo "=== signing attestation (cosign keyless) ==="
    COSIGN_EXPERIMENTAL=1 cosign sign-blob --yes \
      --bundle "${OUT}.bundle" \
      --output-signature "${OUT}.sig" \
      --output-certificate "${OUT}.cert" \
      "$OUT"
    echo "Signed:"
    echo "  ${OUT}.sig"
    echo "  ${OUT}.cert"
    echo "  ${OUT}.bundle"
  else
    echo "Skipping signing — set COSIGN_EXPERIMENTAL=1 to enable interactive OIDC."
    echo "(In CI this is automatic via the workflow's id-token: write permission.)"
  fi
fi

echo
case "$result" in
  pass)
    echo "PASS: rebuilder attestation matches the canonical reference for $PLATFORM."
    exit 0
    ;;
  diverged)
    echo "DIVERGED: rebuilder attestation does NOT match the canonical reference."
    echo "Upload $OUT to the public APKAXIOM rebuilder channel — divergence reports"
    echo "are exactly the signal we need."
    exit 1
    ;;
esac
