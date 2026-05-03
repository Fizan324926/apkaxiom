#!/usr/bin/env bash
# Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
#
# sbom.sh — emit a CycloneDX SBOM for APKAXIOM. Per ADR-0008.
#
# We emit *two* SBOMs because they answer different questions:
#   - cargo-cyclonedx  → "what Rust crates does the workspace declare?"
#                        (resolution-time view; matches Cargo.lock)
#   - syft .           → "what is on disk in this repo right now?"
#                        (filesystem view; matches the committed tree,
#                         including vendored crates, Lean files when they
#                         arrive, etc.)
#
# Both outputs are CycloneDX 1.5 JSON. CI attaches both to the artifact
# bundle (signed with cosign in sign-hashes.sh).
#
# Outputs:
#   target/sbom-cargo.cdx.json       (cargo-cyclonedx)
#   target/sbom-syft.cdx.json        (syft)
#   target/sbom-merged.cdx.json      (the union, used by downstream)

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

mkdir -p target

# 1) Rust workspace SBOM.
# cargo-cyclonedx writes one BOM per workspace member, named `bom.cdx.json`
# beside each `Cargo.toml`. We pick up `axiom-ir`'s file (it is the only
# crate that pulls in third-party deps), copy it to target/, and clean up
# the rest. Other crates' BOMs are discarded — they would be a strict
# subset of the axiom-ir bom in P1.1.
echo "=== cargo-cyclonedx (workspace) ==="
cargo cyclonedx --format json --quiet 2>&1 | tail -5 || {
  echo "FAIL: cargo-cyclonedx failed" >&2
  exit 1
}
# Each workspace member gets `<dir>/<name>.cdx.json`. Merge them all
# (jq union) into one workspace bom. We also clean up the per-crate
# files at the end so the tree stays tidy.
mapfile -t crate_boms < <({
  find crates -maxdepth 3 -name '*.cdx.json' 2>/dev/null
  find tools  -maxdepth 3 -name '*.cdx.json' 2>/dev/null
} | sort)
if [[ ${#crate_boms[@]} -eq 0 ]]; then
  echo "FAIL: cargo-cyclonedx wrote no .cdx.json files under crates/ or tools/" >&2
  exit 1
fi
# Merge per-crate boms. cargo-cyclonedx 0.5.x emits CycloneDX 1.3 with
# `metadata.tools` as an array; later spec versions nest tools under
# `metadata.tools.components`. The expression below tolerates both.
jq -s '
  def comp_key: .["bom-ref"] // (.name // "") + ":" + (.version // "");
  def tool_key: (.name // "") + ":" + (.version // "");
  def tools_components: if type=="array" then . else (.components // []) end;
  reduce .[1:][] as $b (.[0];
    . as $acc |
    .components = (($acc.components // []) + ($b.components // [])
                    | unique_by(comp_key)) |
    .metadata.tools = (
      ($acc.metadata.tools | tools_components) + ($b.metadata.tools | tools_components)
      | unique_by(tool_key)
    )
  )
' "${crate_boms[@]}" > target/sbom-cargo.cdx.json
# Clean up per-crate boms so the working tree stays tidy.
for f in "${crate_boms[@]}"; do rm -f "$f"; done

# 2) Filesystem SBOM via syft. Excludes target/ and buck-out/ to keep the
# scan focused on source-of-truth files.
echo "=== syft (filesystem) ==="
syft scan dir:. \
  --output cyclonedx-json=target/sbom-syft.cdx.json \
  --quiet \
  --exclude './target' --exclude './buck-out' --exclude './result' \
  --exclude './.git' || {
  echo "FAIL: syft failed" >&2
  exit 1
}

# 3) Merge.  Trivial union — components from both into one bom; keep the
# cargo bom as the primary metadata source.
echo "=== merging into sbom-merged.cdx.json ==="
jq -s '
  def comp_key: .["bom-ref"] // (.name // "") + ":" + (.version // "");
  def tool_key: (.name // "") + ":" + (.version // "");
  def tools_components: if type=="array" then . else (.components // []) end;
  .[0] as $cargo | .[1] as $syft |
  $cargo
  | .components = (($cargo.components // []) + ($syft.components // [])
                    | unique_by(comp_key))
  | .metadata.tools = (
      ($cargo.metadata.tools | tools_components) + ($syft.metadata.tools | tools_components)
      | unique_by(tool_key)
    )
' target/sbom-cargo.cdx.json target/sbom-syft.cdx.json > target/sbom-merged.cdx.json

echo
echo "Wrote:"
echo "  target/sbom-cargo.cdx.json    ($(wc -c < target/sbom-cargo.cdx.json) bytes)"
echo "  target/sbom-syft.cdx.json     ($(wc -c < target/sbom-syft.cdx.json) bytes)"
echo "  target/sbom-merged.cdx.json   ($(wc -c < target/sbom-merged.cdx.json) bytes)"
echo "Components in merged:"
jq -r '.components | length' target/sbom-merged.cdx.json
