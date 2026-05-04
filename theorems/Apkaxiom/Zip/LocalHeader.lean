/-
P1.5 — ZIP Local File Header (LFH) formalization.

The LFH is the per-entry header that prefixes every file inside a ZIP
archive. Layout per APPNOTE.TXT 6.3.10 §4.3.7:

    offset  size  field
      0     4     local file header signature  (0x04034b50, "PK\x03\x04")
      4     2     version needed to extract
      6     2     general purpose bit flag
      8     2     compression method
     10     2     last mod file time
     12     2     last mod file date
     14     4     CRC-32
     18     4     compressed size
     22     4     uncompressed size
     26     2     file name length              (n)
     28     2     extra field length            (m)
     30     n     file name
   30+n     m     extra field

Only the fixed-size 30-byte header + the two length-prefixed regions
matter for parsing soundness; the file body that follows is decoded
by the central-directory layer (P1.6).

The model below is *executable* — `parseLfh` evaluates inside Lean
via `#eval` and via `lake env lean --run` driver scripts (used by the
differential harness at `tests/differential`). The Rust reference
parser at `crates/axiom-zip-ref` mirrors this implementation
byte-for-byte; the differential harness asserts equivalence across
the 1,500+ corpus inputs.
-/

import Std

namespace Apkaxiom.Zip.LocalHeader

/-- The four-byte ZIP local-file-header signature, little-endian. -/
def lfhSignature : UInt32 := 0x04034b50

/-- Fixed-size portion of the LFH: 30 bytes. -/
def fixedSize : Nat := 30

/-- Total length the LFH occupies in the byte stream, given its
declared filename + extra-field lengths. -/
def totalSize (nameLen extraLen : UInt16) : Nat :=
  fixedSize + nameLen.toNat + extraLen.toNat

/-- Parsed LFH structure. Field names mirror the APPNOTE.TXT spec,
plus the filename + extra-field bytes the header points at. -/
structure Lfh where
  versionNeeded     : UInt16
  generalFlags      : UInt16
  compressionMethod : UInt16
  lastModTime       : UInt16
  lastModDate       : UInt16
  crc32             : UInt32
  compressedSize    : UInt32
  uncompressedSize  : UInt32
  fileName          : ByteArray
  extraField        : ByteArray
deriving Inhabited

/-- Parse failure modes. Each variant is a *closed* category that
the differential harness compares between Lean and the Rust
reference parser. -/
inductive ParseError : Type where
  | shortHeader    -- input shorter than the 30-byte fixed prefix
  | badSignature   -- magic ≠ 0x04034b50
  | shortName      -- filename region runs past EOF
  | shortExtra     -- extra-field region runs past EOF
deriving Repr, DecidableEq

instance : ToString ParseError where
  toString
    | .shortHeader  => "shortHeader"
    | .badSignature => "badSignature"
    | .shortName    => "shortName"
    | .shortExtra   => "shortExtra"

/-- Tag enumeration for cross-language interop. The Rust reference
parser uses the same byte assignments; the differential harness
diffs `tag` values when comparing error categories. -/
def ParseError.tag : ParseError → UInt8
  | .shortHeader  => 1
  | .badSignature => 2
  | .shortName    => 3
  | .shortExtra   => 4

/-- The four error tags are pairwise distinct. -/
theorem ParseError.tag_injective : Function.Injective ParseError.tag := by
  intro a b h
  cases a <;> cases b <;> simp [ParseError.tag] at h <;> rfl

/-- Read a little-endian `UInt16` from `bs` at offset `o`. Returns
`none` if the read would run past the end of the array. -/
def readU16 (bs : ByteArray) (o : Nat) : Option UInt16 :=
  if o + 1 < bs.size then
    let b0 := bs.get! o
    let b1 := bs.get! (o + 1)
    some (b0.toUInt16 ||| (b1.toUInt16 <<< 8))
  else
    none

/-- Read a little-endian `UInt32` from `bs` at offset `o`. -/
def readU32 (bs : ByteArray) (o : Nat) : Option UInt32 :=
  if o + 3 < bs.size then
    let b0 := bs.get! o
    let b1 := bs.get! (o + 1)
    let b2 := bs.get! (o + 2)
    let b3 := bs.get! (o + 3)
    some (b0.toUInt32
        ||| (b1.toUInt32 <<< 8)
        ||| (b2.toUInt32 <<< 16)
        ||| (b3.toUInt32 <<< 24))
  else
    none

/-- Slice `bs[o : o+len]` into a fresh `ByteArray`. -/
def slice (bs : ByteArray) (o len : Nat) : Option ByteArray :=
  if o + len ≤ bs.size then
    some (bs.extract o (o + len))
  else
    none

/-- Reference parser. Returns the parsed structure, the total bytes
consumed (so the caller can resume on the next entry), or a typed
error. The implementation is *byte-faithful* to APPNOTE.TXT and
mirrored by `axiom_zip_ref::lfh::parse_lfh` in Rust. -/
def parseLfh (bs : ByteArray) : Except ParseError (Lfh × Nat) := Id.run do
  -- Fixed header bounds.
  if bs.size < fixedSize then
    return .error .shortHeader
  -- Magic.
  let .some sig := readU32 bs 0
    | return .error .shortHeader
  if sig ≠ lfhSignature then
    return .error .badSignature
  -- Body fields.
  let .some versionNeeded     := readU16 bs 4   | return .error .shortHeader
  let .some generalFlags      := readU16 bs 6   | return .error .shortHeader
  let .some compressionMethod := readU16 bs 8   | return .error .shortHeader
  let .some lastModTime       := readU16 bs 10  | return .error .shortHeader
  let .some lastModDate       := readU16 bs 12  | return .error .shortHeader
  let .some crc32             := readU32 bs 14  | return .error .shortHeader
  let .some compressedSize    := readU32 bs 18  | return .error .shortHeader
  let .some uncompressedSize  := readU32 bs 22  | return .error .shortHeader
  let .some nameLen           := readU16 bs 26  | return .error .shortHeader
  let .some extraLen          := readU16 bs 28  | return .error .shortHeader
  -- Variable-length regions.
  let .some fileName  := slice bs 30 nameLen.toNat
    | return .error .shortName
  let .some extraField := slice bs (30 + nameLen.toNat) extraLen.toNat
    | return .error .shortExtra
  let header : Lfh :=
    { versionNeeded, generalFlags, compressionMethod
    , lastModTime, lastModDate
    , crc32, compressedSize, uncompressedSize
    , fileName, extraField }
  return .ok (header, totalSize nameLen extraLen)

/-- One concrete success witness, used as the smoke check that
`parseLfh` is wired up correctly. The byte sequence below is the
minimal valid LFH (zero-length filename, zero-length extra field). -/
def minimalLfhBytes : ByteArray :=
  ByteArray.mk #[
    -- signature 0x04034b50
    0x50, 0x4b, 0x03, 0x04,
    -- versionNeeded
    0x14, 0x00,
    -- generalFlags
    0x00, 0x00,
    -- compressionMethod
    0x00, 0x00,
    -- lastModTime / lastModDate
    0x00, 0x00, 0x00, 0x00,
    -- crc32 / compressedSize / uncompressedSize
    0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00,
    -- nameLen / extraLen
    0x00, 0x00,
    0x00, 0x00
  ]

/-- Project the error component of a parse result, for elaboration-
time checks where `Lfh` itself lacks `DecidableEq`. -/
def parseError (bs : ByteArray) : Option ParseError :=
  match parseLfh bs with
  | .error e => some e
  | .ok _    => none

/-- The minimal valid LFH parses successfully. -/
example : parseError minimalLfhBytes = none := by native_decide

/-- A truncated input fails with `shortHeader`. -/
example :
    parseError (ByteArray.mk #[0x50, 0x4b, 0x03]) = some .shortHeader := by
  native_decide

/-- An input with the wrong magic fails with `badSignature`. -/
example :
    parseError (ByteArray.mk #[
      0xff, 0xff, 0xff, 0xff,
      0x14, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00
    ]) = some .badSignature := by
  native_decide

end Apkaxiom.Zip.LocalHeader
