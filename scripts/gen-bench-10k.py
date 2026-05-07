#!/usr/bin/env python3
"""Generate synthetic benchmark APKs for APKAXIOM Bench-10K KPI evaluation.

Produces valid ZIP files readable by axiom-l1-rs. Each APK contains:
  - AndroidManifest.xml  (minimal AXML stub, 2-byte magic 0x0300)
  - classes.dex          (DEX magic dex\n035\0 + zero header)
  - assets/data_NNNN.bin (padding to reach target size)

No real DEX bytecode or resources. The pipeline only needs the ZIP structure
and the two magic-probed entries to exercise L0+L1 fully.
"""
import argparse
import hashlib
import os
import random
import struct
import zipfile

# Minimal AXML: RES_XML_TYPE=0x0003, header_size=0x0008, file_size=0x001c,
# then a minimal string-pool chunk (just enough to not be zero-length).
AXML_STUB = struct.pack('<HHI', 0x0003, 0x0008, 0x001c) + b'\x00' * 16

# DEX magic + 104 zero bytes = 112-byte minimal DEX header stub.
DEX_STUB = b'dex\n035\x00' + b'\x00' * 104


def make_apk(path: str, target_size: int, seed: int) -> int:
    """Write one synthetic APK; return actual file size."""
    rng = random.Random(seed)
    with zipfile.ZipFile(path, 'w', compression=zipfile.ZIP_STORED,
                         allowZip64=False) as zf:
        zf.writestr('AndroidManifest.xml', AXML_STUB)
        zf.writestr('classes.dex', DEX_STUB)
        # Padding: split into ≤64 KB chunks to keep the CDR manageable.
        overhead = 600  # rough ZIP overhead
        remaining = max(0, target_size - overhead - len(AXML_STUB) - len(DEX_STUB))
        chunk_size = 65_536
        part = 0
        while remaining > 0:
            n = min(chunk_size, remaining)
            # Use random bytes so DEFLATE doesn't compress to near-zero.
            data = bytes(rng.getrandbits(8) for _ in range(n))
            zf.writestr(f'assets/data_{part:04d}.bin', data)
            remaining -= n
            part += 1
    return os.path.getsize(path)


def sha256_file(path: str) -> str:
    h = hashlib.sha256()
    with open(path, 'rb') as f:
        for chunk in iter(lambda: f.read(65536), b''):
            h.update(chunk)
    return h.hexdigest()


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--out', default='corpus/bench-10k/synthetic',
                    help='Output directory')
    ap.add_argument('--small',  type=int, default=3000,
                    help='Number of small APKs  (50 KB – 500 KB)')
    ap.add_argument('--medium', type=int, default=5000,
                    help='Number of medium APKs (1 MB – 10 MB)')
    ap.add_argument('--large',  type=int, default=1500,
                    help='Number of large APKs  (10 MB – 30 MB)')
    ap.add_argument('--seed', type=int, default=42)
    args = ap.parse_args()

    os.makedirs(args.out, exist_ok=True)
    rng = random.Random(args.seed)

    strata = [
        ('small',  args.small,     50_000,    500_000),
        ('medium', args.medium,  1_000_000, 10_000_000),
        ('large',  args.large,  10_000_000, 30_000_000),
    ]

    total = sum(c for _, c, _, _ in strata)
    idx = 0
    toml_lines = [
        '[corpus]',
        'name = "APKAXIOM-Bench-10K-Synthetic"',
        'generated = "2026-05-07"',
        'generator = "scripts/gen-bench-10k.py"',
        '',
    ]

    for category, count, lo, hi in strata:
        print(f'Generating {count} {category} APKs [{lo//1024}KB – {hi//1024//1024}MB]…')
        for i in range(count):
            target = rng.randint(lo, hi)
            fname = f'{category}_{i:05d}.apk'
            path = os.path.join(args.out, fname)
            actual = make_apk(path, target, seed=idx)
            digest = sha256_file(path)
            toml_lines.append('[[apk]]')
            toml_lines.append(f'file     = "{fname}"')
            toml_lines.append(f'category = "{category}"')
            toml_lines.append(f'size     = {actual}')
            toml_lines.append(f'sha256   = "{digest}"')
            toml_lines.append('')
            idx += 1
            if idx % 500 == 0:
                print(f'  {idx}/{total} — {fname} ({actual//1024}KB)')

    manifest_path = os.path.join(args.out, 'corpus.toml')
    with open(manifest_path, 'w') as f:
        f.write('\n'.join(toml_lines))

    print(f'\nDone: {idx} APKs written to {args.out}/')
    print(f'Manifest: {manifest_path}')


if __name__ == '__main__':
    main()
