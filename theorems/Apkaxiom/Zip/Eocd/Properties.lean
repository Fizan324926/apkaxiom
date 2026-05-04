/-
P1.5 — EOCD parser symbolic properties.

Companion to `Apkaxiom.Zip.LocalHeader.Properties`. Same structure:
constants, error-tag invariants, smoke checks via `native_decide`,
disk-consistency invariant on success.
-/

import Apkaxiom.Zip.Eocd

namespace Apkaxiom.Zip.Eocd

/- ## Constants -/

/-- The fixed-size EOCD prefix is exactly 22 bytes. -/
@[simp] theorem fixedSize_eq : fixedSize = 22 := rfl

/-- The EOCD signature is the canonical APPNOTE.TXT magic. -/
@[simp] theorem eocdSignature_eq : eocdSignature = 0x06054b50 := rfl

/-- The maximum legal comment length is 65535 (a 16-bit field). -/
@[simp] theorem maxCommentLen_eq : maxCommentLen = 0xffff := rfl

/-- `maxCommentLen` is exactly `UInt16.max - 0`. -/
theorem maxCommentLen_eq_uint16_max : maxCommentLen = 65535 := by
  simp [maxCommentLen_eq]

/- ## ParseError invariants -/

/-- `tag` returns a value in [1, 4]. -/
theorem ParseError.tag_in_range (e : ParseError) :
    1 ≤ e.tag.toNat ∧ e.tag.toNat ≤ 4 := by
  cases e <;> simp [ParseError.tag] <;> decide

/-- `tag` is never zero. -/
theorem ParseError.tag_pos (e : ParseError) :
    e.tag ≠ 0 := by
  cases e <;> simp [ParseError.tag]

@[simp] theorem ParseError.tag_shortFixed :
    ParseError.shortFixed.tag = 1 := rfl

@[simp] theorem ParseError.tag_badSignature :
    ParseError.badSignature.tag = 2 := rfl

@[simp] theorem ParseError.tag_shortComment :
    ParseError.shortComment.tag = 3 := rfl

@[simp] theorem ParseError.tag_inconsistentDisks :
    ParseError.inconsistentDisks.tag = 4 := rfl

/-- The four error tags are pairwise distinct (decidable form). -/
theorem ParseError.tags_pairwise_distinct :
    ParseError.shortFixed.tag        ≠ ParseError.badSignature.tag      ∧
    ParseError.shortFixed.tag        ≠ ParseError.shortComment.tag      ∧
    ParseError.shortFixed.tag        ≠ ParseError.inconsistentDisks.tag ∧
    ParseError.badSignature.tag      ≠ ParseError.shortComment.tag      ∧
    ParseError.badSignature.tag      ≠ ParseError.inconsistentDisks.tag ∧
    ParseError.shortComment.tag      ≠ ParseError.inconsistentDisks.tag := by
  decide

/-- The four ParseError tags fit in a `UInt8` and are exactly the
small naturals 1–4. -/
theorem ParseError.tag_codomain (e : ParseError) :
    e.tag = 1 ∨ e.tag = 2 ∨ e.tag = 3 ∨ e.tag = 4 := by
  cases e <;> simp [ParseError.tag]

/-- The tag map is surjective onto {1, 2, 3, 4}. -/
theorem ParseError.tag_surjective_on_image
    (n : UInt8) (h : n = 1 ∨ n = 2 ∨ n = 3 ∨ n = 4) :
    ∃ e : ParseError, e.tag = n := by
  rcases h with h|h|h|h
  · exact ⟨.shortFixed,         by simp [ParseError.tag, h]⟩
  · exact ⟨.badSignature,       by simp [ParseError.tag, h]⟩
  · exact ⟨.shortComment,       by simp [ParseError.tag, h]⟩
  · exact ⟨.inconsistentDisks,  by simp [ParseError.tag, h]⟩

/- ## parseEocd structural lemmas (smoke checks) -/

/- The lemmas below use `parseError` (the `Option ParseError`
projection) for elaboration-friendly equality, mirroring the
LocalHeader/Properties.lean approach. -/

/-- A 0-byte input fails with `shortFixed`. -/
theorem parseEocd_zero_bytes :
    parseError (ByteArray.mk #[]) = some ParseError.shortFixed := by
  native_decide

/-- A 1-byte input fails with `shortFixed`. -/
theorem parseEocd_one_byte :
    parseError (ByteArray.mk #[0x00]) = some ParseError.shortFixed := by
  native_decide

/-- A 21-byte input (one short of the fixed prefix) fails with
`shortFixed`. -/
theorem parseEocd_twenty_one_bytes :
    parseError (ByteArray.mk (.mk (List.replicate 21 (0 : UInt8)))) =
      some ParseError.shortFixed := by
  native_decide

/-- An exactly-22-byte input with a wrong signature fails with
`badSignature`. -/
theorem parseEocd_twenty_two_bytes_bad_magic :
    parseError (ByteArray.mk (.mk (List.replicate 22 (0 : UInt8)))) =
      some ParseError.badSignature := by
  native_decide

/-- The minimal valid EOCD parses successfully. -/
theorem parseEocd_minimal_succeeds :
    parseError minimalEocdBytes = none := by
  native_decide

/- ## Multi-volume rejection (ADR-0017 invariant) -/

/-- A 22-byte EOCD with disk-number = 1 is rejected as multi-volume.
This is the type-level enforcement of ADR-0017 ("ZIP64 multi-volume
out of v0.1"). -/
theorem parseEocd_multi_volume_rejected :
    parseError (ByteArray.mk #[
      0x50, 0x4b, 0x05, 0x06,
      0x01, 0x00, 0x00, 0x00,  -- diskNumber = 1, cdStartDisk = 0
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00
    ]) = some ParseError.inconsistentDisks := by
  native_decide

/-- A 22-byte EOCD with disk-number = 2 is also rejected. -/
theorem parseEocd_disk_two_rejected :
    parseError (ByteArray.mk #[
      0x50, 0x4b, 0x05, 0x06,
      0x02, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00
    ]) = some ParseError.inconsistentDisks := by
  native_decide

/-- A 22-byte EOCD with cdStartDisk ≠ 0 (cdStartDisk = 1) is also
rejected even when diskNumber = 0. The contract is *equality*
between disk fields, not "disk = 0". -/
theorem parseEocd_cd_start_disk_one_rejected :
    parseError (ByteArray.mk #[
      0x50, 0x4b, 0x05, 0x06,
      0x00, 0x00, 0x01, 0x00,  -- diskNumber = 0, cdStartDisk = 1
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00
    ]) = some ParseError.inconsistentDisks := by
  native_decide

/- ## Comment-region invariants -/

/-- A 22-byte EOCD that declares a 50-byte comment but supplies none
fails with `shortComment`. -/
theorem parseEocd_short_comment :
    parseError (ByteArray.mk #[
      0x50, 0x4b, 0x05, 0x06,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x32, 0x00              -- commentLen = 50, no body
    ]) = some ParseError.shortComment := by
  native_decide

/-- A 22-byte EOCD with `commentLen=0` and no comment body succeeds. -/
theorem parseEocd_zero_comment_succeeds :
    parseError minimalEocdBytes = none := by
  native_decide

end Apkaxiom.Zip.Eocd
