#!/usr/bin/env python3
"""
Divergence Test Harness v2

Re-tests each crafted PoC APK using --min-sdk-version to bypass the binary
manifest parsing issue, and also tests crafted mutations of REAL APKs.
"""

import subprocess
import os
import sys
import json
import zipfile
import struct
import zlib
import io
import shutil
import traceback

ANDROGUARD_PYTHON = '/root/security_research_tools/envs/main/bin/python3'
POC_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)))
REAL_APK_DIR = '/root/apkaxiom/corpus/bench-10k/real-fdroid'
MUTATED_DIR = os.path.join(POC_DIR, 'mutated_real')
os.makedirs(MUTATED_DIR, exist_ok=True)


def run_apksigner(apk_path, min_sdk=21):
    """Run apksigner verify with --min-sdk-version override."""
    result = {'tool': 'apksigner', 'verdict': 'UNKNOWN', 'errors': [], 'warnings': [],
              'raw': '', 'exit_code': -1}
    try:
        cmd = ['apksigner', 'verify', '--verbose', '--print-certs',
               '--min-sdk-version', str(min_sdk), apk_path]
        proc = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
        result['exit_code'] = proc.returncode
        result['raw'] = proc.stdout + proc.stderr
        combined = result['raw']

        if proc.returncode == 0:
            if 'WARNING' in combined:
                result['verdict'] = 'PASS_WITH_WARNINGS'
                for line in combined.split('\n'):
                    if 'WARNING' in line:
                        result['warnings'].append(line.strip())
            else:
                result['verdict'] = 'PASS'
        else:
            result['verdict'] = 'REJECT'
            for line in combined.split('\n'):
                if line.strip() and ('ERROR' in line or 'Exception' in line or 'DOES NOT VERIFY' in line):
                    result['errors'].append(line.strip())
    except subprocess.TimeoutExpired:
        result['verdict'] = 'TIMEOUT'
    except Exception as e:
        result['verdict'] = 'ERROR'
        result['errors'].append(str(e))
    return result


def run_androguard(apk_path):
    """Run Androguard APK parsing."""
    result = {'tool': 'androguard', 'verdict': 'UNKNOWN', 'errors': [], 'warnings': [],
              'details': {}, 'exit_code': -1}
    script = f'''
import sys, json, traceback
try:
    from androguard.core.apk import APK
    r = {{"parsed": False, "signed": False, "v1": False, "v2": False, "v3": False,
          "errors": [], "warnings": [], "entries": []}}
    try:
        a = APK("{apk_path}")
        r["parsed"] = True
        r["package"] = a.get_package()
        r["entries"] = a.get_files()
        r["signed"] = a.is_signed()
        r["v1"] = a.is_signed_v1()
        r["v2"] = a.is_signed_v2()
        r["v3"] = a.is_signed_v3()
    except Exception as e:
        r["errors"].append(f"Parse error: {{e}}")
    print(json.dumps(r))
except Exception as e:
    print(json.dumps({{"fatal": str(e)}}))
'''
    try:
        proc = subprocess.run([ANDROGUARD_PYTHON, '-c', script],
                              capture_output=True, text=True, timeout=30)
        result['exit_code'] = proc.returncode
        if proc.stdout.strip():
            try:
                data = json.loads(proc.stdout.strip().split('\n')[-1])
                result['details'] = data
                if data.get('fatal'):
                    result['verdict'] = 'CRASH'
                    result['errors'].append(data['fatal'])
                elif data.get('errors'):
                    result['verdict'] = 'REJECT'
                    result['errors'] = data['errors']
                elif data.get('parsed'):
                    result['verdict'] = 'PASS'
                    if data.get('warnings'):
                        result['verdict'] = 'PASS_WITH_WARNINGS'
                        result['warnings'] = data['warnings']
                else:
                    result['verdict'] = 'REJECT'
            except json.JSONDecodeError:
                result['verdict'] = 'CRASH'
                result['errors'].append(f"Bad output: {proc.stdout[:200]}")
    except subprocess.TimeoutExpired:
        result['verdict'] = 'TIMEOUT'
    except Exception as e:
        result['verdict'] = 'ERROR'
        result['errors'].append(str(e))
    return result


def find_eocd(data):
    sig = b'\x50\x4b\x05\x06'
    pos = len(data) - 22
    while pos >= 0:
        if data[pos:pos+4] == sig:
            comment_len = struct.unpack('<H', data[pos+20:pos+22])[0]
            if pos + 22 + comment_len == len(data):
                return pos
        pos -= 1
    return -1


def find_cd_start(data, eocd_off):
    return struct.unpack('<I', data[eocd_off+16:eocd_off+20])[0]


# =========================================================================
# Mutations on REAL signed APKs (these have valid signatures)
# =========================================================================

def get_real_apk(index=0):
    """Get a real APK path by index."""
    apks = sorted(os.listdir(REAL_APK_DIR))
    if index < len(apks):
        return os.path.join(REAL_APK_DIR, apks[index])
    return None


def mutate_real_extra_after_eocd(real_apk):
    """Append data after EOCD of a real signed APK."""
    with open(real_apk, 'rb') as f:
        data = f.read()
    result = data + b'\xDE\xAD\xBE\xEF' * 64
    path = os.path.join(MUTATED_DIR, 'real_extra_after_eocd.apk')
    with open(path, 'wb') as f:
        f.write(result)
    return path, 'Real APK + 256 bytes after EOCD'


def mutate_real_prepended_data(real_apk):
    """Prepend 112 bytes of fake DEX header before a real signed APK."""
    with open(real_apk, 'rb') as f:
        data = f.read()

    prefix = b'dex\n035\x00' + b'\x00' * (112 - 8)
    result = bytearray(prefix + data)

    # Fix EOCD CD offset
    eocd_off = find_eocd(bytes(result))
    if eocd_off != -1:
        old_cd_off = struct.unpack('<I', result[eocd_off+16:eocd_off+20])[0]
        struct.pack_into('<I', result, eocd_off+16, old_cd_off + len(prefix))

        # Fix CD entries
        cd_off = old_cd_off + len(prefix)
        cd_size = struct.unpack('<I', result[eocd_off+12:eocd_off+16])[0]
        pos = cd_off
        while pos < cd_off + cd_size:
            if result[pos:pos+4] == b'\x50\x4b\x01\x02':
                old_lfh = struct.unpack('<I', result[pos+42:pos+46])[0]
                struct.pack_into('<I', result, pos+42, old_lfh + len(prefix))
                fname_len = struct.unpack('<H', result[pos+28:pos+30])[0]
                extra_len = struct.unpack('<H', result[pos+30:pos+32])[0]
                comment_len = struct.unpack('<H', result[pos+32:pos+34])[0]
                pos += 46 + fname_len + extra_len + comment_len
            else:
                break

    path = os.path.join(MUTATED_DIR, 'real_prepended_dex.apk')
    with open(path, 'wb') as f:
        f.write(bytes(result))
    return path, 'Real APK with 112-byte DEX header prepended (Janus)'


def mutate_real_lfh_name(real_apk):
    """Change last byte of AndroidManifest.xml filename in LFH only."""
    with open(real_apk, 'rb') as f:
        data = bytearray(f.read())

    target = b'AndroidManifest.xml'
    pos = 0
    while pos < len(data):
        idx = data.find(b'\x50\x4b\x03\x04', pos)
        if idx == -1:
            break
        fname_len = struct.unpack('<H', data[idx+26:idx+28])[0]
        fname = bytes(data[idx+30:idx+30+fname_len])
        if fname == target:
            data[idx+30+len(target)-1] = ord('L')
            break
        pos = idx + 1

    path = os.path.join(MUTATED_DIR, 'real_lfh_name_mismatch.apk')
    with open(path, 'wb') as f:
        f.write(bytes(data))
    return path, 'Real APK with LFH AndroidManifest.xml -> AndroidManifest.xmL'


def mutate_real_duplicate_eocd(real_apk):
    """Add a second EOCD after the real one."""
    with open(real_apk, 'rb') as f:
        data = f.read()

    eocd_off = find_eocd(data)
    real_eocd = data[eocd_off:]

    # Create a second EOCD pointing to a different (fake) CD
    fake_eocd = struct.pack('<I', 0x06054b50)
    fake_eocd += struct.pack('<H', 0) + struct.pack('<H', 0)
    fake_eocd += struct.pack('<H', 0) + struct.pack('<H', 0)
    fake_eocd += struct.pack('<I', 0) + struct.pack('<I', 0)
    fake_eocd += struct.pack('<H', 0)

    result = data + fake_eocd

    path = os.path.join(MUTATED_DIR, 'real_dual_eocd.apk')
    with open(path, 'wb') as f:
        f.write(result)
    return path, 'Real APK with second EOCD appended (0 entries)'


def mutate_real_uncovered_gap(real_apk):
    """Insert uncovered data between LFH entries and CD."""
    with open(real_apk, 'rb') as f:
        data = f.read()

    eocd_off = find_eocd(data)
    cd_off = find_cd_start(data, eocd_off)

    # Find the APK signing block (if any) - it sits between entries and CD
    # Look for "APK Sig Block 42" magic before CD
    magic = b'APK Sig Block 42'
    sig_block_end = data.rfind(magic, 0, cd_off)

    if sig_block_end != -1:
        # Signing block present - insert gap before the signing block
        sig_block_end += len(magic)
        # Read the leading size to find start of signing block
        trailing_size = struct.unpack('<Q', data[sig_block_end - 24:sig_block_end - 16])[0]
        sig_block_start = sig_block_end - trailing_size - 8

        hidden = b'HIDDEN_UNCOVERED_DATA_' * 5
        result = bytearray(data[:sig_block_start] + hidden + data[sig_block_start:])

        # Fix CD offset in EOCD
        new_eocd_off = find_eocd(bytes(result))
        old_cd_off = struct.unpack('<I', result[new_eocd_off+16:new_eocd_off+20])[0]
        struct.pack_into('<I', result, new_eocd_off+16, old_cd_off + len(hidden))
    else:
        # No signing block - insert between entries and CD
        hidden = b'HIDDEN_UNCOVERED_DATA_' * 5
        result = bytearray(data[:cd_off] + hidden + data[cd_off:])
        new_eocd_off = find_eocd(bytes(result))
        struct.pack_into('<I', result, new_eocd_off+16, cd_off + len(hidden))

    path = os.path.join(MUTATED_DIR, 'real_uncovered_gap.apk')
    with open(path, 'wb') as f:
        f.write(bytes(result))
    return path, 'Real APK with 110-byte uncovered gap before CD'


def mutate_real_crc_mismatch(real_apk):
    """Change the CRC in the CD for classes.dex (not in LFH)."""
    with open(real_apk, 'rb') as f:
        data = bytearray(f.read())

    eocd_off = find_eocd(bytes(data))
    cd_off = find_cd_start(bytes(data), eocd_off)
    cd_size = struct.unpack('<I', data[eocd_off+12:eocd_off+16])[0]

    # Find AndroidManifest.xml in CD and flip its CRC
    pos = cd_off
    modified = False
    while pos < cd_off + cd_size:
        if data[pos:pos+4] != b'\x50\x4b\x01\x02':
            break
        fname_len = struct.unpack('<H', data[pos+28:pos+30])[0]
        fname = bytes(data[pos+46:pos+46+fname_len])
        if fname == b'AndroidManifest.xml':
            old_crc = struct.unpack('<I', data[pos+16:pos+20])[0]
            struct.pack_into('<I', data, pos+16, old_crc ^ 0x00000001)  # flip 1 bit
            modified = True
            break
        extra_len = struct.unpack('<H', data[pos+30:pos+32])[0]
        comment_len = struct.unpack('<H', data[pos+32:pos+34])[0]
        pos += 46 + fname_len + extra_len + comment_len

    if not modified:
        return None, 'SKIP: could not find AndroidManifest.xml in CD'

    path = os.path.join(MUTATED_DIR, 'real_crc_mismatch.apk')
    with open(path, 'wb') as f:
        f.write(bytes(data))
    return path, 'Real APK with CRC bit-flip in CD for AndroidManifest.xml'


def mutate_real_entry_count(real_apk):
    """Set EOCD entry count to 0xFFFF."""
    with open(real_apk, 'rb') as f:
        data = bytearray(f.read())

    eocd_off = find_eocd(bytes(data))
    struct.pack_into('<H', data, eocd_off + 8, 0xFFFF)
    struct.pack_into('<H', data, eocd_off + 10, 0xFFFF)

    path = os.path.join(MUTATED_DIR, 'real_entry_count_overflow.apk')
    with open(path, 'wb') as f:
        f.write(bytes(data))
    return path, 'Real APK with EOCD entry count = 0xFFFF'


def mutate_real_signing_block_extra_pair(real_apk):
    """Add an unknown ID pair to the APK signing block of a real APK."""
    with open(real_apk, 'rb') as f:
        data = f.read()

    eocd_off = find_eocd(data)
    cd_off = find_cd_start(data, eocd_off)

    magic = b'APK Sig Block 42'
    magic_pos = data.rfind(magic, 0, cd_off)

    if magic_pos == -1:
        return None, 'SKIP: no signing block found'

    magic_end = magic_pos + len(magic)
    trailing_size = struct.unpack('<Q', data[magic_pos - 8:magic_pos])[0]
    block_start = magic_end - trailing_size - 8

    # Extract existing pairs
    leading_size = struct.unpack('<Q', data[block_start:block_start+8])[0]
    pairs_data = data[block_start+8:magic_pos-8]

    # Add an unknown pair
    evil_payload = b'INJECTED_UNKNOWN_BLOCK_' * 4
    evil_pair = struct.pack('<Q', 4 + len(evil_payload))  # pair size
    evil_pair += struct.pack('<I', 0xDEAD0001)  # unknown block ID
    evil_pair += evil_payload

    new_pairs = pairs_data + evil_pair
    new_block_size = len(new_pairs) + 8 + 16

    new_block = struct.pack('<Q', new_block_size)
    new_block += new_pairs
    new_block += struct.pack('<Q', new_block_size)
    new_block += magic

    # Replace the old signing block
    result = bytearray(data[:block_start] + new_block + data[cd_off:])

    # Fix CD offset
    new_cd_off = block_start + len(new_block)
    new_eocd_off = find_eocd(bytes(result))
    struct.pack_into('<I', result, new_eocd_off + 16, new_cd_off)

    path = os.path.join(MUTATED_DIR, 'real_extra_signing_pair.apk')
    with open(path, 'wb') as f:
        f.write(bytes(result))
    return path, 'Real APK with injected unknown signing block pair (0xDEAD0001)'


def test_poc_with_min_sdk():
    """Re-test crafted PoCs with --min-sdk-version to bypass manifest issue."""
    poc_files = sorted([f for f in os.listdir(POC_DIR) if f.endswith('.apk') and f.startswith('poc')])

    print("\n" + "=" * 90)
    print("PART 1: Crafted PoCs re-tested with --min-sdk-version 21")
    print("=" * 90)

    results = []
    for poc_file in poc_files:
        poc_path = os.path.join(POC_DIR, poc_file)
        apk_r = run_apksigner(poc_path, min_sdk=21)
        ag_r = run_androguard(poc_path)

        diverged = False
        if apk_r['verdict'] in ('PASS', 'PASS_WITH_WARNINGS') and ag_r['verdict'] in ('REJECT', 'CRASH'):
            diverged = True
            dtype = 'apksigner=ACCEPT, androguard=REJECT'
        elif apk_r['verdict'] in ('REJECT',) and ag_r['verdict'] in ('PASS', 'PASS_WITH_WARNINGS'):
            diverged = True
            dtype = 'apksigner=REJECT, androguard=ACCEPT'
        else:
            dtype = 'AGREE'

        results.append({
            'poc': poc_file,
            'apksigner': apk_r['verdict'],
            'androguard': ag_r['verdict'],
            'diverged': diverged,
            'type': dtype,
            'apksigner_errors': apk_r.get('errors', [])[:2],
            'apksigner_warnings': apk_r.get('warnings', [])[:2],
        })

        status = "***DIVERGENCE***" if diverged else "agree"
        print(f"  {poc_file:50s} apksigner={apk_r['verdict']:25s} androguard={ag_r['verdict']:20s} {status}")
        if apk_r['errors']:
            for e in apk_r['errors'][:1]:
                if 'Exception' in e or 'ERROR' in e:
                    print(f"    apksigner err: {e[:110]}")
        if apk_r['warnings']:
            for w in apk_r['warnings'][:2]:
                print(f"    apksigner warn: {w[:110]}")

    return results


def test_mutated_real_apks():
    """Create and test mutations of real signed APKs."""
    print("\n" + "=" * 90)
    print("PART 2: Mutations of REAL signed APKs")
    print("=" * 90)

    # Use a v1-signed APK and a v2-signed APK
    v1_apk = None
    v2_apk = None
    for apk_name in sorted(os.listdir(REAL_APK_DIR)):
        if v1_apk and v2_apk:
            break
        apk_path = os.path.join(REAL_APK_DIR, apk_name)
        proc = subprocess.run(['apksigner', 'verify', '--verbose', apk_path],
                              capture_output=True, text=True, timeout=30)
        if proc.returncode == 0:
            if 'v1 scheme (JAR signing): true' in proc.stdout and not v1_apk:
                v1_apk = apk_path
                print(f"  Using v1 APK: {apk_name}")
            if 'v2 scheme (APK Signature Scheme v2): true' in proc.stdout and not v2_apk:
                v2_apk = apk_path
                print(f"  Using v2 APK: {apk_name}")

    if not v1_apk:
        v1_apk = v2_apk
    if not v2_apk:
        v2_apk = v1_apk

    mutations = [
        mutate_real_extra_after_eocd,
        mutate_real_prepended_data,
        mutate_real_lfh_name,
        mutate_real_duplicate_eocd,
        mutate_real_uncovered_gap,
        mutate_real_crc_mismatch,
        mutate_real_entry_count,
        mutate_real_signing_block_extra_pair,
    ]

    results = []
    for real_apk, label in [(v1_apk, 'v1'), (v2_apk, 'v2')]:
        if not real_apk:
            continue
        print(f"\n  --- Mutations of {label}-signed APK: {os.path.basename(real_apk)} ---")

        for mutate_fn in mutations:
            try:
                path, desc = mutate_fn(real_apk)
            except Exception as e:
                print(f"    SKIP {mutate_fn.__name__}: {e}")
                continue

            if path is None:
                print(f"    SKIP {mutate_fn.__name__}: {desc}")
                continue

            apk_r = run_apksigner(path)
            ag_r = run_androguard(path)

            diverged = False
            if apk_r['verdict'] in ('PASS', 'PASS_WITH_WARNINGS') and ag_r['verdict'] in ('REJECT', 'CRASH'):
                diverged = True
            elif apk_r['verdict'] in ('REJECT',) and ag_r['verdict'] in ('PASS', 'PASS_WITH_WARNINGS'):
                diverged = True

            status = "***DIVERGENCE***" if diverged else "agree"
            print(f"    {desc[:60]:62s} apksigner={apk_r['verdict']:25s} androguard={ag_r['verdict']:20s} {status}")

            if apk_r['warnings']:
                for w in apk_r['warnings'][:2]:
                    print(f"      warn: {w[:100]}")
            if apk_r['errors']:
                for e in apk_r['errors'][:1]:
                    if e.strip():
                        print(f"      err:  {e[:100]}")

            results.append({
                'base': os.path.basename(real_apk),
                'scheme': label,
                'mutation': mutate_fn.__name__,
                'desc': desc,
                'apksigner': apk_r['verdict'],
                'androguard': ag_r['verdict'],
                'diverged': diverged,
                'apksigner_warnings': apk_r.get('warnings', []),
                'apksigner_errors': apk_r.get('errors', []),
            })

    return results


def main():
    print("=" * 90)
    print("APK Parsing Divergence Test Harness v2")
    print("=" * 90)

    poc_results = test_poc_with_min_sdk()
    mutation_results = test_mutated_real_apks()

    # Summary
    print("\n" + "=" * 90)
    print("SUMMARY")
    print("=" * 90)

    poc_divs = [r for r in poc_results if r['diverged']]
    mut_divs = [r for r in mutation_results if r['diverged']]

    print(f"\nCrafted PoCs: {len(poc_divs)}/{len(poc_results)} divergences")
    for r in poc_divs:
        print(f"  {r['poc']}: {r['type']}")

    print(f"\nReal APK mutations: {len(mut_divs)}/{len(mutation_results)} divergences")
    for r in mut_divs:
        print(f"  {r['mutation']} ({r['scheme']}): apksigner={r['apksigner']}, androguard={r['androguard']}")
        if r.get('apksigner_warnings'):
            for w in r['apksigner_warnings'][:2]:
                print(f"    warn: {w[:100]}")

    # Save results
    all_data = {
        'poc_results': poc_results,
        'mutation_results': mutation_results,
        'summary': {
            'poc_divergences': len(poc_divs),
            'poc_total': len(poc_results),
            'mutation_divergences': len(mut_divs),
            'mutation_total': len(mutation_results),
        }
    }
    json_path = os.path.join(os.path.dirname(POC_DIR), 'divergence_results_v2.json')
    with open(json_path, 'w') as f:
        json.dump(all_data, f, indent=2, default=str)
    print(f"\nResults saved to: {json_path}")


if __name__ == '__main__':
    main()
