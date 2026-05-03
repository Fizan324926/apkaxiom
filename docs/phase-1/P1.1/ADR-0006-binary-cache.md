# ADR-0006 — Two-layer binary-cache strategy: magic-nix-cache + Cachix

**Status:** Accepted (P1.1)
**Date:** 2026-05-03
**Owner:** G13 — Platform Infrastructure
**Supersedes:** none
**Related:** ADR-0002 (Buck2), ADR-0004 (Nix flake), ADR-0008 (provenance)

---

## Context

Phase 1's [§5 spec](./README.md) names **Cachix** as the binary cache and asks for **≥90 % hit rate on warm CI runs**. Two operational facts force a slightly different shape:

1. Cachix's free tier is per-organisation and 5 GB-capped. APKAXIOM's nix-store closure (rust-toolchain + buck2 + bazelisk + cosign + syft + cargo-* + diffoscope on the debug shell) is comfortably under that today, but adding Lean 4 + mathlib4 in P1.2 will exceed it within a sub-phase.

2. GitHub Actions ships its own Nix-store cache via `DeterminateSystems/magic-nix-cache-action`. It is **free**, **unlimited** within a workflow run, and requires zero out-of-band provisioning. Its limitation is that it is invisible to dev laptops — it accelerates CI only.

A single-cache solution forces a trade. magic-nix-cache wins for CI; Cachix wins for shared dev environments. Choosing one penalises the other.

## Decision

We adopt a **two-layer** binary-cache strategy:

- **Layer 1 (required, live in P1.1) — `DeterminateSystems/magic-nix-cache-action`** in CI. Plumbed in `.github/workflows/ci.yml` as the very first step of every job; no opt-in required. **This layer alone satisfies the spec's "≥90 % hit rate on warm CI" gate** — see *Telemetry* below.
- **Layer 2 (optional follow-up, dev acceleration only) — public Cachix cache `apkaxiom.cachix.org`**. Configured stub-in-place in `flake.nix` `nixConfig.extra-substituters`. Provisioning is deferred until a developer reports the dev-laptop `nix develop` cold start as an actual pain point. The provisioning checklist below stays accurate so we can flip the switch in minutes when the time comes.

Both layers are **opt-in upgrades over the public NixOS cache**. The build still completes if either is unavailable; the difference is wall-time.

## Consequences

- **CI hit rate** is the magic-nix-cache hit rate, computed by parsing the action's diagnostic endpoint at `MAGIC_NIX_CACHE_DIAGNOSTIC_ENDPOINT`. We emit a per-run summary as a CI annotation and append it to `docs/phase-1/P1.1/wall-time.ndjson` (per ADR-0009). The K10 ≥90 % gate is enforced in the rollup workflow.
- **Dev hit rate** is the Cachix hit rate, queried via the Cachix HTTP API. A scheduled workflow records the weekly figure in the same NDJSON file under a separate `kind: dev-cache` namespace.
- **No private build artifacts in either cache.** APKAXIOM is open source; both caches are public substituters. If we ever need to publish a private artifact (e.g. a pre-disclosure security build), it must not enter either cache.

### Optional follow-ups

These activate the Layer-2 dev cache. They are **not** P1.1 acceptance
criteria; the CI hit-rate gate is met by Layer 1 alone.

1. G13 lead creates the `apkaxiom` Cachix cache (free public tier).
2. Capture the public-key line and overwrite the placeholder in `flake.nix` `nixConfig.extra-trusted-public-keys`.
3. Push a CI secret `CACHIX_AUTH_TOKEN` (write-token, masked) for the cache-publish job.
4. Add the `cachix/cachix-action@v15` step to `ci.yml` after the magic-nix-cache step.
5. Verify `nix develop --accept-flake-config --command nix path-info --closure-size .#devShells.<system>.default` reports the same closure as before, just faster.
6. Record the ISO date of activation in this ADR's *Activation history* section.

### Activation history

| Date | Action | By |
|---|---|---|
| _not yet activated_ | — | — |

## Trade-offs

- **Two caches to operate.** Marginal: both are managed services with web UIs.
- **Cachix outage modes.** If the cache is unavailable, Nix falls back to NixOS cache + source build. Slower, not broken.
- **Determinate's magic-nix-cache is a fork-of-cache logic.** It is well-maintained and used by Determinate Nix itself; risk is low. If it ever stops being maintained, swap for the upstream `cachix/cachix-action` with a self-hosted cache. Documented as a follow-up ADR if it ever becomes load-bearing.

## Why not (and what we considered)

| Alternative | Why we rejected it |
|---|---|
| **Cachix only** | Penalises CI: every workflow run pulls a chunk of the closure from `cachix.org` rather than from GitHub's edge cache, costing wall-time. |
| **magic-nix-cache only** | Dev laptops gain nothing. Onboarding is "wait 8 minutes for `nix develop`" forever. |
| **Self-hosted S3-backed cache (`s3://`)** | Operational overhead at this scale is not justified. Revisit at P1.18 if the closure exceeds Cachix free-tier limits. |
| **`nix-build` on every CI invocation, no cache** | Full uncached `nix develop` on a cold runner is ~12 min — already over the K10 budget for the rest of the build. |

## References

- magic-nix-cache: https://github.com/DeterminateSystems/magic-nix-cache-action
- Cachix: https://www.cachix.org/
- Nix substituters: https://nixos.org/manual/nix/stable/command-ref/conf-file#conf-substituters
