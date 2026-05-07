#!/usr/bin/env python3
"""Adversarial-500 corpus generator.

Generates 500 synthetic adversarial APK-shaped ZIP files from the
existing signed fixture, covering 10 categories × 50 variants each.
Outputs to corpus/adversarial-500/.

Categories
----------
A  truncated-eocd       — EOCD truncated at various byte offsets (1..50)
B  dual-eocd            — second EOCD injected at varying positions
C  negative-cd-offset   — CD offset field set to 0xFFFFFFFF (ZIP64 sentinel
                          or just bogus)
D  lfh-cdr-mismatch     — LFH filename length != CDR filename length for
                          each entry, 50 variants with increasing delta
E  zip64-wrong-offset   — ZIP64 EOCD present but with wrong CD offset
F  empty-sigblock       — APK signing block with zero pairs, all zeros fill
G  zeros-sigblock       — entire signing-block region overwritten with 0x00
H  bad-magic-variants   — magic bytes mutated 1 bit at a time (bits 0..49)
I  oversized-comment    — EOCD comment length field set > actual remaining
J  mismatched-size      — leading/trailing size_of_block set to diverging
                          values (delta +1 to +50)
"""
import struct
import os
import sys
import hashlib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC  = os.path.join(ROOT, 'corpus/signing/v1-v2-v3/wifiautoff-v1v2v3.apk')
OUT  = os.path.join(ROOT, 'corpus/adversarial-500')
os.makedirs(OUT, exist_ok=True)

MAGIC = b'APK Sig Block 42'
EOCD_SIG = b'\x50\x4b\x05\x06'

def find_eocd(data: bytes) -> int:
    return data.rfind(EOCD_SIG)

def find_block(data: bytes):
    eocd = find_eocd(data)
    if eocd < 0:
        return None
    cd_off = struct.unpack('<I', data[eocd + 16:eocd + 20])[0]
    if cd_off < 24:
        return None
    if data[cd_off - 16:cd_off] != MAGIC:
        return None
    sob = struct.unpack('<Q', data[cd_off - 24:cd_off - 16])[0]
    block_off = cd_off - sob - 8
    return cd_off, sob, block_off

def w(name: str, data: bytes):
    p = os.path.join(OUT, name)
    with open(p, 'wb') as f:
        f.write(data)

base = open(SRC, 'rb').read()
loc  = find_block(base)
if not loc:
    print('error: source fixture has no signing block', file=sys.stderr)
    sys.exit(1)
cd_off, sob, block_off = loc
eocd_off = find_eocd(base)
count = 0

# ── A: truncated-eocd (50 variants) ──────────────────────────────────────────
for i in range(1, 51):
    trunc = base[:-i] if i < len(base) else base[:0]
    w(f'A-truncated-eocd-{i:03d}.apk', trunc)
    count += 1

# ── B: dual-eocd (50 variants) ───────────────────────────────────────────────
# Inject a second EOCD record at various positions before the real one.
fake_eocd = EOCD_SIG + b'\x00' * 18  # minimal 22-byte EOCD
for i in range(50):
    insert_at = max(0, eocd_off - (i + 1) * 4)
    d = bytearray(base)
    result = d[:insert_at] + fake_eocd + d[insert_at:]
    w(f'B-dual-eocd-{i:03d}.apk', bytes(result))
    count += 1

# ── C: negative-cd-offset / bogus (50 variants) ──────────────────────────────
for i in range(50):
    d = bytearray(base)
    # Vary the CD offset between common sentinel/bogus values
    bogus_offsets = [
        0xFFFFFFFF,  # ZIP64 sentinel
        0xFFFFFFFE,
        0x00000000,
        cd_off + 1 + i,        # slightly past actual cd
        cd_off - len(base) - 1  # underflow
    ]
    val = bogus_offsets[i % len(bogus_offsets)] + i
    val &= 0xFFFFFFFF
    struct.pack_into('<I', d, eocd_off + 16, val)
    w(f'C-bad-cd-offset-{i:03d}.apk', bytes(d))
    count += 1

# ── D: LFH-CDR mismatch (50 variants) ────────────────────────────────────────
# Find first LFH and CDR, corrupt their filename-length fields.
LFH_SIG = b'\x50\x4b\x03\x04'
CDR_SIG = b'\x50\x4b\x01\x02'
lfh_off = base.find(LFH_SIG)
cdr_off = base.find(CDR_SIG)
for i in range(50):
    d = bytearray(base)
    if lfh_off >= 0 and lfh_off + 30 < len(d):
        orig_len = struct.unpack('<H', d[lfh_off + 26:lfh_off + 28])[0]
        struct.pack_into('<H', d, lfh_off + 26, (orig_len + i + 1) & 0xFFFF)
    w(f'D-lfh-cdr-mismatch-{i:03d}.apk', bytes(d))
    count += 1

# ── E: ZIP64-wrong-offset (50 variants) ──────────────────────────────────────
# Inject a ZIP64 EOCD locator just before the EOCD with a wrong CD offset.
Z64_EOCD_LOC_SIG = b'\x50\x4b\x06\x07'
z64_cd_offset_base = cd_off + 0x1000
for i in range(50):
    # Build a fake ZIP64 EOCD locator (20 bytes)
    bad_z64_off = (z64_cd_offset_base + i * 0x100) & 0xFFFFFFFFFFFFFFFF
    locator = (Z64_EOCD_LOC_SIG
               + struct.pack('<I', 0)           # disk with start
               + struct.pack('<Q', bad_z64_off) # relative offset
               + struct.pack('<I', 1))          # total disks
    result = base[:eocd_off] + locator + base[eocd_off:]
    w(f'E-zip64-wrong-offset-{i:03d}.apk', result)
    count += 1

# ── F: empty-sigblock (50 variants) ──────────────────────────────────────────
# Replace the signing block region with a minimal block that has zero pairs.
# Minimum valid empty block: 8 (sob_leading) + 0 (pairs) + 8 (sob_trailing) + 16 (magic) = 32
min_sob = 24  # 8 sob_trailing + 16 magic — trailing sob is size of block body + magic
# Actually sob = size of block including magic but not the leading u64 copy.
# Canonical: leading_u64 = sob = sob_trailing = size of everything after the leading u64
# to end of block excluding trailing u64: so pairs region + 8 + 16 = 24 for empty.
for i in range(50):
    # vary: add i bytes of zero padding as "junk pair data" that is incomplete
    pad = b'\x00' * i
    empty_sob = 8 + len(pad) + 16  # trailing u64 + pad + magic
    empty_block = (struct.pack('<Q', empty_sob)   # leading size
                   + pad
                   + struct.pack('<Q', empty_sob)  # trailing size
                   + MAGIC)
    new_apk = base[:block_off] + empty_block + base[cd_off:]
    # patch EOCD to point to new cd_off
    new_cd_off = block_off + len(empty_block)
    ea = find_eocd(new_apk)
    if ea >= 0 and ea + 20 <= len(new_apk):
        d = bytearray(new_apk)
        struct.pack_into('<I', d, ea + 16, new_cd_off)
        new_apk = bytes(d)
    w(f'F-empty-sigblock-{i:03d}.apk', new_apk)
    count += 1

# ── G: zeros-sigblock (50 variants) ──────────────────────────────────────────
# Overwrite the signing block with zeroes of varying lengths.
for i in range(50):
    zero_len = (block_off if block_off > 0 else 1)
    # zero out `block_size + i` bytes of the block region
    block_size = cd_off - block_off
    fill = b'\x00' * min(block_size + i, len(base) - block_off)
    d = bytearray(base)
    d[block_off:block_off + len(fill)] = fill
    w(f'G-zeros-sigblock-{i:03d}.apk', bytes(d))
    count += 1

# ── H: bad-magic-variants (50 variants, 1 bit each) ──────────────────────────
magic_off = cd_off - 16
for i in range(50):
    d = bytearray(base)
    byte_idx = i // 8
    bit_idx  = i %  8
    if magic_off + byte_idx < len(d):
        d[magic_off + byte_idx] ^= (1 << bit_idx)
    w(f'H-bad-magic-bit{i:03d}.apk', bytes(d))
    count += 1

# ── I: oversized-comment (50 variants) ───────────────────────────────────────
for i in range(1, 51):
    d = bytearray(base)
    # Set comment length to i bytes more than the file has
    existing_comment_len = struct.unpack('<H', d[eocd_off + 20:eocd_off + 22])[0]
    new_len = (existing_comment_len + i) & 0xFFFF
    struct.pack_into('<H', d, eocd_off + 20, new_len)
    w(f'I-oversized-comment-{i:03d}.apk', bytes(d))
    count += 1

# ── J: mismatched-size (50 variants, delta +1..+50) ──────────────────────────
for i in range(1, 51):
    d = bytearray(base)
    # leading u64 stays at sob, trailing u64 set to sob + i
    trailing_off = cd_off - 24
    struct.pack_into('<Q', d, trailing_off, sob + i)
    w(f'J-size-mismatch-delta{i:03d}.apk', bytes(d))
    count += 1

print(f'generated {count} adversarial APKs → {OUT}')
assert count == 500, f'expected 500, got {count}'
