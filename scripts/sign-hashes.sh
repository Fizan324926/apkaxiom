#!/usr/bin/env bash
# Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
#
# sign-hashes.sh — sign every committed reproducibility-hashes file with
# cosign keyless (Sigstore Fulcio + Rekor). Per ADR-0008.
#
# Two modes:
#   1. CI mode (default when COSIGN_EXPERIMENTAL=1 + GITHUB_ACTIONS=true):
#      uses the GitHub OIDC token to fetch a short-lived Fulcio cert and
#      logs the signature to the public Rekor transparency log. Requires
#      `id-token: write` permission on the workflow.
#   2. Local mode (interactive): cosign opens a browser for OIDC; useful
#      for personal-laptop "I want to attest a build" workflows.
#
# Outputs (alongside each `reproducibility-hashes.<plat>.txt`):
#   reproducibility-hashes.<plat>.txt.sig   — DSSE signature
#   reproducibility-hashes.<plat>.txt.cert  — Fulcio cert (PEM)
#   reproducibility-hashes.<plat>.txt.bundle — Sigstore bundle (offline-verifiable)
#
# Exits 0 if every reference file got signed; 1 if cosign failed; 2 if there
# were no reference files to sign.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

REF_DIR="docs/phase-1/P1.1"
shopt -s nullglob
files=("$REF_DIR"/reproducibility-hashes.*.txt)

if [[ ${#files[@]} -eq 0 ]]; then
  echo "FAIL: no reference-hash files at $REF_DIR/reproducibility-hashes.*.txt" >&2
  exit 2
fi

export COSIGN_EXPERIMENTAL=1

for f in "${files[@]}"; do
  echo "=== signing $f ==="
  cosign sign-blob --yes \
    --bundle "${f}.bundle" \
    --output-signature "${f}.sig" \
    --output-certificate "${f}.cert" \
    "$f"
done

echo
echo "Signed ${#files[@]} reference file(s):"
for f in "${files[@]}"; do
  echo "  $f"
done
echo
echo "Verify with:"
echo "  cosign verify-blob --bundle <file>.bundle \\"
echo "    --certificate-identity-regexp '.*' \\"
echo "    --certificate-oidc-issuer-regexp '.*' \\"
echo "    <file>"
