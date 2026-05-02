# P1.11 — Lean APK Signing Block v1/v2/v3/v3.1

> Mechanize all four APK signing schemes in Lean. Cross-check against AOSP `apksigner` on 2,000 signed APKs. Includes adversarial samples (length extension, downgrade attacks).

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../README.md §6 (Layer 1)](../../README.md#layer-1)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P1.11 |
| Owner(s) | G1 |
| Duration | Weeks 9–14 |
| Critical-path | **yes** — gates the verified-signing path |
| Hard prerequisites | P1.6 (full ZIP layer formalized — signing block sits inside ZIP) |

## 2. Goal & Scope

All four APK signing schemes (v1 JAR, v2, v3, v3.1) are formalized in Lean 4. A theorem states: `Lean.verifySignature accepts iff apksigner verify accepts` on the same input. Adversarial samples (modified signatures, length-extension attempts, scheme-downgrade attacks) are part of the corpus.

This is the **largest pure-Lean sub-phase** in Phase 1 — ~3,000 LOC of Lean across the four schemes plus the cross-scheme dispatch logic.

### In scope
- `theorems/Apkaxiom/Signing/V1.lean` (JAR signing, ~700 LOC)
- `theorems/Apkaxiom/Signing/V2.lean` (~800 LOC)
- `theorems/Apkaxiom/Signing/V3.lean` (~900 LOC)
- `theorems/Apkaxiom/Signing/V3_1.lean` (~600 LOC, smaller delta over V3)
- Cross-scheme dispatch theorem
- Soundness against AOSP `apksigner` reference
- Differential corpus: 2,000 signed APKs (mix of all 4 schemes) + adversarial samples

### Out of scope
- Third-party signing-block formats (Stamp, Channel, Vasdolly, Packer NG) — Phase 2.
- Rust extraction (P1.16).
- Signature *creation* — we only formalize *verification*.

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P1.6** | Full ZIP layer theorems — signing block is a ZIP-internal construct |
| **P1.4** | AXIOM-IR types for signature certificates |
| **P1.10** | HACL\* BLAKE3 verified hash bindings (signing v2/v3/v3.1 use SHA-256, also via HACL\*) |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Lean 4 + mathlib4** | pinned | Formalization |
| **AOSP tools/apksig** | pinned commit per Android version | Reference signature verifier |
| **apksigner** binary | from Android SDK Build Tools 35.x+ | Reference verifier |
| **HACL\*** | from P1.10 | SHA-256, RSA-PKCS1, RSA-PSS, ECDSA, Ed25519 verifiers |
| **fiat-crypto** | latest | Alternative source for elliptic-curve ops if HACL\* coverage thin |
| **Java 17/21** (HAVE) | for apksigner | apksigner is Java |
| **Bazel sub-workspace** (from P1.5) | for AOSP libs | Reproducible apksigner build |
| **bouncy-castle** | reference Java crypto | Cross-check (only as oracle) |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **Android SDK Build Tools** (`apksigner`) | reference verifier | **Free** | https://developer.android.com/tools/releases/build-tools | Apache 2.0; needs Android SDK manager or direct download |
| **AOSP `tools/apksig` source** | source for apksigner | **Free** OSS (Apache 2.0) | https://android.googlesource.com/platform/tools/apksig | Already partially synced in P1.5 |
| **HACL\* / EverCrypt** | verified crypto | **Free** OSS (Apache 2.0) | https://github.com/hacl-star/hacl-star | From P1.10 |
| **fiat-crypto** | verified field arithmetic | **Free** OSS | https://github.com/mit-plv/fiat-crypto | MIT |
| **Bouncy Castle** | reference Java crypto | **Free** OSS (MIT) | https://www.bouncycastle.org | Only used as cross-check |
| **AndroZoo signed corpus** | 2,000 real signed APKs | **Free academic** | https://androzoo.uni.lu | API key needed (provisioned in P1.3) |
| **F-Droid** | signed-by-known-keys APKs | **Free** | https://f-droid.org/archive/ | Useful for ground-truth verifier checks |
| **NIST CVE database** | known signature attacks | **Free** | https://nvd.nist.gov | Janus exploit (CVE-2017-13156) etc. |

**API key:** AndroZoo academic-access (already requested in P1.3 — must be approved by start of this sub-phase).

## 6. System Inventory — Have vs Need

### Already present
- ✅ Lean 4 / Lake (P1.2)
- ✅ Java 21 / javac 17 (HAVE)
- ✅ HACL\* infrastructure (P1.10)
- ✅ AOSP partial sync (P1.5; tools/apksig is part of it)

### Missing — must install
- ❌ **Android SDK Build Tools** — `apksigner` binary
- ❌ **Bouncy Castle** JARs — `apt install libbcprov-java`

### Install commands

```bash
# Android SDK command-line tools
mkdir -p ~/android-sdk && cd ~/android-sdk
curl -L https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip -o cmdline.zip
unzip cmdline.zip && rm cmdline.zip
mv cmdline-tools latest && mkdir cmdline-tools && mv latest cmdline-tools/
export ANDROID_HOME=~/android-sdk
export PATH=$ANDROID_HOME/cmdline-tools/latest/bin:$PATH
yes | sdkmanager "build-tools;35.0.0"
# apksigner now at $ANDROID_HOME/build-tools/35.0.0/apksigner

# Bouncy Castle (Java cross-check)
sudo apt-get install -y libbcprov-java
```

Disk: ~ 2 GB for Android SDK Build Tools.

## 7. Working Directory & Files Produced

```
apkaxiom/
├── theorems/
│   └── Apkaxiom/
│       └── Signing/
│           ├── V1.lean                  # NEW — ~700 LOC
│           ├── V2.lean                  # NEW — ~800 LOC
│           ├── V3.lean                  # NEW — ~900 LOC
│           ├── V3_1.lean                # NEW — ~600 LOC
│           ├── Dispatch.lean             # NEW — cross-scheme dispatch
│           └── Crypto.lean               # NEW — Lean side of HACL*-verified primitives
├── corpus/
│   └── signing/
│       ├── v1-valid/                     # ~500 APKs
│       ├── v2-valid/                     # ~500 APKs
│       ├── v3-valid/                     # ~500 APKs
│       ├── v3.1-valid/                   # ~500 APKs
│       └── adversarial/                  # ~500 attack samples (Janus, length-ext, downgrade)
├── tests/
│   └── differential/
│       └── src/main.rs                   # extended for signature verification
└── docs/
    └── lean-signing.md                   # NEW
```

## 8. Standalone Output

```bash
nix develop
buck2 build //theorems:signing-all
buck2 test //tests/differential:signing-vs-apksigner
# Output: "2500/2500 APKs Lean ↔ apksigner agreed (incl. 500 adversarial)"
```

## 9. End-to-End Test

For every APK in the 2,000-APK signed corpus + 500 adversarial samples:
1. Lean verifier produces `accept` or `reject` (with reason).
2. AOSP `apksigner verify` produces same.
3. Outputs must agree byte-for-byte.

Particular adversarial cases that **must** fail to verify (Lean and apksigner agree on reject):
- Janus exploit (CVE-2017-13156) — DEX prepended to APK
- Mismatched v1/v2 signers
- Truncated signing block
- Length-extended SHA-256 chunked digest
- Downgrade attack (claims v1 only when v3.1 present)

## 10. Exit Checklist

- [ ] All 4 signing schemes formalized; cumulative Lean LOC ≥ 4,000 (HARD)
- [ ] Cross-scheme dispatch theorem proved
- [ ] HACL\* SHA-256, RSA-PKCS1, RSA-PSS, ECDSA, Ed25519 used (no generic crypto)
- [ ] 2,500-APK Lean ↔ apksigner agreement = 100% (HARD)
- [ ] Theorem re-verify on CI ≤ 45 min (HARD per PHASE_GATES.md §5)
- [ ] All listed adversarial cases reject
- [ ] `docs/lean-signing.md` published

## 11. Hand-Off

| Consumed by | What they need |
|---|---|
| **P1.16** | Extraction target — these theorems become extracted Rust |
| **P1.17** | Soundness regression suite incorporates signing theorems |
| **Phase 2 / G4 §13.7 SLSA** | Signature-scheme verifier as input to supply-chain attestation |
