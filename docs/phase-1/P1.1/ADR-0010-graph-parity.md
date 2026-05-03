# ADR-0010 — Cargo ↔ Buck2 graph-parity gate

**Status:** Accepted (P1.1)
**Date:** 2026-05-03
**Owner:** G13 — Platform Infrastructure
**Related:** ADR-0002 (Buck2), ADR-0007 (hash corpus)

---

## Context

APKAXIOM has **two Rust dependency graphs**:

1. The workspace graph rooted at `Cargo.toml` / `Cargo.lock` — the source of truth for `cargo build`, IDEs, `cargo test`, and developer fast-iter loops.
2. The Reindeer-managed graph at `third-party/rust/Cargo.toml` / `third-party/rust/Cargo.lock` — the source of truth for `buck2 build //:all`.

These are **independent**. A developer who bumps `thiserror` in the workspace `Cargo.toml` without running `make third-party-update` will leave the Reindeer manifest pinned to the old version. Cargo-only tests pass; `buck2 build` quietly links against the stale rlib. Reproducibility hashes shift, the `verify-hashes` step fails, and the diagnosis is "where did this 16-hex hash come from".

We catch this when it happens, not after the fact.

## Decision

A new CI gate `scripts/graph-parity.sh` runs on every PR:

1. `cargo metadata --format-version=1 --locked` enumerates the workspace's resolved deps.
2. The Reindeer `third-party/rust/Cargo.lock` is parsed directly.
3. For every crate that appears in *both*, the version must match.
4. Crates that appear in only one graph are fine (workspace-internal deps and vendor-only deps are normal).

Output: `target/graph-parity.json` (machine-readable) plus a Markdown summary (CI annotation).

The script exits non-zero on any version mismatch. Resolution is `make third-party-update`, which propagates the workspace's `Cargo.lock` into the Reindeer manifest.

## Consequences

- Cargo↔Buck2 dep drift becomes a PR-time error, not a tomorrow-morning surprise.
- `make third-party-update` becomes part of the dep-bump workflow, not an oh-yeah-I-forgot footnote.
- The check is fast (under 1 s on the current dep tree); cheap to run on every PR.

## Trade-offs

- **Two-version-during-migration cases** are uncommon but real (a transitive crate sometimes ships in two majors during ecosystem migration). When they happen, the right resolution is to extend the graph-parity script with a per-crate exception list, not to disable the gate. Empty today.
- **Direct Cargo.lock parsing** of the Reindeer manifest is a pragmatic shortcut. A more principled approach uses `cargo metadata` against `third-party/rust/`, but Reindeer's manifest is not always a fully-resolvable Cargo.toml. The shortcut works because Cargo.lock format is stable; if it changes (Cargo.lock v5+), we update the parser.

## References

- Reindeer: https://github.com/facebookincubator/reindeer
- Cargo.lock format: https://doc.rust-lang.org/cargo/guide/cargo-toml-vs-cargo-lock.html
