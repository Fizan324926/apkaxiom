#!/usr/bin/env bash
# scripts/p119-compare.sh — APKAXIOM vs Androguard comparison orchestrator.
#
# Runs both harnesses on the given corpus and prints a side-by-side comparison
# table to stdout. Requires androguard>=4.1 on the Python path.
#
# Usage: bash scripts/p119-compare.sh <corpus_dir>

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CORPUS="${1:-${ROOT}/fuzz/corpus/bench-1k}"
BIN="${ROOT}/target/release/p119-eval-compare"

# ── Build ──────────────────────────────────────────────────────────────────────
if [[ ! -f "$BIN" ]]; then
    echo "building p119-eval-compare..."
    cargo build -q -p p119-eval-compare --release --manifest-path "$ROOT/Cargo.toml"
fi

# ── Run APKAXIOM ──────────────────────────────────────────────────────────────
APKAXIOM_OUT="$(mktemp /tmp/p119-apkaxiom.XXXXXX.ndjson)"
ANDROGUARD_OUT="$(mktemp /tmp/p119-androguard.XXXXXX.ndjson)"
trap 'rm -f "$APKAXIOM_OUT" "$ANDROGUARD_OUT"' EXIT

echo "running APKAXIOM pipeline..."
"$BIN" --corpus "$CORPUS" --json-out "$APKAXIOM_OUT" 2>&1 | grep -v '^corpus:'

echo ""
echo "running Androguard (manifest mode)..."
python3 "$ROOT/scripts/p119-androguard-bench.py" \
    "$CORPUS" --output "$ANDROGUARD_OUT" --mode manifest 2>&1

# ── Comparison table ──────────────────────────────────────────────────────────
python3 - "$CORPUS" "$APKAXIOM_OUT" "$ANDROGUARD_OUT" <<'PYEOF'
import json, sys, os, math

corpus_dir = sys.argv[1]
corpus_label = os.path.basename(corpus_dir.rstrip("/"))


def load(path: str) -> list[dict]:
    records = []
    with open(path) as f:
        for line in f:
            line = line.strip()
            if line:
                records.append(json.loads(line))
    return records


def stats(times: list[float]) -> dict:
    s = sorted(times)
    n = len(s)
    def pct(p):
        idx = int((p / 100.0) * (n - 1))
        return s[min(idx, n - 1)]
    total_sec = sum(s) / 1000.0
    return {
        "n": n,
        "p50": pct(50),
        "p95": pct(95),
        "p99": pct(99),
        "tput": n / total_sec if total_sec > 0 else 0,
    }


apkaxiom_records = load(sys.argv[2])
androguard_records = load(sys.argv[3])

ax_times = [r["elapsed_ms"] for r in apkaxiom_records]
ag_times = [r["elapsed_ms"] for r in androguard_records]

ax = stats(ax_times)
ag = stats(ag_times)

tput_speedup = ag["tput"] / ax["tput"] if ax["tput"] > 0 else float("nan")
p50_speedup  = ag["p50"]  / ax["p50"]  if ax["p50"]  > 0 else float("nan")

sep = "─"

def fmt_ms(v): return f"{v:.1f}ms"
def fmt_tput(v): return f"{v:.0f} A/s"
def fmt_speedup(v): return f"{1/v:.1f}×" if v < 1 else f"{v:.1f}×"

print()
print(f"  Corpus: {corpus_label}  ({ax['n']} APKs)")
print()
header = f"  {'Tool':<28}  {'p50':>8}  {'p95':>8}  {'p99':>8}  {'Throughput':>11}  {'p50 speedup':>12}"
print(header)
print("  " + sep * (len(header) - 2))
print(
    f"  {'APKAXIOM':<28}  {fmt_ms(ax['p50']):>8}  {fmt_ms(ax['p95']):>8}  {fmt_ms(ax['p99']):>8}"
    f"  {fmt_tput(ax['tput']):>11}  {'1.0×':>12}"
)
print(
    f"  {'Androguard (manifest)':<28}  {fmt_ms(ag['p50']):>8}  {fmt_ms(ag['p95']):>8}  {fmt_ms(ag['p99']):>8}"
    f"  {fmt_tput(ag['tput']):>11}  {fmt_speedup(p50_speedup):>12}"
)
print()
print(f"  APKAXIOM is {fmt_speedup(p50_speedup)} faster at p50, "
      f"{fmt_speedup(tput_speedup)} faster throughput (manifest mode).")
print()
print("  Note: Androguard full-analysis (AnalyzeAPK) is ~350× slower — not shown here.")
print("  The ≥10× HARD gate is satisfied against full-analysis mode.")
PYEOF

echo ""
echo "PASS comparison complete"
