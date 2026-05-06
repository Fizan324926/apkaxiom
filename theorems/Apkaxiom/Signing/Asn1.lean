/-
P1.11 G17 — minimal ASN.1 / DER parser in Lean.

Parses the DER subset needed for X.509 certificates, PKCS#7
SignedData, and SubjectPublicKeyInfo extraction. The full DER
spec covers BER + indefinite-length forms; we limit to DER
(canonical encoding), which is what every APK signature carrier
uses.

Total. Termination measure: the input slice's `size - cur` strictly
decreases on every recursive descent.

Tag classes (per X.690):
  * Universal (0)    — built-in types (SEQUENCE, OID, ...).
  * Application (1)  — application-specific types.
  * ContextSpecific(2) — used inside structured types like
                         PKCS#7's `[0] EXPLICIT SignedData`.
  * Private (3)      — private use.

Tag form:
  * Primitive   (0) — value is bytes.
  * Constructed (1) — value is more TLVs.

Length encoding:
  * Short form (one byte): high bit clear, bits 0..6 = length 0..127.
  * Long form (multi-byte): high bit set, bits 0..6 = length-of-length,
    next bytes are the big-endian length.
-/

import Std

namespace Apkaxiom.Signing.Asn1

/-! ## Tag classes -/

inductive TagClass : Type where
  | universal
  | application
  | contextSpecific
  | private_
deriving Repr, DecidableEq, Inhabited

def TagClass.toU8 : TagClass → UInt8
  | .universal       => 0
  | .application     => 1
  | .contextSpecific => 2
  | .private_        => 3

def TagClass.fromU8 (b : UInt8) : Option TagClass :=
  match b with
  | 0 => some .universal
  | 1 => some .application
  | 2 => some .contextSpecific
  | 3 => some .private_
  | _ => none

example : TagClass.universal.toU8 = 0 := by native_decide
example : TagClass.application.toU8 = 1 := by native_decide
example : TagClass.contextSpecific.toU8 = 2 := by native_decide
example : TagClass.private_.toU8 = 3 := by native_decide

theorem tagclass_roundtrip_universal :
    TagClass.fromU8 TagClass.universal.toU8 = some .universal := by native_decide
theorem tagclass_roundtrip_application :
    TagClass.fromU8 TagClass.application.toU8 = some .application := by native_decide
theorem tagclass_roundtrip_contextSpecific :
    TagClass.fromU8 TagClass.contextSpecific.toU8 = some .contextSpecific := by native_decide
theorem tagclass_roundtrip_private :
    TagClass.fromU8 TagClass.private_.toU8 = some .private_ := by native_decide

theorem tagclass_unknown_byte_4 : TagClass.fromU8 4 = none := by native_decide
theorem tagclass_unknown_byte_255 : TagClass.fromU8 255 = none := by native_decide

/-! ## Universal tags (the ones we care about for X.509 / PKCS#7) -/

inductive UniversalTag : Type where
  /-- BOOLEAN (0x01) — `true`/`false`. -/
  | boolean
  /-- INTEGER (0x02) — multi-byte big-endian. -/
  | integer
  /-- BIT STRING (0x03) — bytes + unused-bits count. -/
  | bitString
  /-- OCTET STRING (0x04) — raw bytes. -/
  | octetString
  /-- NULL (0x05). -/
  | null
  /-- OBJECT IDENTIFIER (0x06). -/
  | oid
  /-- UTF8String (0x0c). -/
  | utf8String
  /-- PrintableString (0x13). -/
  | printableString
  /-- IA5String (0x16). -/
  | ia5String
  /-- UTCTime (0x17). -/
  | utcTime
  /-- GeneralizedTime (0x18). -/
  | generalizedTime
  /-- SEQUENCE (0x30). -/
  | sequence
  /-- SET (0x31). -/
  | set
deriving Repr, DecidableEq, Inhabited

def UniversalTag.toU8 : UniversalTag → UInt8
  | .boolean         => 0x01
  | .integer         => 0x02
  | .bitString       => 0x03
  | .octetString     => 0x04
  | .null            => 0x05
  | .oid             => 0x06
  | .utf8String      => 0x0c
  | .printableString => 0x13
  | .ia5String       => 0x16
  | .utcTime         => 0x17
  | .generalizedTime => 0x18
  | .sequence        => 0x30
  | .set             => 0x31

def UniversalTag.fromU8 (b : UInt8) : Option UniversalTag :=
  if      b = 0x01 then some .boolean
  else if b = 0x02 then some .integer
  else if b = 0x03 then some .bitString
  else if b = 0x04 then some .octetString
  else if b = 0x05 then some .null
  else if b = 0x06 then some .oid
  else if b = 0x0c then some .utf8String
  else if b = 0x13 then some .printableString
  else if b = 0x16 then some .ia5String
  else if b = 0x17 then some .utcTime
  else if b = 0x18 then some .generalizedTime
  else if b = 0x30 then some .sequence
  else if b = 0x31 then some .set
  else none

theorem universal_tag_roundtrip_integer :
    UniversalTag.fromU8 UniversalTag.integer.toU8 = some .integer := by native_decide
theorem universal_tag_roundtrip_oid :
    UniversalTag.fromU8 UniversalTag.oid.toU8 = some .oid := by native_decide
theorem universal_tag_roundtrip_sequence :
    UniversalTag.fromU8 UniversalTag.sequence.toU8 = some .sequence := by native_decide
theorem universal_tag_roundtrip_set :
    UniversalTag.fromU8 UniversalTag.set.toU8 = some .set := by native_decide
theorem universal_tag_roundtrip_octet_string :
    UniversalTag.fromU8 UniversalTag.octetString.toU8 = some .octetString := by native_decide
theorem universal_tag_roundtrip_bit_string :
    UniversalTag.fromU8 UniversalTag.bitString.toU8 = some .bitString := by native_decide
theorem universal_tag_roundtrip_utf8 :
    UniversalTag.fromU8 UniversalTag.utf8String.toU8 = some .utf8String := by native_decide
theorem universal_tag_roundtrip_printable :
    UniversalTag.fromU8 UniversalTag.printableString.toU8 = some .printableString := by native_decide
theorem universal_tag_roundtrip_utc_time :
    UniversalTag.fromU8 UniversalTag.utcTime.toU8 = some .utcTime := by native_decide
theorem universal_tag_roundtrip_generalized_time :
    UniversalTag.fromU8 UniversalTag.generalizedTime.toU8 = some .generalizedTime := by native_decide
theorem universal_tag_roundtrip_boolean :
    UniversalTag.fromU8 UniversalTag.boolean.toU8 = some .boolean := by native_decide
theorem universal_tag_roundtrip_ia5 :
    UniversalTag.fromU8 UniversalTag.ia5String.toU8 = some .ia5String := by native_decide
theorem universal_tag_roundtrip_null :
    UniversalTag.fromU8 UniversalTag.null.toU8 = some .null := by native_decide

theorem universal_tag_unknown_0x40 : UniversalTag.fromU8 0x40 = none := by native_decide
theorem universal_tag_unknown_0xff : UniversalTag.fromU8 0xff = none := by native_decide

/-! ## DER tag (high-level) -/

structure DerTag where
  tagClass    : TagClass
  constructed : Bool
  number      : Nat
deriving Repr, DecidableEq, Inhabited

/-- Parse the identifier byte (or two-byte form for high-tag-number).
    For our X.509 / PKCS#7 subset, single-byte tags are sufficient. -/
def parseTagByte (b : UInt8) : DerTag :=
  let cls : TagClass :=
    match TagClass.fromU8 ((b >>> 6) &&& 0x03) with
    | some c => c
    | none => .universal
  let constructed : Bool := (b &&& 0x20) ≠ 0
  let number : Nat := (b &&& 0x1f).toNat
  { tagClass := cls, constructed := constructed, number := number }

example : parseTagByte 0x30 = { tagClass := .universal, constructed := true, number := 0x10 } := by native_decide
example : parseTagByte 0x06 = { tagClass := .universal, constructed := false, number := 0x06 } := by native_decide
example : parseTagByte 0xa0 = { tagClass := .contextSpecific, constructed := true, number := 0x00 } := by native_decide
example : parseTagByte 0x80 = { tagClass := .contextSpecific, constructed := false, number := 0x00 } := by native_decide
example : parseTagByte 0x31 = { tagClass := .universal, constructed := true, number := 0x11 } := by native_decide
example : parseTagByte 0x02 = { tagClass := .universal, constructed := false, number := 0x02 } := by native_decide
example : parseTagByte 0x04 = { tagClass := .universal, constructed := false, number := 0x04 } := by native_decide
example : parseTagByte 0xa1 = { tagClass := .contextSpecific, constructed := true, number := 0x01 } := by native_decide

/-! ## DER length parsing -/

inductive LengthError : Type where
  | shortInput
  | invalidIndefiniteLength
  | overflow
deriving Repr, DecidableEq, Inhabited

def LengthError.tag : LengthError → UInt8
  | .shortInput              => 1
  | .invalidIndefiniteLength => 2
  | .overflow                => 3

theorem length_error_tag_inj :
    ∀ a b : LengthError, a.tag = b.tag → a = b := by
  intro a b h
  cases a <;> cases b <;> simp [LengthError.tag] at h <;> rfl

/-- Big-endian assemble `n` bytes starting at `bs[off]` into a Nat. -/
def beAssemble (bs : ByteArray) (off : Nat) : Nat → Nat
  | 0 => 0
  | n + 1 => (beAssemble bs off n <<< 8) ||| (bs.get! (off + n)).toNat

/-- Parse a DER length at offset `off`. Returns `(length, bytesConsumed)`. -/
def parseDerLength (bs : ByteArray) (off : Nat) : Except LengthError (Nat × Nat) :=
  if off ≥ bs.size then
    .error .shortInput
  else
    let b0 := bs.get! off
    if b0 < 0x80 then
      .ok (b0.toNat, 1)
    else if b0 = 0x80 then
      .error .invalidIndefiniteLength
    else
      let n := (b0 &&& 0x7f).toNat
      if n > 8 then
        .error .overflow
      else if off + 1 + n > bs.size then
        .error .shortInput
      else
        .ok (beAssemble bs (off + 1) n, n + 1)

/-- Project parseDerLength to a `(Nat × Nat)?` for cheap testing. -/
def parseDerLengthOk (bs : ByteArray) (off : Nat) : Option (Nat × Nat) :=
  match parseDerLength bs off with
  | .ok p => some p
  | .error _ => none

/-- Project parseDerLength to a `LengthError?` for testing the
    error path. -/
def parseDerLengthErr (bs : ByteArray) (off : Nat) : Option LengthError :=
  match parseDerLength bs off with
  | .error e => some e
  | .ok _ => none

example : parseDerLengthOk (ByteArray.mk #[0x42]) 0 = some (0x42, 1) := by native_decide
example : parseDerLengthOk (ByteArray.mk #[0x81, 0xff]) 0 = some (0xff, 2) := by native_decide
example : parseDerLengthOk (ByteArray.mk #[0x82, 0x01, 0x00]) 0 = some (0x100, 3) := by native_decide
example : parseDerLengthOk (ByteArray.mk #[0x82, 0x03, 0x47]) 0 = some (0x347, 3) := by native_decide
example : parseDerLengthErr (ByteArray.mk #[]) 0 = some .shortInput := by native_decide
example : parseDerLengthErr (ByteArray.mk #[0x80]) 0 = some .invalidIndefiniteLength := by native_decide
example : parseDerLengthErr (ByteArray.mk #[0x82, 0x01]) 0 = some .shortInput := by native_decide
example : parseDerLengthErr (ByteArray.mk #[0x89, 0,0,0,0,0,0,0,0,0]) 0 = some .overflow := by native_decide

/-! ## DER TLV (Tag-Length-Value) -/

structure Tlv where
  tag       : DerTag
  /-- Offset of the value bytes within the input. -/
  valueOff  : Nat
  /-- Length of the value bytes. -/
  valueLen  : Nat
  /-- Total bytes consumed (tag-byte + length-bytes + value). -/
  consumed  : Nat
deriving Repr, DecidableEq, Inhabited

inductive TlvError : Type where
  | shortTag
  | length (e : LengthError)
  | shortValue
deriving Repr, Inhabited

def TlvError.tag : TlvError → UInt8
  | .shortTag    => 1
  | .length _    => 2
  | .shortValue  => 3

/-- Parse one DER TLV at `off`. -/
def parseTlv (bs : ByteArray) (off : Nat) : Except TlvError Tlv :=
  if off ≥ bs.size then
    .error .shortTag
  else
    let tagByte := bs.get! off
    let derTag := parseTagByte tagByte
    match parseDerLength bs (off + 1) with
    | .error e => .error (.length e)
    | .ok (valueLen, lenConsumed) =>
      let valueOff := off + 1 + lenConsumed
      if valueOff + valueLen > bs.size then
        .error .shortValue
      else
        .ok {
          tag := derTag
          valueOff := valueOff
          valueLen := valueLen
          consumed := 1 + lenConsumed + valueLen
        }

example :
    let r := parseTlv (ByteArray.mk #[0x30, 0x03, 0x01, 0x02, 0x03]) 0
    r.toOption.isSome := by native_decide

example :
    let r := parseTlv (ByteArray.mk #[0x06, 0x03, 0x55, 0x04, 0x03]) 0
    r.toOption.isSome := by native_decide

example :
    let r := parseTlv (ByteArray.mk #[0x30, 0x03, 0x01, 0x02]) 0
    r.toOption.isNone := by native_decide

/-! ## SEQUENCE walk -/

/-- Walk a sequence of length-determined TLVs starting at `off`,
    bounded by `endOff`. Returns the list of TLVs.

    Total: each step consumes ≥ 2 bytes (tag-byte + length-byte),
    so `endOff - off` strictly decreases. -/
partial def walkSequence (bs : ByteArray) (off endOff : Nat) :
    Except TlvError (List Tlv) := Id.run do
  if off ≥ endOff then
    return .ok []
  match parseTlv bs off with
  | .error e => return .error e
  | .ok tlv =>
    if tlv.consumed = 0 then
      return .error .shortValue  -- defensive
    match walkSequence bs (off + tlv.consumed) endOff with
    | .error e => return .error e
    | .ok rest => return .ok (tlv :: rest)

/-! ## OID parsing (X.690 §8.19) -/

/-- Parse a DER OID's value bytes into a list of arc components.
    The first byte encodes `arc[0] * 40 + arc[1]`; subsequent
    components are base-128 encoded with continuation bits. -/
partial def parseOidArcs (bs : ByteArray) (off endOff : Nat) :
    List Nat := Id.run do
  if off ≥ endOff then return []
  let b0 := bs.get! off
  let arc0 : Nat := (b0.toNat) / 40
  let arc1 : Nat := (b0.toNat) % 40
  let mut arcs : List Nat := [arc1, arc0].reverse
  let mut acc : Nat := 0
  let mut cur := off + 1
  while cur < endOff do
    let b := bs.get! cur
    acc := (acc <<< 7) ||| (b.toNat &&& 0x7f)
    if (b &&& 0x80) = 0 then
      arcs := arcs ++ [acc]
      acc := 0
    cur := cur + 1
  return arcs

/-- Convenience: parse OID arcs from a TLV. -/
def parseOidFromTlv (bs : ByteArray) (tlv : Tlv) : List Nat :=
  parseOidArcs bs tlv.valueOff (tlv.valueOff + tlv.valueLen)

/-! ## Common OIDs (subset relevant to our verifier) -/

/-- `1.2.840.113549.1.1.1` — `rsaEncryption`. -/
def oidRsaEncryption : List Nat := [1, 2, 840, 113549, 1, 1, 1]
/-- `1.2.840.113549.1.1.5` — `sha1WithRSAEncryption`. -/
def oidSha1WithRsa : List Nat := [1, 2, 840, 113549, 1, 1, 5]
/-- `1.2.840.113549.1.1.11` — `sha256WithRSAEncryption`. -/
def oidSha256WithRsa : List Nat := [1, 2, 840, 113549, 1, 1, 11]
/-- `1.2.840.113549.1.1.13` — `sha512WithRSAEncryption`. -/
def oidSha512WithRsa : List Nat := [1, 2, 840, 113549, 1, 1, 13]
/-- `1.2.840.113549.1.1.10` — `RSASSA-PSS`. -/
def oidRsaPss : List Nat := [1, 2, 840, 113549, 1, 1, 10]
/-- `1.2.840.10045.4.3.2` — `ecdsa-with-SHA256`. -/
def oidEcdsaWithSha256 : List Nat := [1, 2, 840, 10045, 4, 3, 2]
/-- `1.2.840.10045.4.3.4` — `ecdsa-with-SHA512`. -/
def oidEcdsaWithSha512 : List Nat := [1, 2, 840, 10045, 4, 3, 4]
/-- `1.3.101.112` — `Ed25519` (RFC 8410). -/
def oidEd25519 : List Nat := [1, 3, 101, 112]
/-- `1.3.14.3.2.26` — `sha1`. -/
def oidSha1 : List Nat := [1, 3, 14, 3, 2, 26]
/-- `2.16.840.1.101.3.4.2.1` — `sha256`. -/
def oidSha256 : List Nat := [2, 16, 840, 1, 101, 3, 4, 2, 1]
/-- `2.16.840.1.101.3.4.2.3` — `sha512`. -/
def oidSha512 : List Nat := [2, 16, 840, 1, 101, 3, 4, 2, 3]
/-- `1.2.840.113549.1.7.2` — PKCS#7 `signedData`. -/
def oidPkcs7SignedData : List Nat := [1, 2, 840, 113549, 1, 7, 2]
/-- `1.2.840.113549.1.7.1` — PKCS#7 `data`. -/
def oidPkcs7Data : List Nat := [1, 2, 840, 113549, 1, 7, 1]
/-- `1.2.840.113549.1.9.4` — PKCS#9 `messageDigest`. -/
def oidMessageDigest : List Nat := [1, 2, 840, 113549, 1, 9, 4]

theorem oid_rsa_encryption_length : oidRsaEncryption.length = 7 := by native_decide
theorem oid_sha256_with_rsa_length : oidSha256WithRsa.length = 7 := by native_decide
theorem oid_ecdsa_sha256_length : oidEcdsaWithSha256.length = 7 := by native_decide
theorem oid_ed25519_length : oidEd25519.length = 4 := by native_decide
theorem oid_sha256_length : oidSha256.length = 9 := by native_decide
theorem oid_pkcs7_signed_data_length : oidPkcs7SignedData.length = 7 := by native_decide

theorem oids_pairwise_distinct_rsa_vs_ecdsa :
    oidSha256WithRsa ≠ oidEcdsaWithSha256 := by native_decide
theorem oids_pairwise_distinct_rsa_vs_ed25519 :
    oidSha256WithRsa ≠ oidEd25519 := by native_decide
theorem oids_pairwise_distinct_sha256_vs_sha512 :
    oidSha256 ≠ oidSha512 := by native_decide

end Apkaxiom.Signing.Asn1
