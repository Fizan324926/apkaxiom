# Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
#
# Root BUCK module. The "all" alias is the canonical "build everything in the
# root cell" target. Reproducibility checks query this alias by name so the
# set of artifacts under repro-tracking is explicit, not implicit.

alias(
    name = "all",
    actual = ":axiom",
    visibility = ["PUBLIC"],
)

filegroup(
    name = "axiom",
    srcs = [
        "//crates/axiom-l0:axiom-l0",
        "//crates/axiom-l1-rs:axiom-l1-rs",
        "//crates/axiom-ir:axiom-ir",
    ],
    visibility = ["PUBLIC"],
)

# Build-graph liveness probe: `buck2 build //:hello_world` is the smallest
# cycle that proves Buck2 + bundled prelude + system toolchains are wired up
# end-to-end. CI uses this as the first action after a clean checkout.
genrule(
    name = "hello_world",
    out = "out.txt",
    cmd = "echo APKAXIOM-BUCK2-OK > $OUT",
    visibility = ["PUBLIC"],
)
