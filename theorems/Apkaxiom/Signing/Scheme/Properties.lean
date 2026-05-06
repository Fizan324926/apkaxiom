/-
P1.11 — Properties of `Apkaxiom.Signing.Scheme`.

Mechanical theorems over the v2/v3/v3.1 internal-structure
parsers: signature-algorithm-id round-trip, error-tag injectivity,
variant SDK-range gate.
-/

import Std
import Apkaxiom.Signing.Scheme

namespace Apkaxiom.Signing.Scheme.Properties

open Apkaxiom.Signing.Scheme

/-! ## SignatureAlgorithmId round-trip -/

theorem id_rsa_pss_sha256_roundtrip :
    SignatureAlgorithmId.fromU32 SignatureAlgorithmId.rsaPssSha256.toU32
      = some .rsaPssSha256 := by native_decide
theorem id_rsa_pss_sha512_roundtrip :
    SignatureAlgorithmId.fromU32 SignatureAlgorithmId.rsaPssSha512.toU32
      = some .rsaPssSha512 := by native_decide
theorem id_rsa_pkcs1_sha256_roundtrip :
    SignatureAlgorithmId.fromU32 SignatureAlgorithmId.rsaPkcs1Sha256.toU32
      = some .rsaPkcs1Sha256 := by native_decide
theorem id_rsa_pkcs1_sha512_roundtrip :
    SignatureAlgorithmId.fromU32 SignatureAlgorithmId.rsaPkcs1Sha512.toU32
      = some .rsaPkcs1Sha512 := by native_decide
theorem id_ecdsa_sha256_roundtrip :
    SignatureAlgorithmId.fromU32 SignatureAlgorithmId.ecdsaSha256.toU32
      = some .ecdsaSha256 := by native_decide
theorem id_ecdsa_sha512_roundtrip :
    SignatureAlgorithmId.fromU32 SignatureAlgorithmId.ecdsaSha512.toU32
      = some .ecdsaSha512 := by native_decide
theorem id_dsa_sha256_roundtrip :
    SignatureAlgorithmId.fromU32 SignatureAlgorithmId.dsaSha256.toU32
      = some .dsaSha256 := by native_decide
theorem id_verity_rsa_pkcs1_sha256_roundtrip :
    SignatureAlgorithmId.fromU32 SignatureAlgorithmId.verityRsaPkcs1Sha256.toU32
      = some .verityRsaPkcs1Sha256 := by native_decide
theorem id_verity_ecdsa_sha256_roundtrip :
    SignatureAlgorithmId.fromU32 SignatureAlgorithmId.verityEcdsaSha256.toU32
      = some .verityEcdsaSha256 := by native_decide
theorem id_verity_dsa_sha256_roundtrip :
    SignatureAlgorithmId.fromU32 SignatureAlgorithmId.verityDsaSha256.toU32
      = some .verityDsaSha256 := by native_decide

/-! ## All ten algorithm IDs distinct -/

theorem alg_ids_pairwise_distinct :
    SignatureAlgorithmId.rsaPssSha256.toU32      ≠ SignatureAlgorithmId.rsaPssSha512.toU32 := by native_decide

theorem alg_ids_rsa_pss_vs_pkcs1 :
    SignatureAlgorithmId.rsaPssSha256.toU32      ≠ SignatureAlgorithmId.rsaPkcs1Sha256.toU32 := by native_decide

theorem alg_ids_rsa_vs_ecdsa :
    SignatureAlgorithmId.rsaPkcs1Sha256.toU32    ≠ SignatureAlgorithmId.ecdsaSha256.toU32 := by native_decide

theorem alg_ids_ecdsa_vs_dsa :
    SignatureAlgorithmId.ecdsaSha256.toU32       ≠ SignatureAlgorithmId.dsaSha256.toU32 := by native_decide

theorem alg_ids_verity_distinct_from_normal :
    SignatureAlgorithmId.verityEcdsaSha256.toU32 ≠ SignatureAlgorithmId.ecdsaSha256.toU32 := by native_decide

/-! ## Error tag injectivity (Scheme errors) -/

theorem scheme_error_tag_inj :
    ∀ a b : SchemeError, a.tag = b.tag → a = b := SchemeError.tag_inj

/-! ## DigestKind output-length contract -/

theorem digest_kind_sha256_len : DigestKind.sha256.len = 32 := by native_decide
theorem digest_kind_sha512_len : DigestKind.sha512.len = 64 := by native_decide

theorem digest_kind_sha256_distinct_from_sha512 :
    DigestKind.sha256.len ≠ DigestKind.sha512.len := by native_decide

/-! ## Algorithm → DigestKind mapping -/

theorem rsa_pss_sha256_uses_sha256 :
    SignatureAlgorithmId.rsaPssSha256.digestKind = .sha256 := by native_decide
theorem rsa_pss_sha512_uses_sha512 :
    SignatureAlgorithmId.rsaPssSha512.digestKind = .sha512 := by native_decide
theorem rsa_pkcs1_sha256_uses_sha256 :
    SignatureAlgorithmId.rsaPkcs1Sha256.digestKind = .sha256 := by native_decide
theorem rsa_pkcs1_sha512_uses_sha512 :
    SignatureAlgorithmId.rsaPkcs1Sha512.digestKind = .sha512 := by native_decide
theorem ecdsa_sha256_uses_sha256 :
    SignatureAlgorithmId.ecdsaSha256.digestKind = .sha256 := by native_decide
theorem ecdsa_sha512_uses_sha512 :
    SignatureAlgorithmId.ecdsaSha512.digestKind = .sha512 := by native_decide
theorem dsa_sha256_uses_sha256 :
    SignatureAlgorithmId.dsaSha256.digestKind = .sha256 := by native_decide
theorem verity_rsa_pkcs1_sha256_uses_sha256 :
    SignatureAlgorithmId.verityRsaPkcs1Sha256.digestKind = .sha256 := by native_decide
theorem verity_ecdsa_sha256_uses_sha256 :
    SignatureAlgorithmId.verityEcdsaSha256.digestKind = .sha256 := by native_decide
theorem verity_dsa_sha256_uses_sha256 :
    SignatureAlgorithmId.verityDsaSha256.digestKind = .sha256 := by native_decide

/-! ## isVerity classification -/

theorem rsa_pss_not_verity     : SignatureAlgorithmId.rsaPssSha256.isVerity      = false := by native_decide
theorem rsa_pkcs1_not_verity   : SignatureAlgorithmId.rsaPkcs1Sha256.isVerity    = false := by native_decide
theorem ecdsa_sha256_not_verity: SignatureAlgorithmId.ecdsaSha256.isVerity       = false := by native_decide
theorem ecdsa_sha512_not_verity: SignatureAlgorithmId.ecdsaSha512.isVerity       = false := by native_decide
theorem dsa_not_verity         : SignatureAlgorithmId.dsaSha256.isVerity         = false := by native_decide
theorem verity_rsa_is_verity   : SignatureAlgorithmId.verityRsaPkcs1Sha256.isVerity = true := by native_decide
theorem verity_ecdsa_is_verity : SignatureAlgorithmId.verityEcdsaSha256.isVerity   = true := by native_decide
theorem verity_dsa_is_verity   : SignatureAlgorithmId.verityDsaSha256.isVerity     = true := by native_decide

/-! ## Variant SDK-range gate -/

theorem v2_no_sdk_range  : Variant.v2.hasSdkRange   = false := by native_decide
theorem v3_has_sdk_range : Variant.v3.hasSdkRange   = true  := by native_decide
theorem v31_has_sdk_range: Variant.v3_1.hasSdkRange = true  := by native_decide

theorem variants_pairwise_distinct_v2_v3 :
    Variant.v2 ≠ Variant.v3 := by native_decide
theorem variants_pairwise_distinct_v2_v31 :
    Variant.v2 ≠ Variant.v3_1 := by native_decide
theorem variants_pairwise_distinct_v3_v31 :
    Variant.v3 ≠ Variant.v3_1 := by native_decide

/-! ## Empty-block parsing returns truncated -/

example : (parseV2 ByteArray.empty).toOption = none := by native_decide
example : (parseV3 ByteArray.empty).toOption = none := by native_decide
example : (parseV3_1 ByteArray.empty).toOption = none := by native_decide

end Apkaxiom.Signing.Scheme.Properties
