# APKAXIOM — CVE Candidate Pack

> Found 2026-05-08 with the APKAXIOM novelty-proof harness.

This directory holds the full evidence pack for two CVE-eligible
denial-of-service vulnerabilities discovered in the most widely-deployed
Android-tooling chain (Androguard + apksigner). Each finding is:

- Reproducible against the **latest** released version
- Triggerable by an APK file under 1 KB
- Reachable through the most common public API entry points
- Proven end-to-end with 9 impact demonstrations (all passing)
- Cross-validated: Python stdlib + jadx handle every PoC gracefully

## What's here

```
test/cve/
├── INDEX.md                                 ← you are here
├── reports/
│   ├── CVE-CANDIDATE-1-androguard.md       ← Androguard 4.1.3 — 4 distinct DoS sites
│   └── CVE-CANDIDATE-2-apksigner.md        ← apksigner 31.0.2 — uncaught Java DoS + OOM
├── poc/                                    ← 10 minimal reproducer APKs
│   ├── poc-A1-zero-bytes.apk        (0 B)   ← crashes both tools
│   ├── poc-A2-eocd-only.apk        (22 B)   ← KeyError in Androguard
│   ├── poc-A3-truncated-eocd.apk   (10 B)   ← struct.error in Androguard
│   ├── poc-A4-dex-zeros.apk       (313 B)   ← DEX ValueError in Androguard
│   ├── poc-B1-lfh-giant-name.apk   (30 B)   ← malformed ZIP
│   ├── poc-B2-cd-overflow.apk      (22 B)   ← CD offset overflow
│   ├── poc-B3-eocd-comment.apk     (22 B)   ← EOCD comment truncation
│   ├── poc-C1-oom-giant-size.apk  (136 B)   ← OOM: 136 B → 2 GB JVM alloc
│   ├── poc-C2-legal-no-manifest.apk(150 B)  ← LEGAL valid ZIP, no manifest
│   └── poc-C3-legal-stub-manifest.apk(152B) ← LEGAL valid ZIP, stub manifest
├── reproduce.sh                            ← crash matrix (quick)
└── prove_impact.sh                         ← full 9-demo end-to-end proof
```

## Run it

```bash
# Quick crash matrix
bash test/cve/reproduce.sh

# Full end-to-end impact proof (9 demonstrations)
bash test/cve/prove_impact.sh
```

## Headline numbers

| Tool | Version | Distinct crash classes | Smallest trigger | Worst impact |
|---|---|---|---|---|
| Androguard | 4.1.3 (latest PyPI) | 4 | 0 bytes | Batch-pipeline kill |
| apksigner | 31.0.2 (build-tools) | 3+ (all uncaught) | 0 bytes | OOM: 136 B → 2 GB alloc (15.8M× amplification) |

## Impact demonstrations (9/9 PASS)

| # | Demo | Result |
|---|---|---|
| 1 | Uncaught exception kills Python process | PASS — `print()` after crash never runs |
| 2 | Uncaught exception kills JVM process | PASS — 7 stack frames leaked |
| 3 | Batch-pipeline poisoning | PASS — 1 poison APK kills entire 5-APK queue |
| 4 | CI/CD pipeline kill | PASS — `apksigner verify` crash halts release |
| 5 | Legal APK triggers crash | PASS — valid ZIP + AOSP-optional field = KeyError |
| 6 | OOM resource exhaustion | PASS — 136 bytes triggers `OutOfMemoryError` |
| 7 | Controllable amplification | PASS — 32/128/512 MB variants all crash JVM |
| 8 | Cross-tool comparison | PASS — zipfile + jadx handle all 10 PoCs gracefully |
| 9 | Full reproduction matrix | PASS — 19/30 Androguard cells crash, 10/10 apksigner crash |

## CVSS v3.1 — both candidates

```
CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:N/I:N/A:H = 7.5 HIGH
```

## Why this matters

Both tools are deployed in:

- App-store ingestion pipelines (Google Play, F-Droid scanners,
  third-party stores)
- Malware analysis sandboxes (CAPE, Joe Sandbox, Hatching Triage,
  VirusTotal, Pithus)
- Bug-bounty automation (HackerOne / Bugcrowd app submissions)
- AV/EDR mobile threat intel
- CI/CD pipelines (Gradle release builds → `apksigner verify`)

A malicious actor who uploads a < 1 KB malformed APK to any of these
pipelines triggers an unhandled exception that terminates the analysis
worker. The OOM variant (poc-C1) amplifies 136 bytes into a 2 GB heap
allocation, exhausting the JVM. Where the pipeline fans out across many
APKs (a typical batch job processes thousands per minute), the impact
is queue starvation or full-pipeline halt.

## Disclosure status

- **Androguard**: not yet reported (vendor: GitHub
  `androguard/androguard`). Ready for coordinated disclosure
  with proposed patches.
- **apksigner**: not yet reported (vendor: AOSP via
  `https://source.android.com/security/bulletin/`). Ready for AOSP
  Vulnerability Reporting submission.

## Discovery methodology

The novelty-proof harness (`tools/novelty-proof/`) emits structural
findings on a corpus of APKs. For the CVE hunt I extended the harness
with `test/diagnostic/fuzz_androguard.py` — a structural-mutation
fuzzer that:

1. Generates 14+ distinct minimal APK variants (each violating one ZIP
   invariant: missing EOCD, oversized fields, truncated headers, CD
   size lies, etc.).
2. Drives each variant through the three most common Androguard API
   entry points and apksigner's CLI.
3. Records every uncaught exception with source-line attribution.
4. Deduplicates on `(exception_class, source_location)` to produce a
   minimal set of distinct crash sites.
5. Cross-validates against baseline tools (Python zipfile, jadx) to
   confirm the vulnerability is in the target tools, not the inputs.

This is the same methodology used by AFL, libFuzzer, and Honggfuzz —
applied to a structured-input grammar (ZIP/APK) rather than raw bytes.
The minimality of the reproducers (0–313 bytes) is what makes these
CVE-grade rather than fuzz noise.
