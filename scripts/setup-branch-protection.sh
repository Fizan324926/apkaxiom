#!/usr/bin/env bash
# Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
#
# setup-branch-protection.sh — apply APKAXIOM's branch-protection policy
# on the `main` branch via the GitHub API. Run-by-human; requires repo
# admin auth (`gh auth status` must show "admin" scope).
#
# What this configures (all required for PRs to merge into main):
#   - At least 1 review from a CODEOWNER on every changed path.
#   - Stale review dismissal on new commits.
#   - "Conversation resolution" required.
#   - Required status checks: build (linux-x86_64 / *), build (linux-aarch64 / *),
#     build (darwin-arm64 / *), cross-runner-determinism, lint, bazel-probe,
#     graph-parity, reindeer-idempotence, audit-toolchains-drift,
#     security-audit, license-check, sbom, attest.
#   - Strict "branch up to date with base before merging".
#   - Linear history (no merge commits in main).
#   - Force-push: disabled.
#   - Branch deletion: disabled.
#
# This is the policy half of the gate. The file CODEOWNERS is the other
# half. Run this script once per fresh repo, then re-run if any required-
# status-check name changes.

set -euo pipefail

OWNER="${OWNER:-Fizan324926}"
REPO="${REPO:-apkaxiom}"
BRANCH="${BRANCH:-main}"

if ! gh auth status --hostname github.com >/dev/null 2>&1; then
  echo "FAIL: gh CLI is not authenticated. Run \`gh auth login\` first." >&2
  exit 2
fi

REQUIRED_CHECKS=(
  "build (linux-x86_64 / runner 1)"
  "build (linux-x86_64 / runner 2)"
  "build (linux-aarch64 / runner 1)"
  "build (linux-aarch64 / runner 2)"
  "build (darwin-arm64 / runner 1)"
  "build (darwin-arm64 / runner 2)"
  "cross-runner determinism"
  "lint (cargo fmt + clippy)"
  "bazel sub-workspace probe"
  "graph-parity"
  "reindeer-idempotence"
  "audit-toolchains-drift"
  "determinism-lint"
  "security-audit"
  "license-check"
  "sbom"
  "attest (slsa l1 provenance)"
)

# JSON body. Fields named per
# https://docs.github.com/en/rest/branches/branch-protection
checks_json=$(printf '%s\n' "${REQUIRED_CHECKS[@]}" | jq -R . | jq -s '
  map({context: .})
')

body=$(jq -n \
  --argjson checks "$checks_json" \
  '{
    required_status_checks: {
      strict: true,
      checks: $checks
    },
    enforce_admins: true,
    required_pull_request_reviews: {
      required_approving_review_count: 1,
      dismiss_stale_reviews: true,
      require_code_owner_reviews: true,
      require_last_push_approval: true
    },
    restrictions: null,
    required_conversation_resolution: true,
    required_linear_history: true,
    allow_force_pushes: false,
    allow_deletions: false,
    block_creations: false,
    lock_branch: false,
    allow_fork_syncing: false
  }')

echo "=== Applying branch protection on $OWNER/$REPO@$BRANCH ==="
gh api -X PUT \
  -H "Accept: application/vnd.github+json" \
  "/repos/$OWNER/$REPO/branches/$BRANCH/protection" \
  --input - <<<"$body" \
  | jq '{name: .name // "ok", required_status_checks: .required_status_checks.checks | length, codeowner_reviews: .required_pull_request_reviews.require_code_owner_reviews}'

echo
echo "PASS: branch protection applied. Verify with:"
echo "  gh api /repos/$OWNER/$REPO/branches/$BRANCH/protection | jq ."
