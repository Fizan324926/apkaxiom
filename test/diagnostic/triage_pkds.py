#!/usr/bin/env python3
"""triage_pkds.py — forensic triage of the unknown sigblock pair `0x504b4453`.

In a 1000-APK F-Droid scan, 25 APKs carry an `UNKNOWN_SIGBLOCK_PAIR` with
ID `0x504b4453` (ASCII `PKDS`). This is not in AOSP's published list of
APK Signing Block IDs. apksigner and Androguard accept these APKs without
warning. APKAXIOM surfaces the pair verbatim so analysts can decide.

This script extracts and characterises every PKDS value across a corpus
so the unknown can be triaged systematically. It does not claim a
verdict — it produces the data an analyst needs.

Output (one row per APK):
  apk path, pair_len, prefix_hash, first 32 bytes hex, value entropy

If every PKDS pair shares the same fixed prefix and the rest is
high-entropy, the pair is almost certainly cryptographic metadata from
some third-party signing pipeline (build attestation, store signing
scheme). If prefixes vary or entropy is low, that is suspicious.

Usage:
    python3 test/diagnostic/triage_pkds.py <corpus_dir>
"""
import collections
import hashlib
import math
import os
import struct
import sys


def shannon_entropy(buf: bytes) -> float:
    """Bits per byte. 8.0 = uniformly random; <4.0 = highly structured."""
    if not buf:
        return 0.0
    counts = [0] * 256
    for b in buf:
        counts[b] += 1
    n = len(buf)
    h = 0.0
    for c in counts:
        if c:
            p = c / n
            h -= p * math.log2(p)
    return h


def find_pkds(apk_bytes: bytes) -> bytes | None:
    """Return the value bytes of pair id 0x504b4453, or None."""
    eocd_off = apk_bytes.rfind(b"PK\x05\x06")
    if eocd_off < 0:
        return None
    cdr_off = struct.unpack_from("<I", apk_bytes, eocd_off + 16)[0]
    if cdr_off < 16 or apk_bytes[cdr_off - 16:cdr_off] != b"APK Sig Block 42":
        return None
    block_size = struct.unpack_from("<Q", apk_bytes, cdr_off - 24)[0]
    block_start = cdr_off - 8 - block_size
    off = block_start + 8
    end = cdr_off - 24
    while off < end:
        plen = struct.unpack_from("<Q", apk_bytes, off)[0]
        pid = struct.unpack_from("<I", apk_bytes, off + 8)[0]
        if pid == 0x504B4453:
            return bytes(apk_bytes[off + 12 : off + 8 + plen])
        off += 8 + plen
    return None


def main() -> int:
    if len(sys.argv) < 2:
        print(__doc__, file=sys.stderr)
        return 2
    corpus = sys.argv[1]

    rows = []
    prefix_counts: collections.Counter[bytes] = collections.Counter()

    for name in sorted(os.listdir(corpus)):
        if not name.endswith(".apk"):
            continue
        path = os.path.join(corpus, name)
        try:
            with open(path, "rb") as f:
                data = f.read()
        except OSError:
            continue
        value = find_pkds(data)
        if value is None:
            continue
        prefix = value[:5]
        prefix_counts[prefix] += 1
        rows.append({
            "apk": name,
            "pair_len": len(value),
            "prefix5_hex": prefix.hex(),
            "first32_hex": value[:32].hex(),
            "tail32_hex": value[-32:].hex(),
            "entropy_bits_per_byte": shannon_entropy(value),
            "value_sha256": hashlib.sha256(value).hexdigest(),
        })

    if not rows:
        print("No PKDS pairs found in corpus.", file=sys.stderr)
        return 1

    print(f"PKDS triage report — {len(rows)} APKs in {corpus}")
    print()
    print("Prefix-distribution (first 5 bytes shared across APKs):")
    for prefix, count in prefix_counts.most_common():
        share = 100.0 * count / len(rows)
        print(f"  {prefix.hex():<14}  {count:>4} APKs ({share:5.1f}%)")
    print()

    avg_entropy = sum(r["entropy_bits_per_byte"] for r in rows) / len(rows)
    print(f"Average entropy: {avg_entropy:.3f} bits/byte  "
          f"(8.000 = random, <4.000 = highly structured)")
    print()

    # Verdict heuristic — strict, only conclusions we can defend.
    one_prefix = len(prefix_counts) == 1
    high_entropy = avg_entropy >= 7.5
    distinct_values = len({r["value_sha256"] for r in rows}) == len(rows)

    print("Heuristic findings (defensible):")
    print(f"  All APKs share the same 5-byte prefix:    "
          f"{'YES' if one_prefix else 'NO'}")
    print(f"  Average entropy ≥ 7.5 bits/byte:          "
          f"{'YES' if high_entropy else 'NO'}")
    print(f"  Each APK's PKDS value is unique:          "
          f"{'YES' if distinct_values else 'NO'}")
    print()

    if one_prefix and high_entropy and distinct_values:
        print("CONCLUSION:")
        print("  The 5-byte prefix is a fixed format identifier; the")
        print("  remaining bytes are high-entropy and unique per APK.")
        print("  This pattern matches a deterministic per-APK cryptographic")
        print("  artefact — most likely a third-party signing-scheme pair")
        print("  emitted by a build pipeline (store re-signing, build")
        print("  attestation, source stamp). It is NOT consistent with a")
        print("  malicious watermark (which would more often share values")
        print("  across APKs from the same actor).")
        print()
        print("  Recommendation: treat 0x504b4453 as a low-risk informational")
        print("  finding; if attribution to a specific signer is required,")
        print("  a structural decoder for this pair format should be added.")
    else:
        print("CONCLUSION:")
        print("  Pattern is INCONSISTENT with a known signing-scheme")
        print("  artefact. Manual review recommended.")

    print()
    print("Per-APK detail (top 10 alphabetical):")
    for row in rows[:10]:
        print(f"  {row['apk']:<48}  len={row['pair_len']:<5}  "
              f"H={row['entropy_bits_per_byte']:5.2f}  "
              f"prefix={row['prefix5_hex']}")
    if len(rows) > 10:
        print(f"  ... {len(rows) - 10} more")

    return 0


if __name__ == "__main__":
    sys.exit(main())
