#!/usr/bin/env python3
"""
Scan all real F-Droid APKs for:
  1. Signature verification anomalies (apksigner warnings/errors)
  2. Structural anomalies that might indicate natural divergence cases
  3. Signing scheme distribution (v1-only vs v2 vs v3)
"""

import subprocess
import os
import sys
import json
import struct
import time

REAL_DIR = '/root/apkaxiom/corpus/bench-10k/real-fdroid'
ANDROGUARD_PYTHON = '/root/security_research_tools/envs/main/bin/python3'


def find_eocd(data):
    sig = b'\x50\x4b\x05\x06'
    pos = len(data) - 22
    while pos >= 0:
        if data[pos:pos+4] == sig:
            cl = struct.unpack('<H', data[pos+20:pos+22])[0]
            if pos + 22 + cl == len(data):
                return pos
        pos -= 1
    return -1


def structural_analysis(apk_path):
    """Analyze ZIP structural properties."""
    result = {'anomalies': []}
    try:
        with open(apk_path, 'rb') as f:
            data = f.read()

        result['size'] = len(data)

        # Check for multiple EOCD signatures
        eocd_sig = b'\x50\x4b\x05\x06'
        eocd_positions = []
        pos = 0
        while True:
            idx = data.find(eocd_sig, pos)
            if idx == -1:
                break
            eocd_positions.append(idx)
            pos = idx + 1
        result['eocd_count'] = len(eocd_positions)
        if len(eocd_positions) > 1:
            result['anomalies'].append(f'Multiple EOCD signatures: {len(eocd_positions)} found')

        # Find actual EOCD
        eocd_off = find_eocd(data)
        if eocd_off == -1:
            result['anomalies'].append('No valid EOCD found')
            return result

        # Check for trailing data
        comment_len = struct.unpack('<H', data[eocd_off+20:eocd_off+22])[0]
        expected_end = eocd_off + 22 + comment_len
        if expected_end != len(data):
            result['anomalies'].append(f'Trailing data after EOCD: {len(data) - expected_end} bytes')

        # Check for prepended data (first bytes not a LFH)
        if data[:4] != b'\x50\x4b\x03\x04':
            result['anomalies'].append(f'Does not start with LFH: {data[:4].hex()}')

        # Check for signing block
        cd_off = struct.unpack('<I', data[eocd_off+16:eocd_off+20])[0]
        magic = b'APK Sig Block 42'
        magic_pos = data.rfind(magic, 0, cd_off)
        result['has_signing_block'] = magic_pos != -1

        # Check CD entry count
        cd_entries_disk = struct.unpack('<H', data[eocd_off+8:eocd_off+10])[0]
        cd_entries_total = struct.unpack('<H', data[eocd_off+10:eocd_off+12])[0]
        if cd_entries_disk != cd_entries_total:
            result['anomalies'].append(f'Entry count mismatch: disk={cd_entries_disk}, total={cd_entries_total}')

        result['entry_count'] = cd_entries_total
        result['comment_len'] = comment_len

        # Walk CD entries and check for LFH mismatches
        cd_size = struct.unpack('<I', data[eocd_off+12:eocd_off+16])[0]
        pos = cd_off
        entries_found = 0
        while pos < cd_off + cd_size and entries_found < cd_entries_total:
            if data[pos:pos+4] != b'\x50\x4b\x01\x02':
                break

            cd_fname_len = struct.unpack('<H', data[pos+28:pos+30])[0]
            cd_extra_len = struct.unpack('<H', data[pos+30:pos+32])[0]
            cd_comment_len = struct.unpack('<H', data[pos+32:pos+34])[0]
            cd_fname = data[pos+46:pos+46+cd_fname_len]
            cd_crc = struct.unpack('<I', data[pos+16:pos+20])[0]
            cd_comp = struct.unpack('<H', data[pos+10:pos+12])[0]
            lfh_off = struct.unpack('<I', data[pos+42:pos+46])[0]

            # Check LFH
            if lfh_off < len(data) and data[lfh_off:lfh_off+4] == b'\x50\x4b\x03\x04':
                lfh_fname_len = struct.unpack('<H', data[lfh_off+26:lfh_off+28])[0]
                lfh_extra_len = struct.unpack('<H', data[lfh_off+28:lfh_off+30])[0]
                lfh_fname = data[lfh_off+30:lfh_off+30+lfh_fname_len]
                lfh_crc = struct.unpack('<I', data[lfh_off+14:lfh_off+18])[0]

                if cd_fname != lfh_fname:
                    result['anomalies'].append(
                        f'LFH/CD name mismatch: CD={cd_fname!r}, LFH={lfh_fname!r}')

                if cd_crc != lfh_crc and lfh_crc != 0:  # 0 may be data descriptor
                    result['anomalies'].append(
                        f'CRC mismatch for {cd_fname!r}: CD={cd_crc:#x}, LFH={lfh_crc:#x}')

                if lfh_extra_len != cd_extra_len:
                    # This is actually common and normal per ZIP spec
                    pass

            entries_found += 1
            pos += 46 + cd_fname_len + cd_extra_len + cd_comment_len

        if entries_found != cd_entries_total:
            result['anomalies'].append(
                f'Entry count in EOCD ({cd_entries_total}) != entries found ({entries_found})')

    except Exception as e:
        result['anomalies'].append(f'Analysis error: {e}')

    return result


def scan_apksigner(apk_path):
    """Run apksigner verify."""
    try:
        proc = subprocess.run(
            ['apksigner', 'verify', '--verbose', apk_path],
            capture_output=True, text=True, timeout=30)
        combined = proc.stdout + proc.stderr
        return {
            'exit_code': proc.returncode,
            'verifies': proc.returncode == 0 and 'Verifies' in proc.stdout,
            'v1': 'v1 scheme (JAR signing): true' in combined,
            'v2': 'v2 scheme (APK Signature Scheme v2): true' in combined,
            'v3': 'v3 scheme (APK Signature Scheme v3): true' in combined,
            'warnings': [l.strip() for l in combined.split('\n') if 'WARNING' in l],
            'errors': [l.strip() for l in combined.split('\n') if 'ERROR' in l],
        }
    except subprocess.TimeoutExpired:
        return {'exit_code': -1, 'verifies': False, 'timeout': True}
    except Exception as e:
        return {'exit_code': -1, 'verifies': False, 'error': str(e)}


def main():
    apks = sorted([f for f in os.listdir(REAL_DIR) if f.endswith('.apk')])
    print(f"Scanning {len(apks)} real APKs...")

    scheme_counts = {'v1_only': 0, 'v2_only': 0, 'v3_only': 0,
                     'v1_v2': 0, 'v1_v2_v3': 0, 'v2_v3': 0, 'other': 0}
    anomalies_found = []
    warnings_found = []
    errors_found = []
    failures = []

    for i, apk_name in enumerate(apks):
        apk_path = os.path.join(REAL_DIR, apk_name)

        # apksigner
        apk_result = scan_apksigner(apk_path)

        if not apk_result.get('verifies'):
            failures.append((apk_name, apk_result))

        if apk_result.get('warnings'):
            # Only count non-META-INF warnings (those are expected)
            non_meta_warnings = [w for w in apk_result['warnings']
                                 if 'META-INF/' not in w]
            if non_meta_warnings:
                warnings_found.append((apk_name, non_meta_warnings))

        if apk_result.get('errors'):
            errors_found.append((apk_name, apk_result['errors']))

        # Scheme classification
        v1 = apk_result.get('v1', False)
        v2 = apk_result.get('v2', False)
        v3 = apk_result.get('v3', False)
        if v1 and v2 and v3:
            scheme_counts['v1_v2_v3'] += 1
        elif v1 and v2:
            scheme_counts['v1_v2'] += 1
        elif v2 and v3:
            scheme_counts['v2_v3'] += 1
        elif v1 and not v2 and not v3:
            scheme_counts['v1_only'] += 1
        elif v2 and not v3:
            scheme_counts['v2_only'] += 1
        elif v3:
            scheme_counts['v3_only'] += 1
        else:
            scheme_counts['other'] += 1

        # Structural analysis
        struct_result = structural_analysis(apk_path)
        if struct_result.get('anomalies'):
            anomalies_found.append((apk_name, struct_result['anomalies']))

        if (i + 1) % 100 == 0:
            print(f"  Scanned {i+1}/{len(apks)}...")

    # Summary
    print("\n" + "=" * 80)
    print("CORPUS SCAN RESULTS")
    print("=" * 80)

    print(f"\nTotal APKs: {len(apks)}")
    print(f"\nSigning scheme distribution:")
    for k, v in sorted(scheme_counts.items(), key=lambda x: -x[1]):
        if v > 0:
            print(f"  {k:15s}: {v:4d} ({100*v/len(apks):.1f}%)")

    print(f"\nVerification failures: {len(failures)}")
    for name, result in failures[:10]:
        print(f"  {name}")
        if result.get('errors'):
            for e in result['errors'][:2]:
                print(f"    {e[:100]}")

    print(f"\nNon-META-INF warnings: {len(warnings_found)}")
    for name, warns in warnings_found[:10]:
        print(f"  {name}")
        for w in warns[:3]:
            print(f"    {w[:100]}")

    print(f"\nStructural anomalies: {len(anomalies_found)}")
    for name, anoms in anomalies_found[:20]:
        print(f"  {name}")
        for a in anoms[:3]:
            print(f"    {a[:100]}")

    # Identify v1-only APKs (vulnerable to Janus)
    v1_only_apks = []
    for apk_name in apks:
        apk_path = os.path.join(REAL_DIR, apk_name)
        r = scan_apksigner(apk_path)
        if r.get('v1') and not r.get('v2') and not r.get('v3'):
            v1_only_apks.append(apk_name)
    # Actually let's not re-scan, use the already-collected data
    print(f"\nv1-only signed APKs (vulnerable to Janus-class attacks): {scheme_counts['v1_only']}")
    print(f"  These APKs can have arbitrary data prepended/appended without")
    print(f"  invalidating the v1 (JAR) signature verification in apksigner.")

    # Save results
    results = {
        'total': len(apks),
        'scheme_counts': scheme_counts,
        'failures': [(n, str(r)) for n, r in failures],
        'warnings': [(n, w) for n, w in warnings_found],
        'anomalies': anomalies_found,
    }
    json_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'corpus_scan_results.json')
    with open(json_path, 'w') as f:
        json.dump(results, f, indent=2, default=str)
    print(f"\nResults saved to: {json_path}")


if __name__ == '__main__':
    main()
