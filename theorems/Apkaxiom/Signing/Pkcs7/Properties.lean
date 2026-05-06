/-
P1.11 G17 — properties of `Apkaxiom.Signing.Pkcs7`.

Tag injectivity, structural-type round-trip, and stub-behaviour
witnesses for the SignedData parser.
-/

import Std
import Apkaxiom.Signing.Pkcs7

namespace Apkaxiom.Signing.Pkcs7.Properties

open Apkaxiom.Signing.Pkcs7

/-! ## Pkcs7Error tag distinctness -/

theorem error_tag_content_vs_type :
    Pkcs7Error.missingContentInfo.tag ≠ Pkcs7Error.wrongContentType.tag := by native_decide
theorem error_tag_signed_vs_signers :
    Pkcs7Error.missingSignedData.tag ≠ Pkcs7Error.missingSignerInfos.tag := by native_decide
theorem error_tag_content_vs_signed :
    Pkcs7Error.missingContentInfo.tag ≠ Pkcs7Error.missingSignedData.tag := by native_decide
theorem error_tag_type_vs_signers :
    Pkcs7Error.wrongContentType.tag ≠ Pkcs7Error.missingSignerInfos.tag := by native_decide

/-! ## SignerInfo / SignedData are inhabited -/

example : Inhabited SignerInfo := inferInstance
example : Inhabited SignedData := inferInstance

/-- Default SignedData is empty. -/
example : (default : SignedData).certificates = [] := by rfl
example : (default : SignedData).signerInfos = [] := by rfl

/-! ## parseContentInfo stub -/

theorem parse_content_info_returns_empty :
    parseContentInfo ByteArray.empty
      = .ok ({ certificates := [], signerInfos := [] } : SignedData) := by
  rfl

theorem parse_content_info_independent_of_input :
    ∀ bs : ByteArray,
      parseContentInfo bs
        = .ok ({ certificates := [], signerInfos := [] } : SignedData) := by
  intro _; rfl

end Apkaxiom.Signing.Pkcs7.Properties
