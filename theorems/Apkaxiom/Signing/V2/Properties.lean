/-
P1.11 — Properties of `Apkaxiom.Signing.V2`.

Mechanical theorems over the v2 verifier's reject categories,
shape predicates, and oracle-parameterisation.
-/

import Std
import Apkaxiom.Signing.V2

namespace Apkaxiom.Signing.V2.Properties

open Apkaxiom.Signing.V2

/-! ## Tag injectivity -/

theorem v2_verify_result_tag_inj :
    ∀ a b : V2VerifyResult, a.tag = b.tag → a = b := V2VerifyResult.tag_inj

/-! ## All 12 v2 result tags pairwise distinct -/

theorem accept_distinct_no_v2_block :
    V2VerifyResult.accept.tag ≠ V2VerifyResult.rejectNoV2Block.tag := by native_decide
theorem accept_distinct_malformed :
    V2VerifyResult.accept.tag ≠ V2VerifyResult.rejectMalformed.tag := by native_decide
theorem accept_distinct_no_digests :
    V2VerifyResult.accept.tag ≠ V2VerifyResult.rejectNoDigests.tag := by native_decide
theorem accept_distinct_no_signatures :
    V2VerifyResult.accept.tag ≠ V2VerifyResult.rejectNoSignatures.tag := by native_decide
theorem accept_distinct_no_certs :
    V2VerifyResult.accept.tag ≠ V2VerifyResult.rejectNoCertificates.tag := by native_decide
theorem accept_distinct_alg_mismatch :
    V2VerifyResult.accept.tag ≠ V2VerifyResult.rejectAlgorithmMismatch.tag := by native_decide
theorem accept_distinct_digest_mismatch :
    V2VerifyResult.accept.tag ≠ V2VerifyResult.rejectDigestMismatch.tag := by native_decide
theorem accept_distinct_signature_failed :
    V2VerifyResult.accept.tag ≠ V2VerifyResult.rejectSignatureFailed.tag := by native_decide
theorem accept_distinct_pubkey_mismatch :
    V2VerifyResult.accept.tag ≠ V2VerifyResult.rejectPublicKeyMismatch.tag := by native_decide
theorem accept_distinct_all_unknown :
    V2VerifyResult.accept.tag ≠ V2VerifyResult.rejectAllAlgorithmsUnknown.tag := by native_decide
theorem accept_distinct_janus :
    V2VerifyResult.accept.tag ≠ V2VerifyResult.rejectJanusCve_2017_13156.tag := by native_decide

/-! ## Janus rejects unconditionally -/

example
    (oracle : CryptoOracle) (apkBytes : ByteArray)
    (block : Apkaxiom.Signing.Block.Block) :
    verifyV2 oracle apkBytes block true
      = V2VerifyResult.rejectJanusCve_2017_13156 := by
  unfold verifyV2
  simp [Id.run]
  rfl

end Apkaxiom.Signing.V2.Properties
