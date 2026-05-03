# ADR-0007 — Reproducibility hash corpus: full transitive

**Status:** Accepted (P1.1)
**Date:** 2026-05-03
**Owner:** G13 — Platform Infrastructure
**Related:** ADR-0002 (Buck2), ADR-0004 (Nix flake)

---

## Context

`make repro-check` and the per-platform `reproducibility-hashes.<plat>.txt` references compare a **set of artifact hashes**. The set has to be chosen: too narrow and reproducibility regressions slip through; too wide and CI noise drowns out signal.

The Phase 1 spec under-specifies this. The first implementation (in commit `6e5bf1f`) covered only first-party `.rmeta`/`.rlib`. That excludes:

- Test binaries — the most likely place for host paths to leak.
- Vendored third-party `.rmeta`/`.rlib` — proc-macro and serde-style crates have historically embedded build-time iteration order.
- Genrule outputs — the `//:hello_world` smoke target, and any future genrule.
- The Bazel sub-workspace's outputs (none today; tracked separately under P1.13).

A reproducibility-blind area in Phase 1 is a reproducibility-blind area in *every* downstream phase that depends on it.

## Decision

The hash corpus is the **full transitive set of build outputs** of the `//:all` alias plus the `*-test` targets plus declared `genrule` outputs:

1. **First-party Rust libraries.** Every `lib<crate>-<id>.{rmeta,rlib}` for every workspace member.
2. **First-party test binaries.** The executable inside each `__<target>-test__/<crate>` directory under `buck-out/v2/gen/`.
3. **Vendored third-party libraries.** Same shape as (1) for every Reindeer-managed crate. The list lives in `scripts/_hash-artifacts.sh` `VENDORED_CRATES`. New entries go in deliberately, with the `make third-party` change that introduces the dep.
4. **Declared `genrule` outputs.** Today: `//:hello_world` writes `out.txt`.

Excluded by design:

- `*.d` dep files (contain absolute paths).
- `*.json` action-metadata.
- `linker_wrapper.sh`, `*-link-diag.{args,txt}`, `__*_linker_args.txt` (build-host-specific tempfile names).
- Symlinks (followed; their targets are hashed instead).

### Stable artifact keying

Buck2 places artifacts under `buck-out/v2/gen/<cell>/<config-hash>/<package>/<target>/<artifact-class>/<file>`. The config-hash segment is stable across two clean builds with the same Buck2 binary, **but shifts** between Buck2 versions. We therefore strip it from the corpus key:

```
buck-out/v2/gen/root/904931f735703749/crates/...   →   crates/...
```

The diff stays meaningful; per-Buck2-bump churn does not become per-artifact churn.

### Merkle root

After emitting the sorted body, `_hash-artifacts.sh` appends a single line:

```
<blake3>  CORPUS_ROOT[blake3]
```

This is the BLAKE3 of the sorted body. `verify-hashes.sh` and `repro-check.sh` short-circuit to a one-line equality check on the root before walking individual lines. CORPUS\_ROOT is the field a Trustix or rebuilder peer publishes; the per-artifact lines are the diagnostic detail behind it.

## Consequences

- **Adding a vendored dep is a deliberate act.** The `VENDORED_CRATES` list in `_hash-artifacts.sh` is the explicit signal that "we now reproduce against this dep too". Forgetting to add a new dep means it is not under reproducibility coverage; CI's `graph-parity` job (ADR-0010) backstops by complaining when the workspace and Reindeer lockfiles diverge.
- **Reference-hash bumps are gated.** Updating the committed reference is an ADR-0004 review; CI's `verify-hashes` step ensures no PR can land with a stale reference.
- **Corpus-coverage gap is visible.** Any artifact under `buck-out/` that is NOT covered is also not under repro-coverage. The `audit-toolchains` snapshot (ADR-0007) records the resolved target list; corpus drift becomes a reviewable diff.

## Trade-offs

- **Per-vendor-bump rebake.** Every dep update flips the reference. Acceptable: dep updates are rare and reviewed.
- **Test-binary linker-determinism.** Linkers (especially Apple's `ld`) historically embed build-time data. We mitigate via SOURCE\_DATE\_EPOCH + `--remap-path-prefix`; if a regression slips through, `repro-budget.sh` (ADR-0009) localises it.

## Promotion path

This ADR covers only Buck2 outputs. Bazel sub-workspace outputs (P1.13+) join the corpus then; each new build system gets a sibling section in `_hash-artifacts.sh`.

## References

- Buck2 buck-out layout: https://buck2.build/docs/concepts/buck_out/
- Reproducible Builds: https://reproducible-builds.org/docs/
- BLAKE3: https://github.com/BLAKE3-team/BLAKE3
