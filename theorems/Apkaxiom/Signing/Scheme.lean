/-
P1.11 — APK Signature Scheme v2 / v3 / v3.1 internal structure.

Each scheme block is a length-prefixed sequence of *signers*.
A signer carries:

  signer (length-prefixed):
    signed_data (length-prefixed)
    [v3 / v3.1 only: min_sdk u32, max_sdk u32]
    signatures (length-prefixed sequence of length-prefixed
                signature elements)
    public_key (length-prefixed bytes — SubjectPublicKeyInfo DER)

`signed_data` further decomposes as:

  signed_data:
    digests (length-prefixed sequence)
    certificates (length-prefixed sequence of length-prefixed
                  X.509 DER certificates)
    [v3 / v3.1 only: min_sdk u32, max_sdk u32]
    additional_attributes (length-prefixed sequence of
                           length-prefixed (id u32 || bytes value))

Digest element: `algorithm_id u32 || length-prefixed digest`.
Signature element: `algorithm_id u32 || length-prefixed signature`.

The Rust mirror lives at `crates/axiom-sigblock/src/scheme.rs`.
The differential harness asserts this Lean parser and the Rust
parser produce byte-equivalent JSON output on every fixture in
`corpus/signing/`.
-/

import Std
import Apkaxiom.Zip.LocalHeader
import Apkaxiom.Signing.Block

namespace Apkaxiom.Signing.Scheme

open Apkaxiom.Zip.LocalHeader (readU16 readU32 slice)

/-! ## Signature-algorithm IDs -/

/-- Wire IDs assigned by AOSP `tools/apksig`. The `Unknown`
constructor preserves IDs that aren't on the known list, so the
parser is total and the differential harness can compare error
categories without losing information. -/
inductive SignatureAlgorithmId : Type where
  /-- `0x0101` — RSA-PSS+SHA-256, 1 MiB-chunked SHA-256 digest. -/
  | rsaPssSha256
  /-- `0x0102` — RSA-PSS+SHA-512, 1 MiB-chunked SHA-512 digest. -/
  | rsaPssSha512
  /-- `0x0103` — RSA-PKCS1-v1.5+SHA-256, chunked SHA-256. -/
  | rsaPkcs1Sha256
  /-- `0x0104` — RSA-PKCS1-v1.5+SHA-512, chunked SHA-512. -/
  | rsaPkcs1Sha512
  /-- `0x0201` — ECDSA+SHA-256, chunked SHA-256. -/
  | ecdsaSha256
  /-- `0x0202` — ECDSA+SHA-512, chunked SHA-512. -/
  | ecdsaSha512
  /-- `0x0301` — DSA+SHA-256, chunked SHA-256. -/
  | dsaSha256
  /-- `0x0421` — RSA-PKCS1+SHA-256 over Verity tree root. -/
  | verityRsaPkcs1Sha256
  /-- `0x0423` — ECDSA+SHA-256 over Verity tree root. -/
  | verityEcdsaSha256
  /-- `0x0425` — DSA+SHA-256 over Verity tree root. -/
  | verityDsaSha256
deriving Repr, DecidableEq, Inhabited

/-- Wire ID for an algorithm. -/
def SignatureAlgorithmId.toU32 : SignatureAlgorithmId → UInt32
  | .rsaPssSha256          => 0x0101
  | .rsaPssSha512          => 0x0102
  | .rsaPkcs1Sha256        => 0x0103
  | .rsaPkcs1Sha512        => 0x0104
  | .ecdsaSha256           => 0x0201
  | .ecdsaSha512           => 0x0202
  | .dsaSha256             => 0x0301
  | .verityRsaPkcs1Sha256  => 0x0421
  | .verityEcdsaSha256     => 0x0423
  | .verityDsaSha256       => 0x0425

/-- Lift a wire ID to a known algorithm. Returns `none` for IDs
the spec hasn't assigned. -/
def SignatureAlgorithmId.fromU32 (id : UInt32) : Option SignatureAlgorithmId :=
  if id = 0x0101 then some .rsaPssSha256
  else if id = 0x0102 then some .rsaPssSha512
  else if id = 0x0103 then some .rsaPkcs1Sha256
  else if id = 0x0104 then some .rsaPkcs1Sha512
  else if id = 0x0201 then some .ecdsaSha256
  else if id = 0x0202 then some .ecdsaSha512
  else if id = 0x0301 then some .dsaSha256
  else if id = 0x0421 then some .verityRsaPkcs1Sha256
  else if id = 0x0423 then some .verityEcdsaSha256
  else if id = 0x0425 then some .verityDsaSha256
  else none

/-- Digest kind used by the chunked-digest computation. -/
inductive DigestKind : Type where
  /-- 32-byte SHA-256 digest. -/
  | sha256
  /-- 64-byte SHA-512 digest. -/
  | sha512
deriving Repr, DecidableEq, Inhabited

/-- Output length of a digest kind, in bytes. -/
def DigestKind.len : DigestKind → Nat
  | .sha256 => 32
  | .sha512 => 64

/-- Underlying digest algorithm of a signature scheme. -/
def SignatureAlgorithmId.digestKind : SignatureAlgorithmId → DigestKind
  | .rsaPssSha256 | .rsaPkcs1Sha256 | .ecdsaSha256
  | .dsaSha256 | .verityRsaPkcs1Sha256 | .verityEcdsaSha256
  | .verityDsaSha256          => .sha256
  | .rsaPssSha512 | .rsaPkcs1Sha512 | .ecdsaSha512 => .sha512

/-- True iff the algorithm is one of the Verity tree-root variants. -/
def SignatureAlgorithmId.isVerity : SignatureAlgorithmId → Bool
  | .verityRsaPkcs1Sha256 | .verityEcdsaSha256 | .verityDsaSha256 => true
  | _ => false

/-! ## Scheme variant -/

/-- Variant tag for the v2 / v3 / v3.1 layout. v3 and v3.1 share
the same wire shape (only the carrier ID differs); the variant
controls whether `(min_sdk, max_sdk)` u32 pairs appear in the
signer envelope and `signed_data`. -/
inductive Variant : Type where
  /-- v2 (id `0x7109871a`). No SDK range fields. -/
  | v2
  /-- v3 (id `0xf05368c0`). Signer envelope and `signed_data`
  both carry `(min_sdk, max_sdk)`; the parser asserts they match. -/
  | v3
  /-- v3.1 (id `0x1b93ad61`). Same shape as v3. -/
  | v3_1
deriving Repr, DecidableEq, Inhabited

/-- True iff the variant carries an SDK range. -/
def Variant.hasSdkRange : Variant → Bool
  | .v2 => false
  | .v3 | .v3_1 => true

/-! ## Parsed structures -/

/-- One entry of the digests sequence. -/
structure DigestEntry where
  algorithmId : UInt32
  algorithm   : Option SignatureAlgorithmId
  digest      : ByteArray
deriving Inhabited

/-- One entry of the signatures sequence. -/
structure SignatureEntry where
  algorithmId : UInt32
  algorithm   : Option SignatureAlgorithmId
  signature   : ByteArray
deriving Inhabited

/-- One additional-attribute entry. -/
structure AttributeEntry where
  id    : UInt32
  value : ByteArray
deriving Inhabited

/-- One signer inside a v2/v3/v3.1 block. -/
structure Signer where
  /-- Verbatim `signed_data` bytes — the SHA over THESE bytes is
  what each signature algorithm signs. -/
  signedData            : ByteArray
  /-- Digests declared by this signer (one per algorithm). -/
  digests               : List DigestEntry
  /-- X.509 certificate chain — first cert is the leaf. -/
  certificates          : List ByteArray
  /-- Additional attributes (id u32, value bytes). -/
  additionalAttributes  : List AttributeEntry
  /-- Signatures declared by this signer (one per algorithm). -/
  signatures            : List SignatureEntry
  /-- Subject Public Key Info DER — leaf cert's public key. -/
  publicKey             : ByteArray
  /-- `(min_sdk, max_sdk)` — present only for v3 / v3.1 signers. -/
  sdkRange              : Option (UInt32 × UInt32)
deriving Inhabited

/-! ## Parse errors -/

inductive SchemeError : Type where
  /-- A length-prefix declared more bytes than remain in the
  containing slice. -/
  | lengthOverflow
  /-- A length-prefix is missing (slice too short). -/
  | truncated
  /-- The signers sequence is empty. -/
  | noSigners
  /-- `(min_sdk, max_sdk)` mismatch between signer envelope and
  `signed_data` envelope (v3 / v3.1). -/
  | v3SdkRangeMismatch
deriving Repr, DecidableEq, Inhabited

instance : ToString SchemeError where
  toString
    | .lengthOverflow      => "lengthOverflow"
    | .truncated           => "truncated"
    | .noSigners           => "noSigners"
    | .v3SdkRangeMismatch  => "v3SdkRangeMismatch"

/-- Tag enumeration for cross-language interop. -/
def SchemeError.tag : SchemeError → UInt8
  | .lengthOverflow      => 1
  | .truncated           => 2
  | .noSigners           => 3
  | .v3SdkRangeMismatch  => 4

/-- The four error tags are pairwise distinct. -/
theorem SchemeError.tag_inj :
    ∀ a b : SchemeError, a.tag = b.tag → a = b := by
  intro a b h
  cases a <;> cases b <;> simp [SchemeError.tag] at h <;> rfl

/-! ## Length-prefixed slice helper -/

/-- Read a length-prefixed slice at offset `off`. Returns
`(slice, nextOff)`. -/
def takeLpSlice (buf : ByteArray) (off : Nat) :
    Except SchemeError (ByteArray × Nat) := Id.run do
  if off + 4 > buf.size then
    return .error .truncated
  let .some n := readU32 buf off
    | return .error .truncated
  let nN : Nat := n.toNat
  let start := off + 4
  let endo := start + nN
  if endo > buf.size then
    return .error .lengthOverflow
  let .some s := slice buf start nN
    | return .error .lengthOverflow
  return .ok (s, endo)

/-! ## Sequence walkers -/

/-- Walk a digest sequence — list of length-prefixed
`(algorithm_id u32 || length-prefixed digest)`.

Total: each iteration's `next` is the offset *after* a fully-
read length-prefixed slice (≥ 4 bytes including the length
prefix), so `seq.size - next < seq.size - cur` strictly. -/
def parseDigestSeq
    (seq : ByteArray) (acc : List DigestEntry) (cur : Nat) :
    Except SchemeError (List DigestEntry) :=
  if cur = seq.size then
    .ok acc.reverse
  else if h : cur > seq.size then
    .error .lengthOverflow
  else match takeLpSlice seq cur with
    | .error e => .error e
    | .ok (elt, next) =>
      if h2 : next ≤ cur then
        .error .lengthOverflow
      else if elt.size < 4 then
        .error .truncated
      else match readU32 elt 0 with
        | none => .error .truncated
        | some alg =>
          match takeLpSlice elt 4 with
          | .error e => .error e
          | .ok (digestSlice, _) =>
            let entry : DigestEntry :=
              { algorithmId := alg
              , algorithm := SignatureAlgorithmId.fromU32 alg
              , digest := digestSlice }
            parseDigestSeq seq (entry :: acc) next
termination_by seq.size - cur
decreasing_by
  simp_wf
  omega

/-- Walk a signature sequence. Total — same termination argument
as `parseDigestSeq`. -/
def parseSignatureSeq
    (seq : ByteArray) (acc : List SignatureEntry) (cur : Nat) :
    Except SchemeError (List SignatureEntry) :=
  if cur = seq.size then
    .ok acc.reverse
  else if cur > seq.size then
    .error .lengthOverflow
  else match takeLpSlice seq cur with
    | .error e => .error e
    | .ok (elt, next) =>
      if next ≤ cur then
        .error .lengthOverflow
      else if elt.size < 4 then
        .error .truncated
      else match readU32 elt 0 with
        | none => .error .truncated
        | some alg =>
          match takeLpSlice elt 4 with
          | .error e => .error e
          | .ok (sigSlice, _) =>
            let entry : SignatureEntry :=
              { algorithmId := alg
              , algorithm := SignatureAlgorithmId.fromU32 alg
              , signature := sigSlice }
            parseSignatureSeq seq (entry :: acc) next
termination_by seq.size - cur
decreasing_by
  simp_wf
  omega

/-- Walk an attribute sequence — list of length-prefixed
`(id u32 || bytes)`. Total. -/
def parseAttributeSeq
    (seq : ByteArray) (acc : List AttributeEntry) (cur : Nat) :
    Except SchemeError (List AttributeEntry) :=
  if cur = seq.size then
    .ok acc.reverse
  else if cur > seq.size then
    .error .lengthOverflow
  else match takeLpSlice seq cur with
    | .error e => .error e
    | .ok (elt, next) =>
      if next ≤ cur then
        .error .lengthOverflow
      else if elt.size < 4 then
        .error .truncated
      else match readU32 elt 0 with
        | none => .error .truncated
        | some id =>
          match slice elt 4 (elt.size - 4) with
          | none => .error .lengthOverflow
          | some valueSlice =>
            let entry : AttributeEntry := { id := id, value := valueSlice }
            parseAttributeSeq seq (entry :: acc) next
termination_by seq.size - cur
decreasing_by
  simp_wf
  omega

/-- Walk a sequence of length-prefixed length-prefixed elements
(used for the certificate sequence). Total. -/
def parseLpLpSeq
    (seq : ByteArray) (acc : List ByteArray) (cur : Nat) :
    Except SchemeError (List ByteArray) :=
  if cur = seq.size then
    .ok acc.reverse
  else if cur > seq.size then
    .error .lengthOverflow
  else match takeLpSlice seq cur with
    | .error e => .error e
    | .ok (elt, next) =>
      if next ≤ cur then
        .error .lengthOverflow
      else
        parseLpLpSeq seq (elt :: acc) next
termination_by seq.size - cur
decreasing_by
  simp_wf
  omega

/-! ## Per-signer parser -/

/-- Parse a single signer envelope. The variant controls whether
SDK-range fields appear. -/
def parseSigner (signer : ByteArray) (variant : Variant) :
    Except SchemeError Signer := Id.run do
  match takeLpSlice signer 0 with
  | .error e => return .error e
  | .ok (signedDataSlice, p0) =>
  let signedData := signedDataSlice
  let mut p : Nat := p0
  -- v3 / v3.1 carry signer-level (min_sdk, max_sdk).
  let mut sdkRangeSigner : Option (UInt32 × UInt32) := none
  if variant.hasSdkRange then
    if p + 8 > signer.size then
      return .error .truncated
    let .some smin := readU32 signer p
      | return .error .truncated
    let .some smax := readU32 signer (p + 4)
      | return .error .truncated
    p := p + 8
    sdkRangeSigner := some (smin, smax)
  -- signatures
  match takeLpSlice signer p with
  | .error e => return .error e
  | .ok (sigsSeq, p1) =>
  -- public_key
  match takeLpSlice signer p1 with
  | .error e => return .error e
  | .ok (publicKey, _) =>
  -- Walk signed_data.
  match takeLpSlice signedDataSlice 0 with
  | .error e => return .error e
  | .ok (digsSeq, dp) =>
  match takeLpSlice signedDataSlice dp with
  | .error e => return .error e
  | .ok (certsSeq, dp2) =>
  let mut dpAfterCerts : Nat := dp2
  let mut sdkRangeSigned : Option (UInt32 × UInt32) := none
  if variant.hasSdkRange then
    if dpAfterCerts + 8 > signedDataSlice.size then
      return .error .truncated
    let .some dmin := readU32 signedDataSlice dpAfterCerts
      | return .error .truncated
    let .some dmax := readU32 signedDataSlice (dpAfterCerts + 4)
      | return .error .truncated
    dpAfterCerts := dpAfterCerts + 8
    sdkRangeSigned := some (dmin, dmax)
  match takeLpSlice signedDataSlice dpAfterCerts with
  | .error e => return .error e
  | .ok (attrsSeq, _) =>
  -- v3/v3.1: signer- and signed-data SDK ranges must agree.
  if variant.hasSdkRange = true then
    match sdkRangeSigner, sdkRangeSigned with
    | some (smin, smax), some (dmin, dmax) =>
      if smin ≠ dmin ∨ smax ≠ dmax then
        return .error .v3SdkRangeMismatch
    | _, _ => return .error .truncated
  match parseDigestSeq digsSeq [] 0 with
  | .error e => return .error e
  | .ok digests =>
  match parseLpLpSeq certsSeq [] 0 with
  | .error e => return .error e
  | .ok certificates =>
  match parseAttributeSeq attrsSeq [] 0 with
  | .error e => return .error e
  | .ok additionalAttributes =>
  match parseSignatureSeq sigsSeq [] 0 with
  | .error e => return .error e
  | .ok signatures =>
  return .ok
    { signedData
    , digests
    , certificates
    , additionalAttributes
    , signatures
    , publicKey
    , sdkRange := sdkRangeSigner }

/-! ## Block-level parser -/

/-- Walk a v2 / v3 / v3.1 block — outer length-prefixed sequence
of signers. Total. -/
def parseSignersSeq
    (seq : ByteArray) (variant : Variant)
    (acc : List Signer) (cur : Nat) :
    Except SchemeError (List Signer) :=
  if cur = seq.size then
    .ok acc.reverse
  else if cur > seq.size then
    .error .lengthOverflow
  else match takeLpSlice seq cur with
    | .error e => .error e
    | .ok (signerBuf, next) =>
      if next ≤ cur then
        .error .lengthOverflow
      else match parseSigner signerBuf variant with
        | .error e => .error e
        | .ok s => parseSignersSeq seq variant (s :: acc) next
termination_by seq.size - cur
decreasing_by
  simp_wf
  omega

/-- Parse a v2 / v3 / v3.1 block. -/
def parseBlock (block : ByteArray) (variant : Variant) :
    Except SchemeError (List Signer) := Id.run do
  match takeLpSlice block 0 with
  | .error e => return .error e
  | .ok (signersSeq, _) =>
    match parseSignersSeq signersSeq variant [] 0 with
    | .error e => return .error e
    | .ok signers =>
      if signers.isEmpty then
        return .error .noSigners
      else
        return .ok signers

/-- Parse a v2 block. -/
def parseV2 (block : ByteArray) : Except SchemeError (List Signer) :=
  parseBlock block .v2

/-- Parse a v3 block. -/
def parseV3 (block : ByteArray) : Except SchemeError (List Signer) :=
  parseBlock block .v3

/-- Parse a v3.1 block. -/
def parseV3_1 (block : ByteArray) : Except SchemeError (List Signer) :=
  parseBlock block .v3_1

/-! ## Smoke tests -/

example : SignatureAlgorithmId.rsaPssSha256.toU32 = 0x0101 := by native_decide
example : SignatureAlgorithmId.rsaPkcs1Sha256.toU32 = 0x0103 := by native_decide
example : SignatureAlgorithmId.ecdsaSha256.toU32 = 0x0201 := by native_decide
example : SignatureAlgorithmId.dsaSha256.toU32 = 0x0301 := by native_decide
example : SignatureAlgorithmId.verityEcdsaSha256.toU32 = 0x0423 := by native_decide

/-- All 10 known algorithm IDs round-trip. -/
example :
    SignatureAlgorithmId.fromU32 0x0101 = some .rsaPssSha256
  ∧ SignatureAlgorithmId.fromU32 0x0102 = some .rsaPssSha512
  ∧ SignatureAlgorithmId.fromU32 0x0103 = some .rsaPkcs1Sha256
  ∧ SignatureAlgorithmId.fromU32 0x0104 = some .rsaPkcs1Sha512
  ∧ SignatureAlgorithmId.fromU32 0x0201 = some .ecdsaSha256
  ∧ SignatureAlgorithmId.fromU32 0x0202 = some .ecdsaSha512
  ∧ SignatureAlgorithmId.fromU32 0x0301 = some .dsaSha256
  ∧ SignatureAlgorithmId.fromU32 0x0421 = some .verityRsaPkcs1Sha256
  ∧ SignatureAlgorithmId.fromU32 0x0423 = some .verityEcdsaSha256
  ∧ SignatureAlgorithmId.fromU32 0x0425 = some .verityDsaSha256 := by
  native_decide

/-- Unknown IDs surface as `none`. -/
example : SignatureAlgorithmId.fromU32 0xdeadbeef = none := by
  native_decide

/-- Digest kinds report the spec'd byte lengths. -/
example : DigestKind.sha256.len = 32 := by native_decide
example : DigestKind.sha512.len = 64 := by native_decide

/-- v2 has no SDK range; v3 / v3.1 do. -/
example : Variant.v2.hasSdkRange = false := by native_decide
example : Variant.v3.hasSdkRange = true := by native_decide
example : Variant.v3_1.hasSdkRange = true := by native_decide

/-- Verity-marker classification matches the spec. -/
example : SignatureAlgorithmId.verityEcdsaSha256.isVerity = true := by
  native_decide
example : SignatureAlgorithmId.rsaPkcs1Sha256.isVerity = false := by
  native_decide

/-- Empty block → `truncated` error (the outer length-prefix
read fails before signersSeq is even sliced). -/
example : (parseV2 ByteArray.empty).toOption = none := by native_decide

end Apkaxiom.Signing.Scheme
