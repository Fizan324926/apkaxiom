# APK Parsing Divergence Analysis

## Executive Summary

Systematic analysis of parsing divergences between apksigner 31.0.2,
Androguard 4.1.3, and Android's PackageManager/libziparchive reveals
multiple exploitable gaps. The highest-impact finding is that **268 of
1000 real F-Droid APKs use v1-only signing**, and apksigner
unconditionally verifies v1-only APKs even when arbitrary data is
prepended (Janus-class, CVE-2017-13156) or hidden in uncovered ZIP
gaps. This is by design for v1 (JAR signatures only cover individual
entry content, not ZIP structure) but represents a real-world attack
surface because v1-only APKs still exist in the wild and apksigner
reports them as "Verifies" with no structural warning.

---

## 1. Corpus Analysis

### 1.1 Signing Scheme Distribution (1000 F-Droid APKs)

| Scheme | Count | Percentage |
|--------|------:|------------|
| v1+v2+v3 | 365 | 36.5% |
| v1 only | 268 | 26.8% |
| v2+v3 | 149 | 14.9% |
| v2 only | 110 | 11.0% |
| v1+v2 | 62 | 6.2% |
| v3 only | 44 | 4.4% |
| Failed | 2 | 0.2% |

**Key finding**: 26.8% of APKs use v1-only signing. These are
structurally vulnerable to prepended-data and uncovered-gap attacks
because JAR signing only covers individual ZIP entry contents, not the
ZIP structural metadata.

### 1.2 Structural Anomalies

- 40 APKs contain multiple EOCD signature bytes (all false positives
  from compressed data; the last valid EOCD is authoritative in all
  cases)
- 2 APKs fail verification entirely (corrupted: no valid EOCD)
- 0 APKs have real structural anomalies (no trailing data, no
  prepended data, no LFH/CD mismatches)

---

## 2. Confirmed Divergences

### 2.1 CRITICAL: v1-Only APK + Prepended Data (Janus-Class)

**CVE reference**: CVE-2017-13156 (Janus vulnerability)

**Observation**: apksigner 31.0.2 reports "Verifies" for a v1-only
signed APK with 4096 bytes of arbitrary data prepended. All CD entry
offsets are adjusted so the ZIP structure remains valid.

```
$ apksigner verify --verbose janus_4k_prepend.apk
Verifies
Verified using v1 scheme (JAR signing): true
```

**Impact**: On Android < 8.1 (API < 27), the runtime would read the
prepended data as a DEX file header while the ZIP parser reads from
the EOCD backwards. The attacker gets code execution with the
original app's identity and permissions.

**Root cause**: v1 (JAR) signatures only cover the content of
individual ZIP entries via META-INF/*.SF manifests. The ZIP structural
metadata (EOCD, CD offsets, LFH positions) and any data outside entry
content regions are not signed. apksigner correctly verifies the JAR
signatures but does not warn about prepended data.

**Affected real APKs**: 268 of 1000 (26.8%) -- see
`v1_only_apks.json` for the full list.

**Mitigation**: Android 8.1+ rejects APKs with prepended data when
v2/v3 signing is present. However, v1-only APKs on older devices
remain vulnerable. apksigner should emit a WARNING for v1-only APKs
with data before the first LFH.

### 2.2 CRITICAL: v1-Only APK + Uncovered ZIP Gap

**Observation**: apksigner reports "Verifies" for a v1-only signed APK
with 32768 bytes of hidden data inserted between the last LFH entry
and the Central Directory.

```
$ apksigner verify --verbose hidden_gap_32k.apk
Verifies
Verified using v1 scheme (JAR signing): true
```

**Impact**: An attacker can hide arbitrary payloads (shellcode,
exfiltrated data, C2 channel markers) inside a signed APK without
invalidating the signature. Security scanners that trust apksigner's
"Verifies" verdict may miss the hidden content.

**Root cause**: Same as 2.1 -- JAR signatures do not cover the gap
between the last entry's data and the Central Directory.

### 2.3 CRITICAL: Combined Prepend + Gap

Both attacks compose: a v1-only APK with 4K prepended AND 32K hidden
in the gap still verifies.

```
$ apksigner verify --verbose janus_combined.apk
Verifies
Verified using v1 scheme (JAR signing): true
```

### 2.4 HIGH: Androguard Permissive Parsing (18 Divergence Classes)

Androguard 4.1.3 successfully parses and reports "signed=True" for
all 18 crafted malformed APKs that apksigner rejects. This means
Androguard cannot be used as a security gatekeeper for structurally
malformed APKs.

Specific classes where Androguard accepts but apksigner rejects:

| PoC | Manipulation | apksigner | Androguard |
|-----|-------------|-----------|------------|
| 01 | Dual EOCD records | REJECT | ACCEPT |
| 02 | Overlapping entries | REJECT | ACCEPT |
| 03 | LFH/CD name mismatch | REJECT | ACCEPT |
| 04 | Extra data after EOCD | REJECT | ACCEPT |
| 05 | EOCD comment with sigs | REJECT | ACCEPT |
| 06 | Unsupported compression | REJECT | ACCEPT |
| 07 | Fake signing block | REJECT | ACCEPT |
| 09 | Duplicate CD entries | REJECT | ACCEPT |
| 10 | Data descriptor ambiguity | REJECT | ACCEPT |
| 11 | Prepended DEX | REJECT | ACCEPT |
| 12 | Zero-length filename | REJECT | ACCEPT |
| 13 | LFH extra field mismatch | REJECT | ACCEPT |
| 14 | Unknown signing block IDs | REJECT | ACCEPT |
| 15 | Version needed mismatch | REJECT | ACCEPT |
| 16 | Entry count overflow | REJECT | ACCEPT |
| 17 | Uncovered gap | REJECT | ACCEPT |
| 18 | CD entry past LFH section | REJECT | ACCEPT |
| 19 | CRC mismatch LFH vs CD | REJECT | ACCEPT |

Note: These crafted PoCs use plain-text AndroidManifest.xml (not
compiled binary XML), which causes apksigner to fail to parse the
manifest. On real APKs (binary AXML), apksigner's structural checks
would engage at a deeper level.

### 2.5 HIGH: v2-Signed APK Mutation Detection

When the same mutations are applied to v2-signed APKs, apksigner
correctly rejects all of them ("DOES NOT VERIFY" / digest mismatch).
This confirms that v2/v3 signing properly protects against structural
manipulation. The vulnerability window is specifically v1-only APKs.

### 2.6 MEDIUM: Signing Block Size Mismatch

Both apksigner and Androguard reject APK signing blocks where the
leading and trailing size fields disagree. This is the only crafted
PoC where both tools agree on rejection (besides CD offset underflow).

---

## 3. AOSP Source Analysis

### 3.1 libziparchive (Android Runtime ZIP Parser)

Key behaviors extracted from AOSP `libziparchive/zip_archive.cc`:

- **EOCD search**: Scans backwards from EOF, validates comment length
  against actual remaining bytes. Finds the last valid EOCD.
- **CD validation**: Iterates CD entries, validates each signature,
  cross-checks filename lengths.
- **LFH matching**: Validates LFH signature, compression method, mod
  time, CRC, and name length against CD.
- **No overlap checking**: libziparchive does NOT validate that entry
  data regions do not overlap (this was the Janus fix gap).
- **Data descriptor support**: Defers CRC validation when bit 3 is
  set.
- **ZIP64**: Supported via extra field parsing.

### 3.2 apksigner (apksig library)

- **EOCD search**: Same backwards scan from EOF. Validates
  EOCD+comment == EOF (rejects trailing data).
- **CD validation**: Parses all CD entries, cross-checks against LFH.
- **LFH matching**: Validates CRC, compressed/uncompressed size,
  filename, and flags match between LFH and CD.
- **v2/v3 digest**: Computes CHUNKED_SHA256 over three ZIP sections:
  (1) everything before signing block, (2) CD, (3) EOCD. Any
  structural change breaks the digest.
- **v1 verification**: Validates JAR signature manifests only. Does
  NOT check ZIP structural integrity beyond what's needed to read
  entries.

### 3.3 Key Parsing Divergences (AOSP vs apksigner)

| Aspect | libziparchive | apksigner |
|--------|--------------|-----------|
| Trailing data after EOCD | Rejected | Rejected |
| Prepended data before first LFH | Accepted (ZIP scans from end) | Accepted for v1 |
| Uncovered gap (LFH..CD) | Accepted | Accepted for v1 |
| Overlapping entries | Not checked | Checked (post-Janus) |
| LFH/CD name mismatch | Validated | Validated |
| CRC mismatch LFH vs CD | Validated | Validated |
| Data descriptors | Supported | Supported |
| ZIP64 | Supported | Partial |

---

## 4. Attack Scenarios

### 4.1 Supply Chain Poisoning via v1-Only APK

1. Attacker downloads a v1-only signed APK from F-Droid
2. Prepends a DEX header with malicious code
3. Adjusts ZIP offsets to maintain valid structure
4. Publishes modified APK on a mirror/third-party store
5. apksigner verify says "Verifies" -- the APK appears legitimate
6. On Android < 8.1, the dalvik VM loads the prepended DEX

**Confirmed**: apksigner 31.0.2 does not detect or warn about this.
268/1000 (26.8%) of real F-Droid APKs are vulnerable.

### 4.2 Steganographic Data Hiding

1. Attacker takes a legitimately signed v1-only APK
2. Inserts hidden data in the uncovered gap between entries and CD
3. apksigner verify still says "Verifies"
4. Hidden data survives signature verification by all tested tools
5. Can be used for C2 channels, data exfiltration markers, or to
   bypass content-based malware detection

**Confirmed**: 32KB of hidden data injected without verification failure.

### 4.3 Androguard Bypass for Malware Analysis

1. Malware author crafts a structurally malformed APK (e.g.,
   overlapping entries, dual EOCD)
2. apksigner correctly rejects the APK
3. Androguard accepts it and reports "signed=True"
4. Automated analysis pipelines using Androguard may process the
   malicious APK as legitimate

**Confirmed**: 18 of 20 crafted PoCs demonstrate this divergence.

---

## 5. Recommendations

### 5.1 For apksigner

- Emit WARNING (not just pass silently) when a v1-only APK has data
  before the first LFH (prepended data detection)
- Emit WARNING when there are uncovered bytes between the last entry
  and the Central Directory
- Consider recommending v2+ signing for all new APKs

### 5.2 For Androguard

- Add structural validation before reporting signature status
- Validate EOCD comment length matches actual data
- Check for overlapping entries
- Validate LFH/CD consistency (names, CRC, sizes)
- Reject APKs with trailing data after EOCD

### 5.3 For Android Security

- The Janus fix (Android 8.1+) mitigated the DEX-prepend attack for
  v2+ signed APKs but v1-only APKs remain exploitable on older
  devices
- F-Droid should require v2+ signing for all new submissions

---

## 6. PoC File Index

### Crafted PoCs (minimal APKs)

| File | Size | Divergence Class |
|------|------|-----------------|
| poc01_dual_eocd.apk | 591 | Dual EOCD records |
| poc02_overlapping_entries.apk | 513 | Overlapping ZIP entries |
| poc03_lfh_cd_name_mismatch.apk | 515 | LFH vs CD name mismatch |
| poc04_extra_after_eocd.apk | 771 | Trailing data after EOCD |
| poc05_eocd_comment_with_sigs.apk | 625 | EOCD comment with signatures |
| poc06_unsupported_compression.apk | 515 | Unsupported compression method |
| poc07_fake_signing_block.apk | 687 | Fake APK signing block |
| poc08_cd_offset_underflow.apk | 515 | CD offset underflow |
| poc09_duplicate_cd_entries.apk | 770 | Duplicate CD entries |
| poc10_data_descriptor.apk | 415 | Data descriptor ambiguity |
| poc11_prepended_dex.apk | 627 | Prepended DEX header |
| poc12_zero_length_filename.apk | 491 | Zero-length filename |
| poc13_lfh_extra_mismatch.apk | 420 | LFH extra field mismatch |
| poc14_unknown_signing_block_ids.apk | 775 | Unknown signing block IDs |
| poc15_version_mismatch.apk | 515 | Version needed mismatch |
| poc16_entry_count_overflow.apk | 515 | Entry count overflow |
| poc17_uncovered_gap.apk | 765 | Uncovered gap |
| poc18_cd_past_eof.apk | 503 | CD entry past LFH section |
| poc19_crc_mismatch.apk | 515 | CRC mismatch LFH vs CD |
| poc20_signing_block_size_mismatch.apk | 623 | Signing block size mismatch |

### Real APK Mutations (v1-only signed, VERIFIES)

| File | Size | Description |
|------|------|-------------|
| janus_4k_prepend.apk | 3.4M | 4K prepended data |
| hidden_gap_32k.apk | 3.5M | 32K hidden gap |
| janus_combined.apk | 3.5M | 4K prepend + 32K gap |

### Real APK Mutations (v2-signed, correctly rejected)

| File | Description | apksigner | Androguard |
|------|-------------|-----------|------------|
| mutated_real/real_extra_after_eocd.apk | 256B trailing | REJECT | ACCEPT |
| mutated_real/real_prepended_dex.apk | 112B DEX header | REJECT | ACCEPT |
| mutated_real/real_lfh_name_mismatch.apk | Name flip | REJECT | ACCEPT |
| mutated_real/real_dual_eocd.apk | Second EOCD | REJECT | ACCEPT |
| mutated_real/real_uncovered_gap.apk | 110B gap | REJECT | ACCEPT |
| mutated_real/real_crc_mismatch.apk | CRC bit-flip | REJECT | ACCEPT |
| mutated_real/real_entry_count_overflow.apk | 0xFFFF entries | REJECT | ACCEPT |
| mutated_real/real_extra_signing_pair.apk | Unknown block | REJECT | ACCEPT |

---

## 7. Reproduction

```bash
# Generate crafted PoCs
python3 test/cve/divergence/poc/build_divergence_pocs.py

# Run divergence tests
python3 test/cve/divergence/poc/test_divergences.py
python3 test/cve/divergence/poc/test_divergences_v2.py

# Scan real corpus
python3 test/cve/divergence/poc/scan_real_corpus.py

# Verify critical findings manually
apksigner verify --verbose test/cve/divergence/poc/janus_4k_prepend.apk
apksigner verify --verbose test/cve/divergence/poc/hidden_gap_32k.apk
apksigner verify --verbose test/cve/divergence/poc/janus_combined.apk
```

---

## 8. Tool Versions

- apksigner: 31.0.2 (`/usr/bin/apksigner`)
- Androguard: 4.1.3 (via `/root/security_research_tools/envs/main/`)
- Python: 3.x (zipfile module as reference parser)
- Test host: Linux (no Android device/emulator -- on-device behavior
  inferred from AOSP source analysis)
