#!/usr/bin/env python3
"""fuzz_androguard.py — generate structurally malformed APKs and feed them
to Androguard 4.1.3 to discover crash classes.

Each generator emits a ZIP-shaped blob with one or more invariant
violations, then we drive the most common Androguard API entry points:
    APK(path)
    APK(path).get_androidversion_name()
    AnalyzeAPK(path)

Any exception that escapes the call is recorded as a candidate DoS
vector. Crashes are minimised by binary search over relevant fields
where possible.

Output: a deduplicated list of (crash_class, top_frame, minimal_apk_bytes).
"""
import io
import os
import struct
import sys
import traceback
import logging
import zipfile
from pathlib import Path

logging.disable(logging.CRITICAL)


def silent_call(fn, *args, **kwargs):
    """Run fn capturing crash + top androguard frame."""
    devnull_fd = os.open(os.devnull, os.O_WRONLY)
    old_stderr = os.dup(2)
    os.dup2(devnull_fd, 2)
    try:
        try:
            fn(*args, **kwargs)
            return None
        except Exception as e:
            tb = traceback.extract_tb(sys.exc_info()[2])
            af = [f for f in tb if "androguard" in f.filename]
            loc = af[-1] if af else tb[-1]
            return (
                type(e).__name__,
                str(e)[:120],
                f"{loc.filename.split('/site-packages/')[-1]}:{loc.lineno}",
            )
    finally:
        os.dup2(old_stderr, 2)
        os.close(old_stderr)
        os.close(devnull_fd)


def test_apk_bytes(blob: bytes) -> dict:
    """Run all three entry points on the same blob bytes."""
    import tempfile

    fd, path = tempfile.mkstemp(suffix=".apk")
    os.write(fd, blob)
    os.close(fd)
    out = {}
    try:
        # APK constructor only
        from androguard.core.apk import APK

        out["APK_ctor"] = silent_call(APK, path)
        # APK + get_androidversion_name (the most-called accessor)
        def _gav(p):
            APK(p).get_androidversion_name()

        out["APK_get_name"] = silent_call(_gav, path)
        # AnalyzeAPK (full pipeline)
        from androguard.misc import AnalyzeAPK

        out["AnalyzeAPK"] = silent_call(AnalyzeAPK, path)
    finally:
        os.unlink(path)
    return out


# ── Generators ───────────────────────────────────────────────────────────────

def gen_minimal_apk_with_axml(package="com.example") -> bytes:
    """A well-formed minimal APK that should not crash anything."""
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w", compression=zipfile.ZIP_STORED) as zf:
        zf.writestr("AndroidManifest.xml", b"\x03\x00\x08\x00<manifest/>")
        zf.writestr("classes.dex", b"dex\n035\x00" + b"\x00" * 100)
    return buf.getvalue()


def gen_eocd_only() -> bytes:
    """File containing only an EOCD record — no LFHs, no CDR."""
    return (
        b"PK\x05\x06"
        + struct.pack("<HH", 0, 0)
        + struct.pack("<HH", 0, 0)
        + struct.pack("<II", 0, 0)
        + struct.pack("<H", 0)
    )


def gen_truncated_eocd() -> bytes:
    """EOCD signature but only 10 bytes of fixed prefix (truncated)."""
    return b"PK\x05\x06" + b"\x00" * 6


def gen_eocd_giant_comment() -> bytes:
    """EOCD with comment_len = 0xFFFF but file ends immediately after."""
    return (
        b"PK\x05\x06"
        + struct.pack("<HH", 0, 0)
        + struct.pack("<HH", 0, 0)
        + struct.pack("<II", 0, 0)
        + struct.pack("<H", 0xFFFF)  # huge comment, no body
    )


def gen_cd_offset_past_eof() -> bytes:
    """EOCD declares cd_offset past end of file."""
    return (
        b"PK\x05\x06"
        + struct.pack("<HH", 0, 0)
        + struct.pack("<HH", 1, 1)
        + struct.pack("<II", 100, 0xFFFFFFFE)
        + struct.pack("<H", 0)
    )


def gen_cd_size_overflow() -> bytes:
    """cd_size = 0xFFFFFFFE — implausibly large central directory."""
    return (
        b"PK\x05\x06"
        + struct.pack("<HH", 0, 0)
        + struct.pack("<HH", 1, 1)
        + struct.pack("<II", 0xFFFFFFFE, 0)
        + struct.pack("<H", 0)
    )


def gen_max_entries_eocd() -> bytes:
    """EOCD claims 0xFFFE entries but file is empty."""
    return (
        b"PK\x05\x06"
        + struct.pack("<HH", 0, 0)
        + struct.pack("<HH", 0xFFFE, 0xFFFE)
        + struct.pack("<II", 0, 0)
        + struct.pack("<H", 0)
    )


def gen_empty_zip_signature_only() -> bytes:
    """Just the ZIP local-file-header signature, nothing else."""
    return b"PK\x03\x04"


def gen_lfh_giant_name() -> bytes:
    """LFH declares name_len = 0xFFFF but file ends mid-name."""
    return (
        b"PK\x03\x04"
        + struct.pack("<H", 20)
        + struct.pack("<HH", 0, 0)
        + struct.pack("<HH", 0, 0)
        + struct.pack("<I", 0)
        + struct.pack("<II", 0, 0)
        + struct.pack("<HH", 0xFFFF, 0)  # claims 65535-byte name
        # ... no name bytes follow
    )


def gen_lfh_giant_csize() -> bytes:
    """LFH declares csize = 0xFFFFFFFE."""
    name = b"x"
    return (
        b"PK\x03\x04"
        + struct.pack("<H", 20)
        + struct.pack("<HH", 0, 0)
        + struct.pack("<HH", 0, 0)
        + struct.pack("<I", 0)
        + struct.pack("<II", 0xFFFFFFFE, 0xFFFFFFFE)
        + struct.pack("<HH", len(name), 0)
        + name
    )


def gen_zero_byte() -> bytes:
    """One byte file."""
    return b"\x00"


def gen_zero_bytes() -> bytes:
    """Empty file."""
    return b""


def gen_apk_with_dex_starting_zeros(blob_size=64) -> bytes:
    """APK whose classes.dex body is all zeros (DEX endian tag = 0x00000000)."""
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w", compression=zipfile.ZIP_STORED) as zf:
        zf.writestr("AndroidManifest.xml", b"\x03\x00\x08\x00<manifest/>")
        zf.writestr("classes.dex", b"\x00" * blob_size)
    return buf.getvalue()


def gen_zip_with_no_cdr() -> bytes:
    """LFH + body + EOCD, but no CDR records in between (cd_offset 0,
    cd_size 0). Some parsers require cd_size > 0."""
    name = b"a"
    body = b"x"
    lfh = (
        b"PK\x03\x04"
        + struct.pack("<H", 20)
        + struct.pack("<HH", 0, 0)
        + struct.pack("<HH", 0, 0)
        + struct.pack("<I", 0)
        + struct.pack("<II", len(body), len(body))
        + struct.pack("<HH", len(name), 0)
        + name
        + body
    )
    eocd = (
        b"PK\x05\x06"
        + struct.pack("<HH", 0, 0)
        + struct.pack("<HH", 0, 0)  # 0 entries
        + struct.pack("<II", 0, 0)  # 0 cd_size, 0 cd_offset
        + struct.pack("<H", 0)
    )
    return lfh + eocd


GENERATORS = [
    ("minimal-good",          gen_minimal_apk_with_axml),
    ("zero-bytes",            gen_zero_bytes),
    ("zero-byte",             gen_zero_byte),
    ("eocd-only",             gen_eocd_only),
    ("truncated-eocd",        gen_truncated_eocd),
    ("eocd-giant-comment",    gen_eocd_giant_comment),
    ("cd-offset-past-eof",    gen_cd_offset_past_eof),
    ("cd-size-overflow",      gen_cd_size_overflow),
    ("max-entries",           gen_max_entries_eocd),
    ("lfh-only",              gen_empty_zip_signature_only),
    ("lfh-giant-name",        gen_lfh_giant_name),
    ("lfh-giant-csize",       gen_lfh_giant_csize),
    ("zip-no-cdr",            gen_zip_with_no_cdr),
    ("dex-all-zeros",         gen_apk_with_dex_starting_zeros),
]


def main() -> int:
    out_dir = Path("/tmp/cve-hunt/fuzz-corpus")
    out_dir.mkdir(parents=True, exist_ok=True)

    print(f"{'case':<22} {'size':>8}  {'APK_ctor':<48}  {'APK_get_name':<48}  {'AnalyzeAPK':<48}")
    print("-" * 180)

    crash_db: dict[tuple[str, str], list[tuple[str, str, int]]] = {}

    for name, gen in GENERATORS:
        try:
            blob = gen()
        except Exception as e:
            print(f"  generator-failed {name}: {e}")
            continue
        out_dir.joinpath(f"{name}.apk").write_bytes(blob)
        results = test_apk_bytes(blob)

        def fmt(r):
            if r is None:
                return "OK"
            return f"{r[0]} ({r[2]}): {r[1][:24]}"

        print(
            f"{name:<22} {len(blob):>8,}  "
            f"{fmt(results['APK_ctor']):<48}  "
            f"{fmt(results['APK_get_name']):<48}  "
            f"{fmt(results['AnalyzeAPK']):<48}"
        )

        for entry, crash in results.items():
            if crash is None:
                continue
            key = (crash[0], crash[2])  # (exception class, source location)
            crash_db.setdefault(key, []).append((name, entry, len(blob)))

    print()
    print(f"Distinct (exception, location) pairs found: {len(crash_db)}")
    for (exc, loc), trips in sorted(crash_db.items()):
        smallest = min(trips, key=lambda t: t[2])
        print(f"  {exc:<28}  {loc:<48}  smallest reproducer: {smallest[0]} "
              f"({smallest[2]:,} bytes via {smallest[1]})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
