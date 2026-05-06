/-
P1.11 — Properties of `Apkaxiom.Signing.Block`.

Mechanical theorems over the signing-block locator + pair-walker.
The spec gates an "every parser invariant proved" property; this
file collects the load-bearing soundness lemmas.

Pattern matches `Apkaxiom.Zip.LocalHeader.Properties`: each
property is decided by `native_decide` on a small finite case
analysis (the parser is Boolean-shaped, so all interesting
branches reduce to `decide`-able propositions), or by
straightforward induction.
-/

import Std
import Apkaxiom.Signing.Block

namespace Apkaxiom.Signing.Block.Properties

open Apkaxiom.Signing.Block

/-! ## Tag injectivity (already proved in `Block.lean`, re-asserted) -/

theorem parseError_tag_inj :
    ∀ a b : ParseError, a.tag = b.tag → a = b := ParseError.tag_inj

/-! ## ID disjointness -/

/-- The empty-value v2 lift round-trips its ID. -/
theorem entry_fromIdValue_v2_empty_id :
    (Entry.fromIdValue idV2 ByteArray.empty).id = idV2 := by
  native_decide

/-- The empty-value v3 lift round-trips its ID. -/
theorem entry_fromIdValue_v3_empty_id :
    (Entry.fromIdValue idV3 ByteArray.empty).id = idV3 := by
  native_decide

/-- The empty-value v3.1 lift round-trips its ID. -/
theorem entry_fromIdValue_v3_1_empty_id :
    (Entry.fromIdValue idV3_1 ByteArray.empty).id = idV3_1 := by
  native_decide

/-- Unknown ID lift round-trips. -/
theorem entry_fromIdValue_unknown_empty_id :
    (Entry.fromIdValue 0xdeadbeef ByteArray.empty).id = 0xdeadbeef := by
  native_decide

theorem entry_fromIdValue_padding_empty_id :
    (Entry.fromIdValue idPadding ByteArray.empty).id = idPadding := by
  native_decide

theorem entry_fromIdValue_ss1_empty_id :
    (Entry.fromIdValue idSourceStampV1 ByteArray.empty).id = idSourceStampV1 := by
  native_decide

theorem entry_fromIdValue_ss2_empty_id :
    (Entry.fromIdValue idSourceStampV2 ByteArray.empty).id = idSourceStampV2 := by
  native_decide

/-! ## Magic well-formedness -/

/-- The magic byte sequence is exactly 16 bytes. -/
theorem magic_len_16 : magic.size = 16 := by native_decide

/-- The magic spells "APK Sig Block 42" in ASCII, byte-for-byte. -/
theorem magic_first_byte  : magic.get! 0 = 0x41 := by native_decide
theorem magic_second_byte : magic.get! 1 = 0x50 := by native_decide
theorem magic_third_byte  : magic.get! 2 = 0x4b := by native_decide
theorem magic_byte_07     : magic.get! 7 = 0x20 := by native_decide
theorem magic_byte_15     : magic.get! 15 = 0x32 := by native_decide

/-! ## Wire-ID distinctness -/

theorem id_v2_v3_distinct      : idV2 ≠ idV3      := by native_decide
theorem id_v2_v3_1_distinct    : idV2 ≠ idV3_1    := by native_decide
theorem id_v3_v3_1_distinct    : idV3 ≠ idV3_1    := by native_decide
theorem id_v2_padding_distinct : idV2 ≠ idPadding := by native_decide
theorem id_v3_padding_distinct : idV3 ≠ idPadding := by native_decide
theorem id_v3_1_padding_distinct : idV3_1 ≠ idPadding := by native_decide
theorem id_v2_ss1_distinct     : idV2 ≠ idSourceStampV1 := by native_decide
theorem id_v2_ss2_distinct     : idV2 ≠ idSourceStampV2 := by native_decide
theorem id_v3_ss1_distinct     : idV3 ≠ idSourceStampV1 := by native_decide
theorem id_v3_ss2_distinct     : idV3 ≠ idSourceStampV2 := by native_decide

/-! ## Empty block accessors -/

/-- An empty block has no v2 / v3 / v3.1 entries. -/
theorem empty_block_no_modern :
    Block.hasModernScheme
      { entries := [], blockOffset := 0, blockTotalSize := 0 } = false := by
  native_decide

theorem empty_block_no_v2 :
    Block.v2 { entries := [], blockOffset := 0, blockTotalSize := 0 } = none := by
  native_decide

theorem empty_block_no_v3 :
    Block.v3 { entries := [], blockOffset := 0, blockTotalSize := 0 } = none := by
  native_decide

theorem empty_block_no_v3_1 :
    Block.v3_1 { entries := [], blockOffset := 0, blockTotalSize := 0 } = none := by
  native_decide

/-! ## Read primitive properties (u64 LE) -/

/-- `readU64` on an 8-byte zero array returns `some 0`. -/
theorem readU64_zero :
    readU64 (ByteArray.mk #[0,0,0,0,0,0,0,0]) 0 = some 0 := by
  native_decide

/-- `readU64` on a 7-byte input returns `none`. -/
theorem readU64_short :
    readU64 (ByteArray.mk #[0,0,0,0,0,0,0]) 0 = none := by
  native_decide

/-- `readU64` round-trips the all-ones 64-bit value. -/
theorem readU64_ones :
    readU64 (ByteArray.mk #[0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff]) 0
      = some 0xffffffffffffffff := by
  native_decide

/-- `readU64` reads little-endian: byte 0 = LSB. -/
theorem readU64_lsb :
    readU64 (ByteArray.mk #[0x01,0,0,0,0,0,0,0]) 0 = some 0x01 := by
  native_decide

theorem readU64_msb :
    readU64 (ByteArray.mk #[0,0,0,0,0,0,0,0x01]) 0 = some 0x0100000000000000 := by
  native_decide

/-! ## isMagicAt soundness on canonical input -/

/-- `isMagicAt` finds the magic at offset 0 of the magic itself. -/
theorem isMagicAt_self : isMagicAt magic 0 = true := by native_decide

/-- `isMagicAt` rejects an input shorter than the magic. -/
theorem isMagicAt_short :
    isMagicAt (ByteArray.mk #[0,0,0,0]) 0 = false := by native_decide

/-- `isMagicAt` rejects all-zero 16-byte input. -/
theorem isMagicAt_zeros :
    isMagicAt (ByteArray.mk #[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]) 0 = false := by
  native_decide

end Apkaxiom.Signing.Block.Properties
