/-
P1.11 — Properties of `Apkaxiom.Signing.V3`.

Mechanical theorems over the v3 verifier — same shape as V2 plus
the SDK-range gate and downgrade-attempt classification.
-/

import Std
import Apkaxiom.Signing.V3

namespace Apkaxiom.Signing.V3.Properties

open Apkaxiom.Signing.V3

/-! ## Tag injectivity -/

theorem v3_verify_result_tag_inj :
    ∀ a b : V3VerifyResult, a.tag = b.tag → a = b := V3VerifyResult.tag_inj

/-! ## All 14 result tags distinct -/

theorem accept_distinct_no_v3_block :
    V3VerifyResult.accept.tag ≠ V3VerifyResult.rejectNoV3Block.tag := by native_decide
theorem accept_distinct_malformed :
    V3VerifyResult.accept.tag ≠ V3VerifyResult.rejectMalformed.tag := by native_decide
theorem accept_distinct_sdk_range :
    V3VerifyResult.accept.tag ≠ V3VerifyResult.rejectSdkRangeMismatch.tag := by native_decide
theorem accept_distinct_downgrade :
    V3VerifyResult.accept.tag ≠ V3VerifyResult.rejectDowngradeAttempt.tag := by native_decide
theorem accept_distinct_janus :
    V3VerifyResult.accept.tag ≠ V3VerifyResult.rejectJanusCve_2017_13156.tag := by native_decide

theorem sdk_range_distinct_downgrade :
    V3VerifyResult.rejectSdkRangeMismatch.tag ≠ V3VerifyResult.rejectDowngradeAttempt.tag
      := by native_decide

theorem downgrade_distinct_janus :
    V3VerifyResult.rejectDowngradeAttempt.tag ≠ V3VerifyResult.rejectJanusCve_2017_13156.tag
      := by native_decide

/-! ## Proof-of-rotation attribute ID -/

theorem proof_of_rotation_attr_id : proofOfRotationAttrId = 0x3ba06f8c := by native_decide

/-! ## SDK-range coverage -/

theorem signer_with_no_range_no_coverage
    (s : Apkaxiom.Signing.Scheme.Signer) (h : s.sdkRange = none) (api : UInt32) :
    signerCoversApiLevel s api = false := by
  unfold signerCoversApiLevel
  rw [h]

theorem signer_with_range_covers_within
    (s : Apkaxiom.Signing.Scheme.Signer) (lo hi api : UInt32)
    (hRange : s.sdkRange = some (lo, hi))
    (hLo : lo ≤ api) (hHi : api ≤ hi) :
    signerCoversApiLevel s api = true := by
  unfold signerCoversApiLevel
  rw [hRange]
  simp [hLo, hHi]

/-! ## Janus rejects unconditionally -/

example
    (oracle : CryptoOracle) (apkBytes : ByteArray)
    (block : Apkaxiom.Signing.Block.Block) :
    verifyV3 oracle apkBytes block true
      = V3VerifyResult.rejectJanusCve_2017_13156 := by
  unfold verifyV3
  simp [Id.run]
  rfl

end Apkaxiom.Signing.V3.Properties
