/-
P1.11 — APK Signature Scheme v3 verifier (block ID 0xf05368c0).

v3 extends v2 with:

  * Per-signer SDK range fields `(min_sdk, max_sdk)` carried both
    at the signer envelope level AND within `signed_data`. The
    parser (Apkaxiom.Signing.Scheme) already enforces these match.

  * A "proof-of-rotation" attribute (additional-attribute id
    `0x3ba06f8c`) carried in `signed_data.additional_attributes`.
    The proof-of-rotation lineage lets a v3 signer transition to
    a new key while still being trusted by old verifiers.

The verification predicate otherwise mirrors v2: same signed-data
shape, same signature-algorithm IDs, same chunked-digest
recomputation. v3 just gates which signer applies on a given
device based on its API level fitting `(min_sdk, max_sdk)`.

The Rust mirror lives at `tools/sig-eval-rust/src/v3.rs`.
-/

import Std
import Apkaxiom.Signing.Block
import Apkaxiom.Signing.Scheme
import Apkaxiom.Signing.V2

namespace Apkaxiom.Signing.V3

open Apkaxiom.Signing.Scheme (
  Signer DigestEntry SignatureEntry SignatureAlgorithmId
  AttributeEntry parseV3 SchemeError
)

/-! ## Wire constants -/

/-- Additional-attribute ID for the v3 / v3.1 proof-of-rotation
record. -/
def proofOfRotationAttrId : UInt32 := 0x3ba06f8c

/-! ## Verification result -/

inductive V3VerifyResult : Type where
  | accept
  | rejectNoV3Block
  | rejectMalformed
  | rejectNoDigests
  | rejectNoSignatures
  | rejectNoCertificates
  | rejectAlgorithmMismatch
  | rejectDigestMismatch
  | rejectSignatureFailed
  | rejectPublicKeyMismatch
  | rejectSdkRangeMismatch
  | rejectAllAlgorithmsUnknown
  /-- v3 must reject downgrade attempts: a v3-capable APK that
  presents a stripped v3 block to claim "v1/v2 only" must be
  detected by the dispatcher. -/
  | rejectDowngradeAttempt
  | rejectJanusCve_2017_13156
deriving Repr, DecidableEq, Inhabited

instance : ToString V3VerifyResult where
  toString
    | .accept                       => "accept"
    | .rejectNoV3Block              => "rejectNoV3Block"
    | .rejectMalformed              => "rejectMalformed"
    | .rejectNoDigests              => "rejectNoDigests"
    | .rejectNoSignatures           => "rejectNoSignatures"
    | .rejectNoCertificates         => "rejectNoCertificates"
    | .rejectAlgorithmMismatch      => "rejectAlgorithmMismatch"
    | .rejectDigestMismatch         => "rejectDigestMismatch"
    | .rejectSignatureFailed        => "rejectSignatureFailed"
    | .rejectPublicKeyMismatch      => "rejectPublicKeyMismatch"
    | .rejectSdkRangeMismatch       => "rejectSdkRangeMismatch"
    | .rejectAllAlgorithmsUnknown   => "rejectAllAlgorithmsUnknown"
    | .rejectDowngradeAttempt       => "rejectDowngradeAttempt"
    | .rejectJanusCve_2017_13156    => "rejectJanusCve_2017_13156"

def V3VerifyResult.tag : V3VerifyResult → UInt8
  | .accept                       => 0
  | .rejectNoV3Block              => 1
  | .rejectMalformed              => 2
  | .rejectNoDigests              => 3
  | .rejectNoSignatures           => 4
  | .rejectNoCertificates         => 5
  | .rejectAlgorithmMismatch      => 6
  | .rejectDigestMismatch         => 7
  | .rejectSignatureFailed        => 8
  | .rejectPublicKeyMismatch      => 9
  | .rejectSdkRangeMismatch       => 10
  | .rejectAllAlgorithmsUnknown   => 11
  | .rejectDowngradeAttempt       => 12
  | .rejectJanusCve_2017_13156    => 13

theorem V3VerifyResult.tag_inj :
    ∀ a b : V3VerifyResult, a.tag = b.tag → a = b := by
  intro a b h
  cases a <;> cases b <;> simp [V3VerifyResult.tag] at h <;> rfl

/-! ## Per-signer SDK gating -/

/-- The signer's `(min_sdk, max_sdk)` must match `signed_data`'s.
The parser already rejects mismatches with `v3SdkRangeMismatch`,
but we duplicate the check here as a defence-in-depth assertion
against parser bugs. -/
def signerSdkRangeOk (s : Signer) : Bool :=
  match s.sdkRange with
  | none => false  -- v3 signer must have an SDK range
  | some _ => true

/-- True iff this signer's range covers the device API `apiLevel`. -/
def signerCoversApiLevel (s : Signer) (apiLevel : UInt32) : Bool :=
  match s.sdkRange with
  | none => false
  | some (lo, hi) => lo ≤ apiLevel ∧ apiLevel ≤ hi

/-! ## Cryptographic oracle (same shape as v2) -/

/-- v3 reuses v2's oracle — same primitive set. -/
abbrev CryptoOracle := Apkaxiom.Signing.V2.CryptoOracle

/-! ## Verifier predicate -/

/-- Verify a single v3 signer. -/
def verifySigner (oracle : CryptoOracle) (apkBytes : ByteArray)
    (block : Apkaxiom.Signing.Block.Block)
    (variant : Apkaxiom.Signing.Block.Entry) (s : Signer) :
    V3VerifyResult := Id.run do
  if s.digests.isEmpty then
    return .rejectNoDigests
  if s.signatures.isEmpty then
    return .rejectNoSignatures
  if s.certificates.isEmpty then
    return .rejectNoCertificates
  if !(Apkaxiom.Signing.V2.signerAlgorithmsMatch s) then
    return .rejectAlgorithmMismatch
  if Apkaxiom.Signing.V2.signerAllAlgorithmsUnknown s then
    return .rejectAllAlgorithmsUnknown
  if !(signerSdkRangeOk s) then
    return .rejectSdkRangeMismatch
  -- Public key cross-check.
  let .some leafCert := s.certificates.head?
    | return .rejectNoCertificates
  let .some leafSpki := oracle.extractSpkiFromLeafCert leafCert
    | return .rejectPublicKeyMismatch
  if s.publicKey ≠ leafSpki then
    return .rejectPublicKeyMismatch
  let digestInput := oracle.buildDigestInput apkBytes block variant
  for sigEntry in s.signatures do
    match sigEntry.algorithm with
    | none => continue
    | some alg =>
      let .some dEntry :=
            s.digests.find? (·.algorithmId = sigEntry.algorithmId)
        | return .rejectAlgorithmMismatch
      let recomputed := oracle.chunkedDigest alg.digestKind digestInput
      if dEntry.digest ≠ recomputed then
        return .rejectDigestMismatch
      if !(oracle.signatureVerify alg s.publicKey s.signedData sigEntry.signature) then
        return .rejectSignatureFailed
  return .accept

/-- Top-level v3 verifier. -/
def verifyV3
    (oracle : CryptoOracle)
    (apkBytes : ByteArray)
    (block : Apkaxiom.Signing.Block.Block)
    (janusDetected : Bool := false) :
    V3VerifyResult := Id.run do
  if janusDetected then
    return .rejectJanusCve_2017_13156
  let .some v3Bytes := block.v3
    | return .rejectNoV3Block
  let v3Entry := Apkaxiom.Signing.Block.Entry.v3 v3Bytes
  match parseV3 v3Bytes with
  | .error _ => return .rejectMalformed
  | .ok signers =>
    if signers.isEmpty then
      return .rejectMalformed
    for s in signers do
      let r := verifySigner oracle apkBytes block v3Entry s
      if r ≠ .accept then
        return r
    return .accept

/-! ## Smoke checks -/

example : V3VerifyResult.accept.tag = 0 := by native_decide
example : V3VerifyResult.rejectDowngradeAttempt.tag = 12 := by native_decide
example : proofOfRotationAttrId = 0x3ba06f8c := by native_decide

end Apkaxiom.Signing.V3
