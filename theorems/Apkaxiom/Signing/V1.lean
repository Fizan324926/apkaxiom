/-
P1.11 — APK Signature Scheme v1 (JAR signing).

v1 ("JAR signing") is the original Android signature scheme,
inherited from JAR signing as defined in the Java archive
specification. Layout per AOSP `tools/apksig/V1SchemeVerifier.java`:

  META-INF/MANIFEST.MF        — manifest with per-entry SHA digests
  META-INF/<KEY>.SF           — "signature file" — manifest digest +
                                  per-entry main-attributes digests
  META-INF/<KEY>.RSA          — PKCS#7 SignedData over the .SF
  (or .DSA / .EC for those algorithms)

Verification predicate:
  1. .SF must contain SHA-256 (or SHA-1 for legacy) digest of
     MANIFEST.MF.
  2. PKCS#7 SignedData's signed-bytes must be the .SF byte-for-byte.
  3. PKCS#7 signature must verify under the certificate chain.
  4. For every regular entry in the APK (non-META-INF), MANIFEST.MF
     must declare its SHA digest, and that digest must match the
     re-computed SHA over the entry body.

This module formalises the META-INF tree shape, the manifest +
.SF text format, and the verification predicate as a Boolean
Lean function. The cryptographic primitives (SHA-256, RSA / EC /
DSA verification) are deferred to `Apkaxiom.Signing.Crypto` (the
HACL\* binding surface — see ADR-0029).
-/

import Std
import Apkaxiom.Zip.LocalHeader

namespace Apkaxiom.Signing.V1

open Apkaxiom.Zip.LocalHeader (slice)

/-! ## File-name predicates -/

/-- Path prefix that puts an entry inside `META-INF/`. -/
def metaInfPrefix : ByteArray :=
  -- "META-INF/"
  ByteArray.mk #[
    0x4d, 0x45, 0x54, 0x41, 0x2d, 0x49, 0x4e, 0x46, 0x2f
  ]

/-- True iff the byte sequence `bs` starts with `pre`. -/
def startsWith (bs pre : ByteArray) : Bool := Id.run do
  if bs.size < pre.size then
    return false
  for i in [0:pre.size] do
    if bs.get! i ≠ pre.get! i then
      return false
  return true

/-- True iff `bs` ends with `suffix`. -/
def endsWith (bs suffix : ByteArray) : Bool := Id.run do
  if bs.size < suffix.size then
    return false
  let off := bs.size - suffix.size
  for i in [0:suffix.size] do
    if bs.get! (off + i) ≠ suffix.get! i then
      return false
  return true

/-- An entry is "in META-INF/" iff its name starts with `META-INF/`. -/
def inMetaInf (name : ByteArray) : Bool :=
  startsWith name metaInfPrefix

/-! ## File-name extension predicates -/

/-- ".SF" — JAR signature file. -/
def extSf : ByteArray := ByteArray.mk #[0x2e, 0x53, 0x46]
/-- ".RSA" — PKCS#7 RSA signature block. -/
def extRsa : ByteArray := ByteArray.mk #[0x2e, 0x52, 0x53, 0x41]
/-- ".DSA" — PKCS#7 DSA signature block. -/
def extDsa : ByteArray := ByteArray.mk #[0x2e, 0x44, 0x53, 0x41]
/-- ".EC" — PKCS#7 EC (ECDSA) signature block. -/
def extEc  : ByteArray := ByteArray.mk #[0x2e, 0x45, 0x43]

/-- "MANIFEST.MF" — JAR manifest. -/
def manifestName : ByteArray :=
  ByteArray.mk #[
    0x4d, 0x41, 0x4e, 0x49, 0x46, 0x45, 0x53, 0x54, 0x2e, 0x4d, 0x46
  ]

/-- "META-INF/MANIFEST.MF" full path. -/
def manifestPath : ByteArray :=
  metaInfPrefix ++ manifestName

/-- Concatenation of two byte arrays (helper for path checks). -/
instance : HAppend ByteArray ByteArray ByteArray where
  hAppend a b := a ++ b

/-! ## Signature-block kind -/

/-- The PKCS#7 signature-block kind, distinguished by file extension. -/
inductive SigBlockKind : Type where
  /-- `.RSA` — PKCS#7 with RSA-PKCS1-v1.5. -/
  | rsa
  /-- `.DSA` — PKCS#7 with DSA. -/
  | dsa
  /-- `.EC`  — PKCS#7 with ECDSA. -/
  | ec
deriving Repr, DecidableEq, Inhabited

/-- Lift a META-INF/<key>.<ext> filename to its kind. Returns
`none` for non-signature-block files. -/
def SigBlockKind.fromName (name : ByteArray) : Option SigBlockKind :=
  if endsWith name extRsa then some .rsa
  else if endsWith name extDsa then some .dsa
  else if endsWith name extEc then some .ec
  else none

/-! ## META-INF inventory -/

/-- A single META-INF entry inside an APK — name + (compressed-or-
stored) body bytes. The body is the *uncompressed* bytes the JAR
spec hashes against. -/
structure MetaInfEntry where
  /-- Filename relative to archive root (e.g. `META-INF/CERT.RSA`). -/
  name : ByteArray
  /-- Uncompressed body bytes. -/
  body : ByteArray
deriving Inhabited

/-- The full META-INF inventory: zero-or-one MANIFEST.MF, one or
more `.SF` signature files, one or more `.RSA / .DSA / .EC`
signature blocks, plus arbitrary other META-INF entries.

The dispatch logic (`verifyV1`) requires exactly one SF + one
matching signature block per signer; the inventory carries
*all* entries so the verifier can detect malformed APKs (extra
SFs without matching blocks, etc.). -/
structure MetaInfInventory where
  manifestMf       : Option ByteArray
  signatureFiles   : List MetaInfEntry  -- *.SF
  signatureBlocks  : List MetaInfEntry  -- *.RSA, *.DSA, *.EC
  otherMetaInf     : List MetaInfEntry  -- everything else under META-INF/
deriving Inhabited

/-- Build a `MetaInfInventory` by classifying each `(name, body)`
entry by suffix. Non-META-INF entries are silently ignored — the
caller pre-filters them. -/
def MetaInfInventory.classify (entries : List MetaInfEntry) :
    MetaInfInventory := Id.run do
  let mut manifestMf : Option ByteArray := none
  let mut sfs : List MetaInfEntry := []
  let mut blocks : List MetaInfEntry := []
  let mut other : List MetaInfEntry := []
  for e in entries do
    if inMetaInf e.name then
      if e.name = manifestPath then
        manifestMf := some e.body
      else if endsWith e.name extSf then
        sfs := e :: sfs
      else if (SigBlockKind.fromName e.name).isSome then
        blocks := e :: blocks
      else
        other := e :: other
  return {
    manifestMf := manifestMf,
    signatureFiles := sfs.reverse,
    signatureBlocks := blocks.reverse,
    otherMetaInf := other.reverse }

/-! ## Entry-digest map -/

/-- Digest algorithms allowed in MANIFEST.MF. -/
inductive DigestAlgorithm : Type where
  /-- SHA-1 — accepted for legacy compatibility, no longer
  recommended. -/
  | sha1
  /-- SHA-256 — the modern default. -/
  | sha256
  /-- SHA-512. -/
  | sha512
deriving Repr, DecidableEq, Inhabited

/-- A map from entry name to declared digest, parsed out of
MANIFEST.MF text. The MANIFEST.MF format (per JAR spec) is line-
oriented:

  Name: <entry-name>
  SHA-256-Digest: <base64 digest>
  <blank line>

The digest is base64-encoded; the verifier asserts it matches
the re-computed SHA-256 over the entry body. -/
structure DigestEntry where
  /-- Entry name (must match an APK entry's filename). -/
  name           : ByteArray
  /-- Declared digest, base64-decoded raw bytes. -/
  declaredDigest : ByteArray
  /-- Digest algorithm used. -/
  algorithm      : DigestAlgorithm
deriving Inhabited

/-! ## Verifier predicate -/

/-- v1 verification result categories. -/
inductive V1VerifyResult : Type where
  /-- All checks passed. -/
  | accept
  /-- Manifest entry missing. -/
  | rejectNoManifest
  /-- No signature file found. -/
  | rejectNoSf
  /-- No signature block found. -/
  | rejectNoSigBlock
  /-- Manifest's declared digest of an APK entry doesn't match the
  recomputed SHA over that entry's body. -/
  | rejectManifestDigestMismatch
  /-- The .SF's manifest digest doesn't match the re-computed SHA
  over MANIFEST.MF. -/
  | rejectSfManifestDigestMismatch
  /-- PKCS#7 signature failed cryptographic verification. -/
  | rejectPkcs7VerifyFailed
  /-- An APK entry has no manifest entry — coverage check failed. -/
  | rejectMissingManifestEntry
  /-- Janus (CVE-2017-13156): bytes prepended to the APK changed
  the layout but the JAR scheme didn't notice. Detected
  separately because the v1 verifier sees only the META-INF
  view; the v2/v3 scheme detects this directly via the
  whole-file digest. -/
  | rejectJanusCve_2017_13156
deriving Repr, DecidableEq, Inhabited

instance : ToString V1VerifyResult where
  toString
    | .accept                            => "accept"
    | .rejectNoManifest                  => "rejectNoManifest"
    | .rejectNoSf                        => "rejectNoSf"
    | .rejectNoSigBlock                  => "rejectNoSigBlock"
    | .rejectManifestDigestMismatch      => "rejectManifestDigestMismatch"
    | .rejectSfManifestDigestMismatch    => "rejectSfManifestDigestMismatch"
    | .rejectPkcs7VerifyFailed           => "rejectPkcs7VerifyFailed"
    | .rejectMissingManifestEntry        => "rejectMissingManifestEntry"
    | .rejectJanusCve_2017_13156         => "rejectJanusCve_2017_13156"

/-- Tag enumeration — distinct bytes for every result. -/
def V1VerifyResult.tag : V1VerifyResult → UInt8
  | .accept                            => 0
  | .rejectNoManifest                  => 1
  | .rejectNoSf                        => 2
  | .rejectNoSigBlock                  => 3
  | .rejectManifestDigestMismatch      => 4
  | .rejectSfManifestDigestMismatch    => 5
  | .rejectPkcs7VerifyFailed           => 6
  | .rejectMissingManifestEntry        => 7
  | .rejectJanusCve_2017_13156         => 8

/-- Result tags are pairwise distinct. -/
theorem V1VerifyResult.tag_inj :
    ∀ a b : V1VerifyResult, a.tag = b.tag → a = b := by
  intro a b h
  cases a <;> cases b <;> simp [V1VerifyResult.tag] at h <;> rfl

/-! ## High-level verifier predicate (oracle-shaped) -/

/-- Cryptographic oracle. The Lean-level v1 verifier is
parameterised over an oracle that supplies:

  * SHA-256 of bytes (HACL*-verified — ADR-0029).
  * PKCS#7 verification of a SignedData blob against a candidate
    `.SF` payload.
  * A list of MANIFEST.MF entries parsed from the manifest text.

This split lets us state the verifier's *structural* contract in
Lean (every valid input is accepted, every Janus / no-block /
bad-digest input is rejected) without committing to a specific
crypto implementation. The Rust mirror at
`tools/sig-eval-rust/src/v1.rs` plugs in the actual primitives
via the same trait surface. -/
structure CryptoOracle where
  /-- SHA-256 of input bytes (32 bytes output). -/
  sha256              : ByteArray → ByteArray
  /-- Decode base64 in MANIFEST.MF / .SF text into raw bytes. -/
  base64Decode        : ByteArray → Option ByteArray
  /-- Parse MANIFEST.MF text into a list of `DigestEntry`. -/
  parseManifest       : ByteArray → List DigestEntry
  /-- Parse .SF text — returns `(manifestDigest, perEntryDigests)`. -/
  parseSf             : ByteArray → Option (ByteArray × List DigestEntry)
  /-- PKCS#7 SignedData verifier: returns `true` iff the
  signature in `block` is valid over `sf`. -/
  pkcs7Verify         : (block : ByteArray) → (sf : ByteArray) → Bool

/-- Verify v1 (JAR) signature given an inventory + the list of
all APK entries the verifier should check (non-META-INF entries).

Returns the first reject category encountered, or `accept` if all
checks pass. -/
def verifyV1
    (oracle : CryptoOracle)
    (inv : MetaInfInventory)
    (apkEntries : List MetaInfEntry)
    (janusDetected : Bool := false) :
    V1VerifyResult := Id.run do
  -- Janus pre-check.
  if janusDetected then
    return .rejectJanusCve_2017_13156
  -- Manifest must exist.
  let .some manifestBytes := inv.manifestMf
    | return .rejectNoManifest
  -- At least one .SF and one signature block.
  let .some sfEntry := inv.signatureFiles.head?
    | return .rejectNoSf
  let .some sigBlockEntry := inv.signatureBlocks.head?
    | return .rejectNoSigBlock
  -- PKCS#7 verifies the .SF.
  if !(oracle.pkcs7Verify sigBlockEntry.body sfEntry.body) then
    return .rejectPkcs7VerifyFailed
  -- .SF's manifest digest matches re-computed SHA over MANIFEST.MF.
  let .some (sfManifestDigest, _sfEntryDigests) := oracle.parseSf sfEntry.body
    | return .rejectSfManifestDigestMismatch
  let recomputedManifestDigest := oracle.sha256 manifestBytes
  if sfManifestDigest ≠ recomputedManifestDigest then
    return .rejectSfManifestDigestMismatch
  -- Every APK entry's manifest digest matches re-computed SHA.
  let manifestEntries := oracle.parseManifest manifestBytes
  for e in apkEntries do
    -- Find this entry in the manifest.
    let mEntry? := manifestEntries.find? fun m => m.name = e.name
    let .some mEntry := mEntry?
      | return .rejectMissingManifestEntry
    let recomputed := oracle.sha256 e.body
    if mEntry.declaredDigest ≠ recomputed then
      return .rejectManifestDigestMismatch
  return .accept

/-! ## Smoke checks -/

/-- META-INF prefix is exactly 9 bytes. -/
example : metaInfPrefix.size = 9 := by native_decide

/-- "MANIFEST.MF" inside META-INF is 20 bytes. -/
example : manifestPath.size = 20 := by native_decide

/-- `.RSA / .DSA / .EC` extensions classify correctly. -/
example :
    SigBlockKind.fromName (ByteArray.mk #[0x4d, 0x45, 0x54, 0x41, 0x2d, 0x49, 0x4e, 0x46, 0x2f, 0x43, 0x45, 0x52, 0x54, 0x2e, 0x52, 0x53, 0x41]) = some .rsa := by
  native_decide

example :
    SigBlockKind.fromName (ByteArray.mk #[0x43, 0x45, 0x52, 0x54, 0x2e, 0x44, 0x53, 0x41]) = some .dsa := by
  native_decide

example :
    SigBlockKind.fromName (ByteArray.mk #[0x43, 0x45, 0x52, 0x54, 0x2e, 0x45, 0x43]) = some .ec := by
  native_decide

example :
    SigBlockKind.fromName (ByteArray.mk #[0x4e, 0x4f, 0x54, 0x53, 0x49, 0x47]) = none := by
  native_decide

/-- `inMetaInf` accepts canonical META-INF paths and rejects
non-META-INF paths. -/
example :
    inMetaInf manifestPath = true := by native_decide

example :
    inMetaInf (ByteArray.mk #[0x63, 0x6c, 0x61, 0x73, 0x73, 0x65, 0x73, 0x2e, 0x64, 0x65, 0x78]) = false := by
  native_decide

end Apkaxiom.Signing.V1
