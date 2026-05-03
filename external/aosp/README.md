# AOSP Sub-Workspace

AOSP harnesses live here. This directory is its own Bazel workspace
(`MODULE.bazel`), pinned to Bazel **7.4.1** via `.bazelversion`. The rest of
the repo uses Buck2; Bazel is confined to this subtree because the AOSP
build system has decades of Bazel-specific rule logic that would be
impractical to port.

## Phase 1 status

Stub only. `bazel info` succeeds against this directory; nothing actually
builds yet. AOSP harness rules and dependencies are introduced in **P1.13**
(soong-style native build harness) and **P1.14** (frameworks/base + selinux
+ vold harnesses).

## Layout

```
external/aosp/
├── .bazelrc          ← reproducibility flags + bzlmod enable
├── .bazelversion     ← 7.4.1, pinned via Bazelisk
├── BUILD.bazel       ← empty root package, expanded in P1.13
├── MODULE.bazel      ← bzlmod module declaration
├── WORKSPACE         ← legacy workspace marker (kept for tooling compat)
└── README.md         ← this file
```

## Why a separate workspace?

- **Buck2 cannot natively consume Bazel rules.** AOSP's Soong build emits
  Bazel-style `BUILD` files; converting them to Buck2 syntax for every
  release of AOSP would be a perpetual chore.
- **Confinement.** Bazel-specific logic stays inside this directory.
- **Reproducibility.** `.bazelversion` + `MODULE.bazel` (with `flake.lock`-pinned
  Bazelisk delivering the right Bazel binary) keeps this subtree byte-stable.

## Common operations

From the repo root, prefer the Makefile targets:
```bash
make bazel-info   # bazel info (probe the sub-workspace)
make bazel-build  # bazel build //... (will be a no-op until P1.13 lands rules)
```

Direct invocation:
```bash
cd external/aosp && bazel info
```
