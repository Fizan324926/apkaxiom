/-
P1.6 — ZIP Central Directory Record (CDR) formalization.

The CDR is the per-entry record stored in the *central directory* (CD)
of a ZIP archive. There is exactly one CDR per file entry, indexed
from the EOCD's `cdOffset` and stored sequentially. Layout per
APPNOTE.TXT 6.3.10 §4.3.12:

    offset  size  field
      0     4     central file header signature   (0x02014b50, "PK\x01\x02")
      4     2     version made by
      6     2     version needed to extract
      8     2     general purpose bit flag
     10     2     compression method
     12     2     last mod file time
     14     2     last mod file date
     16     4     CRC-32
     20     4     compressed size
     24     4     uncompressed size
     28     2     file name length                (n)
     30     2     extra field length              (m)
     32     2     file comment length             (k)
     34     2     disk number start
     36     2     internal file attributes
     38     4     external file attributes
     42     4     relative offset of local header
     46     n     file name
   46+n     m     extra field
 46+n+m     k     file comment

The CDR is *self-describing* (length fields tell us how far to walk)
and points back into the archive at the LFH for that entry via the
relative-offset field at byte 42. The cross-record consistency layer
(`Apkaxiom.Zip.Consistency`) reasons about the validity of that
offset against the byte stream the CDR was parsed from.

Five `ParseError` variants — three short-* errors (one per
variable-length region) plus the two fixed-position errors
(`shortHeader`, `badSignature`). The Rust reference parser
(`axiom_zip_ref::cdr::parse_cdr`) and the AOSP wire-format probe agree
on tag values 1..5 by construction.
-/

import Std
import Apkaxiom.Zip.LocalHeader  -- shared utilities: readU16/readU32/slice

namespace Apkaxiom.Zip.CentralDirectory

open Apkaxiom.Zip.LocalHeader (readU16 readU32 slice)

/-- The four-byte ZIP central-directory-record signature, little-endian.
The same magic appears at byte 0 of every CDR. -/
def cdrSignature : UInt32 := 0x02014b50

/-- Fixed-size portion of the CDR: 46 bytes. -/
def fixedSize : Nat := 46

/-- Maximum legal filename length (16-bit field). -/
def maxNameLen : Nat := 0xffff

/-- Maximum legal extra-field length (16-bit field). -/
def maxExtraLen : Nat := 0xffff

/-- Maximum legal file-comment length (16-bit field). -/
def maxCommentLen : Nat := 0xffff

/-- Total length the CDR occupies in the byte stream, given its three
declared variable-length regions. -/
def totalSize (nameLen extraLen commentLen : UInt16) : Nat :=
  fixedSize + nameLen.toNat + extraLen.toNat + commentLen.toNat

/-- Parsed CDR structure. Field names mirror the APPNOTE.TXT spec, plus
the three variable-length regions (filename, extra, comment) the
header points at. -/
structure Cdr where
  versionMadeBy             : UInt16
  versionNeeded             : UInt16
  generalFlags              : UInt16
  compressionMethod         : UInt16
  lastModTime               : UInt16
  lastModDate               : UInt16
  crc32                     : UInt32
  compressedSize            : UInt32
  uncompressedSize          : UInt32
  diskNumberStart           : UInt16
  internalFileAttributes    : UInt16
  externalFileAttributes    : UInt32
  /-- Relative offset of the local header for this entry, from the
  beginning of the archive byte stream. The cross-record
  consistency layer asserts this offset is in-bounds and that the
  bytes there parse as a matching LFH. -/
  lfhOffset                 : UInt32
  fileName                  : ByteArray
  extraField                : ByteArray
  fileComment               : ByteArray
deriving Inhabited

/-- Parse failure modes. Each variant is a *closed* category that the
differential harness compares between Lean and the Rust reference
parser. The CDR has *five* variants vs LFH/EOCD's four — there are
three variable-length regions, so three short-* errors plus the
two fixed-position errors. -/
inductive ParseError : Type where
  | shortHeader      -- input shorter than the 46-byte fixed prefix
  | badSignature     -- magic ≠ 0x02014b50
  | shortName        -- filename region runs past EOF
  | shortExtra       -- extra-field region runs past EOF
  | shortComment     -- file-comment region runs past EOF
deriving Repr, DecidableEq

instance : ToString ParseError where
  toString
    | .shortHeader   => "shortHeader"
    | .badSignature  => "badSignature"
    | .shortName     => "shortName"
    | .shortExtra    => "shortExtra"
    | .shortComment  => "shortComment"

/-- Tag enumeration for cross-language interop. The Rust reference
parser uses the same byte assignments; the differential harness
diffs `tag` values when comparing error categories. Note that the
five-variant tag space is *contiguous* `{1,2,3,4,5}`. -/
def ParseError.tag : ParseError → UInt8
  | .shortHeader   => 1
  | .badSignature  => 2
  | .shortName     => 3
  | .shortExtra    => 4
  | .shortComment  => 5

/-- The five error tags are pairwise distinct. -/
theorem ParseError.tag_injective : Function.Injective ParseError.tag := by
  intro a b h
  cases a <;> cases b <;> simp [ParseError.tag] at h <;> rfl

/-- Reference parser. Returns the parsed structure and the total bytes
consumed (so the caller can resume on the next entry), or a typed
error. The implementation is *byte-faithful* to APPNOTE.TXT and
mirrored by `axiom_zip_ref::cdr::parse_cdr` in Rust and by the
AOSP wire-format probe's `parse_cdr` in C++. -/
def parseCdr (bs : ByteArray) : Except ParseError (Cdr × Nat) := Id.run do
  -- Fixed header bounds.
  if bs.size < fixedSize then
    return .error .shortHeader
  -- Magic.
  let .some sig := readU32 bs 0
    | return .error .shortHeader
  if sig ≠ cdrSignature then
    return .error .badSignature
  -- Body fields (offsets per APPNOTE.TXT §4.3.12).
  let .some versionMadeBy        := readU16 bs 4   | return .error .shortHeader
  let .some versionNeeded        := readU16 bs 6   | return .error .shortHeader
  let .some generalFlags         := readU16 bs 8   | return .error .shortHeader
  let .some compressionMethod    := readU16 bs 10  | return .error .shortHeader
  let .some lastModTime          := readU16 bs 12  | return .error .shortHeader
  let .some lastModDate          := readU16 bs 14  | return .error .shortHeader
  let .some crc32                := readU32 bs 16  | return .error .shortHeader
  let .some compressedSize       := readU32 bs 20  | return .error .shortHeader
  let .some uncompressedSize     := readU32 bs 24  | return .error .shortHeader
  let .some nameLen              := readU16 bs 28  | return .error .shortHeader
  let .some extraLen             := readU16 bs 30  | return .error .shortHeader
  let .some commentLen           := readU16 bs 32  | return .error .shortHeader
  let .some diskNumberStart      := readU16 bs 34  | return .error .shortHeader
  let .some internalFileAttrs    := readU16 bs 36  | return .error .shortHeader
  let .some externalFileAttrs    := readU32 bs 38  | return .error .shortHeader
  let .some lfhOffset            := readU32 bs 42  | return .error .shortHeader
  -- Variable-length regions, in declared order: name → extra → comment.
  let .some fileName    := slice bs fixedSize nameLen.toNat
    | return .error .shortName
  let .some extraField  := slice bs (fixedSize + nameLen.toNat) extraLen.toNat
    | return .error .shortExtra
  let .some fileComment := slice bs
                            (fixedSize + nameLen.toNat + extraLen.toNat)
                            commentLen.toNat
    | return .error .shortComment
  let header : Cdr :=
    { versionMadeBy, versionNeeded, generalFlags
    , compressionMethod, lastModTime, lastModDate
    , crc32, compressedSize, uncompressedSize
    , diskNumberStart
    , internalFileAttributes := internalFileAttrs
    , externalFileAttributes := externalFileAttrs
    , lfhOffset
    , fileName, extraField, fileComment }
  return .ok (header, totalSize nameLen extraLen commentLen)

/-- Walk a byte stream that *only* contains contiguous CDRs (the
"central directory" region of an archive, sliced out by the EOCD's
`cdOffset` + `cdSize`) and return the list of parsed records.
Stops on the first parse error and returns it.

**Terminating** — the recursion is well-founded on the measure
`bs.size - off`. Each successful step either exits (when
`parseCdr` consumes zero bytes — defensive branch) or strictly
decreases the measure (when `parseCdr` consumes ≥ 1 byte). -/
def parseCdrSequenceGo (bs : ByteArray) (off : Nat)
    (acc : List Cdr) : Except ParseError (List Cdr) :=
  if _h : off ≥ bs.size then
    .ok acc.reverse
  else
    let view := bs.extract off bs.size
    match parseCdr view with
    | .error e          => .error e
    | .ok (cdr, n)      =>
        if _hn : n = 0 then
          -- Defensive: a zero-length parse would loop forever.
          -- `parseCdr` always consumes ≥ `fixedSize` bytes on success,
          -- so this branch is unreachable in practice. We exit early
          -- to make the recursion terminating in *all* cases.
          .ok (cdr :: acc).reverse
        else
          parseCdrSequenceGo bs (off + n) (cdr :: acc)
termination_by bs.size - off
decreasing_by
  simp_wf
  omega

def parseCdrSequence (bs : ByteArray) : Except ParseError (List Cdr) :=
  parseCdrSequenceGo bs 0 []

/-- Project the error component of a parse result, for elaboration-time
checks where `Cdr` itself lacks `DecidableEq` (because `ByteArray` does
not). -/
def parseError (bs : ByteArray) : Option ParseError :=
  match parseCdr bs with
  | .error e => some e
  | .ok _    => none

/-- One concrete success witness, used as the smoke check that
`parseCdr` is wired up correctly. The byte sequence below is the
minimal valid CDR: zero-length filename / extra / comment, all
attribute fields zero. The 4-byte signature `50 4b 01 02` is the
little-endian encoding of `0x02014b50`. -/
def minimalCdrBytes : ByteArray :=
  ByteArray.mk #[
    -- signature 0x02014b50
    0x50, 0x4b, 0x01, 0x02,
    -- versionMadeBy / versionNeeded
    0x14, 0x00, 0x14, 0x00,
    -- generalFlags / compressionMethod
    0x00, 0x00, 0x00, 0x00,
    -- lastModTime / lastModDate
    0x00, 0x00, 0x00, 0x00,
    -- crc32
    0x00, 0x00, 0x00, 0x00,
    -- compressedSize
    0x00, 0x00, 0x00, 0x00,
    -- uncompressedSize
    0x00, 0x00, 0x00, 0x00,
    -- nameLen / extraLen / commentLen
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    -- diskNumberStart / internalFileAttrs
    0x00, 0x00, 0x00, 0x00,
    -- externalFileAttrs
    0x00, 0x00, 0x00, 0x00,
    -- lfhOffset
    0x00, 0x00, 0x00, 0x00
  ]

/-- The minimal valid CDR parses successfully. -/
example : parseError minimalCdrBytes = none := by native_decide

/-- A truncated input fails with `shortHeader`. -/
example :
    parseError (ByteArray.mk #[0x50, 0x4b, 0x01]) = some .shortHeader := by
  native_decide

/-- A 46-byte input with the wrong magic fails with `badSignature`. -/
example :
    parseError (ByteArray.mk #[
      0xff, 0xff, 0xff, 0xff,
      0x14, 0x00, 0x14, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00
    ]) = some .badSignature := by
  native_decide

/-- A CDR claiming a 1-byte filename but providing no payload fails
with `shortName`. -/
example :
    parseError (ByteArray.mk #[
      -- signature
      0x50, 0x4b, 0x01, 0x02,
      -- fixed body up to nameLen
      0x14, 0x00, 0x14, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      -- nameLen = 1, extraLen = 0, commentLen = 0
      0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
      -- remaining fixed-position fields
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00
      -- (no name byte appended → shortName)
    ]) = some .shortName := by
  native_decide

/-- A CDR claiming a 1-byte extra field but providing no payload fails
with `shortExtra`. -/
example :
    parseError (ByteArray.mk #[
      0x50, 0x4b, 0x01, 0x02,
      0x14, 0x00, 0x14, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      -- nameLen = 0, extraLen = 1, commentLen = 0
      0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00
    ]) = some .shortExtra := by
  native_decide

/-- A CDR claiming a 1-byte comment but providing no payload fails
with `shortComment`. -/
example :
    parseError (ByteArray.mk #[
      0x50, 0x4b, 0x01, 0x02,
      0x14, 0x00, 0x14, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      -- nameLen = 0, extraLen = 0, commentLen = 1
      0x00, 0x00, 0x00, 0x00, 0x01, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00
    ]) = some .shortComment := by
  native_decide

end Apkaxiom.Zip.CentralDirectory
