#!/usr/bin/env bash
# Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
#
# _hash-artifacts.sh — emit the canonical reproducibility-hash corpus for
# the current build to stdout. Format: "<sha256>  <stable-rel-path>" lines,
# sorted, deduplicated. Last line is "<blake3>  CORPUS_ROOT" — a single
# Merkle root over the entire corpus for quick equality checks.
#
# Corpus (per ADR-0007, "full transitive"):
#   1. Every `*.rmeta` and `*.rlib` under `buck-out/v2/gen/` whose target
#      filename starts with `lib<crate>-` for any crate in the build graph.
#   2. Every test binary — the top-level executable inside an
#      `__<crate>-test__/` directory.
#   3. Every `genrule` declared output (`out.txt` etc.).
#
# Stable-rel-path normalisation:
#   - The buck2 *configuration hash* (the 16-hex segment under
#     `gen/<cell>/`) is replaced with the literal `<cfg>` so a config-hash
#     bump (which is normal between buck2 versions) does not create a
#     spurious diff. Two clean builds on the same buck2 version produce
#     the same config hash, so we lose no signal at the per-build level.
#
# Excluded by design:
#   - `*.d` dep files                — contain absolute paths.
#   - `*.json` action-metadata       — internal bookkeeping.
#   - `linker_wrapper.sh`            — build-host-specific.
#   - `*-link-diag.{args,txt}`       — debug artefacts.
#   - `__*_linker_args.txt`          — tempfile names.
#
# Used by: repro-check.sh, hash-snapshot.sh, verify-hashes.sh,
# rebuilder-attest.sh, repro-budget.sh.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

export SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH:-315532800}"
export TZ="${TZ:-UTC}"
export LC_ALL="${LC_ALL:-C.UTF-8}"
export LANG="${LANG:-C.UTF-8}"

BUILD="${1:-build}"  # "build" (default) or "skip" if caller already built.

if [[ "$BUILD" != "skip" ]]; then
  buck2 build //:all \
    //crates/axiom-l0:axiom-l0-test \
    //crates/axiom-l1-rs:axiom-l1-rs-test \
    //crates/axiom-ir:axiom-ir-test \
    //crates/axiom-extract-hello:axiom-extract-hello \
    //crates/axiom-extract-hello:axiom-extract-hello-test \
    //tools/lean-to-rust:lean-to-rust \
    //tools/lean-to-rust:lean-to-rust-test \
    //tools/translation-validator:translation-validator \
    //theorems:hello \
    //:hello_world >&2
fi

TMP="$(mktemp)"
trap 'rm -f "$TMP"' EXIT

# Normalise a buck-out path to a stable relative form. Strips the
# 16-hex config-hash segment and the `buck-out/v2/gen/<cell>/` prefix.
normalise() {
  # buck-out/v2/gen/root/904931f735703749/crates/.../foo
  # → crates/.../foo (with the cfg replaced by literal "<cfg>")
  sed -E 's,^buck-out/v2/gen/[^/]+/[0-9a-f]{16}/,,'
}

emit() {
  # $1 = absolute path to file
  local abs="$1"
  local rel
  rel=$(realpath --relative-to="$PWD" "$abs")
  rel=$(printf '%s' "$rel" | normalise)
  sha256sum "$abs" | awk -v key="$rel" '{printf "%s  %s\n", $1, key}' >> "$TMP"
}

# Crate names participating in the build graph. Adding a vendored crate to
# this list is the explicit signal that "we now reproduce against this dep
# too". Silent additions to the dep tree do not get covered until this list
# is updated — see ADR-0007 for the policy.
FIRST_PARTY_CRATES=(
  axiom_l0
  axiom_l1_rs
  axiom_ir
  axiom_extract_hello   # P1.2 — auto-extracted from Hello.lean
)
FIRST_PARTY_BINS=(
  lean_to_rust          # P1.2 prototype extractor
  translation_validator # P1.2 operational-equivalence harness
)
VENDORED_CRATES=(thiserror thiserror_impl proc_macro2 quote syn unicode_ident)

# 1) rlib/rmeta for every crate.
for crate in "${FIRST_PARTY_CRATES[@]}" "${VENDORED_CRATES[@]}"; do
  while IFS= read -r path; do
    [[ -z "$path" || ! -f "$path" ]] && continue
    emit "$path"
  done < <(find buck-out -type f \
            \( -name "lib${crate}-*.rmeta" -o -name "lib${crate}-*.rlib" \) \
            2>/dev/null | sort -u)
done

# 2) Test binaries. The actual executable lives at
# `__<target>-test__/<crate>` (no extension); siblings are diagnostics
# we exclude.
for crate in "${FIRST_PARTY_CRATES[@]}"; do
  target="${crate//_/-}-test"
  while IFS= read -r path; do
    [[ -z "$path" || ! -f "$path" ]] && continue
    [[ -x "$path" ]] || continue
    # Skip diagnostic files inside the test directory.
    case "$(basename "$path")" in
      "$crate") emit "$path" ;;
    esac
  done < <(find buck-out -type f -path "*__${target}__/*" \
            ! -name '*.d' ! -name '*.json' ! -name '*.txt' \
            ! -name '*.rmeta' ! -name '*.rlib' \
            ! -name '*.args' ! -name '*.sh' \
            2>/dev/null | sort -u)
done

# 3) Genrule outputs:
#    - //:hello_world (smoke target from P1.1)
#    - //theorems:hello (P1.2 — manifest of Lean .olean hashes)
while IFS= read -r path; do
  [[ -z "$path" || ! -f "$path" ]] && continue
  emit "$path"
done < <({
  find buck-out -type f -path "*__hello_world__*" -name "out.txt" 2>/dev/null
  find buck-out -type f -path "*__hello__*" -name "olean-manifest.txt" 2>/dev/null
} | sort -u)

# 4) First-party binary tools (release-mode equivalents emitted under
# `bin-pic-static_pic-link/`). Their stable names are the crate name
# without the `lean_to_rust`-style `_` prefix.
for bin in "${FIRST_PARTY_BINS[@]}"; do
  while IFS= read -r path; do
    [[ -z "$path" || ! -f "$path" ]] && continue
    [[ -x "$path" ]] || continue
    case "$(basename "$path")" in
      "$bin") emit "$path" ;;
    esac
  done < <(find buck-out -type f -path "*__${bin//_/-}__*" \
            ! -name '*.d' ! -name '*.json' ! -name '*.txt' \
            ! -name '*.rmeta' ! -name '*.rlib' \
            ! -name '*.args' ! -name '*.sh' \
            2>/dev/null | sort -u)
done

# Body of the corpus, sorted + deduped.
sort -u "$TMP" > "$TMP.sorted"
cat "$TMP.sorted"

# Merkle-style root: BLAKE3 of the sorted corpus body itself. This single
# number is what `verify-hashes` short-circuits on — a one-line equality
# check against the committed reference. Falls back to sha256 if b3sum
# is missing (won't happen inside `nix develop`).
if command -v b3sum >/dev/null 2>&1; then
  root_alg=blake3
  root_hash=$(b3sum --no-names "$TMP.sorted")
else
  root_alg=sha256
  root_hash=$(sha256sum "$TMP.sorted" | awk '{print $1}')
fi
printf '%s  CORPUS_ROOT[%s]\n' "$root_hash" "$root_alg"
rm -f "$TMP.sorted"
