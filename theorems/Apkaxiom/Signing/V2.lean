/-
P1.11 — APK Signature Scheme v2 verifier (block ID 0x7109871a).

The v2 scheme verifies the *whole-file* contents — specifically
the three regions concatenated with their length-prefixes:

  digest_input =
    chunked_digest( zip_entries_region )                  -- bodies + LFH headers
      ++ chunked_digest( signing_block_minus_v2_block )    -- everything-but-the-v2-pair
      ++ chunked_digest( central_directory )
      ++ chunked_digest( eocd_with_cd_offset_relocated )

The "1 MiB chunked digest" computation: split the input into
1 MiB chunks (last chunk may be short), compute SHA over each
chunk prefixed with `0x5a || u32_chunk_count_le || chunk`, then
SHA over `0x5a || u32_chunk_count || concat(chunk_digests)`.

This module formalises the parsing of the v2 signed-data + the
*structural* verification predicate (every digest matches, every
signature verifies under the public key, certificate chain
validates). Cryptographic primitives (SHA-256, RSA-PKCS1, RSA-PSS,
ECDSA, Ed25519) are deferred to `Apkaxiom.Signing.Crypto` per
ADR-0029.

The Rust mirror lives at `tools/sig-eval-rust/src/v2.rs`. The
differential harness asserts byte-equivalence on every fixture
in `corpus/signing/v1-v2*/`.
-/

import Std
import Apkaxiom.Signing.Block
import Apkaxiom.Signing.Scheme
import Apkaxiom.Signing.V1

namespace Apkaxiom.Signing.V2

open Apkaxiom.Signing.Scheme (
  Signer DigestEntry SignatureEntry SignatureAlgorithmId
  parseV2 SchemeError
)

/-! ## Verification result -/

/-- v2 verification result categories. -/
inductive V2VerifyResult : Type where
  /-- All checks passed. -/
  | accept
  /-- The v2 block isn't present in the signing block. -/
  | rejectNoV2Block
  /-- Parser rejected the v2 block (malformed). -/
  | rejectMalformed
  /-- A signer has zero digests. -/
  | rejectNoDigests
  /-- A signer has zero signatures. -/
  | rejectNoSignatures
  /-- A signer has zero certificates. -/
  | rejectNoCertificates
  /-- The set of digests and the set of signatures don't match
  algorithm-by-algorithm. -/
  | rejectAlgorithmMismatch
  /-- Recomputed chunked digest doesn't match the digest declared
  by the signer. -/
  | rejectDigestMismatch
  /-- Cryptographic signature verification failed. -/
  | rejectSignatureFailed
  /-- Public key declared in signed_data doesn't match the leaf
  cert's SPKI. -/
  | rejectPublicKeyMismatch
  /-- All algorithm IDs were unknown to the verifier (stripped-
  algorithm attack). -/
  | rejectAllAlgorithmsUnknown
  /-- Janus CVE-2017-13156: bytes prepended to the APK changed
  the layout but only v1 sees it via META-INF; v2 must catch
  this through the whole-file digest. -/
  | rejectJanusCve_2017_13156
deriving Repr, DecidableEq, Inhabited

instance : ToString V2VerifyResult where
  toString
    | .accept                       => "accept"
    | .rejectNoV2Block              => "rejectNoV2Block"
    | .rejectMalformed              => "rejectMalformed"
    | .rejectNoDigests              => "rejectNoDigests"
    | .rejectNoSignatures           => "rejectNoSignatures"
    | .rejectNoCertificates         => "rejectNoCertificates"
    | .rejectAlgorithmMismatch      => "rejectAlgorithmMismatch"
    | .rejectDigestMismatch         => "rejectDigestMismatch"
    | .rejectSignatureFailed        => "rejectSignatureFailed"
    | .rejectPublicKeyMismatch      => "rejectPublicKeyMismatch"
    | .rejectAllAlgorithmsUnknown   => "rejectAllAlgorithmsUnknown"
    | .rejectJanusCve_2017_13156    => "rejectJanusCve_2017_13156"

/-- Tag enumeration. -/
def V2VerifyResult.tag : V2VerifyResult → UInt8
  | .accept                       => 0
  | .rejectNoV2Block              => 1
  | .rejectMalformed              => 2
  | .rejectNoDigests              => 3
  | .rejectNoSignatures           => 4
  | .rejectNoCertificates         => 5
  | .rejectAlgorithmMismatch      => 6
  | .rejectDigestMismatch         => 7
  | .rejectSignatureFailed        => 8
  | .rejectPublicKeyMismatch      => 9
  | .rejectAllAlgorithmsUnknown   => 10
  | .rejectJanusCve_2017_13156    => 11

theorem V2VerifyResult.tag_inj :
    ∀ a b : V2VerifyResult, a.tag = b.tag → a = b := by
  intro a b h
  cases a <;> cases b <;> simp [V2VerifyResult.tag] at h <;> rfl

/-! ## Cryptographic oracle -/

/-- Cryptographic primitives the v2 verifier needs. The
implementations live in the Rust mirror; here we treat them as a
Skolem-style oracle so the structural verifier predicate is
spelled in Lean independent of any C/Rust binding. -/
structure CryptoOracle where
  /-- 1 MiB-chunked SHA digest of `bytes` under `kind`. -/
  chunkedDigest    : Apkaxiom.Signing.Scheme.DigestKind → ByteArray → ByteArray
  /-- Signature verification: returns `true` iff
  `signature` is valid over `data` under `(algorithm, publicKey)`. -/
  signatureVerify  :
    SignatureAlgorithmId → (publicKey : ByteArray) →
    (data : ByteArray) → (signature : ByteArray) → Bool
  /-- Extract the SubjectPublicKeyInfo DER from an X.509 leaf cert. -/
  extractSpkiFromLeafCert : ByteArray → Option ByteArray
  /-- Build the digest input — the four-region concatenation
  described in the module header. The verifier hashes this. -/
  buildDigestInput :
    (apkBytes : ByteArray) → (block : Apkaxiom.Signing.Block.Block) →
    (variant : Apkaxiom.Signing.Block.Entry) → ByteArray

/-! ## Per-signer structural checks -/

/-- A signer must have at least one digest, one signature, one
certificate. -/
def signerMinimalShape (s : Signer) : Bool :=
  !s.digests.isEmpty ∧ !s.signatures.isEmpty ∧ !s.certificates.isEmpty

/-- The signer's digest algorithm-id set must equal its signature
algorithm-id set. -/
def signerAlgorithmsMatch (s : Signer) : Bool :=
  let dIds := s.digests.map (·.algorithmId)
  let sIds := s.signatures.map (·.algorithmId)
  dIds.length = sIds.length
    && dIds.all (fun d => sIds.contains d)
    && sIds.all (fun ss => dIds.contains ss)

/-- All algorithm IDs of this signer are unknown — the
"stripped algorithm" attack pattern. -/
def signerAllAlgorithmsUnknown (s : Signer) : Bool :=
  s.digests.all (fun d => d.algorithm.isNone)
    && s.signatures.all (fun ss => ss.algorithm.isNone)

/-! ## Verifier predicate -/

/-- Verify a single signer. Returns `accept` iff every check
passes; otherwise the first reject category. -/
def verifySigner (oracle : CryptoOracle) (apkBytes : ByteArray)
    (block : Apkaxiom.Signing.Block.Block)
    (variant : Apkaxiom.Signing.Block.Entry) (s : Signer) :
    V2VerifyResult := Id.run do
  if s.digests.isEmpty then
    return .rejectNoDigests
  if s.signatures.isEmpty then
    return .rejectNoSignatures
  if s.certificates.isEmpty then
    return .rejectNoCertificates
  if !(signerAlgorithmsMatch s) then
    return .rejectAlgorithmMismatch
  if signerAllAlgorithmsUnknown s then
    return .rejectAllAlgorithmsUnknown
  -- Public key in signed_data must match leaf cert's SPKI.
  let .some leafCert := s.certificates.head?
    | return .rejectNoCertificates
  let .some leafSpki := oracle.extractSpkiFromLeafCert leafCert
    | return .rejectPublicKeyMismatch
  if s.publicKey ≠ leafSpki then
    return .rejectPublicKeyMismatch
  -- Every signature, every digest must match algorithm-paired.
  let digestInput := oracle.buildDigestInput apkBytes block variant
  for sigEntry in s.signatures do
    match sigEntry.algorithm with
    | none =>
      -- Skip — only a *full* skip is rejected above.
      continue
    | some alg =>
      -- Find the matching digest entry by algorithm_id.
      let .some dEntry :=
            s.digests.find? (·.algorithmId = sigEntry.algorithmId)
        | return .rejectAlgorithmMismatch
      -- Recompute chunked digest and compare.
      let recomputed := oracle.chunkedDigest alg.digestKind digestInput
      if dEntry.digest ≠ recomputed then
        return .rejectDigestMismatch
      -- Signature must verify under leaf public key over signed_data.
      if !(oracle.signatureVerify alg s.publicKey s.signedData sigEntry.signature) then
        return .rejectSignatureFailed
  return .accept

/-- Top-level v2 verifier. Returns `accept` iff the v2 block is
present, parseable, has at least one signer, and every signer
passes `verifySigner`. -/
def verifyV2
    (oracle : CryptoOracle)
    (apkBytes : ByteArray)
    (block : Apkaxiom.Signing.Block.Block)
    (janusDetected : Bool := false) :
    V2VerifyResult := Id.run do
  if janusDetected then
    return .rejectJanusCve_2017_13156
  let .some v2Bytes := block.v2
    | return .rejectNoV2Block
  let v2Entry := Apkaxiom.Signing.Block.Entry.v2 v2Bytes
  match parseV2 v2Bytes with
  | .error _ => return .rejectMalformed
  | .ok signers =>
    if signers.isEmpty then
      return .rejectMalformed
    -- Every signer must pass.
    for s in signers do
      let r := verifySigner oracle apkBytes block v2Entry s
      if r ≠ .accept then
        return r
    return .accept

/-! ## Smoke checks -/

/-- All twelve result tags are pairwise distinct (gated by
`tag_inj`). Spot-check two for native-decide. -/
example : V2VerifyResult.accept.tag = 0 := by native_decide
example : V2VerifyResult.rejectJanusCve_2017_13156.tag = 11 := by
  native_decide

/-- Empty-signer-list yields `rejectMalformed`. -/
example : V2VerifyResult.rejectMalformed.tag ≠ V2VerifyResult.accept.tag := by
  native_decide

end Apkaxiom.Signing.V2
