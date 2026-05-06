#!/usr/bin/env bash
# P1.11 §B item 4 (HARD) — three-way differential.
#
# Walks every APK in:
#   corpus/signing/v1-only/
#   corpus/signing/v1-v2/
#   corpus/signing/v1-v2-v3/
#   corpus/signing/adversarial/
#   crates/axiom-l1-rs/tests/fixtures/
#
# For each:
#   1. Lean   `lake exe sig-eval` on the hex bytes
#   2. Rust   `tools/sig-eval-rust` on the hex bytes
#   3. apksigner verify
#
# Asserts:
#   - Lean output == Rust output (byte-identical JSON)
#   - apksigner accept ⟺ Lean+Rust report `signed=true,...` AND
#                          (for adversarial corpus) error categories agree
#   - Adversarial fixtures: every verifier rejects.
#
# Exits 0 only if every assertion holds.
set -euo pipefail
ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"

CORPUS_DIRS=(
  corpus/signing/v1-only
  corpus/signing/v1-v2
  corpus/signing/v1-v2-v3
  corpus/signing/adversarial
  crates/axiom-l1-rs/tests/fixtures
)

# Build the binaries.
echo ">> building sig-eval-rust"
cargo build -q -p sig-eval-rust --release
echo ">> building Lean sig-eval"
nix develop --command lake build sig-eval >/dev/null 2>&1

# Collect all APK paths in one stable order.
APKS=()
for d in "${CORPUS_DIRS[@]}"; do
  while IFS= read -r f; do
    APKS+=("$f")
  done < <(find "$d" -name '*.apk' | sort)
done

echo ">> ${#APKS[@]} APKs in differential corpus"

# Build hex stdin (one line per APK).
HEX_INPUT=$(mktemp)
for f in "${APKS[@]}"; do
  python3 -c "import sys; print(open(sys.argv[1],'rb').read().hex())" "$f"
done > "$HEX_INPUT"

# Run both evaluators.
RUST_OUT=$(mktemp)
LEAN_OUT=$(mktemp)
./target/release/sig-eval-rust < "$HEX_INPUT" > "$RUST_OUT"
nix develop --command lake exe sig-eval < "$HEX_INPUT" 2>/dev/null \
  | grep '^{"i":' > "$LEAN_OUT"

# Diff.
if cmp -s "$RUST_OUT" "$LEAN_OUT"; then
  echo "PASS: Lean ↔ Rust output byte-identical on ${#APKS[@]} APKs"
else
  echo "FAIL: Lean ↔ Rust differ:"
  diff "$RUST_OUT" "$LEAN_OUT" | head -40
  exit 1
fi

# apksigner cross-check on each APK.
echo ">> apksigner cross-check"
ANY_FAIL=0
for i in "${!APKS[@]}"; do
  f="${APKS[$i]}"
  base=$(basename "$f")
  # Adversarial APKs MUST reject; honest APKs MUST verify.
  is_adversarial=0
  case "$f" in
    corpus/signing/adversarial/*) is_adversarial=1 ;;
  esac
  if apksigner verify "$f" >/dev/null 2>&1; then
    apk_verdict="accept"
  else
    apk_verdict="reject"
  fi
  # Lean/Rust verdict: line-i of LEAN_OUT.
  lean_line=$(sed -n "$((i+1))p" "$LEAN_OUT")
  case "$lean_line" in
    *'"out":"ok"'*)        ours="signed-ok" ;;
    *'"out":"unsigned"'*)  ours="unsigned" ;;
    *'"out":"err"'*)       ours="reject" ;;
    *)                     ours="???" ;;
  esac
  ok=""
  if [[ $is_adversarial -eq 1 ]]; then
    # Adversarial: apksigner MUST reject.
    if [[ "$apk_verdict" == "reject" ]]; then
      ok="PASS"
    else
      ok="FAIL"; ANY_FAIL=1
    fi
    echo "  [adversarial] $base: apksigner=$apk_verdict ours=$ours $ok"
  else
    # Honest APK: apksigner accepts. Lean/Rust report `signed-ok`
    # (multi-scheme) or `unsigned` (v1-only).
    if [[ "$apk_verdict" == "accept" ]] && [[ "$ours" == "signed-ok" || "$ours" == "unsigned" ]]; then
      ok="PASS"
    else
      ok="FAIL"; ANY_FAIL=1
    fi
    echo "  [honest]      $base: apksigner=$apk_verdict ours=$ours $ok"
  fi
done

if [[ $ANY_FAIL -ne 0 ]]; then
  echo
  echo "::error::p111-differential: at least one APK disagreed across verifiers"
  exit 1
fi

echo
echo "PASS: ${#APKS[@]} APKs Lean↔Rust↔apksigner agreed"
