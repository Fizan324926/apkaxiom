/-
P1.11 G17 — properties of `Apkaxiom.Signing.Asn1`.

Mechanical theorems over the DER tag classifier, length parser,
TLV walker, and OID round-trips.
-/

import Std
import Apkaxiom.Signing.Asn1

namespace Apkaxiom.Signing.Asn1.Properties

open Apkaxiom.Signing.Asn1

/-! ## Tag class injectivity / round-trip -/

theorem tag_class_to_u8_inj :
    ∀ a b : TagClass, a.toU8 = b.toU8 → a = b := by
  intro a b h
  cases a <;> cases b <;> simp [TagClass.toU8] at h <;> rfl

theorem tag_class_from_u8_universal :
    TagClass.fromU8 0 = some .universal := by native_decide
theorem tag_class_from_u8_application :
    TagClass.fromU8 1 = some .application := by native_decide
theorem tag_class_from_u8_context :
    TagClass.fromU8 2 = some .contextSpecific := by native_decide
theorem tag_class_from_u8_private :
    TagClass.fromU8 3 = some .private_ := by native_decide

theorem tag_class_unknown_byte_4 : TagClass.fromU8 4 = none := by native_decide
theorem tag_class_unknown_byte_5 : TagClass.fromU8 5 = none := by native_decide
theorem tag_class_unknown_byte_64 : TagClass.fromU8 64 = none := by native_decide
theorem tag_class_unknown_byte_128 : TagClass.fromU8 128 = none := by native_decide
theorem tag_class_unknown_byte_192 : TagClass.fromU8 192 = none := by native_decide
theorem tag_class_unknown_byte_255 : TagClass.fromU8 255 = none := by native_decide

/-! ## Universal-tag injectivity / round-trip -/

theorem universal_tag_to_u8_inj :
    ∀ a b : UniversalTag, a.toU8 = b.toU8 → a = b := by
  intro a b h
  cases a <;> cases b <;> simp [UniversalTag.toU8] at h <;> rfl

/-- All 13 known tag bytes are pairwise distinct. -/
theorem universal_tag_bool_vs_int : UniversalTag.boolean.toU8 ≠ UniversalTag.integer.toU8 := by native_decide
theorem universal_tag_int_vs_bit : UniversalTag.integer.toU8 ≠ UniversalTag.bitString.toU8 := by native_decide
theorem universal_tag_bit_vs_oct : UniversalTag.bitString.toU8 ≠ UniversalTag.octetString.toU8 := by native_decide
theorem universal_tag_oct_vs_null : UniversalTag.octetString.toU8 ≠ UniversalTag.null.toU8 := by native_decide
theorem universal_tag_null_vs_oid : UniversalTag.null.toU8 ≠ UniversalTag.oid.toU8 := by native_decide
theorem universal_tag_oid_vs_utf8 : UniversalTag.oid.toU8 ≠ UniversalTag.utf8String.toU8 := by native_decide
theorem universal_tag_utf8_vs_printable : UniversalTag.utf8String.toU8 ≠ UniversalTag.printableString.toU8 := by native_decide
theorem universal_tag_printable_vs_ia5 : UniversalTag.printableString.toU8 ≠ UniversalTag.ia5String.toU8 := by native_decide
theorem universal_tag_ia5_vs_utc : UniversalTag.ia5String.toU8 ≠ UniversalTag.utcTime.toU8 := by native_decide
theorem universal_tag_utc_vs_gen : UniversalTag.utcTime.toU8 ≠ UniversalTag.generalizedTime.toU8 := by native_decide
theorem universal_tag_gen_vs_seq : UniversalTag.generalizedTime.toU8 ≠ UniversalTag.sequence.toU8 := by native_decide
theorem universal_tag_seq_vs_set : UniversalTag.sequence.toU8 ≠ UniversalTag.set.toU8 := by native_decide

/-! ## Length-error injectivity -/

theorem length_error_inj_short_indef : LengthError.shortInput ≠ LengthError.invalidIndefiniteLength := by decide
theorem length_error_inj_short_overflow : LengthError.shortInput ≠ LengthError.overflow := by decide
theorem length_error_inj_indef_overflow : LengthError.invalidIndefiniteLength ≠ LengthError.overflow := by decide

/-! ## DER length parser — happy paths -/

theorem parse_der_length_short_form_zero :
    parseDerLengthOk (ByteArray.mk #[0x00]) 0 = some (0, 1) := by native_decide
theorem parse_der_length_short_form_one :
    parseDerLengthOk (ByteArray.mk #[0x01]) 0 = some (1, 1) := by native_decide
theorem parse_der_length_short_form_127 :
    parseDerLengthOk (ByteArray.mk #[0x7f]) 0 = some (127, 1) := by native_decide
theorem parse_der_length_long_form_128 :
    parseDerLengthOk (ByteArray.mk #[0x81, 0x80]) 0 = some (128, 2) := by native_decide
theorem parse_der_length_long_form_2byte :
    parseDerLengthOk (ByteArray.mk #[0x82, 0x01, 0x00]) 0 = some (256, 3) := by native_decide
theorem parse_der_length_long_form_3byte :
    parseDerLengthOk (ByteArray.mk #[0x83, 0x01, 0x00, 0x00]) 0 = some (65536, 4) := by native_decide
theorem parse_der_length_long_form_4byte :
    parseDerLengthOk (ByteArray.mk #[0x84, 0xff, 0xff, 0xff, 0xff]) 0
      = some (0xffffffff, 5) := by native_decide

/-! ## DER length parser — error paths -/

theorem parse_der_length_empty :
    parseDerLengthErr (ByteArray.mk #[]) 0 = some .shortInput := by native_decide
theorem parse_der_length_indef :
    parseDerLengthErr (ByteArray.mk #[0x80]) 0 = some .invalidIndefiniteLength := by native_decide
theorem parse_der_length_truncated_long_form :
    parseDerLengthErr (ByteArray.mk #[0x82, 0x01]) 0 = some .shortInput := by native_decide
theorem parse_der_length_overflow :
    parseDerLengthErr (ByteArray.mk #[0x89, 0,0,0,0,0,0,0,0,0]) 0 = some .overflow := by native_decide

/-! ## OID arc decoding -/

theorem oid_arcs_rsa_encryption :
    parseOidArcs (ByteArray.mk #[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01]) 0 9
      = [1, 2, 840, 113549, 1, 1, 1] := by native_decide

theorem oid_arcs_sha256_with_rsa :
    parseOidArcs (ByteArray.mk #[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b]) 0 9
      = [1, 2, 840, 113549, 1, 1, 11] := by native_decide

theorem oid_arcs_ed25519 :
    parseOidArcs (ByteArray.mk #[0x2b, 0x65, 0x70]) 0 3
      = [1, 3, 101, 112] := by native_decide

theorem oid_arcs_ecdsa_sha256 :
    parseOidArcs (ByteArray.mk #[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x02]) 0 8
      = [1, 2, 840, 10045, 4, 3, 2] := by native_decide

theorem oid_arcs_sha256 :
    parseOidArcs (ByteArray.mk #[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01]) 0 9
      = [2, 16, 840, 1, 101, 3, 4, 2, 1] := by native_decide

/-! ## OID constants — pairwise distinct -/

theorem oid_rsa_vs_ecdsa : oidRsaEncryption ≠ oidEcdsaWithSha256 := by native_decide
theorem oid_rsa_vs_ed25519 : oidRsaEncryption ≠ oidEd25519 := by native_decide
theorem oid_ecdsa_sha256_vs_sha512 : oidEcdsaWithSha256 ≠ oidEcdsaWithSha512 := by native_decide
theorem oid_sha1_vs_sha256_with_rsa : oidSha1 ≠ oidSha256WithRsa := by native_decide
theorem oid_pkcs7_signed_vs_data : oidPkcs7SignedData ≠ oidPkcs7Data := by native_decide
theorem oid_message_digest_vs_data : oidMessageDigest ≠ oidPkcs7Data := by native_decide

/-! ## Tag-byte parser — class extraction -/

theorem parse_tag_byte_seq_universal :
    (parseTagByte 0x30).tagClass = .universal := by native_decide
theorem parse_tag_byte_seq_constructed :
    (parseTagByte 0x30).constructed = true := by native_decide
theorem parse_tag_byte_seq_number :
    (parseTagByte 0x30).number = 0x10 := by native_decide

theorem parse_tag_byte_int_universal :
    (parseTagByte 0x02).tagClass = .universal := by native_decide
theorem parse_tag_byte_int_primitive :
    (parseTagByte 0x02).constructed = false := by native_decide

theorem parse_tag_byte_context_a0_class :
    (parseTagByte 0xa0).tagClass = .contextSpecific := by native_decide
theorem parse_tag_byte_context_a0_constructed :
    (parseTagByte 0xa0).constructed = true := by native_decide
theorem parse_tag_byte_context_a0_number :
    (parseTagByte 0xa0).number = 0 := by native_decide

theorem parse_tag_byte_context_a3_number :
    (parseTagByte 0xa3).number = 3 := by native_decide

end Apkaxiom.Signing.Asn1.Properties
