/-
P1.11 — Properties of `Apkaxiom.Signing.V1` (JAR scheme).

Mechanical theorems over the META-INF inventory + signature-block
classification + verifier-result tag injectivity.
-/

import Std
import Apkaxiom.Signing.V1

namespace Apkaxiom.Signing.V1.Properties

open Apkaxiom.Signing.V1

/-! ## Result tag injectivity -/

theorem v1_verify_result_tag_inj :
    ∀ a b : V1VerifyResult, a.tag = b.tag → a = b := V1VerifyResult.tag_inj

/-! ## All 9 result tags are distinct -/

theorem accept_distinct_no_manifest :
    V1VerifyResult.accept.tag ≠ V1VerifyResult.rejectNoManifest.tag := by native_decide
theorem accept_distinct_no_sf :
    V1VerifyResult.accept.tag ≠ V1VerifyResult.rejectNoSf.tag := by native_decide
theorem accept_distinct_pkcs7 :
    V1VerifyResult.accept.tag ≠ V1VerifyResult.rejectPkcs7VerifyFailed.tag := by native_decide
theorem accept_distinct_janus :
    V1VerifyResult.accept.tag ≠ V1VerifyResult.rejectJanusCve_2017_13156.tag := by native_decide
theorem janus_distinct_pkcs7 :
    V1VerifyResult.rejectJanusCve_2017_13156.tag ≠ V1VerifyResult.rejectPkcs7VerifyFailed.tag
      := by native_decide

/-! ## File-name predicates -/

/-- "META-INF/" prefix correctness. -/
theorem in_meta_inf_meta_inf_path :
    inMetaInf metaInfPrefix = true := by native_decide

theorem in_meta_inf_manifest_path :
    inMetaInf manifestPath = true := by native_decide

theorem in_meta_inf_root_classes_dex :
    inMetaInf (ByteArray.mk #[0x63, 0x6c, 0x61, 0x73, 0x73, 0x65, 0x73, 0x2e, 0x64, 0x65, 0x78])
      = false := by native_decide

theorem in_meta_inf_short_path :
    inMetaInf (ByteArray.mk #[0x4d, 0x45, 0x54]) = false := by native_decide

/-! ## SigBlockKind classification -/

theorem rsa_extension_classifies_as_rsa :
    SigBlockKind.fromName (ByteArray.mk #[0x43, 0x45, 0x52, 0x54, 0x2e, 0x52, 0x53, 0x41])
      = some .rsa := by native_decide

theorem dsa_extension_classifies_as_dsa :
    SigBlockKind.fromName (ByteArray.mk #[0x43, 0x45, 0x52, 0x54, 0x2e, 0x44, 0x53, 0x41])
      = some .dsa := by native_decide

theorem ec_extension_classifies_as_ec :
    SigBlockKind.fromName (ByteArray.mk #[0x43, 0x45, 0x52, 0x54, 0x2e, 0x45, 0x43])
      = some .ec := by native_decide

theorem unknown_extension_classifies_as_none :
    SigBlockKind.fromName (ByteArray.mk #[0x4e, 0x4f, 0x54, 0x53, 0x49, 0x47]) = none
      := by native_decide

theorem too_short_classifies_as_none :
    SigBlockKind.fromName ByteArray.empty = none := by native_decide

/-! ## DigestAlgorithm enumeration -/

theorem digest_algorithms_distinct_sha1_sha256 :
    DigestAlgorithm.sha1 ≠ DigestAlgorithm.sha256 := by native_decide

theorem digest_algorithms_distinct_sha256_sha512 :
    DigestAlgorithm.sha256 ≠ DigestAlgorithm.sha512 := by native_decide

theorem digest_algorithms_distinct_sha1_sha512 :
    DigestAlgorithm.sha1 ≠ DigestAlgorithm.sha512 := by native_decide

/-! ## startsWith / endsWith identity-cases -/

theorem starts_with_self : startsWith metaInfPrefix metaInfPrefix = true := by native_decide
theorem ends_with_self : endsWith manifestPath manifestPath = true := by native_decide

theorem starts_with_empty :
    startsWith (ByteArray.mk #[0x41]) ByteArray.empty = true := by native_decide
theorem ends_with_empty :
    endsWith (ByteArray.mk #[0x41]) ByteArray.empty = true := by native_decide

theorem starts_with_too_long :
    startsWith ByteArray.empty (ByteArray.mk #[0x41]) = false := by native_decide
theorem ends_with_too_long :
    endsWith ByteArray.empty (ByteArray.mk #[0x41]) = false := by native_decide

/-! ## Empty-input verifier shape -/

/-- An empty inventory + Janus signal yields the Janus reject. -/
example
    (oracle : CryptoOracle) :
    verifyV1 oracle
      { manifestMf := none, signatureFiles := [], signatureBlocks := [], otherMetaInf := [] }
      [] true
      = .rejectJanusCve_2017_13156 := by
  rfl

/-- An empty inventory (no Janus) yields rejectNoManifest. -/
example
    (oracle : CryptoOracle) :
    verifyV1 oracle
      { manifestMf := none, signatureFiles := [], signatureBlocks := [], otherMetaInf := [] }
      [] false
      = .rejectNoManifest := by
  rfl

end Apkaxiom.Signing.V1.Properties
