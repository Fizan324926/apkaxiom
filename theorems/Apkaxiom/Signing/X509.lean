/-
P1.11 G17 — X.509 v3 certificate parser in Lean (DER subset).

Mirrors the Rust `x509_cert` crate's behaviour on the bytes
APK signing produces. Sufficient for SPKI extraction (the load-
bearing operation the verifier needs).

X.509 v3 layout (RFC 5280 §4.1):

  Certificate ::= SEQUENCE {
    tbsCertificate       TBSCertificate,
    signatureAlgorithm   AlgorithmIdentifier,
    signatureValue       BIT STRING
  }

  TBSCertificate ::= SEQUENCE {
    version              [0] EXPLICIT Version DEFAULT v1,
    serialNumber         CertificateSerialNumber,
    signature            AlgorithmIdentifier,
    issuer               Name,
    validity             Validity,
    subject              Name,
    subjectPublicKeyInfo SubjectPublicKeyInfo,
    issuerUniqueID       [1] IMPLICIT UniqueIdentifier OPTIONAL,
    subjectUniqueID      [2] IMPLICIT UniqueIdentifier OPTIONAL,
    extensions           [3] EXPLICIT Extensions OPTIONAL
  }

  SubjectPublicKeyInfo ::= SEQUENCE {
    algorithm            AlgorithmIdentifier,
    subjectPublicKey     BIT STRING
  }

  AlgorithmIdentifier ::= SEQUENCE {
    algorithm            OBJECT IDENTIFIER,
    parameters           ANY DEFINED BY algorithm OPTIONAL
  }

The verifier only needs:
  1. Parse the outer Certificate SEQUENCE.
  2. Walk into TBSCertificate.
  3. Extract the SPKI byte range (verbatim DER).
The cert chain validation lives in the cryptographic-oracle
layer, not here.
-/

import Std
import Apkaxiom.Signing.Asn1

namespace Apkaxiom.Signing.X509

open Apkaxiom.Signing.Asn1

/-! ## Errors -/

inductive X509Error : Type where
  /-- Outer SEQUENCE missing or malformed. -/
  | missingOuter
  /-- TBSCertificate SEQUENCE missing or malformed. -/
  | missingTbs
  /-- SubjectPublicKeyInfo missing. -/
  | missingSpki
  /-- A required field has the wrong tag. -/
  | wrongTag (expected actual : UInt8)
  /-- A wrapped DER read failed. -/
  | tlv (e : TlvError)
deriving Repr, Inhabited

def X509Error.tag : X509Error → UInt8
  | .missingOuter   => 1
  | .missingTbs     => 2
  | .missingSpki    => 3
  | .wrongTag _ _   => 4
  | .tlv _          => 5

/-! ## SPKI byte-range extraction -/

/-- Tag byte for a constructed Universal SEQUENCE = `0x30`. -/
def sequenceTagByte : UInt8 := 0x30
/-- Tag byte for a constructed ContextSpecific [0] = `0xa0`. -/
def context0TagByte : UInt8 := 0xa0
/-- Tag byte for an INTEGER = `0x02`. -/
def integerTagByte : UInt8 := 0x02

/-- Extract the verbatim DER bytes of the SubjectPublicKeyInfo
    sub-structure from a leaf X.509 certificate.

    Algorithm (per RFC 5280):

      1. Outer Certificate = SEQUENCE — peel.
      2. tbsCertificate = SEQUENCE — peel; this is where SPKI lives.
      3. Inside tbsCertificate, walk past:
         - optional `[0]` version
         - serialNumber INTEGER
         - signature SEQUENCE (algorithm)
         - issuer SEQUENCE
         - validity SEQUENCE
         - subject SEQUENCE
         The next SEQUENCE is the SubjectPublicKeyInfo.

    Returns the SPKI's full TLV bytes (tag-byte + length + value). -/
def extractSpkiDer (bs : ByteArray) : Except X509Error ByteArray := Id.run do
  -- Step 1: parse outer Certificate.
  match parseTlv bs 0 with
  | .error e => return .error (.tlv e)
  | .ok outer =>
    if bs.get! 0 ≠ sequenceTagByte then
      return .error .missingOuter
    -- Step 2: parse tbsCertificate (first child of outer).
    let tbs_off := outer.valueOff
    match parseTlv bs tbs_off with
    | .error e => return .error (.tlv e)
    | .ok tbs =>
      if bs.get! tbs_off ≠ sequenceTagByte then
        return .error .missingTbs
      -- Step 3: walk children of TBSCertificate.
      let mut cur := tbs.valueOff
      let endo := tbs.valueOff + tbs.valueLen
      -- Optional [0] EXPLICIT version
      if cur < endo && bs.get! cur = context0TagByte then
        match parseTlv bs cur with
        | .error e => return .error (.tlv e)
        | .ok t => cur := cur + t.consumed
      -- serialNumber (INTEGER)
      if cur < endo then
        match parseTlv bs cur with
        | .error e => return .error (.tlv e)
        | .ok t => cur := cur + t.consumed
      -- signature (SEQUENCE)
      if cur < endo then
        match parseTlv bs cur with
        | .error e => return .error (.tlv e)
        | .ok t => cur := cur + t.consumed
      -- issuer (SEQUENCE)
      if cur < endo then
        match parseTlv bs cur with
        | .error e => return .error (.tlv e)
        | .ok t => cur := cur + t.consumed
      -- validity (SEQUENCE)
      if cur < endo then
        match parseTlv bs cur with
        | .error e => return .error (.tlv e)
        | .ok t => cur := cur + t.consumed
      -- subject (SEQUENCE)
      if cur < endo then
        match parseTlv bs cur with
        | .error e => return .error (.tlv e)
        | .ok t => cur := cur + t.consumed
      -- subjectPublicKeyInfo (SEQUENCE) — THIS is what we want.
      if cur ≥ endo then
        return .error .missingSpki
      if bs.get! cur ≠ sequenceTagByte then
        return .error .missingSpki
      match parseTlv bs cur with
      | .error e => return .error (.tlv e)
      | .ok spki =>
        let spki_total := spki.consumed
        let .some sub := slice bs cur spki_total
          | return .error .missingSpki
        return .ok sub
where
  slice (bs : ByteArray) (o len : Nat) : Option ByteArray :=
    if o + len ≤ bs.size then some (bs.extract o (o + len)) else none

/-! ## Smoke checks (synthesized minimal certs) -/

/-- A minimal SEQUENCE { INTEGER 1 } that is NOT a valid X.509
    cert — extractSpkiDer should reject because the inner SEQUENCE
    walk fails. -/
example :
    (extractSpkiDer (ByteArray.mk #[0x30, 0x03, 0x02, 0x01, 0x01])).toOption.isNone := by
  native_decide

/-- An empty input returns missingOuter via the TLV layer. -/
example :
    (extractSpkiDer ByteArray.empty).toOption.isNone := by
  native_decide

end Apkaxiom.Signing.X509
