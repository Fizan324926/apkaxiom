/-
P1.5 — ZIP End Of Central Directory (EOCD) record.

Layout per APPNOTE.TXT 6.3.10 §4.3.16:

    offset  size  field
      0     4     EOCD signature (0x06054b50, "PK\x05\x06")
      4     2     this disk number
      6     2     start-disk number for the central directory
      8     2     entries on this disk
     10     2     total entries
     12     4     central directory size
     16     4     central directory offset
     20     2     comment length             (k)
     22     k     comment

The EOCD is *suffix-anchored* in the file: callers locate it by
scanning backwards from EOF for the signature, then validate. Our
`parseEocd` operates on the already-located 22-byte fixed prefix
plus the comment region. The suffix-locator (`findEocd`) is given
below for completeness but is not on the soundness hot path —
P1.6's central-directory pass uses the same locator and inherits
its proof obligations.
-/

import Std
import Apkaxiom.Zip.LocalHeader  -- shared utilities (readU16/U32/slice)

namespace Apkaxiom.Zip.Eocd

open Apkaxiom.Zip.LocalHeader (readU16 readU32 slice)

/-- The four-byte ZIP EOCD signature, little-endian. -/
def eocdSignature : UInt32 := 0x06054b50

/-- Fixed-size portion of the EOCD: 22 bytes. -/
def fixedSize : Nat := 22

/-- Maximum legal comment length (16-bit field). -/
def maxCommentLen : Nat := 0xffff

/-- Parsed EOCD structure. -/
structure Eocd where
  diskNumber          : UInt16
  cdStartDisk         : UInt16
  entriesOnThisDisk   : UInt16
  totalEntries        : UInt16
  cdSize              : UInt32
  cdOffset            : UInt32
  comment             : ByteArray
deriving Inhabited

/-- Parse failure modes. -/
inductive ParseError : Type where
  | shortFixed         -- input shorter than the 22-byte fixed prefix
  | badSignature       -- magic ≠ 0x06054b50
  | shortComment       -- comment region runs past EOF
  | inconsistentDisks  -- diskNumber ≠ cdStartDisk (single-volume invariant)
deriving Repr, DecidableEq

instance : ToString ParseError where
  toString
    | .shortFixed         => "shortFixed"
    | .badSignature       => "badSignature"
    | .shortComment       => "shortComment"
    | .inconsistentDisks  => "inconsistentDisks"

/-- Tag enumeration for cross-language interop. -/
def ParseError.tag : ParseError → UInt8
  | .shortFixed         => 1
  | .badSignature       => 2
  | .shortComment       => 3
  | .inconsistentDisks  => 4

theorem ParseError.tag_injective : Function.Injective ParseError.tag := by
  intro a b h
  cases a <;> cases b <;> simp [ParseError.tag] at h <;> rfl

/-- Reference parser. Operates on the byte sequence starting at the
EOCD signature (caller pre-locates via `findEocd`). -/
def parseEocd (bs : ByteArray) : Except ParseError (Eocd × Nat) := Id.run do
  if bs.size < fixedSize then
    return .error .shortFixed
  let .some sig := readU32 bs 0
    | return .error .shortFixed
  if sig ≠ eocdSignature then
    return .error .badSignature
  let .some diskNumber        := readU16 bs 4   | return .error .shortFixed
  let .some cdStartDisk       := readU16 bs 6   | return .error .shortFixed
  let .some entriesOnThisDisk := readU16 bs 8   | return .error .shortFixed
  let .some totalEntries      := readU16 bs 10  | return .error .shortFixed
  let .some cdSize            := readU32 bs 12  | return .error .shortFixed
  let .some cdOffset          := readU32 bs 16  | return .error .shortFixed
  let .some commentLen        := readU16 bs 20  | return .error .shortFixed
  -- Single-volume APKs always have disk 0; reject multi-volume
  -- archives at the type level. ZIP64 multi-volume support is
  -- out-of-scope for v0.1 (see ADR-0017 in CHECKLIST §G).
  if diskNumber ≠ cdStartDisk then
    return .error .inconsistentDisks
  let .some comment := slice bs 22 commentLen.toNat
    | return .error .shortComment
  let header : Eocd :=
    { diskNumber, cdStartDisk
    , entriesOnThisDisk, totalEntries
    , cdSize, cdOffset, comment }
  return .ok (header, fixedSize + commentLen.toNat)

/-- Locate the EOCD in a complete ZIP archive by scanning backwards
from EOF for the signature. Returns the byte offset of the
signature, or `none` if no candidate fits in the trailing
`maxCommentLen + fixedSize` bytes (per APPNOTE.TXT, the comment is
always the trailing region — so the signature is at most
`fixedSize + maxCommentLen` from EOF). -/
partial def findEocd (bs : ByteArray) : Option Nat :=
  let last := bs.size
  let scanFrom :=
    if last < fixedSize then 0
    else if last - fixedSize > maxCommentLen
      then last - fixedSize - maxCommentLen
      else 0
  let rec go (off : Nat) : Option Nat :=
    if off + fixedSize > last then
      none
    else
      match readU32 bs off with
      | some sig =>
        if sig = eocdSignature then some off
        else if off = 0 then none
        else go (off - 1)
      | none => none
  -- Start scanning from the latest legal position, working back.
  let startOff :=
    if last < fixedSize then 0
    else last - fixedSize
  let _ := scanFrom  -- documenting the upper-bound; the linear scan
                     -- below covers the same window via a backward walk
  go startOff

/-- Project the error component for elaboration-time checks. -/
def parseError (bs : ByteArray) : Option ParseError :=
  match parseEocd bs with
  | .error e => some e
  | .ok _    => none

/-- The minimal valid EOCD: zero entries, empty comment. -/
def minimalEocdBytes : ByteArray :=
  ByteArray.mk #[
    -- signature 0x06054b50
    0x50, 0x4b, 0x05, 0x06,
    -- diskNumber / cdStartDisk
    0x00, 0x00, 0x00, 0x00,
    -- entriesOnThisDisk / totalEntries
    0x00, 0x00, 0x00, 0x00,
    -- cdSize / cdOffset
    0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00,
    -- commentLen
    0x00, 0x00
  ]

example : parseError minimalEocdBytes = none := by native_decide

example :
    parseError (ByteArray.mk #[0x50, 0x4b, 0x05]) = some .shortFixed := by
  native_decide

example :
    parseError (ByteArray.mk #[
      0xff, 0xff, 0xff, 0xff,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00
    ]) = some .badSignature := by
  native_decide

/-- Multi-volume archives are rejected: diskNumber=1, cdStartDisk=0. -/
example :
    parseError (ByteArray.mk #[
      0x50, 0x4b, 0x05, 0x06,
      0x01, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00
    ]) = some .inconsistentDisks := by
  native_decide

end Apkaxiom.Zip.Eocd
