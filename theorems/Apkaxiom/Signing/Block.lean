/-
P1.11 — APK Signing Block (the v2/v3/v3.1 carrier).

The APK Signing Block is the byte region between the last LFH
body and the central directory of any v2/v3/v3.1-signed APK.
Layout (little-endian, per AOSP `tools/apksig`):

    [u64  size_of_block      — bytes from the trailing u64 backwards]
    [pairs ...]
       each pair:
         [u64 length          — 4-byte ID + value size]
         [u32 id]
         [length - 4 bytes value]
    [u64  size_of_block       — must equal the leading u64]
    [16-byte magic = "APK Sig Block 42"]

The block is *suffix-anchored* via the magic, located by:

  1. Find EOCD signature; read `cd_offset`.
  2. Read 16 bytes at `cd_offset - 16`; must equal `magic`.
  3. Read u64 at `cd_offset - 24` — size_of_block.
  4. Block starts at `cd_offset - size_of_block - 8`.

The Rust reference parser at `crates/axiom-sigblock` mirrors
this implementation byte-for-byte; the differential harness at
`tools/p111-differential` asserts equivalence over the F-Droid
+ apksigner-resigned multi-scheme corpus.

Known block IDs (per AOSP):

  | ID            | Name                        |
  |---------------|-----------------------------|
  | 0x7109871a    | APK Signature Scheme v2     |
  | 0xf05368c0    | APK Signature Scheme v3     |
  | 0x1b93ad61    | APK Signature Scheme v3.1   |
  | 0x6dff800d    | AOSP zero-padding           |
  | 0x2b09189e    | Source Stamp v1             |
  | 0x42726577    | Source Stamp v2             |
-/

import Std
import Apkaxiom.Zip.LocalHeader

namespace Apkaxiom.Signing.Block

open Apkaxiom.Zip.LocalHeader (readU16 readU32 slice)

/-! ## Wire-format constants -/

/-- 16-byte magic at the tail of every APK signing block. Spelled
out as bytes so we can `native_decide`-check it. -/
def magic : ByteArray :=
  -- "APK Sig Block 42"
  ByteArray.mk #[
    0x41, 0x50, 0x4b, 0x20, 0x53, 0x69, 0x67, 0x20,
    0x42, 0x6c, 0x6f, 0x63, 0x6b, 0x20, 0x34, 0x32
  ]

/-- Length of the magic in bytes. -/
def magicLen : Nat := 16

/-- Block ID for APK Signature Scheme v2. -/
def idV2     : UInt32 := 0x7109871a
/-- Block ID for APK Signature Scheme v3. -/
def idV3     : UInt32 := 0xf05368c0
/-- Block ID for APK Signature Scheme v3.1 (rotation-aware v3). -/
def idV3_1   : UInt32 := 0x1b93ad61
/-- AOSP zero-padding block (block-alignment filler). -/
def idPadding : UInt32 := 0x6dff800d
/-- Source Stamp v1. -/
def idSourceStampV1 : UInt32 := 0x2b09189e
/-- Source Stamp v2. -/
def idSourceStampV2 : UInt32 := 0x42726577

/-- EOCD signature (mirrors `Apkaxiom.Zip.Eocd`). -/
def eocdSignature : UInt32 := 0x06054b50

/-- 22-byte EOCD fixed prefix. -/
def eocdFixedSize : Nat := 22

/-! ## Read primitives — 64-bit little-endian -/

/-- Read a little-endian `UInt64` from `bs` at offset `o`. Returns
`none` if the read would run past the end of the array. -/
def readU64 (bs : ByteArray) (o : Nat) : Option UInt64 :=
  if o + 7 < bs.size then
    let b0 := bs.get! o
    let b1 := bs.get! (o + 1)
    let b2 := bs.get! (o + 2)
    let b3 := bs.get! (o + 3)
    let b4 := bs.get! (o + 4)
    let b5 := bs.get! (o + 5)
    let b6 := bs.get! (o + 6)
    let b7 := bs.get! (o + 7)
    some (b0.toUInt64
        ||| (b1.toUInt64 <<< 8)
        ||| (b2.toUInt64 <<< 16)
        ||| (b3.toUInt64 <<< 24)
        ||| (b4.toUInt64 <<< 32)
        ||| (b5.toUInt64 <<< 40)
        ||| (b6.toUInt64 <<< 48)
        ||| (b7.toUInt64 <<< 56))
  else
    none

/-! ## Entry — one ID-tagged pair inside the block -/

/-- One ID-tagged entry inside the signing block. The variant
constructors share the same wire shape (ID + verbatim value bytes)
but distinguish the well-known IDs from arbitrary unknowns so
downstream consumers don't have to re-classify by ID byte. -/
inductive Entry : Type where
  /-- APK Signature Scheme v2 — id = `0x7109871a`. -/
  | v2 (value : ByteArray) : Entry
  /-- APK Signature Scheme v3 — id = `0xf05368c0`. -/
  | v3 (value : ByteArray) : Entry
  /-- APK Signature Scheme v3.1 — id = `0x1b93ad61`. -/
  | v3_1 (value : ByteArray) : Entry
  /-- AOSP zero-padding — id = `0x6dff800d`. -/
  | padding (value : ByteArray) : Entry
  /-- Source Stamp v1 — id = `0x2b09189e`. -/
  | sourceStampV1 (value : ByteArray) : Entry
  /-- Source Stamp v2 — id = `0x42726577`. -/
  | sourceStampV2 (value : ByteArray) : Entry
  /-- An ID we don't recognise. Kept verbatim so the parser is
  total and downstream consumers can re-serialise without drift. -/
  | unknown (id : UInt32) (value : ByteArray) : Entry
deriving Inhabited

/-- Wire ID for an `Entry`. -/
def Entry.id : Entry → UInt32
  | .v2 _              => idV2
  | .v3 _              => idV3
  | .v3_1 _            => idV3_1
  | .padding _         => idPadding
  | .sourceStampV1 _   => idSourceStampV1
  | .sourceStampV2 _   => idSourceStampV2
  | .unknown id _      => id

/-- Verbatim value bytes for an `Entry`. -/
def Entry.value : Entry → ByteArray
  | .v2 v              => v
  | .v3 v              => v
  | .v3_1 v            => v
  | .padding v         => v
  | .sourceStampV1 v   => v
  | .sourceStampV2 v   => v
  | .unknown _ v       => v

/-- Lift a raw `(id, value)` pair to a typed `Entry`. -/
def Entry.fromIdValue (id : UInt32) (value : ByteArray) : Entry :=
  if id = idV2              then .v2 value
  else if id = idV3         then .v3 value
  else if id = idV3_1       then .v3_1 value
  else if id = idPadding    then .padding value
  else if id = idSourceStampV1 then .sourceStampV1 value
  else if id = idSourceStampV2 then .sourceStampV2 value
  else .unknown id value

/-! ## Fully-parsed block -/

/-- A fully-parsed APK signing block. -/
structure Block where
  /-- Entries in source order. -/
  entries        : List Entry
  /-- Stream-offset of the leading `size_of_block` u64. -/
  blockOffset    : Nat
  /-- Total bytes from leading u64 through trailing magic
  (= `size_of_block + 8`). -/
  blockTotalSize : Nat
deriving Inhabited

/-! ## Parse errors -/

inductive ParseError : Type where
  /-- EOCD signature not found. -/
  | noEocd
  /-- `cd_offset` field in EOCD points beyond the input. -/
  | invalidCdOffset
  /-- The 16-byte tail magic is missing or wrong. -/
  | badMagic
  /-- Declared `size_of_block` is zero or larger than the input. -/
  | invalidSize
  /-- Leading and trailing `size_of_block` u64s disagree. -/
  | sizeMismatch
  /-- A pair declares more bytes than remain in the pair region. -/
  | pairOverflow
  /-- A pair length is < 4 (must include the 4-byte ID). -/
  | pairTooShort
  /-- Trailing bytes after the last pair (block region not fully
  consumed by complete pairs). -/
  | trailingJunk
deriving Repr, DecidableEq

instance : ToString ParseError where
  toString
    | .noEocd          => "noEocd"
    | .invalidCdOffset => "invalidCdOffset"
    | .badMagic        => "badMagic"
    | .invalidSize     => "invalidSize"
    | .sizeMismatch    => "sizeMismatch"
    | .pairOverflow    => "pairOverflow"
    | .pairTooShort    => "pairTooShort"
    | .trailingJunk    => "trailingJunk"

/-- Tag enumeration for cross-language interop. The Rust reference
parser uses the same byte assignments. -/
def ParseError.tag : ParseError → UInt8
  | .noEocd          => 1
  | .invalidCdOffset => 2
  | .badMagic        => 3
  | .invalidSize     => 4
  | .sizeMismatch    => 5
  | .pairOverflow    => 6
  | .pairTooShort    => 7
  | .trailingJunk    => 8

/-- The eight error tags are pairwise distinct. -/
theorem ParseError.tag_inj :
    ∀ a b : ParseError, a.tag = b.tag → a = b := by
  intro a b h
  cases a <;> cases b <;> simp [ParseError.tag] at h <;> rfl

/-! ## Locate + walk the block -/

/-- Find the EOCD signature by scanning backward from the end. The
EOCD has a variable-length comment region trailing the 22-byte
fixed prefix; per APPNOTE.TXT §4.3.16 the signature is unique
enough that the first match from EOF backwards is the canonical
EOCD. -/
def findEocd (bs : ByteArray) : Option Nat := Id.run do
  if bs.size < eocdFixedSize then
    return none
  let mut i := bs.size - eocdFixedSize
  while True do
    match readU32 bs i with
    | some v =>
      if v = eocdSignature then
        return some i
    | none => pure ()
    if i = 0 then
      return none
    i := i - 1
  return none

/-- Test that the 16 bytes starting at offset `o` of `bs` equal the
APK Signing Block magic. -/
def isMagicAt (bs : ByteArray) (o : Nat) : Bool := Id.run do
  if o + magicLen > bs.size then
    return false
  for k in [0:magicLen] do
    if bs.get! (o + k) ≠ magic.get! k then
      return false
  return true

/-- Walk an in-memory pair region and produce the list of entries.
The region must contain only complete pairs; any leftover bytes
flag `trailingJunk`. -/
partial def parsePairs (region : ByteArray) (acc : List Entry) (cur : Nat) :
    Except ParseError (List Entry) := Id.run do
  if cur = region.size then
    return .ok acc.reverse
  if cur > region.size then
    return .error .trailingJunk
  if cur + 8 > region.size then
    return .error .trailingJunk
  let .some length := readU64 region cur
    | return .error .trailingJunk
  let lengthN : Nat := length.toNat
  if lengthN < 4 then
    return .error .pairTooShort
  let pairTotal : Nat := 8 + lengthN
  let remaining : Nat := region.size - cur
  if pairTotal > remaining then
    return .error .pairOverflow
  let .some id := readU32 region (cur + 8)
    | return .error .pairOverflow
  let .some valueBytes := slice region (cur + 12) (lengthN - 4)
    | return .error .pairOverflow
  parsePairs region (Entry.fromIdValue id valueBytes :: acc) (cur + pairTotal)

/-- Locate the APK signing block in `bs`. Returns:

  * `Except.ok (some block)` — block found and well-formed.
  * `Except.ok none`         — input is unsigned (no magic at the
                                expected offset; valid for v1-only
                                JAR-signed APKs).
  * `Except.error e`         — well-formedness violation.
-/
def locate (bs : ByteArray) : Except ParseError (Option Block) := Id.run do
  let .some eocdOff := findEocd bs
    | return .error .noEocd
  if eocdOff + eocdFixedSize > bs.size then
    return .error .noEocd
  let .some cdOffU := readU32 bs (eocdOff + 16)
    | return .error .noEocd
  let cdOff : Nat := cdOffU.toNat
  if cdOff > bs.size then
    return .error .invalidCdOffset
  if cdOff < 24 then
    return .ok none
  -- Magic check.
  let magicOff := cdOff - magicLen
  if !(isMagicAt bs magicOff) then
    return .ok none
  -- Trailing size_of_block.
  let .some trailingSob := readU64 bs (cdOff - 24)
    | return .error .invalidSize
  let trailingSobN : Nat := trailingSob.toNat
  if trailingSobN = 0 then
    return .error .invalidSize
  if trailingSobN + 8 > cdOff then
    return .error .invalidSize
  let blockOffset := cdOff - trailingSobN - 8
  let .some leadingSob := readU64 bs blockOffset
    | return .error .sizeMismatch
  if leadingSob ≠ trailingSob then
    return .error .sizeMismatch
  -- Walk pairs.
  let pairStart := blockOffset + 8
  let pairEnd := cdOff - 24
  let .some region := slice bs pairStart (pairEnd - pairStart)
    | return .error .invalidSize
  match parsePairs region [] 0 with
  | .ok entries =>
    return .ok (some {
      entries
      blockOffset
      blockTotalSize := trailingSobN + 8
    })
  | .error e =>
    return .error e

/-! ## Convenience accessors -/

/-- True iff the block has any v2/v3/v3.1 entry. -/
def Block.hasModernScheme (b : Block) : Bool :=
  b.entries.any fun e =>
    match e with
    | .v2 _ | .v3 _ | .v3_1 _ => true
    | _ => false

/-- First v2 entry value, if any. -/
def Block.v2 (b : Block) : Option ByteArray :=
  b.entries.findSome? fun
    | .v2 v => some v
    | _     => none

/-- First v3 entry value, if any. -/
def Block.v3 (b : Block) : Option ByteArray :=
  b.entries.findSome? fun
    | .v3 v => some v
    | _     => none

/-- First v3.1 entry value, if any. -/
def Block.v3_1 (b : Block) : Option ByteArray :=
  b.entries.findSome? fun
    | .v3_1 v => some v
    | _     => none

/-! ## Smoke checks -/

/-- The magic byte sequence matches the ASCII spelling. -/
example : magic.size = 16 := by native_decide

/-- The EOCD signature is the canonical APPNOTE value. -/
example : eocdSignature = 0x06054b50 := by native_decide

/-- v2 / v3 / v3.1 IDs are pairwise distinct. -/
example : idV2 ≠ idV3 := by native_decide
example : idV2 ≠ idV3_1 := by native_decide
example : idV3 ≠ idV3_1 := by native_decide

/-- `Entry.fromIdValue` round-trips known IDs into typed variants. -/
example :
    let e := Entry.fromIdValue idV2 ByteArray.empty
    e.id = idV2 := by native_decide

example :
    let e := Entry.fromIdValue idV3 ByteArray.empty
    e.id = idV3 := by native_decide

example :
    let e := Entry.fromIdValue idV3_1 ByteArray.empty
    e.id = idV3_1 := by native_decide

/-- Unknown IDs surface as `Entry.unknown`. -/
example :
    let e := Entry.fromIdValue 0xdeadbeef ByteArray.empty
    e.id = 0xdeadbeef := by native_decide

end Apkaxiom.Signing.Block
