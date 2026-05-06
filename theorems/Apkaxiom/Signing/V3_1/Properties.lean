/-
P1.11 — Properties of `Apkaxiom.Signing.V3_1`.

v3.1 reuses the v3 result type; the v3.1-specific properties are
the conventional minimum-SDK gate + the v3/v3.1 coexistence
invariant.
-/

import Std
import Apkaxiom.Signing.V3_1

namespace Apkaxiom.Signing.V3_1.Properties

open Apkaxiom.Signing.V3_1
open Apkaxiom.Signing.Block (Block Entry)

/-- The conventional v3.1 minimum-SDK gate is Android 13 = API 33. -/
theorem conventional_min_sdk_is_33 : conventionalMinSdk = 33 := by native_decide

/-! ## Coexistence invariant -/

/-- An empty block has no v3.1 → coexistence trivially holds. -/
theorem coexist_ok_on_empty :
    coexistenceOk { entries := [], blockOffset := 0, blockTotalSize := 0 }
      = true := by native_decide

/-- A block with v2 only → no v3.1 → coexistence holds. -/
theorem coexist_ok_v2_only :
    coexistenceOk { entries := [.v2 ByteArray.empty], blockOffset := 0, blockTotalSize := 0 }
      = true := by native_decide

/-- A block with v3 only → no v3.1 → coexistence holds. -/
theorem coexist_ok_v3_only :
    coexistenceOk { entries := [.v3 ByteArray.empty], blockOffset := 0, blockTotalSize := 0 }
      = true := by native_decide

/-- A block with v3.1 alone (no v3) → coexistence FAILS. -/
theorem coexist_fail_v3_1_alone :
    coexistenceOk { entries := [.v3_1 ByteArray.empty], blockOffset := 0, blockTotalSize := 0 }
      = false := by native_decide

/-- A block with v3 + v3.1 → coexistence holds. -/
theorem coexist_ok_v3_and_v3_1 :
    coexistenceOk
      { entries := [.v3 ByteArray.empty, .v3_1 ByteArray.empty]
      , blockOffset := 0, blockTotalSize := 0 }
      = true := by native_decide

/-- A block with v2 + v3.1 (NO v3) → coexistence FAILS. -/
theorem coexist_fail_v3_1_with_v2_no_v3 :
    coexistenceOk
      { entries := [.v2 ByteArray.empty, .v3_1 ByteArray.empty]
      , blockOffset := 0, blockTotalSize := 0 }
      = false := by native_decide

end Apkaxiom.Signing.V3_1.Properties
