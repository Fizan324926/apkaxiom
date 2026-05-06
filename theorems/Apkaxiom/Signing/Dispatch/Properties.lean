/-
P1.11 — Properties of `Apkaxiom.Signing.Dispatch`.

Mechanical theorems over the cross-scheme dispatcher's accept-set,
SchemeVariant disjointness, and Janus-rejection invariant.
-/

import Std
import Apkaxiom.Signing.Dispatch

namespace Apkaxiom.Signing.Dispatch.Properties

open Apkaxiom.Signing.Dispatch
open Apkaxiom.Signing.Block (Block Entry)

/-! ## SchemeVariant disjointness -/

theorem variant_v3_1_distinct_v3 : SchemeVariant.v3_1 ≠ SchemeVariant.v3 := by decide
theorem variant_v3_distinct_v2   : SchemeVariant.v3   ≠ SchemeVariant.v2 := by decide
theorem variant_v2_distinct_v1   : SchemeVariant.v2   ≠ SchemeVariant.v1 := by decide
theorem variant_v1_distinct_none : SchemeVariant.v1   ≠ SchemeVariant.none := by decide
theorem variant_v3_1_distinct_v2   : SchemeVariant.v3_1 ≠ SchemeVariant.v2   := by decide
theorem variant_v3_1_distinct_v1   : SchemeVariant.v3_1 ≠ SchemeVariant.v1   := by decide
theorem variant_v3_1_distinct_none : SchemeVariant.v3_1 ≠ SchemeVariant.none := by decide
theorem variant_v3_distinct_v1     : SchemeVariant.v3   ≠ SchemeVariant.v1   := by decide
theorem variant_v3_distinct_none   : SchemeVariant.v3   ≠ SchemeVariant.none := by decide
theorem variant_v2_distinct_none   : SchemeVariant.v2   ≠ SchemeVariant.none := by decide

/-! ## strongestPresent precedence -/

theorem strongest_v3_1_when_v3_1_present :
    strongestPresent
      { entries := [.v2 ByteArray.empty, .v3 ByteArray.empty, .v3_1 ByteArray.empty]
      , blockOffset := 0, blockTotalSize := 0 } true
      = SchemeVariant.v3_1 := by native_decide

theorem strongest_v3_when_no_v3_1 :
    strongestPresent
      { entries := [.v2 ByteArray.empty, .v3 ByteArray.empty]
      , blockOffset := 0, blockTotalSize := 0 } true
      = SchemeVariant.v3 := by native_decide

theorem strongest_v2_when_only_v2 :
    strongestPresent
      { entries := [.v2 ByteArray.empty]
      , blockOffset := 0, blockTotalSize := 0 } true
      = SchemeVariant.v2 := by native_decide

theorem strongest_v1_when_v1_only :
    strongestPresent
      { entries := []
      , blockOffset := 0, blockTotalSize := 0 } true
      = SchemeVariant.v1 := by native_decide

theorem strongest_none_when_unsigned :
    strongestPresent
      { entries := []
      , blockOffset := 0, blockTotalSize := 0 } false
      = SchemeVariant.none := by native_decide

/-! ## Decision shape -/

theorem accept_isAccept_true (v : SchemeVariant) :
    (Decision.accept v).isAccept = true := by rfl

theorem reject_unsigned_not_accept :
    Decision.rejectUnsigned.isAccept = false := by rfl

theorem reject_downgrade_not_accept :
    Decision.rejectDowngradeAttempt.isAccept = false := by rfl

theorem reject_janus_not_accept :
    Decision.rejectJanusCve_2017_13156.isAccept = false := by rfl

theorem accepted_variant_returns_some :
    (Decision.accept SchemeVariant.v3_1).acceptedVariant = some SchemeVariant.v3_1 := by rfl

theorem rejected_variant_returns_none :
    Decision.rejectUnsigned.acceptedVariant = none := by rfl

end Apkaxiom.Signing.Dispatch.Properties
