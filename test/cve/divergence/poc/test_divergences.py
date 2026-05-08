#!/usr/bin/env python3
"""
Divergence Test Harness

Tests each crafted PoC APK against:
  1. apksigner verify (v31.0.2)
  2. Androguard APK parser + signature verification
  3. Python zipfile (reference parser)

Records verdicts and identifies divergences.
"""

import subprocess
import os
import sys
import json
import zipfile
import traceback

# Activate androguard environment
ANDROGUARD_PYTHON = '/root/security_research_tools/envs/main/bin/python3'

POC_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)))
RESULTS = []


def test_apksigner(apk_path):
    """Run apksigner verify and return structured result."""
    result = {
        'tool': 'apksigner',
        'verdict': 'UNKNOWN',
        'errors': [],
        'warnings': [],
        'raw_stdout': '',
        'raw_stderr': '',
        'exit_code': -1
    }
    try:
        proc = subprocess.run(
            ['apksigner', 'verify', '--verbose', '--print-certs', apk_path],
            capture_output=True, text=True, timeout=30
        )
        result['exit_code'] = proc.returncode
        result['raw_stdout'] = proc.stdout
        result['raw_stderr'] = proc.stderr

        combined = proc.stdout + proc.stderr

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
                if 'ERROR' in line or 'Exception' in line or line.strip():
                    result['errors'].append(line.strip())

    except subprocess.TimeoutExpired:
        result['verdict'] = 'TIMEOUT'
        result['errors'].append('Timed out after 30s')
    except Exception as e:
        result['verdict'] = 'ERROR'
        result['errors'].append(str(e))

    return result


def test_androguard(apk_path):
    """Run Androguard APK parsing and signature verification."""
    result = {
        'tool': 'androguard',
        'verdict': 'UNKNOWN',
        'errors': [],
        'warnings': [],
        'details': {},
        'exit_code': -1
    }

    script = f'''
import sys
import json
import traceback

try:
    from androguard.core.apk import APK

    result = {{"parsed": False, "signed": False, "v1": False, "v2": False, "v3": False,
               "errors": [], "warnings": [], "entries": []}}

    try:
        a = APK("{apk_path}")
        result["parsed"] = True
        result["package"] = a.get_package()
        result["entries"] = a.get_files()

        result["signed"] = a.is_signed()
        result["v1"] = a.is_signed_v1()
        result["v2"] = a.is_signed_v2()
        result["v3"] = a.is_signed_v3()

        # Try to get certificates
        try:
            certs_v1 = a.get_certificates_v1()
            result["certs_v1_count"] = len(certs_v1) if certs_v1 else 0
        except Exception as e:
            result["warnings"].append(f"v1 cert error: {{e}}")

        try:
            certs_v2 = a.get_certificates_der_v2()
            result["certs_v2_count"] = len(certs_v2) if certs_v2 else 0
        except Exception as e:
            result["warnings"].append(f"v2 cert error: {{e}}")

    except Exception as e:
        result["errors"].append(f"Parse error: {{e}}")
        result["traceback"] = traceback.format_exc()

    print(json.dumps(result))

except Exception as e:
    print(json.dumps({{"fatal": str(e), "traceback": traceback.format_exc()}}))
'''

    try:
        proc = subprocess.run(
            [ANDROGUARD_PYTHON, '-c', script],
            capture_output=True, text=True, timeout=30
        )
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
                    if data.get('warnings'):
                        result['verdict'] = 'PASS_WITH_WARNINGS'
                        result['warnings'] = data['warnings']
                    else:
                        result['verdict'] = 'PASS'
                else:
                    result['verdict'] = 'REJECT'

            except json.JSONDecodeError:
                result['verdict'] = 'CRASH'
                result['errors'].append(f"Non-JSON output: {proc.stdout[:200]}")

        if proc.stderr.strip():
            result['errors'].append(f"stderr: {proc.stderr[:500]}")

    except subprocess.TimeoutExpired:
        result['verdict'] = 'TIMEOUT'
        result['errors'].append('Timed out after 30s')
    except Exception as e:
        result['verdict'] = 'ERROR'
        result['errors'].append(str(e))

    return result


def test_python_zipfile(apk_path):
    """Test with Python's zipfile module as a reference."""
    result = {
        'tool': 'python_zipfile',
        'verdict': 'UNKNOWN',
        'errors': [],
        'warnings': [],
        'entries': []
    }
    try:
        with zipfile.ZipFile(apk_path, 'r') as zf:
            result['entries'] = zf.namelist()
            # Try to read all entries
            for name in zf.namelist():
                try:
                    zf.read(name)
                except Exception as e:
                    result['warnings'].append(f"Read error for {name}: {e}")
            # Check for bad files
            bad = zf.testzip()
            if bad:
                result['verdict'] = 'PASS_WITH_WARNINGS'
                result['warnings'].append(f"testzip() flagged: {bad}")
            else:
                result['verdict'] = 'PASS'
    except zipfile.BadZipFile as e:
        result['verdict'] = 'REJECT'
        result['errors'].append(f"BadZipFile: {e}")
    except Exception as e:
        result['verdict'] = 'REJECT'
        result['errors'].append(f"{type(e).__name__}: {e}")
    return result


POC_DESCRIPTIONS = {
    'poc01': 'Dual EOCD records - two ZIP trailers pointing to different CDs',
    'poc02': 'Overlapping ZIP entries - two entries share data region',
    'poc03': 'LFH vs CD filename mismatch - last byte differs',
    'poc04': 'Extra data after EOCD - trailing garbage',
    'poc05': 'EOCD comment containing EOCD signature bytes',
    'poc06': 'Unsupported compression method (bzip2) on classes.dex',
    'poc07': 'Fake APK Signing Block with garbage v2 signer',
    'poc08': 'CD offset underflow - EOCD points before real CD',
    'poc09': 'Duplicate CD entries - same filename, different data',
    'poc10': 'Data descriptor ambiguity - bit 3 flag, sizes in DD',
    'poc11': 'Prepended DEX header (Janus-style CVE-2017-13156)',
    'poc12': 'Zero-length filename in CD entry',
    'poc13': 'LFH extra field mismatch - LFH has extra, CD says 0',
    'poc14': 'Signing Block with unknown block IDs',
    'poc15': 'Version needed mismatch between LFH and CD',
    'poc16': 'Entry count overflow - EOCD says 0xFFFF entries',
    'poc17': 'Uncovered gap - hidden data between LFH and CD',
    'poc18': 'CD entry pointing into CD region (fake embedded LFH)',
    'poc19': 'CRC mismatch between LFH and CD for same entry',
    'poc20': 'Signing block leading/trailing size mismatch',
}

AOSP_BEHAVIOR = {
    'poc01': 'Android scans backwards for EOCD, validates comment length. Finds LAST valid EOCD. If both EOCDs are valid, tools and runtime should agree on the last one. Divergence if one tool uses first-found.',
    'poc02': 'libziparchive does NOT validate entry overlaps. apksigner (post-Janus patch) DOES check. HIGH divergence potential: apksigner rejects, Android installs.',
    'poc03': 'libziparchive uses CD name for lookup, reads data from LFH offset. apksigner cross-checks LFH vs CD names. If mismatch, apksigner rejects. Android may still read data correctly.',
    'poc04': 'libziparchive validates EOCD position. apksigner requires EOCD+comment to end exactly at EOF. Extra trailing data = apksigner reject.',
    'poc05': 'EOCD comment with valid comment length is legal. But EOCD sig in comment could confuse backward-scanning parsers into finding wrong EOCD.',
    'poc06': 'Android supports deflate (8) and stored (0). Method 12 (bzip2) is unsupported. Both tools should reject.',
    'poc07': 'Android validates signing block cryptographically. Garbage signer = verification failure. Both should reject for v2.',
    'poc08': 'libziparchive validates CD entries when iterating. Wrong CD offset = parse failure.',
    'poc09': 'Android uses HashMap for ZIP entries (last-wins). apksigner iterates CD linearly. If apksigner verifies first entry but Android uses second, CRITICAL divergence.',
    'poc10': 'Data descriptors are legal but uncommon in APKs. libziparchive handles them. apksigner should handle them per ZIP spec.',
    'poc11': 'Post-CVE-2017-13156 fix: Android rejects APKs with prepended data (v2/v3 signed). V1-only APKs were vulnerable. apksigner checks for this.',
    'poc12': 'Zero-length filenames are technically valid per ZIP spec but unusual. Different tools may handle differently.',
    'poc13': 'LFH extra field size != CD extra field size. Data offset depends on LFH extra. If verifier uses CD extra size to compute offset, it reads wrong data.',
    'poc14': 'Unknown block IDs in signing block should be ignored per spec. Tools should not reject for unknown IDs alone.',
    'poc15': 'High version-needed may cause tools to refuse extraction. Android may be more permissive.',
    'poc16': 'Entry count overflow: 0xFFFF entries but only a few in CD. Tools may read past CD into garbage.',
    'poc17': 'Uncovered gap between entries and CD. V2 signing covers 3 sections (pre-signing, signing-CD, EOCD). Gap becomes part of section 1.',
    'poc18': 'CD entry pointing into CD itself. libziparchive validates LFH offset is within file. If offset is within CD region, behavior varies.',
    'poc19': 'CRC mismatch. libziparchive may check CRC on extraction. apksigner checks CRC during verification. Different CRC sources = divergence.',
    'poc20': 'Signing block size mismatch. apksigner validates both size fields match. Android runtime may only check one.',
}


def classify_divergence(apksigner_result, androguard_result, zipfile_result):
    """Classify the type and severity of divergence."""
    verdicts = {
        'apksigner': apksigner_result['verdict'],
        'androguard': androguard_result['verdict'],
        'zipfile': zipfile_result['verdict'],
    }

    # Check for divergences
    unique_verdicts = set()
    for v in verdicts.values():
        # Normalize: PASS_WITH_WARNINGS is still PASS-ish
        if v in ('PASS', 'PASS_WITH_WARNINGS'):
            unique_verdicts.add('ACCEPT')
        elif v in ('REJECT', 'CRASH'):
            unique_verdicts.add('REJECT')
        elif v == 'TIMEOUT':
            unique_verdicts.add('TIMEOUT')
        else:
            unique_verdicts.add('UNKNOWN')

    if len(unique_verdicts) == 1:
        return 'NO_DIVERGENCE', 'All tools agree'

    # Specific high-value divergences
    apk_v = verdicts['apksigner']
    ag_v = verdicts['androguard']

    if apk_v in ('PASS', 'PASS_WITH_WARNINGS') and ag_v in ('REJECT', 'CRASH'):
        return 'DIVERGENCE_HIGH', 'apksigner accepts but Androguard rejects/crashes'

    if apk_v in ('REJECT',) and ag_v in ('PASS', 'PASS_WITH_WARNINGS'):
        return 'DIVERGENCE_CRITICAL', 'apksigner rejects but Androguard accepts (potential sig bypass if Android also accepts)'

    if apk_v == 'CRASH' or ag_v == 'CRASH':
        return 'DIVERGENCE_CRASH', 'Tool crash indicates parser confusion'

    if apk_v == 'TIMEOUT' or ag_v == 'TIMEOUT':
        return 'DIVERGENCE_DOS', 'Tool timeout indicates potential DoS'

    # Warnings divergence
    if apk_v == 'PASS_WITH_WARNINGS' and ag_v == 'PASS':
        return 'DIVERGENCE_LOW', 'apksigner warns but Androguard accepts silently'

    return 'DIVERGENCE_MEDIUM', f'Tools disagree: apksigner={apk_v}, androguard={ag_v}, zipfile={verdicts["zipfile"]}'


def run_all_pocs():
    """Test all PoC APKs."""
    poc_files = sorted([f for f in os.listdir(POC_DIR) if f.endswith('.apk') and f.startswith('poc')])

    print(f"\nTesting {len(poc_files)} PoC APKs")
    print("=" * 90)

    all_results = []
    divergences = []

    for poc_file in poc_files:
        poc_path = os.path.join(POC_DIR, poc_file)
        poc_id = poc_file.split('_')[0]

        print(f"\n{'─' * 90}")
        print(f"Testing: {poc_file}")
        desc = POC_DESCRIPTIONS.get(poc_id, 'No description')
        print(f"  Class: {desc}")

        # Run all three tools
        apksigner_r = test_apksigner(poc_path)
        androguard_r = test_androguard(poc_path)
        zipfile_r = test_python_zipfile(poc_path)

        # Classify divergence
        div_class, div_desc = classify_divergence(apksigner_r, androguard_r, zipfile_r)

        print(f"  apksigner:  {apksigner_r['verdict']}")
        if apksigner_r['errors']:
            for e in apksigner_r['errors'][:3]:
                if e.strip():
                    print(f"    err: {e[:120]}")
        if apksigner_r['warnings']:
            for w in apksigner_r['warnings'][:3]:
                print(f"    warn: {w[:120]}")

        print(f"  androguard: {androguard_r['verdict']}")
        if androguard_r['errors']:
            for e in androguard_r['errors'][:3]:
                if e.strip():
                    print(f"    err: {e[:120]}")

        print(f"  zipfile:    {zipfile_r['verdict']}")
        if zipfile_r['errors']:
            for e in zipfile_r['errors'][:3]:
                print(f"    err: {e[:120]}")

        print(f"  DIVERGENCE: {div_class} - {div_desc}")

        aosp = AOSP_BEHAVIOR.get(poc_id, '')
        if aosp:
            print(f"  AOSP note:  {aosp[:120]}")

        result_entry = {
            'poc': poc_file,
            'poc_id': poc_id,
            'description': desc,
            'apksigner': apksigner_r,
            'androguard': androguard_r,
            'zipfile': zipfile_r,
            'divergence_class': div_class,
            'divergence_desc': div_desc,
            'aosp_behavior': aosp,
        }
        all_results.append(result_entry)

        if div_class != 'NO_DIVERGENCE':
            divergences.append(result_entry)

    return all_results, divergences


def main():
    print("=" * 90)
    print("APK Parsing Divergence Test Harness")
    print("=" * 90)

    all_results, divergences = run_all_pocs()

    print("\n")
    print("=" * 90)
    print("SUMMARY")
    print("=" * 90)
    print(f"Total PoCs tested: {len(all_results)}")
    print(f"Divergences found: {len(divergences)}")

    if divergences:
        print("\nDivergence breakdown:")
        by_class = {}
        for d in divergences:
            c = d['divergence_class']
            by_class.setdefault(c, []).append(d)

        for cls in sorted(by_class.keys(), key=lambda x: {'DIVERGENCE_CRITICAL': 0, 'DIVERGENCE_HIGH': 1, 'DIVERGENCE_CRASH': 2, 'DIVERGENCE_DOS': 3, 'DIVERGENCE_MEDIUM': 4, 'DIVERGENCE_LOW': 5}.get(x, 6)):
            items = by_class[cls]
            print(f"\n  {cls} ({len(items)}):")
            for item in items:
                print(f"    - {item['poc']}: {item['divergence_desc']}")

    # Save JSON results
    json_path = os.path.join(os.path.dirname(POC_DIR), 'poc_results.json')
    with open(json_path, 'w') as f:
        json.dump({
            'total': len(all_results),
            'divergences': len(divergences),
            'results': all_results
        }, f, indent=2, default=str)
    print(f"\nFull results saved to: {json_path}")


if __name__ == '__main__':
    main()
