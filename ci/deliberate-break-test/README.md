# Deliberate-Break Test Runbook

## Purpose

Confirm the soundness gate is real and fail-closed, not theatrical. Run this
before every phase release and after any Lean toolchain or mathlib upgrade.

## When to run

- Before cutting a phase-boundary release
- After any `lean-toolchain` version bump
- After any `lake-manifest.json` mathlib upgrade
- Quarterly (first week of each quarter)

## Steps

```bash
# 1. Create a sandbox branch
git checkout -b sandbox/break-test

# 2. Inject a sorry into a theorem
sed -i 's/\bby rfl\b/by sorry/' theorems/Apkaxiom/Zip/LocalHeader.lean

# 3. Commit and push
git commit -am "deliberate break: inject sorry into LocalHeader.lean"
git push origin sandbox/break-test

# 4. Open a PR targeting main
gh pr create \
  --base main \
  --head sandbox/break-test \
  --title "[TEST] Deliberate soundness break" \
  --body "Automated test confirming the soundness gate is fail-closed. Do not merge."

# 5. Observe: soundness.yml gate fails at the sorry-audit step.
#    The PR shows a red check and is not mergeable.

# 6. Clean up
git checkout main
git branch -D sandbox/break-test
git push origin --delete sandbox/break-test
gh pr close --delete-branch $(gh pr list --head sandbox/break-test --json number -q '.[0].number')
```

## Expected output

The `HARD gate — sorry audit` step in `soundness.yml` must fail with output
similar to:

```
FAIL sorry-audit: bare sorry found in theorems/
theorems/Apkaxiom/Zip/LocalHeader.lean:42:  by sorry
```

The PR status check must be red and GitHub must block the merge button.

## What to do if the gate does NOT fail

If the deliberate break passes (the gate stays green):

1. Open an incident — the soundness gate is broken.
2. Check that `soundness.yml` path filters include `theorems/**`.
3. Verify the `sorry-audit` step runs `bash ci/soundness/run.sh sorry-audit`
   and that the script exits 1 on a hit.
4. Do not merge any PRs touching theorems until the gate is confirmed working.

## Quarterly log

Record each test run here with date and outcome:

| Date | Branch | Outcome | Operator |
|---|---|---|---|
| (first run) | sandbox/break-test | — | — |
