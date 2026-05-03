# Contributing to APKAXIOM

APKAXIOM is a 3-year nation-grade research project building bit-for-bit
reproducible, formally-verified Android-app analysis. The contribution
bar is high; this document explains the policies that protect it.

> If you are looking for the everyday "how do I build it" run-book, see
> [`docs/phase-1/P1.1/build-and-run.md`](docs/phase-1/P1.1/build-and-run.md).
> This file covers the *contribution* process, not the build process.

---

## 1. Code of Conduct

Be respectful, be specific, be patient. We expect collaborators to focus
on substance over signalling and to assume good faith. Severe or repeated
violations are a steward's call to escalate; absent a steward, ask the
project lead.

## 2. Sign-off — Developer Certificate of Origin

Every commit MUST carry a `Signed-off-by:` trailer asserting the
[Developer Certificate of Origin v1.1](https://developercertificate.org/).
This is the lightweight DCO model; we do not require a separate CLA.

```bash
git commit -s -m "your commit message"
```

Branch protection enforces sign-off on `main`. PRs without sign-off cannot merge.

## 3. ADR change policy

Architectural decisions live in `docs/phase-<N>/P<N>.<M>/ADR-*.md`. Once an ADR
is **Accepted**, the underlying decision is load-bearing. To change:

1. Open a PR titled `ADR-XXXX: <one-line summary of the change>`.
2. The PR adds a new ADR (or supersedes an old one with `Status: Superseded by ADR-YYYY`).
3. The relevant CODEOWNERS group reviews. For Phase-1 hermetic-build assets that is **G13 — Platform Infrastructure**.
4. Merging requires the same gates as any code change (CI green + CODEOWNER approval).

We never edit Accepted ADRs in place except to add a *Provisioning history* row, fix a typo, or mark the document Superseded.

## 4. Reproducibility contract

Every commit on `main` MUST satisfy:

- `nix develop --command make build` succeeds.
- `nix develop --command make test` succeeds.
- `nix develop --command make repro-check` succeeds (two clean builds, byte-identical artifacts).
- `nix develop --command make verify-hashes` succeeds against `docs/phase-1/P1.1/reproducibility-hashes.<platform>.txt`.
- `nix develop --command make graph-parity` succeeds.
- `nix develop --command make reindeer-check` succeeds.
- `nix develop --command make security-audit` clean.
- `nix develop --command make license-check` clean.
- `nix flake check` clean (toolchain-probe + shellcheck + lockfile-freshness).

CI enforces the full set on every PR. None of these gates is optional.

If a change legitimately requires bumping the reference hashes (e.g. a
toolchain upgrade), the PR description MUST include:

- The reason (toolchain bump, intentional build-graph change).
- A pointer to the corresponding ADR (e.g. ADR-0004 for a flake.lock bump).
- Newly-baked `reproducibility-hashes.<platform>.txt` for **every** CI platform.

## 5. Branches and history

- `main` is the only long-lived branch. Feature work happens in PR-scoped branches off main.
- Linear history is enforced: no merge commits land on `main`. Use `gh pr merge --squash` (default) or `--rebase`.
- Force-push to `main`: disabled. Stale-review dismissal: enabled.

## 6. Commit messages

We follow a deliberately-light convention:

```
<sub-phase tag>: <one-line summary, ≤72 chars>

<body — what changed and why; references to ADRs welcome>

Signed-off-by: Name <email>
```

The sub-phase tag is `P<N>.<M>` (e.g. `P1.1`) for code that lives in that
sub-phase, or omitted for cross-cutting changes. Do not embed AI-tool
attribution strings in commit messages, ADRs, or source files; trace
provenance via git history and CODEOWNERS, not via vendor names.

## 7. Authoring new code

When adding a new first-party crate:

1. Create `crates/<name>/` with `Cargo.toml`, `BUCK`, `src/lib.rs`.
2. Add it to the workspace `Cargo.toml` `members` list.
3. Add it to `BUCK`'s `:axiom` filegroup so `make build` covers it.
4. Add it to the `FIRST_PARTY_CRATES` list in `scripts/_hash-artifacts.sh`.
5. Add at least one `rust_test` target with a non-trivial deterministic test.
6. Run `make repro-check` locally and commit a fresh hash snapshot.
7. Update CODEOWNERS with the responsible group.

When adding a third-party Rust dep:

1. Edit the workspace `Cargo.toml`'s `[workspace.dependencies]` table.
2. Run `make third-party-update`.
3. Run `make graph-parity` (must pass).
4. If the dep has a build script, add a `third-party/rust/fixups/<crate>/fixups.toml`.
5. Add the crate name to `VENDORED_CRATES` in `scripts/_hash-artifacts.sh` per ADR-0007.
6. Re-bake reference hashes per item (4) above.

## 8. Reporting security issues

For vulnerabilities, do **not** open a public issue. Email the project
lead (the address in `MAINTAINERS.md`) with a clear PoC and reproduction
steps. We respond within 72 hours with an acknowledgement and a planned
disclosure timeline.

For non-vulnerability bugs, open a normal GitHub issue.

---

By submitting a contribution to APKAXIOM, you affirm the DCO sign-off
above and that your contribution is licensed under Apache-2.0 OR MIT
(matching the repo licence).
