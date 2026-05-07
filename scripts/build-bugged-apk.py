#!/usr/bin/env python3
"""build-bugged-apk.py — generate three structural-attack APKs that exercise
attack classes APKAXIOM is designed to catch but signature-only verifiers miss.

Outputs to corpus/bugged/:

  1. dual-eocd.apk
     Two complete End-Of-Central-Directory records glued together. The first
     EOCD points to a "benign" CDR (package com.example.benign); a second EOCD
     follows that points to a "malicious" CDR (package com.example.malicious).
     Per the ZIP spec a parser should locate the EOCD by scanning back from
     end-of-file, so different parsers may disagree on which view of the
     archive is canonical. Classic Android Master Key class of bug.

  2. overlapping-entries.apk
     Two Local-File-Header entries with the same name (AndroidManifest.xml)
     but different content. APKAXIOM's L0 ZIP parser surfaces the duplicate;
     most signature verifiers and analyzers silently take the last-write-wins.

  3. sigblock-tamper.apk
     A real F-Droid APK where one byte of the unsigned trailing region (after
     the central directory, before EOCD comment field) is flipped. APK v2/v3
     signatures cover the file by hashing chunks but exclude the signing block
     itself; flipping a byte in the EOCD comment region (or padding gap) does
     not invalidate apksigner's verification, but APKAXIOM's BLAKE3 file root
     diverges from the baseline — surfacing supply-chain tamper that signature
     verification cannot detect.
"""
import io
import os
import struct
import sys
import zipfile

OUT_DIR = '/root/apkaxiom/corpus/bugged'
os.makedirs(OUT_DIR, exist_ok=True)


# ────────────────────────────────────────────────────────────────────────────
# Minimal AXML AndroidManifest.xml builders.
# ────────────────────────────────────────────────────────────────────────────
def fake_axml_manifest(package: str) -> bytes:
    """Produce a marker manifest blob — package name appears in plain bytes
    so any string-scanner picks up the divergence. Not a parseable AXML; the
    point is that APKAXIOM's L0 parser sees the entry exists with these bytes,
    while a tool that demands valid AXML may fail or fall back."""
    # AXML-ish header (3 00 08 00) so file(1) reports it as Android resource
    # then a UTF-8 marker payload with the package name.
    header = b'\x03\x00\x08\x00' + b'\x00' * 4
    payload = f'<manifest package="{package}"/>'.encode('utf-8')
    return header + payload + b'\x00' * 32


def fake_dex(marker: str) -> bytes:
    """Minimal DEX-like blob (magic + marker)."""
    return b'dex\n035\x00' + marker.encode('utf-8').ljust(108, b'\x00')


# ────────────────────────────────────────────────────────────────────────────
# Helper: build a self-consistent ZIP from a dict of {name: bytes}.
# Returns the bytes of the ZIP.
# ────────────────────────────────────────────────────────────────────────────
def build_zip(entries: dict[str, bytes]) -> bytes:
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, 'w', compression=zipfile.ZIP_STORED) as zf:
        for name, data in entries.items():
            zf.writestr(name, data)
    return buf.getvalue()


# ────────────────────────────────────────────────────────────────────────────
# 1. Dual-EOCD APK.
# ────────────────────────────────────────────────────────────────────────────
def build_dual_eocd(out_path: str) -> dict:
    """Concatenate two complete ZIPs. The first ZIP holds the "benign" view;
    the second ZIP's content is appended bytewise after the first ZIP's EOCD.

    A parser that finds the LAST EOCD (most lenient ZIP libs do, including
    Python zipfile and many Android tools) will see the malicious view.
    A parser that takes the FIRST EOCD will see the benign view.
    APKAXIOM's L0 layer surfaces the dual-EOCD condition explicitly.
    """
    benign = build_zip({
        'AndroidManifest.xml': fake_axml_manifest('com.example.benign'),
        'classes.dex': fake_dex('BENIGN_PAYLOAD'),
    })
    malicious = build_zip({
        'AndroidManifest.xml': fake_axml_manifest('com.example.malicious'),
        'classes.dex': fake_dex('MALICIOUS_PAYLOAD_pwn'),
    })
    combined = benign + malicious
    with open(out_path, 'wb') as f:
        f.write(combined)
    return {
        'path': out_path,
        'size': len(combined),
        'benign_eocd_offset': len(benign) - 22,
        'malicious_eocd_offset': len(combined) - 22,
    }


# ────────────────────────────────────────────────────────────────────────────
# 2. Overlapping-entries APK.
#    Two LFH entries with the same name but different content.
#    Most ZIP libraries return the last entry on lookup but the first one is
#    on disk too — APKAXIOM surfaces the duplicate as an integrity finding.
# ────────────────────────────────────────────────────────────────────────────
def build_overlapping_entries(out_path: str) -> dict:
    """Build a ZIP where AndroidManifest.xml appears twice with different
    contents. The CDR lists both. Different consumers will disagree on which
    one is "the" manifest.
    """
    # Hand-craft because zipfile.writestr does not allow duplicate names.
    benign_manifest = fake_axml_manifest('com.example.benign')
    malicious_manifest = fake_axml_manifest('com.example.malicious')

    parts = []
    cdrs = []
    offset = 0

    def lfh(name: bytes, data: bytes) -> bytes:
        # Local File Header (signature 0x04034b50), STORED, no extra/data desc.
        # crc32 left zero — non-spec-strict but illustrative; consumers that
        # validate CRC will reject; consumers that don't will accept.
        crc = 0
        return (
            b'PK\x03\x04'                   # signature
            + struct.pack('<H', 20)         # version needed
            + struct.pack('<H', 0)          # gp flag
            + struct.pack('<H', 0)          # method = STORED
            + struct.pack('<HH', 0, 0)      # mtime/mdate
            + struct.pack('<I', crc)        # crc32
            + struct.pack('<I', len(data))  # csize
            + struct.pack('<I', len(data))  # usize
            + struct.pack('<H', len(name))  # name len
            + struct.pack('<H', 0)          # extra len
            + name
            + data
        )

    def cdr(name: bytes, data: bytes, lfh_offset: int) -> bytes:
        # Central Directory Record (signature 0x02014b50)
        return (
            b'PK\x01\x02'                   # signature
            + struct.pack('<H', 20)         # version made by
            + struct.pack('<H', 20)         # version needed
            + struct.pack('<H', 0)          # gp flag
            + struct.pack('<H', 0)          # method
            + struct.pack('<HH', 0, 0)      # mtime/mdate
            + struct.pack('<I', 0)          # crc32
            + struct.pack('<I', len(data))  # csize
            + struct.pack('<I', len(data))  # usize
            + struct.pack('<H', len(name))  # name len
            + struct.pack('<H', 0)          # extra len
            + struct.pack('<H', 0)          # comment len
            + struct.pack('<H', 0)          # disk #
            + struct.pack('<H', 0)          # internal attrs
            + struct.pack('<I', 0)          # external attrs
            + struct.pack('<I', lfh_offset) # local header offset
            + name
        )

    name = b'AndroidManifest.xml'

    # Entry 1 (benign)
    e1_offset = offset
    e1 = lfh(name, benign_manifest)
    parts.append(e1)
    offset += len(e1)
    cdrs.append(cdr(name, benign_manifest, e1_offset))

    # Entry 2 (malicious) — same name
    e2_offset = offset
    e2 = lfh(name, malicious_manifest)
    parts.append(e2)
    offset += len(e2)
    cdrs.append(cdr(name, malicious_manifest, e2_offset))

    # Plus a non-duplicate dex entry for realism
    dex_data = fake_dex('OVERLAP_DEMO')
    dex_offset = offset
    dex_lfh = lfh(b'classes.dex', dex_data)
    parts.append(dex_lfh)
    offset += len(dex_lfh)
    cdrs.append(cdr(b'classes.dex', dex_data, dex_offset))

    # Concatenate LFH section.
    lfh_section = b''.join(parts)
    cdr_section = b''.join(cdrs)
    cdr_offset = len(lfh_section)

    # EOCD
    eocd = (
        b'PK\x05\x06'                       # signature
        + struct.pack('<H', 0)              # this disk
        + struct.pack('<H', 0)              # disk w/ CDR start
        + struct.pack('<H', len(cdrs))      # entries on this disk
        + struct.pack('<H', len(cdrs))      # total entries
        + struct.pack('<I', len(cdr_section))
        + struct.pack('<I', cdr_offset)
        + struct.pack('<H', 0)              # comment len
    )

    blob = lfh_section + cdr_section + eocd
    with open(out_path, 'wb') as f:
        f.write(blob)
    return {
        'path': out_path,
        'size': len(blob),
        'duplicate_name': 'AndroidManifest.xml',
        'entry_count_in_cdr': len(cdrs),
    }


# ────────────────────────────────────────────────────────────────────────────
# 3. Sig-block / unsigned-region tamper.
#    Take a real APK; find the last byte before the EOCD signature; flip it.
#    APK v2 signing covers content+CDR via chunked SHA-256 and SKIPS the
#    APK Signing Block itself. Flipping a byte in the EOCD's comment-length
#    field (zero-comment APKs) or in unused padding does not invalidate the
#    signature, but APKAXIOM's whole-file BLAKE3 root sees it.
# ────────────────────────────────────────────────────────────────────────────
def build_sigblock_tamper(base_apk: str, out_path: str) -> dict:
    """Strategy: tamper inside the APK Signing Block — the only region of an
    APK v2-signed file that is excluded from the v2 chunked-SHA256 digest.

    APK Signing Block layout:
      uint64 size_of_block (excluding this field)
      [pair: uint64 length  +  uint32 id  +  bytes value]+
      uint64 size_of_block  (repeated)
      bytes  magic = "APK Sig Block 42"

    apksigner verifies the v2 signer pair (id 0x7109871a) and ignores
    unknown-id pairs. We append a new pair with a custom id that apksigner
    treats as "additional attribute" / unknown, fix up the block size, and
    rebuild the EOCD's CDR-offset accordingly.

    Result: apksigner verifies (signature still binds to content+CDR);
    APKAXIOM's L1 sigblock parser surfaces an UNKNOWN_PAIR_INSERTION finding,
    and the whole-file BLAKE3 fingerprint diverges from the upstream baseline.
    """
    with open(base_apk, 'rb') as f:
        data = bytearray(f.read())

    # 1. Find EOCD.
    eocd_off = data.rfind(b'PK\x05\x06')
    if eocd_off < 0:
        raise RuntimeError('no EOCD')
    cdr_offset = struct.unpack_from('<I', data, eocd_off + 16)[0]
    cdr_size = struct.unpack_from('<I', data, eocd_off + 12)[0]

    # 2. Find APK Signing Block. magic at cdr_offset - 16.
    magic = b'APK Sig Block 42'
    if bytes(data[cdr_offset - 16:cdr_offset]) != magic:
        raise RuntimeError('no v2 sig block magic at expected offset')
    block_size = struct.unpack_from('<Q', data, cdr_offset - 24)[0]
    block_start = cdr_offset - 8 - block_size  # leading size_of_block field
    leading_size = struct.unpack_from('<Q', data, block_start)[0]
    if leading_size != block_size:
        raise RuntimeError('sig block leading/trailing size mismatch')

    # 3. Inject an extra pair just before the trailing size_of_block field.
    #    Pair: uint64 length, uint32 id, bytes value
    custom_id = 0xDEADBEEF  # apksigner treats unknown ids as benign attrs
    custom_value = b'AXIOM_SIGBLOCK_TAMPER_DEMO'
    pair_len = 4 + len(custom_value)  # id + value
    pair = struct.pack('<Q', pair_len) + struct.pack('<I', custom_id) + custom_value
    pair_total = 8 + pair_len  # length-prefix included

    # Inject pair at offset (cdr_offset - 24) — i.e. just before trailing size+magic.
    inject_off = cdr_offset - 24
    new_data = bytearray(data[:inject_off]) + pair + bytearray(data[inject_off:])

    # 4. Update block sizes (leading and trailing) and EOCD CDR offset.
    new_block_size = block_size + pair_total
    struct.pack_into('<Q', new_data, block_start, new_block_size)
    new_cdr_offset = cdr_offset + pair_total
    # Trailing size lives at new_cdr_offset - 24
    struct.pack_into('<Q', new_data, new_cdr_offset - 24, new_block_size)
    # EOCD CDR offset moves by pair_total
    new_eocd_off = eocd_off + pair_total
    struct.pack_into('<I', new_data, new_eocd_off + 16, new_cdr_offset)

    with open(out_path, 'wb') as f:
        f.write(bytes(new_data))
    return {
        'path': out_path,
        'base': base_apk,
        'original_size': len(data),
        'tampered_size': len(new_data),
        'pair_id_hex': f'0x{custom_id:08x}',
        'pair_value_bytes': len(custom_value),
        'old_block_size': block_size,
        'new_block_size': new_block_size,
    }


# ────────────────────────────────────────────────────────────────────────────
def main() -> int:
    base_apk = '/root/apkaxiom/corpus/bench-10k/real-fdroid/acab.naiveha.subrosa_7.apk'
    if not os.path.exists(base_apk):
        # Pick whatever exists.
        candidates = sorted(
            os.path.join('/root/apkaxiom/corpus/bench-10k/real-fdroid', x)
            for x in os.listdir('/root/apkaxiom/corpus/bench-10k/real-fdroid')
            if x.endswith('.apk')
        )
        if not candidates:
            print('ERROR: no real F-Droid APK available for sigblock-tamper')
            return 1
        base_apk = candidates[0]

    r1 = build_dual_eocd(os.path.join(OUT_DIR, 'dual-eocd.apk'))
    r2 = build_overlapping_entries(os.path.join(OUT_DIR, 'overlapping-entries.apk'))
    r3 = build_sigblock_tamper(base_apk, os.path.join(OUT_DIR, 'sigblock-tamper.apk'))

    for r in (r1, r2, r3):
        print(r)
    return 0


if __name__ == '__main__':
    sys.exit(main())
