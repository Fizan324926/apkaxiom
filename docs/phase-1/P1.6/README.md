# P1.6 — Lean ZIP Layer: Central Directory + Cross-Record Consistency

> The hardest pure-ZIP theorem. Central directory, offset arithmetic, consistency across LFH/CDR/EOCD. Adversarial corpus drawn from public BadPack-class samples.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../README.md §6](../../README.md#layer-1)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P1.6 |
| Owner(s) | G1 (Formal Methods Core) |
| Duration | Weeks 6–10 |
| Critical-path | **yes** |
| Hard prerequisites | P1.5 (LFH + EOCD theorems) |

## 2. Goal & Scope

The ZIP central directory record (CDR) is formalized in Lean 4, including offset arithmetic and the relationship between CDR entries and their referenced local file headers. A consistency theorem states: if `parseCdr` accepts a record `cdr`, then the offset `cdr.offset_to_lfh` is in-bounds and the bytes at that offset parse as a matching LFH.

This sub-phase is where the **first real BadPack-class evasion attacks** are formally distinguished — many BadPack tricks exploit CDR/LFH disagreement. Our theorem disallows them by construction; the differential harness validates we caught the same set Android does.

### In scope
- `theorems/Apkaxiom/Zip/CentralDirectory.lean` (~1,000 LOC).
- `theorems/Apkaxiom/Zip/Consistency.lean` — connecting LFH + CDR + EOCD invariants.
- Theorem `cdr_lfh_offset_valid : ∀ cdr lfh bs, parseCdr bs = ok cdr → cdr.offset_to_lfh < bs.size ∧ parseLfh (bs.drop cdr.offset_to_lfh) = ok lfh ∧ lfh.matches cdr`.
- Adversarial corpus: ≥ 500 BadPack-style malformed ZIPs (sourced from public CVE write-ups and our own variations).
- Differential harness extended to whole-archive parse (LFH + CDR + EOCD jointly).

### Out of scope
- APK Signing Block (P1.11).
- Compressed-data integrity (CRC checks are part of the CDR theorem; deflate decompression is not — it's deterministic and fast in Rust, doesn't need formalization).

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P1.5** | LFH + EOCD theorems and modules |
| **P1.5** | AOSP libziparchive build harness |
| **P1.5** | Differential test driver |

## 4. Required Tools, Libraries, and Languages

Same toolchain as P1.5. New requirements:

| Tool | Version | Purpose |
|---|---|---|
| **AFL++** | 4.x | Adversarial corpus expansion (mutation-based) |
| **honggfuzz** | 2.x | Alternative coverage-guided fuzzer |
| **radamsa** | latest | Mutation tool for fuzz-corpus seeding |
| **z3** | 4.12+ (HAVE) | Useful for confirming offset-arithmetic invariants when Lean's `omega` tactic is too slow |
| **mathlib4** `Mathlib.Tactic.Linarith` / `Mathlib.Tactic.Omega` | from P1.2 | Heavy lifting for offset arithmetic |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL / Account | Notes |
|---|---|---|---|---|
| **AFL++** | fuzzer | **Free** OSS (Apache 2.0) | https://github.com/AFLplusplus/AFLplusplus | Used here as a *generator*; production fuzzing is P1.13/P1.14 with Nyx |
| **honggfuzz** | fuzzer | **Free** OSS | https://github.com/google/honggfuzz | Alt generator |
| **radamsa** | mutator | **Free** OSS | https://gitlab.com/akihe/radamsa | Mutation-only |
| **NIST NVD CVE database** | vulnerability data | **Free** | https://nvd.nist.gov | Source for BadPack-class CVE references (CVE-2023-3...x family); no API key for read-only |
| **Cleafy threat reports** | malware research | **Free** read-only | https://www.cleafy.com/cleafy-labs | Public technical writeups on Android malformation attacks |
| **AndroZoo / DREBIN / MalwareBazaar** | malware corpora | **Free** academic / public | (already provisioned in P1.3) | Sources for real-world BadPack samples |

**No API keys.** NVD has rate limits without a key; we apply for a free key at https://nvd.nist.gov/developers/request-an-api-key for higher rate limits, useful for automated corpus updates.

## 6. System Inventory — Have vs Need

### Already present
- ✅ Everything from P1.5
- ✅ z3 4.12.5 (HAVE on host)

### Missing — must install
- ❌ **AFL++** (`sudo apt-get install -y afl++` or build from source for newer)
- ❌ **honggfuzz** (build from source)
- ❌ **radamsa** (build from source)

### Install commands

```bash
# AFL++
sudo apt-get install -y afl++ afl++-clang
# or for newer:
git clone https://github.com/AFLplusplus/AFLplusplus
cd AFLplusplus && make distrib && sudo make install

# honggfuzz
git clone https://github.com/google/honggfuzz && cd honggfuzz && make && sudo make install

# radamsa
git clone https://gitlab.com/akihe/radamsa && cd radamsa && make && sudo make install
```

## 7. Working Directory & Files Produced

```
apkaxiom/
├── theorems/
│   └── Apkaxiom/
│       └── Zip/
│           ├── CentralDirectory.lean   # NEW — ~1,000 LOC
│           └── Consistency.lean         # NEW — ~600 LOC
├── corpus/
│   └── zip/
│       ├── badpack-cves/                # NEW — labeled CVE samples
│       ├── adversarial-mutated/         # NEW — radamsa+AFL++ output
│       └── full-archive-valid/          # NEW — well-formed end-to-end
├── tests/
│   └── differential/
│       └── src/main.rs                  # extended for full-archive
└── docs/
    └── lean-zip-layer.md                # updated with CDR/consistency
```

## 8. Standalone Output

Two new Lean modules + extended corpus + extended differential harness. Verifiable in isolation:

```bash
nix develop
buck2 build //theorems:zip-cdr //theorems:zip-consistency
buck2 test //tests/differential:full-archive
# Output: "2200/2200 ZIP archives Lean ↔ libziparchive agreed (incl. 500 adversarial)"
```

## 9. End-to-End Test

Full ZIP archives (not just LFHs) flow through Lean and through the AOSP A14 reference. Mismatches fail the test. Adversarial corpus drawn from:
- Real BadPack CVE samples (publicly disclosed, anonymized).
- AFL++ mutations of valid ZIPs.
- radamsa-derived inputs.
- Hand-crafted offset-confusion attacks.

```bash
make zip-full-differential CORPUS=corpus/zip
# Reports: total tests, agreement rate, time per case, divergent inputs (if any)
```

## 10. Exit Checklist

- [ ] CDR theorem stated and proved (≥ 1,000 LOC)
- [ ] Cross-record consistency theorem proved (≥ 600 LOC)
- [ ] Cumulative Lean LOC ≥ 2,000 (HARD per PHASE_GATES.md §5)
- [ ] Theorem re-verify on CI ≤ 25 min
- [ ] ≥ 500 adversarial inputs in corpus
- [ ] 100% Lean ↔ libziparchive agreement on full corpus (HARD)
- [ ] AFL++ + radamsa producing nightly new mutation samples
- [ ] BadPack-class CVE samples reproduce as expected (Lean rejects, libziparchive rejects)
- [ ] Documentation updated

## 11. Hand-Off

| Consumed by | What they need |
|---|---|
| **P1.9** | Full ZIP layer ready for extraction |
| **P1.11** | Signing block sits inside the ZIP envelope; needs the proved consistency property |
| **P1.12** | The full ZIP extraction target |
| **P1.13** | Adversarial corpus seeds Nyx |
| **P1.18** | Adversarial corpus = `Adversarial-500` (per PHASE_GATES.md §4) |
