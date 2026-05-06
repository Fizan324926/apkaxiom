/-
P1.11 G17 — PKCS#7 SignedData parser type signatures (Lean side).

JAR signature blocks (`META-INF/<KEY>.RSA / .DSA / .EC`) are
PKCS#7 SignedData over the corresponding `.SF` payload. Layout:

  ContentInfo ::= SEQUENCE {
    contentType  OBJECT IDENTIFIER,    -- 1.2.840.113549.1.7.2 (signedData)
    content      [0] EXPLICIT SignedData
  }

  SignedData ::= SEQUENCE {
    version          INTEGER,
    digestAlgorithms SET OF AlgorithmIdentifier,
    encapContentInfo SEQUENCE { … },
    certificates     [0] IMPLICIT SET OF Certificate OPTIONAL,
    crls             [1] IMPLICIT SET OF CRL OPTIONAL,
    signerInfos      SET OF SignerInfo
  }

  SignerInfo ::= SEQUENCE {
    version           INTEGER,
    sid               SignerIdentifier,
    digestAlgorithm   AlgorithmIdentifier,
    signedAttrs       [0] IMPLICIT SET OF Attribute OPTIONAL,
    signatureAlgorithm AlgorithmIdentifier,
    signature         OCTET STRING,
    unsignedAttrs     [1] IMPLICIT SET OF Attribute OPTIONAL
  }

This module ships the *type signatures* the verifier predicate
threads through; the mechanical SignedData walker is implemented
in the Rust mirror at `crates/axiom-sigverify/src/scheme_v1.rs`
(via the `cms` crate). Per ADR-0029 the in-Lean walker is part
of operator one-shot P111-OP-3 — the in-Lean ASN.1 layer
(`Apkaxiom.Signing.Asn1`) is the lightweight foundation; the
full PKCS#7 walk is still backed by the Rust crate.
-/

import Std
import Apkaxiom.Signing.Asn1

namespace Apkaxiom.Signing.Pkcs7

open Apkaxiom.Signing.Asn1

/-! ## Errors -/

inductive Pkcs7Error : Type where
  | missingContentInfo
  | wrongContentType
  | missingSignedData
  | missingSignerInfos
  | tlv (e : TlvError)
deriving Repr, Inhabited

def Pkcs7Error.tag : Pkcs7Error → UInt8
  | .missingContentInfo => 1
  | .wrongContentType   => 2
  | .missingSignedData  => 3
  | .missingSignerInfos => 4
  | .tlv _              => 5

/-! ## Parsed shape (the verifier-relevant projection) -/

/-- One SignerInfo. -/
structure SignerInfo where
  /-- DER bytes of the digest-algorithm OID. -/
  digestAlgorithmOid    : ByteArray
  /-- DER bytes of the signature-algorithm OID. -/
  signatureAlgorithmOid : ByteArray
  /-- True iff `signed_attrs` is present. -/
  hasSignedAttrs        : Bool
  /-- DER-encoded `signed_attrs`, IMPLICIT [0] tag rewritten to SET. -/
  signedAttrsDer        : ByteArray
  /-- Signature bytes. -/
  signature             : ByteArray
deriving Inhabited

/-- Fully-parsed SignedData. -/
structure SignedData where
  certificates : List ByteArray
  signerInfos  : List SignerInfo
deriving Inhabited

/-! ## SignedData top-level — types only -/

/-- Slim SignedData parser stub. The Rust mirror at
    `crates/axiom-sigverify/src/scheme_v1.rs::verify_pkcs7_over`
    is the load-bearing implementation. -/
def parseContentInfo (_bs : ByteArray) : Except Pkcs7Error SignedData :=
  .ok { certificates := [], signerInfos := [] }

/-! ## Smoke checks -/

example : Pkcs7Error.missingContentInfo.tag = 1 := by native_decide
example : Pkcs7Error.wrongContentType.tag = 2 := by native_decide
example : Pkcs7Error.missingSignedData.tag = 3 := by native_decide
example : Pkcs7Error.missingSignerInfos.tag = 4 := by native_decide

theorem pkcs7_error_tag_distinct_1_2 :
    Pkcs7Error.missingContentInfo.tag ≠ Pkcs7Error.wrongContentType.tag := by native_decide
theorem pkcs7_error_tag_distinct_3_4 :
    Pkcs7Error.missingSignedData.tag ≠ Pkcs7Error.missingSignerInfos.tag := by native_decide

theorem parse_content_info_empty :
    parseContentInfo ByteArray.empty
      = .ok ({ certificates := [], signerInfos := [] } : SignedData) := by
  rfl

end Apkaxiom.Signing.Pkcs7
