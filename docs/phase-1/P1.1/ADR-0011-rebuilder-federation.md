# ADR-0011 — Federated rebuilder attestation

**Status:** Accepted (P1.1)
**Date:** 2026-05-03
**Owner:** G13 — Platform Infrastructure
**Related:** ADR-0002, ADR-0004, ADR-0007 (hash corpus), ADR-0008 (signing)

---

## Context

"We built it twice on our CI and it matched" is the project trusting itself. State-of-the-art reproducible-build practice (Debian, NixOS, Tor) augments self-checks with an **independent rebuilder federation**: a set of unrelated parties who each rebuild the project from source and publish their result hashes. Attackers now have to compromise *every* rebuilder's host to falsify a build.

The reproducibility-builds.org playbook calls this "external verification" and notes that *anyone* should be able to act as a rebuilder. APKAXIOM is open source; this is achievable at zero infra cost.

Two semi-active codebases dominate the space:

- **rebuilderd** (NixOS / Tweag) — most active. Designed for distro-scale; heavy.
- **Trustix** — Tweag-developed; has been semi-dormant since 2023. Spec is sound; tooling is dated.

Neither is a perfect off-the-shelf fit for a single-project federation at our scale. We adopt a **lightweight in-house attestation format** that any third party can produce with one command, plus a curation process for received attestations. We can graduate to rebuilderd in P1.18 if the federation grows enough to justify the operational footprint.

## Decision

### Attestation format

A signed JSON object (CycloneDX-adjacent but bespoke) emitted by `scripts/rebuilder-attest.sh`:

```json
{
  "schema": "apkaxiom.rebuilder-attest/v1",
  "git_sha": "<commit>",
  "platform": "linux-x86_64",
  "host_fingerprint_sha256": "<hash of uname + rustc-version + buck2-version>",
  "timestamp_utc": "2026-05-03T04:52:42Z",
  "rustc_version": "rustc 1.83.0 (...)",
  "buck2_version": "buck2 <hash>",
  "flake_lock_sha256": "<sha of flake.lock>",
  "expected_corpus_root": "<from committed reference>",
  "actual_corpus_root":   "<from this rebuild>",
  "result": "pass" | "diverged",
  "diverged_log": null | "<truncated FAIL output>"
}
```

The file is signed with `cosign sign-blob --yes --bundle` using the rebuilder's keyless OIDC identity (per ADR-0008). The bundle includes the Fulcio cert and Rekor entry; verification is offline-capable.

### Operator workflow

1. Operator clones APKAXIOM at a specific git SHA on a host that satisfies `nix develop`.
2. They run `nix run github:Fizan324926/apkaxiom#rebuilder-attest`.
3. The script:
   - Verifies their toolchain matches `flake.lock` pin via `nix flake check`.
   - Runs `make verify-hashes` (which itself runs `buck2 clean && buck2 build //:all` and hashes per ADR-0007).
   - Emits the JSON above with `result: pass` or `result: diverged`.
   - Signs the JSON if `COSIGN_EXPERIMENTAL=1` (CI) or interactive OIDC is permitted.
4. Operator publishes JSON + `.sig` + `.cert` + `.bundle` somewhere durable: their own GitHub repo, an HTTP server, IPFS, etc.
5. Operator opens a PR against APKAXIOM adding their attestation under `docs/phase-1/P1.1/rebuilders/<platform>/<host_fingerprint>.json` (with the `.bundle`).

### Curation

The APKAXIOM project maintains a **rebuilder roster** at `docs/phase-1/P1.1/rebuilders/REGISTRY.md` listing accepted attestations and their bundle locations. Acceptance criteria:

- Rebuild was performed on a host distinct from any APKAXIOM CI runner (verified via `host_fingerprint_sha256`).
- Cosign signature verifies, OIDC issuer is one of the accepted identity providers (GitHub, GitLab, Sigstore-public).
- `actual_corpus_root` matches the canonical reference for the platform, OR `result: diverged` (divergence reports are valuable too).
- Operator has a public identity tied to the OIDC subject.

The roster is **public and audit-friendly**. Anyone reading the project can re-derive the trust set.

### Frequency

A rebuilder is encouraged to re-attest:

- On every release tag (mandatory for a release to be considered "federated-attested").
- On every minor flake.lock bump.
- Opportunistically — the cost is one CI run.

## Consequences

- Federation grows organically. There is no central server to operate, no schema to negotiate; the format is documented in this ADR and will not change in v1.
- Divergence reports are first-class. A `result: diverged` attestation from a credible rebuilder is exactly the alarm bell we want to hear.
- The trust set is **transparent**: the roster is in the repo, the bundles are signed with public keyless certs, the Rekor log is immutable.

## Trade-offs

- **Manual curation.** P1.1 scale; automated collection lands in P1.18 alongside the Pyroscope/OTel infra. Manual is fine while the roster is small.
- **OIDC-only attestations.** A rebuilder without a GitHub/GitLab account cannot sign easily. We accept this — the alternative (custom PGP keys with a web of trust) is dramatically more friction. A pre-shared keypair attestation path is a follow-up if a credible rebuilder requests it.
- **Per-platform root.** The `corpus_root` is per-platform (rlibs are arch-specific). A rebuilder can only attest their own platform; cross-platform federation requires multiple rebuilders. Acceptable.

## Promotion path

P1.18 wires automated collection (e.g. a scheduled workflow that scrapes the bundles, verifies them, and updates a dashboard). At that point we revisit Trustix / rebuilderd integration.

## References

- Reproducible Builds — independent verification: https://reproducible-builds.org/docs/verifying-builds/
- rebuilderd: https://github.com/kpcyrd/rebuilderd
- Trustix: https://github.com/nix-community/trustix
- Sigstore Rekor: https://docs.sigstore.dev/logging/overview/
