# mathlib4 Upgrade Runbook

Quarterly procedure for bumping the mathlib4 pin in `lake-manifest.json`.

## When

- Every 90 days (calendar quarter start)
- When a `lean-toolchain` bump requires a matching mathlib version
- When a theorem proof breaks due to a mathlib API change and the fix requires the newer version

Do not upgrade opportunistically mid-phase. Upgrades go on a dedicated branch
and must pass the full soundness gate before merging.

## Steps

### 1. Create a branch

```bash
git checkout -b mathlib-upgrade/YYYY-MM
```

### 2. Dry-run the update

```bash
lake update
git diff lake-manifest.json
```

Review the diff. Key things to look for:
- New mathlib commit hash
- Any new transitive dependencies added or removed
- Version changes to Batteries, Std4, or other core packages

### 3. Build with the new manifest

```bash
nix develop --command lake build Apkaxiom
```

A cold build after a mathlib bump can take 30–60 minutes even with the
Reservoir cache because newly compiled `.olean` files must be fetched or
rebuilt. Allow the full wall-time budget.

If any theorem fails to elaborate:
- Check the mathlib4 changelog for API changes to lemmas we depend on
- Search `mathlib4` issues for the broken lemma name
- Either port the proof to the new API or pin to the previous manifest

### 4. TV regression

```bash
make p19-gates
```

The translation validator must remain green. Mathlib upgrades do not change
the Lean evaluator's output for our ZIP/signing theorems, but confirm anyway.

### 5. Sorry sweep

```bash
make soundness-sorry-audit
```

Confirm no sorry leaked in during the proof repair work.

### 6. Full soundness run

```bash
make soundness
```

All four gates must pass locally before opening the PR.

### 7. Open the PR

The PR diff should contain only `lake-manifest.json` (and any proof repairs).
No logic changes belong in a mathlib bump PR.

```bash
git add lake-manifest.json theorems/
git commit -m "bump mathlib4 to <new-tag>"
git push origin mathlib-upgrade/YYYY-MM
gh pr create \
  --title "mathlib4 bump: <old-tag> → <new-tag>" \
  --body "Quarterly mathlib upgrade. Soundness gate must pass before merge."
```

### 8. Merge

The `soundness` CI gate must be green. After merge, the next CI run will
populate the Reservoir cache so subsequent developers get fast builds.

## Rollback

If the bump causes a proof failure that cannot be repaired in the quarter:

```bash
git revert HEAD   # revert lake-manifest.json to previous pin
git push
```

Open an issue titled `mathlib4 <new-tag>: <lemma> broke — needs port`.
Assign to G1. Resume the upgrade next quarter or when the port is ready.

## Version history

| Quarter | Old tag | New tag | Theorems repaired |
|---|---|---|---|
| (first upgrade) | v4.29.1 | — | — |
