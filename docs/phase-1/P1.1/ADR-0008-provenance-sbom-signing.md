# ADR-0008 — Provenance, SBOM, and signing strategy

**Status:** Accepted (P1.1)
**Date:** 2026-05-03
**Owner:** G13 — Platform Infrastructure
**Related:** ADR-0002, ADR-0004, ADR-0007 (hash corpus), ADR-0011 (rebuilder federation)

---

## Context

The Phase 1 spec §5 lists Sigstore (cosign) as a Phase-1 service but does not go further. State-of-the-art reproducible-build practice in 2026 demands four artefacts attached to every build:

1. **Provenance** — a SLSA-style attestation of *what built it, from what inputs, on which runner*. Required to claim build integrity downstream.
2. **SBOM** — a CycloneDX-formatted bill of materials. Required for vulnerability triage, license review, and downstream supply-chain attestations.
3. **Signature** — a cryptographic signature over the artifact hashes, anchored in a public transparency log. Required to prevent silent substitution.
4. **Vulnerability report** — `cargo audit` against the RustSec advisory DB and `cargo deny` against the workspace policy. Required to detect known-bad deps.

P1.1's window is the right time to wire these in. Adding them later means retrofitting the chain of custody; doing it now means commit zero already attests itself.

## Decision

| Concern | Tool | Format | Where it runs | Where it lives |
|---|---|---|---|---|
| Provenance | `actions/attest-build-provenance@v1` | SLSA L3 in-toto JSONL | `.github/workflows/ci.yml` `attest` job | per-run artifact + GH attestation |
| SBOM (Rust workspace) | `cargo-cyclonedx` (≥ 0.5.9) | CycloneDX 1.3 JSON | `make sbom` (and CI `sbom` job) | `target/sbom-cargo.cdx.json` |
| SBOM (filesystem) | `syft` (anchore/syft 1.18+) | CycloneDX 1.5 JSON | same | `target/sbom-syft.cdx.json` |
| SBOM (merged) | jq union | CycloneDX 1.3 JSON | same | `target/sbom-merged.cdx.json` |
| Hash signing | `cosign sign-blob --yes` keyless | DSSE bundle + Fulcio cert | CI `sign-hashes` job | `reproducibility-hashes.<plat>.txt.{sig,cert,bundle}` |
| Vulnerability scan | `cargo-audit` (≥ 0.22.1) | JSON | CI `security-audit` job | `target/security-audit.{workspace,third-party}.json` |
| Policy gate | `cargo-deny` (≥ 0.19.4) | JSONL | CI `license-check` job | `target/license-check.jsonl` |

### SLSA target level

We aim for **SLSA L3** on release artifacts, **SLSA L1** on every PR build. L3 is reachable on GitHub-hosted runners via `slsa-framework/slsa-github-generator`; L1 is the cheapest meaningful provenance and `actions/attest-build-provenance@v1` provides it inline.

We do not aim for **L4** in Phase 1. L4 needs hermetic + parameterless + verified-builder semantics that depend on Trustix-style cross-rebuilders — that is ADR-0011's territory and is wired up in P1.18.

### Cosign keyless: identity policy

Signatures are bound to `https://github.com/Fizan324926/apkaxiom/.github/workflows/ci.yml@refs/heads/main` via the OIDC subject in the Fulcio cert. `cosign verify-blob` consumers must check that identity (and the Rekor log entry); the `verify-hashes` job in CI does this on every run.

### SBOM scope

`cargo-cyclonedx` covers **Rust workspace deps** (matches `Cargo.lock` resolution).
`syft` covers **filesystem state** (matches the on-disk source-of-truth, including vendored crates and any non-Rust file that participates in the build).

The two SBOMs are union-merged into `sbom-merged.cdx.json`. The merged file is what we attach to releases; the individual files stay around for forensics (e.g. when a vendored crate appears in syft but not cargo-cyclonedx, that is a legitimate flag).

### Tool-pin exceptions

`cargo-audit` and `cargo-deny` and `cargo-cyclonedx` are pulled from `nixpkgs-unstable` rather than the pinned `nixos-24.11`, because:

- `cargo-audit` < 0.22 / `cargo-deny` < 0.19 cannot parse CVSS:4.0 advisories that started landing in the RustSec DB in early 2026.
- `cargo-cyclonedx` < 0.5.9 cannot parse Cargo.lock format version 4.

`flake.nix` declares a second `nixpkgs-unstable` input *only* for these three tools. The risk surface is small and is gated by the same `flake.lock` pin discipline as the primary input. Revisit when 24.11 catches up (target: 25.05).

## Consequences

- Every CI run produces a signed, attested, SBOM-bearing bundle.
- `verify-hashes` consumers get a one-command verification: `cosign verify-blob --certificate-identity-regexp '^https://github.com/Fizan324926/apkaxiom/' --certificate-oidc-issuer-regexp 'token.actions.githubusercontent.com' --bundle X.bundle X` followed by hash equality.
- Vulnerability and policy regressions break the PR-gate, not a quarterly cleanup pass.

## Trade-offs

- **Two SBOM tools.** ~5 s extra in CI. Worth it for the cross-check.
- **Keyless signing means the OIDC identity is on Rekor.** That is a *feature* (transparency) but means signature provenance is publicly tied to the workflow. Same model the Linux Foundation, Kubernetes, etc. use.
- **`nixpkgs-unstable` for three tools.** Documented exception above; revisit when 24.11 ships parsers that handle CVSS:4.0 + Cargo.lock v4.

## References

- SLSA: https://slsa.dev/
- in-toto: https://in-toto.io/
- cosign: https://docs.sigstore.dev/cosign/
- CycloneDX: https://cyclonedx.org/
- cargo-cyclonedx: https://github.com/CycloneDX/cyclonedx-rust-cargo
- cargo-audit: https://github.com/rustsec/rustsec/tree/main/cargo-audit
- cargo-deny: https://github.com/EmbarkStudios/cargo-deny
- syft: https://github.com/anchore/syft
- attest-build-provenance: https://github.com/actions/attest-build-provenance
