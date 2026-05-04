/-
P1.5 — LFH parser symbolic properties.

This module is the *theorem catalogue* for `Apkaxiom.Zip.LocalHeader`.
The differential harness at `tools/zip-differential` checks
behaviour on 1800 corpus inputs; the lemmas below establish the
universal contracts that the harness samples.

Naming: every theorem is prefixed by the lowercase Rust function
name on which it is a contract — e.g. `parseLfh_short_input` is a
contract on `parseLfh` for short inputs. This convention lets the
differential harness's failure mode "rust=X lean=Y" be cross-
referenced to the symbolic statement that should hold.
-/

import Apkaxiom.Zip.LocalHeader

namespace Apkaxiom.Zip.LocalHeader

/- ## Constants -/

/-- The fixed-size LFH prefix is exactly 30 bytes. -/
@[simp] theorem fixedSize_eq : fixedSize = 30 := rfl

/-- The LFH signature is the canonical APPNOTE.TXT magic. -/
@[simp] theorem lfhSignature_eq : lfhSignature = 0x04034b50 := rfl

/-- `totalSize` definitionally equals `30 + n + m`. -/
@[simp] theorem totalSize_def (a b : UInt16) :
    totalSize a b = 30 + a.toNat + b.toNat := rfl

/-- `totalSize` is at least the fixed-size prefix. -/
theorem totalSize_ge_fixed (a b : UInt16) :
    totalSize a b ≥ fixedSize := by
  simp [totalSize_def, fixedSize_eq]
  omega

/-- `totalSize` is monotone in the filename length. -/
theorem totalSize_mono_name (a a' b : UInt16)
    (h : a.toNat ≤ a'.toNat) :
    totalSize a b ≤ totalSize a' b := by
  simp [totalSize_def]
  omega

/-- `totalSize` is monotone in the extra-field length. -/
theorem totalSize_mono_extra (a b b' : UInt16)
    (h : b.toNat ≤ b'.toNat) :
    totalSize a b ≤ totalSize a b' := by
  simp [totalSize_def]
  omega

/-- The `totalSize` of a zero-length name + zero-length extra is
exactly the fixed-size header. -/
theorem totalSize_minimal :
    totalSize 0 0 = fixedSize := by
  simp [totalSize_def, fixedSize_eq]

/- ## ParseError invariants -/

/-- `tag` returns a value in [1, 4]. -/
theorem ParseError.tag_in_range (e : ParseError) :
    1 ≤ e.tag.toNat ∧ e.tag.toNat ≤ 4 := by
  cases e <;> simp [ParseError.tag] <;> decide

/-- `tag` is never zero. -/
theorem ParseError.tag_pos (e : ParseError) :
    e.tag ≠ 0 := by
  cases e <;> simp [ParseError.tag]

/-- `shortHeader.tag = 1`. -/
@[simp] theorem ParseError.tag_shortHeader :
    ParseError.shortHeader.tag = 1 := rfl

/-- `badSignature.tag = 2`. -/
@[simp] theorem ParseError.tag_badSignature :
    ParseError.badSignature.tag = 2 := rfl

/-- `shortName.tag = 3`. -/
@[simp] theorem ParseError.tag_shortName :
    ParseError.shortName.tag = 3 := rfl

/-- `shortExtra.tag = 4`. -/
@[simp] theorem ParseError.tag_shortExtra :
    ParseError.shortExtra.tag = 4 := rfl

/-- The four error tags are pairwise distinct (decidable form). -/
theorem ParseError.tags_pairwise_distinct :
    ParseError.shortHeader.tag ≠ ParseError.badSignature.tag ∧
    ParseError.shortHeader.tag ≠ ParseError.shortName.tag ∧
    ParseError.shortHeader.tag ≠ ParseError.shortExtra.tag ∧
    ParseError.badSignature.tag ≠ ParseError.shortName.tag ∧
    ParseError.badSignature.tag ≠ ParseError.shortExtra.tag ∧
    ParseError.shortName.tag ≠ ParseError.shortExtra.tag := by
  decide

/- ## readU16 / readU32 / slice contracts -/

/-- `readU16` returns `none` exactly when the read would run past EOF. -/
theorem readU16_none_iff (bs : ByteArray) (o : Nat) :
    readU16 bs o = none ↔ ¬(o + 1 < bs.size) := by
  unfold readU16
  split <;> simp_all

/-- `readU16` returns `some` exactly when the read fits. -/
theorem readU16_isSome_iff (bs : ByteArray) (o : Nat) :
    (readU16 bs o).isSome ↔ o + 1 < bs.size := by
  unfold readU16
  split <;> simp_all

/-- `readU32` returns `none` exactly when the read would run past EOF. -/
theorem readU32_none_iff (bs : ByteArray) (o : Nat) :
    readU32 bs o = none ↔ ¬(o + 3 < bs.size) := by
  unfold readU32
  split <;> simp_all

/-- `readU32` returns `some` exactly when the read fits. -/
theorem readU32_isSome_iff (bs : ByteArray) (o : Nat) :
    (readU32 bs o).isSome ↔ o + 3 < bs.size := by
  unfold readU32
  split <;> simp_all

/-- `slice` returns `none` exactly when the requested region runs past EOF. -/
theorem slice_none_iff (bs : ByteArray) (o len : Nat) :
    slice bs o len = none ↔ o + len > bs.size := by
  unfold slice
  split <;> simp_all <;> omega

/-- `slice` returns `some` exactly when the region fits. -/
theorem slice_isSome_iff (bs : ByteArray) (o len : Nat) :
    (slice bs o len).isSome ↔ o + len ≤ bs.size := by
  unfold slice
  split <;> simp_all

/-- When the requested region fits, `slice` returns a `ByteArray`
whose contents come from `extract`. Useful as a rewriting lemma. -/
theorem slice_eq_extract (bs : ByteArray) (o len : Nat)
    (h : o + len ≤ bs.size) :
    slice bs o len = some (bs.extract o (o + len)) := by
  unfold slice
  simp [h]

/- ## parseLfh structural lemmas -/

/- The lemmas below use `parseError` (the `Option ParseError`
projection on `parseLfh`) because `Lfh` deliberately omits
`DecidableEq` — `ByteArray` is not `DecidableEq`-friendly inside
the elaborator. The harness compares ok/error verdicts via
integer tags, so the projection captures everything we need to
check at the type level. -/

/-- A 0-byte input fails with `shortHeader`. -/
theorem parseLfh_zero_bytes :
    parseError (ByteArray.mk #[]) = some ParseError.shortHeader := by
  native_decide

/-- A 1-byte input fails with `shortHeader`. -/
theorem parseLfh_one_byte :
    parseError (ByteArray.mk #[0x00]) = some ParseError.shortHeader := by
  native_decide

/-- A 29-byte input (one short of the fixed prefix) fails with
`shortHeader`. -/
theorem parseLfh_twenty_nine_bytes :
    parseError (ByteArray.mk (.mk (List.replicate 29 (0 : UInt8)))) =
      some ParseError.shortHeader := by
  native_decide

/-- An exactly-30-byte input with a wrong signature fails with
`badSignature`. -/
theorem parseLfh_thirty_bytes_bad_magic :
    parseError (ByteArray.mk (.mk (List.replicate 30 (0 : UInt8)))) =
      some ParseError.badSignature := by
  native_decide

/-- The minimal valid LFH (zero name, zero extra) parses successfully. -/
theorem parseLfh_minimal_succeeds :
    parseError minimalLfhBytes = none := by
  native_decide

/- ## Cross-parser tag agreement (interop with Rust) -/

/-- The four ParseError tags fit in a `UInt8` and are exactly the
small naturals 1–4. The differential harness in
`tools/zip-differential` uses this fact to compare parse-error
categories across Lean and Rust as integer values. -/
theorem ParseError.tag_codomain (e : ParseError) :
    e.tag = 1 ∨ e.tag = 2 ∨ e.tag = 3 ∨ e.tag = 4 := by
  cases e <;> simp [ParseError.tag]

/-- The tag map is a bijection onto {1, 2, 3, 4}. Combined with
`tag_injective` (in the parent module), this gives us a
canonical 4-element interop alphabet. -/
theorem ParseError.tag_surjective_on_image
    (n : UInt8) (h : n = 1 ∨ n = 2 ∨ n = 3 ∨ n = 4) :
    ∃ e : ParseError, e.tag = n := by
  rcases h with h|h|h|h
  · exact ⟨.shortHeader, by simp [ParseError.tag, h]⟩
  · exact ⟨.badSignature, by simp [ParseError.tag, h]⟩
  · exact ⟨.shortName,    by simp [ParseError.tag, h]⟩
  · exact ⟨.shortExtra,   by simp [ParseError.tag, h]⟩

end Apkaxiom.Zip.LocalHeader
