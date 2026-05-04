/-
P1.6 — CDR parser symbolic properties.

Theorem catalogue for `Apkaxiom.Zip.CentralDirectory`. The differential
harness at `tools/zip-differential` checks behaviour on the corpus
inputs; the lemmas below establish the universal contracts that the
harness samples.

Naming convention: every theorem on `parseCdr` is prefixed
`parseCdr_*`; every theorem on `ParseError` lives in the
`ParseError` namespace. This convention lets the differential
harness's "rust=X lean=Y aosp=Z" failure mode be cross-referenced to
the symbolic statement that should hold.
-/

import Apkaxiom.Zip.CentralDirectory
import Apkaxiom.Zip.LocalHeader.Properties

namespace Apkaxiom.Zip.CentralDirectory

/- ## Constants -/

/-- The fixed-size CDR prefix is exactly 46 bytes. -/
@[simp] theorem fixedSize_eq : fixedSize = 46 := rfl

/-- The CDR signature is the canonical APPNOTE.TXT magic. -/
@[simp] theorem cdrSignature_eq : cdrSignature = 0x02014b50 := rfl

/-- All three variable-length-region budgets fit in a 16-bit field. -/
@[simp] theorem maxNameLen_eq : maxNameLen = 0xffff := rfl
@[simp] theorem maxExtraLen_eq : maxExtraLen = 0xffff := rfl
@[simp] theorem maxCommentLen_eq : maxCommentLen = 0xffff := rfl

/-- `totalSize` definitionally equals `46 + n + m + k`. -/
@[simp] theorem totalSize_def (n m k : UInt16) :
    totalSize n m k = 46 + n.toNat + m.toNat + k.toNat := rfl

/-- `totalSize` is at least the fixed-size prefix. -/
theorem totalSize_ge_fixed (n m k : UInt16) :
    totalSize n m k ≥ fixedSize := by
  simp [totalSize_def, fixedSize_eq]
  omega

/-- `totalSize` is monotone in the filename length. -/
theorem totalSize_mono_name (n n' m k : UInt16)
    (h : n.toNat ≤ n'.toNat) :
    totalSize n m k ≤ totalSize n' m k := by
  simp [totalSize_def]
  omega

/-- `totalSize` is monotone in the extra-field length. -/
theorem totalSize_mono_extra (n m m' k : UInt16)
    (h : m.toNat ≤ m'.toNat) :
    totalSize n m k ≤ totalSize n m' k := by
  simp [totalSize_def]
  omega

/-- `totalSize` is monotone in the file-comment length. -/
theorem totalSize_mono_comment (n m k k' : UInt16)
    (h : k.toNat ≤ k'.toNat) :
    totalSize n m k ≤ totalSize n m k' := by
  simp [totalSize_def]
  omega

/-- The `totalSize` of a zero-everything record is exactly the fixed
prefix. -/
theorem totalSize_minimal :
    totalSize 0 0 0 = fixedSize := by
  simp [totalSize_def, fixedSize_eq]

/-- `totalSize` is bounded above by `46 + 3·0xffff`. -/
theorem totalSize_upper_bound (n m k : UInt16) :
    totalSize n m k ≤ 46 + 3 * 0xffff := by
  simp [totalSize_def]
  have h1 : n.toNat < 65536 := n.toNat_lt
  have h2 : m.toNat < 65536 := m.toNat_lt
  have h3 : k.toNat < 65536 := k.toNat_lt
  omega

/- ## ParseError invariants -/

/-- `tag` returns a value in [1, 5]. -/
theorem ParseError.tag_in_range (e : ParseError) :
    1 ≤ e.tag.toNat ∧ e.tag.toNat ≤ 5 := by
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

/-- `shortComment.tag = 5`. -/
@[simp] theorem ParseError.tag_shortComment :
    ParseError.shortComment.tag = 5 := rfl

/-- The five error tags are pairwise distinct (decidable form). -/
theorem ParseError.tags_pairwise_distinct :
    ParseError.shortHeader.tag ≠ ParseError.badSignature.tag ∧
    ParseError.shortHeader.tag ≠ ParseError.shortName.tag ∧
    ParseError.shortHeader.tag ≠ ParseError.shortExtra.tag ∧
    ParseError.shortHeader.tag ≠ ParseError.shortComment.tag ∧
    ParseError.badSignature.tag ≠ ParseError.shortName.tag ∧
    ParseError.badSignature.tag ≠ ParseError.shortExtra.tag ∧
    ParseError.badSignature.tag ≠ ParseError.shortComment.tag ∧
    ParseError.shortName.tag ≠ ParseError.shortExtra.tag ∧
    ParseError.shortName.tag ≠ ParseError.shortComment.tag ∧
    ParseError.shortExtra.tag ≠ ParseError.shortComment.tag := by
  decide

/-- The five ParseError tags fit in a `UInt8` and are exactly the small
naturals 1–5. The differential harness uses this fact to compare
parse-error categories across Lean / Rust / AOSP as integer values. -/
theorem ParseError.tag_codomain (e : ParseError) :
    e.tag = 1 ∨ e.tag = 2 ∨ e.tag = 3 ∨ e.tag = 4 ∨ e.tag = 5 := by
  cases e <;> simp [ParseError.tag]

/-- The tag map is surjective onto its image `{1, 2, 3, 4, 5}`.
Combined with `tag_injective` (in the parent module), this gives a
canonical 5-element interop alphabet. -/
theorem ParseError.tag_surjective_on_image
    (n : UInt8) (h : n = 1 ∨ n = 2 ∨ n = 3 ∨ n = 4 ∨ n = 5) :
    ∃ e : ParseError, e.tag = n := by
  rcases h with h|h|h|h|h
  · exact ⟨.shortHeader,  by simp [ParseError.tag, h]⟩
  · exact ⟨.badSignature, by simp [ParseError.tag, h]⟩
  · exact ⟨.shortName,    by simp [ParseError.tag, h]⟩
  · exact ⟨.shortExtra,   by simp [ParseError.tag, h]⟩
  · exact ⟨.shortComment, by simp [ParseError.tag, h]⟩

/- ## parseCdr structural lemmas -/

/-- A 0-byte input fails with `shortHeader`. -/
theorem parseCdr_zero_bytes :
    parseError (ByteArray.mk #[]) = some ParseError.shortHeader := by
  native_decide

/-- A 1-byte input fails with `shortHeader`. -/
theorem parseCdr_one_byte :
    parseError (ByteArray.mk #[0x00]) = some ParseError.shortHeader := by
  native_decide

/-- A 45-byte input (one short of the fixed prefix) fails with
`shortHeader`. -/
theorem parseCdr_forty_five_bytes :
    parseError (ByteArray.mk (.mk (List.replicate 45 (0 : UInt8)))) =
      some ParseError.shortHeader := by
  native_decide

/-- An exactly-46-byte input with a wrong signature fails with
`badSignature`. -/
theorem parseCdr_forty_six_bytes_bad_magic :
    parseError (ByteArray.mk (.mk (List.replicate 46 (0 : UInt8)))) =
      some ParseError.badSignature := by
  native_decide

/-- The minimal valid CDR (zero everything) parses successfully. -/
theorem parseCdr_minimal_succeeds :
    parseError minimalCdrBytes = none := by
  native_decide

/-- Every byte stream shorter than the CDR fixed prefix is rejected
with `shortHeader`. The fact holds at the *contract* level — the
first thing the parser checks is the length, which leaks no
information about subsequent bytes. -/
theorem parseCdr_short_input_uniform :
    ∀ bs : ByteArray, bs.size < 46 →
      parseError bs = some ParseError.shortHeader := by
  intro bs hsz
  unfold parseError
  unfold parseCdr
  simp [fixedSize_eq, hsz]

/- ## Cross-record disjointness with LFH and EOCD -/

/-- The CDR signature is *not* the LFH signature. -/
theorem cdr_lfh_signature_distinct :
    cdrSignature ≠ Apkaxiom.Zip.LocalHeader.lfhSignature := by
  decide

/-- The CDR signature is *not* the EOCD signature. -/
theorem cdr_eocd_signature_distinct :
    cdrSignature ≠ 0x06054b50 := by
  decide

/-- The CDR fixed prefix (46 bytes) is strictly larger than both the
LFH (30) and the EOCD (22) fixed prefixes. -/
theorem cdr_fixed_size_max :
    fixedSize > Apkaxiom.Zip.LocalHeader.fixedSize ∧
    fixedSize > 22 := by
  refine ⟨?_, ?_⟩
  · simp [fixedSize_eq, Apkaxiom.Zip.LocalHeader.fixedSize_eq]
  · decide

/-- A CDR-shaped byte sequence is rejected by the LFH parser with
`badSignature` once it has the LFH-fixed-prefix length. The minimal
CDR is 46 bytes (≥ 30 = LFH fixed size), so the LFH parser will see
a long-enough input with a wrong magic and reject with
`badSignature`. -/
theorem cdr_bytes_rejected_as_lfh :
    Apkaxiom.Zip.LocalHeader.parseError minimalCdrBytes
      = some Apkaxiom.Zip.LocalHeader.ParseError.badSignature := by
  native_decide

/- ## Boundary inputs - explicit witnesses -/

/-- A 46-byte input with the right signature but `nameLen = 1`,
`extraLen = 0`, `commentLen = 0` and no trailing payload fails
with `shortName`. -/
theorem parseCdr_short_name_witness :
    parseError (ByteArray.mk #[
      0x50, 0x4b, 0x01, 0x02,
      0x14, 0x00, 0x14, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00
    ]) = some ParseError.shortName := by
  native_decide

/-- A 46-byte input with `extraLen = 1` and no trailing payload fails
with `shortExtra`. -/
theorem parseCdr_short_extra_witness :
    parseError (ByteArray.mk #[
      0x50, 0x4b, 0x01, 0x02,
      0x14, 0x00, 0x14, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00
    ]) = some ParseError.shortExtra := by
  native_decide

/-- A 46-byte input with `commentLen = 1` and no trailing payload fails
with `shortComment`. -/
theorem parseCdr_short_comment_witness :
    parseError (ByteArray.mk #[
      0x50, 0x4b, 0x01, 0x02,
      0x14, 0x00, 0x14, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x01, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00
    ]) = some ParseError.shortComment := by
  native_decide

/- ## Region-ordering invariants -/

/-- The three variable-length regions appear in the order
filename → extra → comment in the byte stream. The parser checks
them in that order, so a mid-walk failure has a deterministic
priority: short-name is reported before short-extra is reported
before short-comment, even if the same input would also fail one
of the later checks. The witness below: a CDR claiming
`nameLen=1, extraLen=1, commentLen=1` over a 46-byte input fails
with `shortName` (the *first* check) — not with `shortExtra` or
`shortComment`, even though the latter would also flunk a
sufficiently long input. -/
theorem parseCdr_region_priority_name_first :
    parseError (ByteArray.mk #[
      0x50, 0x4b, 0x01, 0x02,
      0x14, 0x00, 0x14, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      -- nameLen=1, extraLen=1, commentLen=1 — all three would flunk,
      -- but `shortName` is reported first.
      0x01, 0x00, 0x01, 0x00, 0x01, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00
    ]) = some ParseError.shortName := by
  native_decide

/-- Same shape: with `nameLen=0, extraLen=1, commentLen=1`, the
priority order routes to `shortExtra`. -/
theorem parseCdr_region_priority_extra_before_comment :
    parseError (ByteArray.mk #[
      0x50, 0x4b, 0x01, 0x02,
      0x14, 0x00, 0x14, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      -- nameLen=0, extraLen=1, commentLen=1 — both would flunk,
      -- but `shortExtra` is reported first.
      0x00, 0x00, 0x01, 0x00, 0x01, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00
    ]) = some ParseError.shortExtra := by
  native_decide

end Apkaxiom.Zip.CentralDirectory
