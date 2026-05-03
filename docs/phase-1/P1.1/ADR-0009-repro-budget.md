# ADR-0009 — Reproducibility-budget reporter and determinism lints

**Status:** Accepted (P1.1)
**Date:** 2026-05-03
**Owner:** G13 — Platform Infrastructure
**Related:** ADR-0007 (hash corpus)

---

## Context

When `make repro-check` fails the operator gets a `diff -u` of two snapshot files. That tells them *which artifact diverged* but not *where, why, or how to fix it*. The cost of a non-actionable failure is real: a developer who hits a repro-fail before lunch loses the whole afternoon.

Two complementary mechanisms address this:

1. **Reactive — repro-budget reporter.** Run after `repro-check` fails. Names the divergent artifacts, points at the first byte of disagreement, and dumps the contained archive members for `.rlib` files. State-of-the-art is `diffoscope`, but diffoscope is heavy; we provide a lightweight reporter for the 80 % case and gate diffoscope behind the `repro-debug` shell.

2. **Proactive — determinism lints.** A regex-based static check for known nondeterministic patterns (`SystemTime::now()`, `HashMap::iter` without sort, etc.) in first-party code. Most regressions surface here before a `repro-check` ever runs.

Both ship with P1.1 because catching reproducibility regressions late means catching them in CI on six platforms simultaneously — exactly when they cost the most to debug.

## Decision

### Reactive: `scripts/repro-budget.sh`

Inputs (from `repro-check.sh`):

- Path to snapshot A.
- Path to snapshot B.
- Optional path to a copy of build B's artifacts.

Outputs a Markdown section per divergent artifact (capped at 8 to keep output readable) containing:

- Artifact path.
- Hash on each side.
- First byte of disagreement (decimal + hex offset).
- Hex dump (`xxd -s <start> -l 64`) of ±32 bytes around the divergence on each side.
- For `.rlib` files: `ar t` member list diff so the operator immediately sees which `.o` inside the rlib changed.
- Pointer to the `repro-debug` shell for full `diffoscope` investigation.

`repro-check.sh` always invokes the budget reporter on FAIL. CI uploads the report as a workflow annotation so the failure surface is the GitHub UI, not buried in logs.

### Proactive: `scripts/lint-determinism.sh`

Regex scan of `crates/**/*.rs` for the patterns below. Hits are warnings by default; promotion to error is per-rule and tracked in this ADR's *Promotion log*.

| Pattern | Reason |
|---|---|
| `SystemTime::now()` outside `cfg(test)` | Embeds wall clock in build-time codegen. |
| `Instant::now()` outside `cfg(test)` | Same. |
| `process::id()` outside `cfg(test)` | Embeds PID. |
| `thread_rng()` / `rand::random()` outside `cfg(test)` | Non-seeded RNG bakes entropy. |
| `HashMap::iter`-style without an adjacent `BTreeMap` or `sort` | Iteration order varies per process (randomised hasher). |
| `fs::read_dir` without an adjacent sort | Filesystem iteration is OS-dependent. |
| `current_dir()` outside `cfg(test)` | Path leakage. |
| `env!("HOME")` | Path leakage. |

Vendored third-party crates are out of scope — fixing them is a fixup or upstream-PR job, not a lint.

#### Promotion log

| Pattern | Severity | Promoted | Reason |
|---|---|---|---|
| (none yet) | warn | — | All rules start as warnings; promote when a regression actually slips through. |

## Consequences

- A repro-check failure now produces an actionable report inside the failing CI job. Mean time to diagnose drops from hours to minutes.
- Determinism lints catch most regressions at the file-edit level, before they reach `repro-check`.
- The lint set is conservative — false positives are easy to suppress with a `// repro: ok` comment (added when the warning fires legitimately, e.g. inside a logging call) — but the rules themselves are evidence-based, derived from real reproducibility incidents in the Rust ecosystem.

## Trade-offs

- **Regex-based lints miss subtle cases** (e.g. `HashMap` indirected through a generic `BuildHasher`). We accept this — `repro-check` is the backstop. The lints aim to catch the obvious 80 %, not be a complete static-analysis solution.
- **`xxd ±32 bytes` may not be the right window for every diff.** For an rlib divergence in a deep `.o` member, the offset on the rlib itself is opaque. The `ar t` diff covers that case; for everything else, `nix develop .#repro-debug --command diffoscope` is one command away.

## References

- Reproducible Builds documentation pages: https://reproducible-builds.org/docs/
- diffoscope: https://diffoscope.org/
- Rust HashMap hasher randomisation: https://doc.rust-lang.org/std/collections/struct.HashMap.html
