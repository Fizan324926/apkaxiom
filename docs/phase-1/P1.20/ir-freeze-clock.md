# AXIOM-IR v0.1 Freeze Clock

**Started:** 2026-05-04 (P1.15 merge commit)  
**Today:** 2026-05-07  
**Required freeze window:** 28 days (4 weeks)

---

## Countdown

| Item                  | Value                    |
|-----------------------|--------------------------|
| Freeze started        | 2026-05-04               |
| Freeze ends           | 2026-06-01               |
| Days elapsed          | 3                        |
| Days remaining        | **25**                   |
| Status                | CARRY-FORWARD (clock-gated) |

---

## What constitutes a freeze break

Any change to `crates/axiom-ir/src/` or `schema/axiom-ir-*.capnp` that alters
the serialised wire format (field additions, type changes, enum extensions,
removal of optional fields) restarts the clock. Bug fixes to internal Rust
code that do not touch the Cap'n Proto schema are allowed without clock reset.

---

## Auto-close condition

This gate auto-closes at 2026-06-01 if no AXIOM-IR schema changes land between
now and then. No engineering action required; the freeze is a process gate, not
a code gate.
