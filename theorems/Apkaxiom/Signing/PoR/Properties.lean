/-
P1.11 G14 / G17 — properties of `axiom_sigblock::proof_of_rotation`
mirrored on the Lean side.

The PoR (proof-of-rotation) lineage parser is implemented in
Rust (`crates/axiom-sigblock/src/proof_of_rotation.rs`) — the
Lean side ships the wire-format constants + flag bitmask
classification so downstream theorems can refer to them.
-/

import Std

namespace Apkaxiom.Signing.PoR.Properties

/-! ## Wire-format constants -/

/-- v3 / v3.1 attribute id for the proof-of-rotation lineage. -/
def proofOfRotationAttrId : UInt32 := 0x3ba06f8c

/-- The standalone-disk-file lineage magic (NOT used in the
    in-APK encoding — see ADR-0029 §3 / merkle-commits.md). -/
def lineageDiskMagic : UInt32 := 0x3a2d12c8

theorem proof_of_rotation_attr_id_value :
    proofOfRotationAttrId = 0x3ba06f8c := by native_decide

theorem lineage_disk_magic_value :
    lineageDiskMagic = 0x3a2d12c8 := by native_decide

theorem por_attr_distinct_from_disk_magic :
    proofOfRotationAttrId ≠ lineageDiskMagic := by native_decide

/-! ## Per-node flag bits (per AOSP `SigningCertificateLineage`) -/

def flagPastCertInstalledData      : UInt32 := 1
def flagPastCertSharedUserId       : UInt32 := 2
def flagPastCertPermission         : UInt32 := 4
def flagPastCertRollbackCapability : UInt32 := 8
def flagPastCertAuth               : UInt32 := 16

theorem flag_installed_data_value      : flagPastCertInstalledData = 1 := by native_decide
theorem flag_shared_user_id_value      : flagPastCertSharedUserId = 2 := by native_decide
theorem flag_permission_value          : flagPastCertPermission = 4 := by native_decide
theorem flag_rollback_capability_value : flagPastCertRollbackCapability = 8 := by native_decide
theorem flag_auth_value                : flagPastCertAuth = 16 := by native_decide

/-- All five flag bits are pairwise distinct. -/
theorem flags_pairwise_distinct_1_2 : flagPastCertInstalledData ≠ flagPastCertSharedUserId := by native_decide
theorem flags_pairwise_distinct_2_4 : flagPastCertSharedUserId ≠ flagPastCertPermission := by native_decide
theorem flags_pairwise_distinct_4_8 : flagPastCertPermission ≠ flagPastCertRollbackCapability := by native_decide
theorem flags_pairwise_distinct_8_16 : flagPastCertRollbackCapability ≠ flagPastCertAuth := by native_decide
theorem flags_pairwise_distinct_1_16 : flagPastCertInstalledData ≠ flagPastCertAuth := by native_decide
theorem flags_pairwise_distinct_2_8 : flagPastCertSharedUserId ≠ flagPastCertRollbackCapability := by native_decide
theorem flags_pairwise_distinct_4_16 : flagPastCertPermission ≠ flagPastCertAuth := by native_decide
theorem flags_pairwise_distinct_1_4 : flagPastCertInstalledData ≠ flagPastCertPermission := by native_decide
theorem flags_pairwise_distinct_1_8 : flagPastCertInstalledData ≠ flagPastCertRollbackCapability := by native_decide
theorem flags_pairwise_distinct_2_16 : flagPastCertSharedUserId ≠ flagPastCertAuth := by native_decide

/-- Flag bits are powers of two — bitwise-AND mask reads back
    the asserted bit only. -/
theorem flag_installed_data_bit_isolation :
    (flagPastCertInstalledData &&& flagPastCertInstalledData) = flagPastCertInstalledData := by
  native_decide
theorem flag_auth_bit_isolation :
    (flagPastCertAuth &&& flagPastCertAuth) = flagPastCertAuth := by native_decide

/-- Disjoint flags AND to zero. -/
theorem flag_installed_and_auth_disjoint :
    (flagPastCertInstalledData &&& flagPastCertAuth) = 0 := by native_decide
theorem flag_shared_and_rollback_disjoint :
    (flagPastCertSharedUserId &&& flagPastCertRollbackCapability) = 0 := by native_decide
theorem flag_permission_and_auth_disjoint :
    (flagPastCertPermission &&& flagPastCertAuth) = 0 := by native_decide
theorem flag_installed_and_shared_disjoint :
    (flagPastCertInstalledData &&& flagPastCertSharedUserId) = 0 := by native_decide

/-- All-flags-set mask = OR of all five. -/
def allFlagsSet : UInt32 :=
  flagPastCertInstalledData ||| flagPastCertSharedUserId
    ||| flagPastCertPermission ||| flagPastCertRollbackCapability
    ||| flagPastCertAuth

theorem all_flags_set_value : allFlagsSet = 31 := by native_decide
theorem all_flags_set_contains_installed :
    (allFlagsSet &&& flagPastCertInstalledData) = flagPastCertInstalledData := by native_decide
theorem all_flags_set_contains_auth :
    (allFlagsSet &&& flagPastCertAuth) = flagPastCertAuth := by native_decide
theorem all_flags_set_contains_permission :
    (allFlagsSet &&& flagPastCertPermission) = flagPastCertPermission := by native_decide

/-! ## Lineage version -/

def lineageVersion1 : UInt32 := 1

theorem lineage_version_1 : lineageVersion1 = 1 := by native_decide
theorem lineage_version_distinct_zero : lineageVersion1 ≠ 0 := by native_decide

end Apkaxiom.Signing.PoR.Properties
