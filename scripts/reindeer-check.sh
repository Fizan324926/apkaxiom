#!/usr/bin/env bash
# Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
#
# reindeer-check.sh — assert `make third-party` is a no-op against the
# checked-in `third-party/rust/` tree. If running it produces a diff, the
# committed Reindeer output is stale (or the fixups have drifted).
#
# Strategy: hash every committable file under `third-party/rust/` before
# and after running reindeer. The committed-vs-working state of the repo
# does not matter — what matters is whether reindeer is *idempotent*
# against the tree it sees.
#
# Run before merging any change to `third-party/rust/Cargo.toml` or any
# `third-party/rust/fixups/*/fixups.toml`. CI gates on this script.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

TPDIR="third-party/rust"

# Hash every file we care about. The cache-state files in
# `.cargo/.global-cache` and friends are excluded because cargo writes
# them on every invocation; they are gitignored already, but we add belt
# and braces here so an old checkout still produces a clean check.
hash_tree() {
  find "$TPDIR" -type f \
    ! -path "*/.cargo/.global-cache" \
    ! -path "*/.cargo/.package-cache" \
    ! -path "*/.cargo/.package-cache-mutate" \
    -print0 \
    | sort -z \
    | xargs -0 sha256sum \
    | awk '{print $0}'
}

before=$(hash_tree)

echo "Running: reindeer vendor + reindeer buckify"
reindeer --third-party-dir="$TPDIR" vendor
reindeer --third-party-dir="$TPDIR" buckify

after=$(hash_tree)

if [[ "$before" == "$after" ]]; then
  echo
  echo "PASS: reindeer is idempotent against the $TPDIR/ tree."
  exit 0
fi

echo
echo "FAIL: reindeer changed the tree." >&2
echo "Diff (sha256-line view):" >&2
diff <(echo "$before") <(echo "$after") | head -200 >&2
echo >&2
echo "Resolution:" >&2
echo "  - If a third-party Cargo.toml or fixup was edited, this is" >&2
echo "    expected; commit the new tree alongside the manifest change." >&2
echo "  - If you did not touch reindeer config, investigate why a" >&2
echo "    vendored file changed: a transitive dep release, a checksum" >&2
echo "    update, or a reindeer version bump." >&2
exit 1
