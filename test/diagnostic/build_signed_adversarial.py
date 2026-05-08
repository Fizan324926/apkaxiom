#!/usr/bin/env python3
"""build_signed_adversarial.py — produce signed adversarial APK fixtures.

Takes a real, v2-signed F-Droid APK as a base and applies three
research-grade structural mutations that preserve the signature on the
unsigned regions while injecting attack-class structures:

  1. signed-dual-eocd.apk
     The base APK with a second well-formed EOCD record appended.
     The second EOCD's cd_offset/cd_size point at fabricated CDR
     bytes embedded in the file's EOCD-comment region. Detection
     target: MULTIPLE_EOCD_RECORDS (G-3).

  2. signed-overlap.apk
     The base APK with the canonical AndroidManifest.xml replaced
     by two LFH entries of the same name (one benign, one rogue).
     The CDR is rebuilt with two CDRs both naming AndroidManifest.xml.
     Detection target: DUPLICATE_LFH_NAME (G-3).

  3. signed-lfh-cdr-mismatch.apk
     The base APK with the LFH csize/usize/crc fields tampered for
     one entry while leaving the CDR's fields intact. Master-Key
     CVE-9950697 class. Detection target: LFH_CDR_FIELD_MISMATCH (G-3).

These fixtures are NOT v2-signature-valid because the central directory
hash changes when bytes shift — but that's the correct behaviour: real
attackers know this and use sigblock-tamper (G-3 already covers that).
The point of these signed-base fixtures is to ground the structural
detectors against realistic byte sequences (real DEX, real AXML, real
resource tables) so the differential against Androguard/apksigner is
on equal footing.

Usage: python3 test/diagnostic/build_signed_adversarial.py [base_apk]
"""

import os
import struct
import sys
from pathlib import Path

DEFAULT_BASE = (
    "/root/apkaxiom/corpus/bench-10k/real-fdroid/"
    "acab.naiveha.subrosa_7.apk"
)
OUT_DIR = Path("/root/apkaxiom/test/input")


def find_eocd(data: bytes) -> int:
    return data.rfind(b"PK\x05\x06")


def parse_eocd(data: bytes, off: int) -> tuple[int, int, int]:
    n_entries = struct.unpack_from("<H", data, off + 10)[0]
    cd_size = struct.unpack_from("<I", data, off + 12)[0]
    cd_offset = struct.unpack_from("<I", data, off + 16)[0]
    return n_entries, cd_size, cd_offset


def build_dual_eocd(base: bytes, out_path: Path) -> dict:
    """Append a second EOCD just before the canonical one. The
    second EOCD references the same CDR (so it parses) but its
    presence triggers the dual-EOCD detector. This is more
    realistic than two concatenated ZIPs because real attackers
    layer attacks on top of legitimate, signed APKs."""
    eocd_off = find_eocd(base)
    n_entries, cd_size, cd_offset = parse_eocd(base, eocd_off)

    # Build a duplicate EOCD that points at the same CDR.
    rogue_eocd = (
        b"PK\x05\x06"
        + struct.pack("<HH", 0, 0)
        + struct.pack("<HH", n_entries, n_entries)
        + struct.pack("<I", cd_size)
        + struct.pack("<I", cd_offset)
        + struct.pack("<H", 0)  # comment len = 0
    )
    # Insert before the canonical EOCD.
    new = bytearray(base[:eocd_off]) + rogue_eocd + bytearray(base[eocd_off:])
    out_path.write_bytes(bytes(new))
    return {
        "path": str(out_path),
        "size": len(new),
        "rogue_eocd_at": eocd_off,
        "canonical_eocd_at": eocd_off + len(rogue_eocd),
    }


def build_overlap(base: bytes, out_path: Path) -> dict:
    """Append a second AndroidManifest.xml LFH (with rogue content)
    just before the central directory. Add a matching CDR record
    pointing at it. The original AndroidManifest LFH and CDR
    remain intact, so any consumer that walks the CDR sees two
    AndroidManifest entries."""
    # Locate AndroidManifest in the CDR.
    eocd_off = find_eocd(base)
    n_entries, cd_size, cd_offset = parse_eocd(base, eocd_off)

    # Walk CDR to find AndroidManifest.xml.
    cd = base[cd_offset:cd_offset + cd_size]
    target = b"AndroidManifest.xml"
    k = 0
    found_orig: tuple | None = None
    while k + 46 <= len(cd):
        if struct.unpack_from("<I", cd, k)[0] != 0x02014B50:
            break
        name_len = struct.unpack_from("<H", cd, k + 28)[0]
        extra_len = struct.unpack_from("<H", cd, k + 30)[0]
        cmt_len = struct.unpack_from("<H", cd, k + 32)[0]
        name = cd[k + 46 : k + 46 + name_len]
        if name == target:
            found_orig = (k, struct.unpack_from("<I", cd, k + 42)[0])  # cdr_off, lfh_off
            break
        k += 46 + name_len + extra_len + cmt_len

    if not found_orig:
        raise RuntimeError("base APK has no AndroidManifest.xml in CDR")

    # Build a rogue LFH+body to insert just before the original CDR.
    rogue_body = b"<rogue/>"
    rogue_lfh = (
        b"PK\x03\x04"
        + struct.pack("<H", 20)
        + struct.pack("<H", 0)
        + struct.pack("<H", 0)  # method = stored
        + struct.pack("<HH", 0, 0)
        + struct.pack("<I", 0)  # crc
        + struct.pack("<I", len(rogue_body))
        + struct.pack("<I", len(rogue_body))
        + struct.pack("<H", len(target))
        + struct.pack("<H", 0)
        + target
        + rogue_body
    )
    rogue_lfh_offset = cd_offset
    delta = len(rogue_lfh)

    # Build a corresponding CDR record for the rogue LFH.
    rogue_cdr = (
        b"PK\x01\x02"
        + struct.pack("<HH", 20, 20)
        + struct.pack("<H", 0)
        + struct.pack("<H", 0)
        + struct.pack("<HH", 0, 0)
        + struct.pack("<I", 0)
        + struct.pack("<I", len(rogue_body))
        + struct.pack("<I", len(rogue_body))
        + struct.pack("<H", len(target))
        + struct.pack("<HH", 0, 0)
        + struct.pack("<HH", 0, 0)
        + struct.pack("<I", 0)
        + struct.pack("<I", rogue_lfh_offset)
        + target
    )

    # Splice: pre-cdr bytes | rogue LFH | original CDR | rogue CDR | EOCD (updated).
    new = bytearray()
    new.extend(base[:cd_offset])
    new.extend(rogue_lfh)
    new.extend(cd)               # original CDR
    new.extend(rogue_cdr)        # appended CDR
    # Build new EOCD with correct counts/offsets.
    new_cd_offset = cd_offset + delta
    new_cd_size = cd_size + len(rogue_cdr)
    new_n_entries = n_entries + 1
    new_eocd = (
        b"PK\x05\x06"
        + struct.pack("<HH", 0, 0)
        + struct.pack("<HH", new_n_entries, new_n_entries)
        + struct.pack("<I", new_cd_size)
        + struct.pack("<I", new_cd_offset)
        + struct.pack("<H", 0)
    )
    new.extend(new_eocd)
    out_path.write_bytes(bytes(new))
    return {
        "path": str(out_path),
        "size": len(new),
        "duplicate_name": "AndroidManifest.xml",
        "entries": new_n_entries,
    }


def build_lfh_cdr_mismatch(base: bytes, out_path: Path) -> dict:
    """Bump the first entry's LFH csize by +1 byte without touching
    its CDR record. The CDR still says csize=N; the LFH says
    csize=N+1. Master-Key 9950697 class divergence."""
    eocd_off = find_eocd(base)
    _n_entries, cd_size, cd_offset = parse_eocd(base, eocd_off)
    cd = base[cd_offset:cd_offset + cd_size]
    if struct.unpack_from("<I", cd, 0)[0] != 0x02014B50:
        raise RuntimeError("first CDR record not a CDR")
    lfh_off = struct.unpack_from("<I", cd, 42)[0]
    if struct.unpack_from("<I", base, lfh_off)[0] != 0x04034B50:
        raise RuntimeError("first LFH offset doesn't point to LFH")
    cur = struct.unpack_from("<I", base, lfh_off + 18)[0]
    new = bytearray(base)
    struct.pack_into("<I", new, lfh_off + 18, cur + 1)
    out_path.write_bytes(bytes(new))
    return {
        "path": str(out_path),
        "size": len(new),
        "lfh_csize_old": cur,
        "lfh_csize_new": cur + 1,
        "lfh_offset": lfh_off,
    }


def main() -> int:
    base_path = Path(sys.argv[1] if len(sys.argv) > 1 else DEFAULT_BASE)
    if not base_path.exists():
        print(f"ERROR: base APK not found: {base_path}", file=sys.stderr)
        return 1
    base = base_path.read_bytes()
    OUT_DIR.mkdir(parents=True, exist_ok=True)

    r1 = build_dual_eocd(base, OUT_DIR / "signed-dual-eocd.apk")
    r2 = build_overlap(base, OUT_DIR / "signed-overlap.apk")
    r3 = build_lfh_cdr_mismatch(base, OUT_DIR / "signed-lfh-cdr-mismatch.apk")

    for r in (r1, r2, r3):
        print(r)
    return 0


if __name__ == "__main__":
    sys.exit(main())
