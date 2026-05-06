# P1.11 — Lean Signing-Scheme Design

> Mechanized verifier predicates for APK Signature Schemes
> v1 (JAR), v2, v3, v3.1, plus the cross-scheme dispatcher.
> A reviewer should be able to read this file and re-implement
> any scheme in any language, byte-for-byte compatible with the
> Lean spec + Rust mirror.

**Status:** Frozen at P1.11 closure (2026-05-06). Any change to a
scheme's wire format, leaf-formation rule, or verifier predicate
is a protocol change — re-stamp the differential receipts and
bump the changelog.

---

## 1. Goals

A consumer of an APK should be able to:

  1. **Locate the signing block** — given any APK, find the v2/v3/v3.1
     carrier between the last LFH body and the central directory,
     or determine that the APK is unsigned (or v1-only via JAR).
  2. **Walk every signer** — for each present scheme, decode the
     verbatim bytes into a typed structure (digests, certificates,
     signatures, public-key SPKI, SDK ranges).
  3. **Verify the signatures** — given a cryptographic oracle
     (`hash`, `verify`, `extractSpki`), decide accept/reject for
     each scheme + cross-scheme dispatcher.
  4. **Detect adversarial inputs** — Janus, scheme-stripping
     downgrade, length-extension, malformed-block attacks all
     reject identically across Lean ↔ Rust ↔ AOSP `apksigner`.

Goal #4 is the load-bearing soundness gate; verified by
`make p111-differential` on the 7-honest + 9-adversarial
corpus.

## 2. Wire formats

### 2.1 APK signing block (v2/v3/v3.1 carrier)

Layout (little-endian; per AOSP `tools/apksig`):

```text
[u64  size_of_block      — bytes from trailing u64 backwards]
[pairs ...]
   each pair:
     [u64 length          — 4-byte ID + value size]
     [u32 id]
     [length - 4 bytes value]
[u64  size_of_block       — must equal the leading u64]
[16-byte magic = "APK Sig Block 42"]
```

The block sits immediately before the central directory; the
EOCD's `cd_offset` points at the byte AFTER the magic. Locator:

  1. Find EOCD signature; read `cd_offset`.
  2. Read 16 bytes at `cd_offset - 16`; must equal `magic`.
  3. Read u64 at `cd_offset - 24` — `size_of_block`.
  4. Block starts at `cd_offset - size_of_block - 8`.

Known block IDs:

| ID | Name |
|---|---|
| `0x7109871a` | APK Signature Scheme v2 |
| `0xf05368c0` | APK Signature Scheme v3 |
| `0x1b93ad61` | APK Signature Scheme v3.1 |
| `0x6dff800d` | AOSP zero-padding (block alignment) |
| `0x2b09189e` | Source Stamp v1 |
| `0x42726577` | Source Stamp v2 |

Lean: [`Apkaxiom.Signing.Block`](../../../theorems/Apkaxiom/Signing/Block.lean).
Rust: [`crates/axiom-sigblock/src/lib.rs`](../../../crates/axiom-sigblock/src/lib.rs).

### 2.2 v2 / v3 / v3.1 signed-data

Each scheme block is a length-prefixed sequence of *signers*:

```text
signer (length-prefixed):
  signed_data (length-prefixed)
  [v3 / v3.1 only: min_sdk u32, max_sdk u32]
  signatures (length-prefixed sequence of length-prefixed signature elts)
  public_key (length-prefixed bytes — SubjectPublicKeyInfo DER)
```

`signed_data` decomposes as:

```text
signed_data:
  digests (length-prefixed sequence)
  certificates (length-prefixed sequence of length-prefixed X.509 DER)
  [v3 / v3.1 only: min_sdk u32, max_sdk u32]
  additional_attributes (length-prefixed seq of (id u32 || bytes))
```

Digest element: `algorithm_id u32 || length-prefixed digest`.
Signature element: `algorithm_id u32 || length-prefixed signature`.

v3 / v3.1 invariant (mechanized in `Apkaxiom.Signing.Scheme.parseSigner`):
the `(min_sdk, max_sdk)` pairs at the signer envelope and inside
`signed_data` MUST match — otherwise reject with
`SchemeError.v3SdkRangeMismatch`.

Lean: [`Apkaxiom.Signing.Scheme`](../../../theorems/Apkaxiom/Signing/Scheme.lean).
Rust: [`crates/axiom-sigblock/src/scheme.rs`](../../../crates/axiom-sigblock/src/scheme.rs).

### 2.3 Signature-algorithm IDs

| ID | Algorithm |
|---|---|
| `0x0101` | RSA-PSS+SHA-256, 1 MiB-chunked SHA-256 |
| `0x0102` | RSA-PSS+SHA-512, 1 MiB-chunked SHA-512 |
| `0x0103` | RSA-PKCS1-v1.5+SHA-256, chunked SHA-256 |
| `0x0104` | RSA-PKCS1-v1.5+SHA-512, chunked SHA-512 |
| `0x0201` | ECDSA+SHA-256, chunked SHA-256 |
| `0x0202` | ECDSA+SHA-512, chunked SHA-512 |
| `0x0301` | DSA+SHA-256, chunked SHA-256 |
| `0x0421` | RSA-PKCS1+SHA-256 over Verity tree root |
| `0x0423` | ECDSA+SHA-256 over Verity tree root |
| `0x0425` | DSA+SHA-256 over Verity tree root |

Lift in Lean: `Apkaxiom.Signing.Scheme.SignatureAlgorithmId.fromU32`.

### 2.4 v1 (JAR) META-INF

A JAR-signed APK has `META-INF/MANIFEST.MF`, one or more
`META-INF/<KEY>.SF` "signature files", and one or more
`META-INF/<KEY>.{RSA,DSA,EC}` PKCS#7 SignedData blocks.
Verification:

  1. .SF must contain SHA-256 (or SHA-1 legacy) digest of
     MANIFEST.MF.
  2. PKCS#7 SignedData's signed bytes must be the .SF byte-for-byte.
  3. PKCS#7 signature must verify under the certificate chain.
  4. For every regular APK entry (non-META-INF), MANIFEST.MF
     must declare its SHA digest, and that digest must match the
     re-computed SHA over the entry body.

Lean: [`Apkaxiom.Signing.V1`](../../../theorems/Apkaxiom/Signing/V1.lean).

## 3. Verifier predicates

Each scheme exposes a `verifyV*` predicate that returns
`accept` or one of a closed list of reject categories. The
predicate is parameterised over a `CryptoOracle` (v1) /
`CryptoOracle` (v2/v3) — the HACL\* binding-surface declared
in [`Apkaxiom.Signing.Crypto`](../../../theorems/Apkaxiom/Signing/Crypto.lean).

**Reject categories** (per scheme, distinct tag bytes for
cross-language interop):

  - v1 (`V1VerifyResult`): `rejectNoManifest`, `rejectNoSf`,
    `rejectNoSigBlock`, `rejectManifestDigestMismatch`,
    `rejectSfManifestDigestMismatch`, `rejectPkcs7VerifyFailed`,
    `rejectMissingManifestEntry`, `rejectJanusCve_2017_13156`.
  - v2 (`V2VerifyResult`): `rejectNoV2Block`, `rejectMalformed`,
    `rejectNoDigests`, `rejectNoSignatures`,
    `rejectNoCertificates`, `rejectAlgorithmMismatch`,
    `rejectDigestMismatch`, `rejectSignatureFailed`,
    `rejectPublicKeyMismatch`, `rejectAllAlgorithmsUnknown`,
    `rejectJanusCve_2017_13156`.
  - v3 (`V3VerifyResult`): same as v2 plus `rejectSdkRangeMismatch`
    and `rejectDowngradeAttempt`.
  - v3.1: aliased to v3's result (same shape).

## 4. Cross-scheme dispatch

A device picks the strongest scheme it understands:

  - Android 13+ (API 33+): if v3.1 is present, use it.
  - Android 9+  (API 28+): if v3 is present, use it.
  - Android 7+  (API 24+): if v2 is present, use it.
  - Otherwise: fall back to v1 (JAR).

The dispatcher
([`Apkaxiom.Signing.Dispatch.dispatchVerify`](../../../theorems/Apkaxiom/Signing/Dispatch.lean))
runs every PRESENT scheme and folds the results — accept iff
every present scheme accepts AND the v3 / v3.1 coexistence
invariant holds AND at least one scheme is present.

The Boolean acceptance condition
(`dispatchAcceptCondition`) is decidable; the differential
harness asserts equivalence to the dispatcher's `isAccept`
projection on every fixture.

## 5. Threat model

The verifier rejects:

  - **Janus (CVE-2017-13156)** — DEX-prepended APKs. v2+ catches
    via the whole-file digest; v1-only is structurally
    vulnerable, but the dispatcher's parser layer rejects the
    malformed ZIP CD before the v1 verifier even sees the input
    (verified by 2/9 adversarial fixtures).
  - **Scheme stripping (downgrade)** — v3 block ID rewritten to
    padding, or v3.1 present without v3. Caught by
    `Dispatch.coexistenceOk` + the Boolean acceptance condition.
  - **Length-extension** — pair-overflow, pair-too-short, size-
    mismatch — surfaced as `Block.ParseError` variants.
  - **Magic-flip** — `bad-magic.apk`; surfaced as block-not-found
    (parser falls back to "unsigned").
  - **Truncation** — eocd-removed or block-cut-mid-pair; both
    surfaced as `truncated` / `noEocd` errors.

The verifier does NOT (by design) detect:

  - **Replay of an earlier well-formed signed APK in place of a
    newer one.** Freshness is not in scope; higher-level
    protocols (timestamping, version signing) handle this.
  - **EOCD-comment region tampering.** The 0–65 535 byte comment
    after the EOCD is not committed.

## 6. Performance contract

  - **Lean re-verify** (`lake build Apkaxiom.Signing.*`): ≤ 45 min
    on CI (spec gate). Measured ~6 s on dev-shell.
  - **Rust mirror** (`tools/sig-eval-rust`): O(input size)
    streaming over stdin; < 1 ms per APK on the 16-fixture
    corpus.
  - **Differential gate** (`make p111-differential`): runs
    Lean + Rust + apksigner over 16 APKs in ~ 30 s.

## 7. References

  - [APK Signature Scheme v2](https://source.android.com/docs/security/features/apksigning/v2)
  - [APK Signature Scheme v3](https://source.android.com/docs/security/features/apksigning/v3)
  - [APK Signature Scheme v3.1](https://source.android.com/docs/security/features/apksigning/v3-1)
  - [JAR signing specification](https://docs.oracle.com/javase/8/docs/technotes/guides/jar/jar.html#Signed_JAR_File)
  - [CVE-2017-13156 — Janus exploit](https://nvd.nist.gov/vuln/detail/CVE-2017-13156)
  - AOSP `tools/apksig` source — the canonical reference verifier.
  - [ADR-0029](./ADR-0029-hacl-signing-deviation.md) — HACL\* deviation rationale.
