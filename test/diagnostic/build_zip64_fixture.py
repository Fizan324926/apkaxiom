#!/usr/bin/env python3
"""build_zip64_fixture.py — build a minimal ZIP64 archive for parser tests.

The archive contains a single small entry but uses the ZIP64 EOCD record
(0x06064b50) and ZIP64 EOCD locator (0x07064b50) with the canonical EOCD's
cd_offset / cd_size fields set to the 0xFFFFFFFF sentinel — i.e. it forces
the parser through the ZIP64 trailer path even though the archive itself
is small.

Use: python3 test/diagnostic/build_zip64_fixture.py [output_path]
"""

import struct
import sys
from pathlib import Path


def build_zip64() -> bytes:
    name = b"hello.txt"
    body = b"hello, world!\n"
    crc = 0x58988D13  # zlib.crc32(body) — pre-computed

    # Local File Header.
    lfh = (
        b"PK\x03\x04"
        + struct.pack("<H", 20)   # version needed
        + struct.pack("<H", 0)    # gp flags
        + struct.pack("<H", 0)    # method (stored)
        + struct.pack("<HH", 0, 0)
        + struct.pack("<I", crc)
        + struct.pack("<I", len(body))
        + struct.pack("<I", len(body))
        + struct.pack("<H", len(name))
        + struct.pack("<H", 0)
        + name
    )
    body_section = lfh + body

    # Central Directory Record.
    cdr = (
        b"PK\x01\x02"
        + struct.pack("<HH", 45, 45)
        + struct.pack("<H", 0)
        + struct.pack("<H", 0)
        + struct.pack("<HH", 0, 0)
        + struct.pack("<I", crc)
        + struct.pack("<I", len(body))
        + struct.pack("<I", len(body))
        + struct.pack("<H", len(name))
        + struct.pack("<HH", 0, 0)
        + struct.pack("<HH", 0, 0)
        + struct.pack("<I", 0)
        + struct.pack("<I", 0)  # lfh_offset
        + name
    )

    cd_offset = len(body_section)
    cd_size = len(cdr)

    # ZIP64 End-Of-Central-Directory Record.
    z64_eocd = (
        b"PK\x06\x06"
        + struct.pack("<Q", 44)         # size_of_zip64_eocd_record (excl. sig+this field)
        + struct.pack("<HH", 45, 45)    # version
        + struct.pack("<II", 0, 0)      # disk numbers
        + struct.pack("<Q", 1)          # entries on this disk
        + struct.pack("<Q", 1)          # total entries
        + struct.pack("<Q", cd_size)    # cd_size (true 64-bit)
        + struct.pack("<Q", cd_offset)  # cd_offset (true 64-bit)
    )

    z64_eocd_offset = cd_offset + cd_size

    # ZIP64 EOCD Locator.
    z64_locator = (
        b"PK\x06\x07"
        + struct.pack("<I", 0)            # disk number
        + struct.pack("<Q", z64_eocd_offset)
        + struct.pack("<I", 1)            # total disks
    )

    # Canonical EOCD with sentinel.
    eocd = (
        b"PK\x05\x06"
        + struct.pack("<HH", 0, 0)
        + struct.pack("<HH", 0xFFFF, 0xFFFF)  # entries (sentinel)
        + struct.pack("<I", 0xFFFFFFFF)        # cd_size sentinel
        + struct.pack("<I", 0xFFFFFFFF)        # cd_offset sentinel
        + struct.pack("<H", 0)
    )

    return body_section + cdr + z64_eocd + z64_locator + eocd


def main() -> int:
    out = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("test/input/zip64-fixture.apk")
    out.parent.mkdir(parents=True, exist_ok=True)
    blob = build_zip64()
    out.write_bytes(blob)
    print(f"wrote ZIP64 fixture to {out} ({len(blob)} bytes)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
