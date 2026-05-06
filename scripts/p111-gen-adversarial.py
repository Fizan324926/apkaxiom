#!/usr/bin/env python3
# P1.11 §B item 5 — adversarial-corpus generator.
#
# Generates synthetic adversarial APK fixtures from an honest base
# APK by transforming the bytes in well-defined ways. Each output
# must be REJECTED by every verifier (Lean + Rust + apksigner).
#
# Categories:
#
#   janus            — DEX-prepended (CVE-2017-13156). Apksigner v2+
#                      detects via whole-file digest; v1 alone is
#                      vulnerable.
#   bad-magic        — APK-Sig-Block-magic byte flipped → block
#                      not located.
#   size-mismatch    — leading u64 size_of_block ≠ trailing u64.
#   pair-overflow    — first pair's length declared larger than the
#                      block contents.
#   pair-too-short   — first pair's length < 4 (must include id).
#   v3-stripped      — v3 block ID rewritten to padding so devices
#                      see the v3 region as zero-padding (downgrade).
#
# Usage:
#   python3 scripts/p111-gen-adversarial.py
#
# Outputs into corpus/signing/adversarial/.
import struct
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC_V123 = os.path.join(ROOT, 'corpus/signing/v1-v2-v3/wifiautoff-v1v2v3.apk')
SRC_V1   = os.path.join(ROOT, 'corpus/signing/v1-only/wifiautoff-v1.apk')
OUT_DIR  = os.path.join(ROOT, 'corpus/signing/adversarial')

MAGIC = b'APK Sig Block 42'

def find_block(apk: bytes):
    eocd = apk.rfind(b'\x50\x4b\x05\x06')
    if eocd < 0: return None
    cd_off = struct.unpack('<I', apk[eocd+16:eocd+20])[0]
    if cd_off < 24: return None
    if apk[cd_off-16:cd_off] != MAGIC: return None
    sob = struct.unpack('<Q', apk[cd_off-24:cd_off-16])[0]
    return cd_off, sob, cd_off - sob - 8  # (cd_offset, size_of_block, block_offset)

def write(name, bytes_):
    path = os.path.join(OUT_DIR, name)
    with open(path, 'wb') as f: f.write(bytes_)
    print(f'wrote {path} ({len(bytes_)} bytes)')

os.makedirs(OUT_DIR, exist_ok=True)
os.chdir(ROOT)

apk = open(SRC_V123, 'rb').read()
v1apk = open(SRC_V1, 'rb').read()
loc = find_block(apk)
if not loc:
    print('error: source v1+v2+v3 fixture has no signing block', file=sys.stderr)
    sys.exit(1)
cd_off, sob, block_off = loc
print(f'v1+v2+v3 source: cd_offset={cd_off} sob={sob} block_offset={block_off}')

# 1. Janus: prepend a DEX header and Janus-style fake APK structure.
#    DEX magic = "dex\n035\0" (8 bytes), then arbitrary content.
dex_prefix = b'dex\n035\0' + b'\x00' * 200  # 208 bytes of pretend-DEX
janus = dex_prefix + apk
write('janus-dex-prepended.apk', janus)

# 2. Bad magic — flip one byte of the magic.
m_off = cd_off - 16
mutated = bytearray(apk)
mutated[m_off] ^= 0x01  # 'A' → 0x40
write('bad-magic.apk', bytes(mutated))

# 3. Size mismatch — change leading u64 to differ from trailing.
mutated = bytearray(apk)
struct.pack_into('<Q', mutated, block_off, sob + 1)
write('size-mismatch.apk', bytes(mutated))

# 4. Pair-overflow — first pair declares length larger than block.
mutated = bytearray(apk)
# First pair length is the u64 at block_off+8
struct.pack_into('<Q', mutated, block_off + 8, sob + 0xffff)
write('pair-overflow.apk', bytes(mutated))

# 5. Pair-too-short — first pair length < 4.
mutated = bytearray(apk)
struct.pack_into('<Q', mutated, block_off + 8, 3)
write('pair-too-short.apk', bytes(mutated))

# 6. v3-stripped — find v3 block ID (0xf05368c0) and overwrite with
#    padding ID (0x6dff800d). The structural parser is fine; the
#    semantic verifier sees no v3 → downgrade attempt.
mutated = bytearray(apk)
# Walk pairs to find v3.
cur = block_off + 8
end = cd_off - 24
found_v3 = False
while cur < end:
    length = struct.unpack('<Q', mutated[cur:cur+8])[0]
    pid = struct.unpack('<I', mutated[cur+8:cur+12])[0]
    if pid == 0xf05368c0:
        struct.pack_into('<I', mutated, cur + 8, 0x6dff800d)
        found_v3 = True
        break
    cur += 8 + length
if found_v3:
    write('v3-stripped.apk', bytes(mutated))
else:
    print('warn: v3 block not found in source APK; skipping v3-stripped.apk',
          file=sys.stderr)

# 7. v1-only-with-janus — apply janus prefix to a v1-only APK
#    where there's no v2/v3 to detect via whole-file digest. This
#    is the canonical CVE-2017-13156 shape.
janus_v1 = dex_prefix + v1apk
write('v1-janus-cve-2017-13156.apk', janus_v1)

# 8. Truncated EOCD — drop the last 22 bytes of the v1+v2+v3 APK.
write('truncated-eocd.apk', apk[:-22])

# 9. Truncated signing block — chop bytes inside the signing block.
write('truncated-block.apk', apk[:block_off + 8 + 16])

# 10. Empty pair region — set sob to just the trailing 8 + 16 = 24
#     bytes (no pairs), which is well-formed but empty. Verifier
#     should reject for "no signers".
mutated = bytearray(apk)
# We can't easily resize without rebuilding the APK; document this
# as a bookkeeping fixture only.
# (Skipped — empty signing block needs full APK reconstruction.)

print('adversarial corpus generated.')
