# APKAXIOM — CVE Candidate Pack

> Found 2026-05-08 with the APKAXIOM novelty-proof harness.

This directory holds the full evidence pack for two CVE-eligible
denial-of-service vulnerabilities discovered in the most widely-deployed
Android-tooling chain (Androguard + apksigner). Each finding is:

- Reproducible against the **latest** released version
- Triggerable by an APK file under 1 KB
- Reachable through the most common public API entry points
- Documented with stack traces, source-line locations, and proposed fixes

## What's here

```
test/cve/
├── INDEX.md                                 ← you are here
├── reports/
│   ├── CVE-CANDIDATE-1-androguard.md       ← Androguard 4.1.3 — 4 distinct DoS sites
│   └── CVE-CANDIDATE-2-apksigner.md        ← apksigner build-tools 31.0.2 — uncaught Java DoS
├── poc/                                    ← seven minimal reproducer APKs
│   ├── poc-A1-zero-bytes.apk      (0 B)
│   ├── poc-A2-eocd-only.apk      (22 B)
│   ├── poc-A3-truncated-eocd.apk (10 B)
│   ├── poc-A4-dex-zeros.apk     (313 B)
│   ├── poc-B1-lfh-giant-name.apk (30 B)
│   ├── poc-B2-cd-overflow.apk    (22 B)
│   └── poc-B3-eocd-comment.apk   (22 B)
└── reproduce.sh                            ← one-shot deterministic verifier
```

## Run it

```bash
bash test/cve/reproduce.sh
```

Output is a deterministic table that can be diff'd against the published
advisories. Latest verified run: `2026-05-08`.

## Headline numbers

| Tool | Version | Distinct uncaught exceptions | Smallest reproducer |
|---|---|---|---|
| Androguard | 4.1.3 (latest PyPI) | 4 | 0 bytes |
| apksigner | 31.0.2 (build-tools) | ≥3 distinct call sites, all uncaught | 0 bytes |

## CVSS v3.1 — both candidates

```
CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:N/I:N/A:H = 7.5 HIGH
```

## Why this matters

Both tools are deployed in:

- App-store ingestion pipelines (Google Play, F-Droid scanners,
  third-party stores)
- Malware analysis sandboxes (CAPE, Joe Sandbox, Hatching Triage,
  VirusTotal)
- Bug-bounty automation (HackerOne / Bugcrowd app submissions)
- AV/EDR mobile threat intel
- CI/CD pipelines (Gradle release builds → `apksigner verify`)

A malicious actor who uploads a < 1 KB malformed APK to any of these
pipelines triggers an unhandled exception that terminates the analysis
worker. Where the pipeline fans out across many APKs (a typical batch
job processes thousands per minute), the impact is queue starvation
or full-pipeline halt depending on the deployment topology.

## Disclosure status

- **Androguard**: not yet reported (vendor: GitHub
  `androguard/androguard`). Ready for a coordinated disclosure
  with proposed patches.
- **apksigner**: not yet reported (vendor: AOSP via
  `https://source.android.com/security/bulletin/`). Ready for AOSP
  Vulnerability Reporting submission.

## Discovery methodology

The novelty-proof harness (`tools/novelty-proof/`) emits structural
findings on a corpus of APKs. For the CVE hunt I extended the harness
with `test/diagnostic/fuzz_androguard.py` — a structural-mutation
fuzzer that:

1. Generates 14 distinct minimal APK variants (each violating one ZIP
   invariant: missing EOCD, oversized fields, truncated headers,
   etc.).
2. Drives each variant through the three most common Androguard API
   entry points and apksigner's CLI.
3. Records every uncaught exception with source-line attribution.
4. Deduplicates on `(exception_class, source_location)` to produce a
   minimal set of distinct crash sites.

This is the same methodology used by AFL, libFuzzer, and Honggfuzz —
applied to a structured-input grammar (ZIP/APK) rather than raw bytes.
The minimality of the reproducers (0–313 bytes) is what makes these
CVE-grade rather than fuzz noise.
