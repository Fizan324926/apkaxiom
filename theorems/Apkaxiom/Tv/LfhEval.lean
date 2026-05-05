/-
P1.9 — translation-validation Lean evaluator for the LFH parser.

The evaluator reads a list of `<hex-blob>\n` records on stdin (one
per line), runs `Apkaxiom.Zip.LocalHeader.parseLfh` on each, and
emits one stable-shape JSON-line per input on stdout. The
`tools/translation-validator` Rust binary diffs the output against
a Rust evaluator that consumes the same corpus.

Output line shape (one JSON object per input, stable field order):

  {"i":<index>,"out":"ok","sig":<u32>,"vers":<u16>,"flags":<u16>,
   "method":<u16>,"time":<u16>,"date":<u16>,"crc":<u32>,
   "csize":<u32>,"usize":<u32>,"nlen":<u16>,"elen":<u16>,
   "name":"<hex>","extra":"<hex>","consumed":<usize>}

  {"i":<index>,"out":"err","tag":<u8>,"name":"<short-name>"}

The hex encoding is lowercase, fixed-width (no `0x` prefix) so the
output is byte-deterministic across platforms / Lean versions
that may format integers differently.

Run protocol (used by tools/translation-validator):

  lake exe lfh-eval <corpus.txt
-/

import Apkaxiom.Zip.LocalHeader

namespace Apkaxiom.Tv.LfhEval

open Apkaxiom.Zip.LocalHeader

/-- Lower-nibble → ASCII hex character. -/
@[inline] def nibbleHex (n : UInt8) : Char :=
  if n < 10 then Char.ofNat (n.toNat + '0'.toNat)
  else Char.ofNat (n.toNat - 10 + 'a'.toNat)

/-- Lowercase hex of one byte. -/
@[inline] def byteHex (b : UInt8) : String :=
  String.mk [nibbleHex (b >>> 4), nibbleHex (b &&& 0x0f)]

/-- Lowercase hex of a byte array. -/
def hexEncode (bs : ByteArray) : String := Id.run do
  let mut s : String := ""
  for i in [0:bs.size] do
    s := s ++ byteHex (bs.get! i)
  return s

/-- Parse two hex digits (lowercase only). -/
@[inline] def hexNibble (c : Char) : Option UInt8 :=
  if '0' ≤ c ∧ c ≤ '9' then some ((c.toNat - '0'.toNat).toUInt8)
  else if 'a' ≤ c ∧ c ≤ 'f' then some ((c.toNat - 'a'.toNat + 10).toUInt8)
  else none

/-- Decode a lowercase-hex string into bytes; `none` on bad input. -/
def hexDecode (s : String) : Option ByteArray := Id.run do
  let chars := s.toList
  if chars.length % 2 ≠ 0 then return none
  let mut out : ByteArray := ByteArray.empty
  let mut iter := chars
  while !iter.isEmpty do
    match iter with
    | c1 :: c2 :: rest =>
        match hexNibble c1, hexNibble c2 with
        | some hi, some lo => out := out.push ((hi <<< 4) ||| lo); iter := rest
        | _, _ => return none
    | _ => return none
  return some out

/-- JSON-escape a string. Only `\` and `"` need escaping for our
shape (everything else is hex digits or single ASCII chars). -/
def escString (s : String) : String :=
  s.foldl (init := "") fun acc c =>
    if c = '"' then acc ++ "\\\""
    else if c = '\\' then acc ++ "\\\\"
    else acc.push c

/-- Emit the OK shape. -/
def emitOk (i : Nat) (lfh : Lfh) (consumed : Nat) : String :=
  let n := escString (hexEncode lfh.fileName)
  let e := escString (hexEncode lfh.extraField)
  s!"\{\"i\":{i},\"out\":\"ok\",\"sig\":{lfhSignature.toNat},\"vers\":{lfh.versionNeeded.toNat}," ++
  s!"\"flags\":{lfh.generalFlags.toNat},\"method\":{lfh.compressionMethod.toNat}," ++
  s!"\"time\":{lfh.lastModTime.toNat},\"date\":{lfh.lastModDate.toNat}," ++
  s!"\"crc\":{lfh.crc32.toNat},\"csize\":{lfh.compressedSize.toNat}," ++
  s!"\"usize\":{lfh.uncompressedSize.toNat},\"nlen\":{lfh.fileName.size}," ++
  s!"\"elen\":{lfh.extraField.size},\"name\":\"{n}\",\"extra\":\"{e}\"," ++
  s!"\"consumed\":{consumed}}"

/-- Emit the error shape. -/
def emitErr (i : Nat) (err : ParseError) : String :=
  let name := match err with
    | .shortHeader => "shortHeader"
    | .badSignature => "badSignature"
    | .shortName => "shortName"
    | .shortExtra => "shortExtra"
  s!"\{\"i\":{i},\"out\":\"err\",\"tag\":{(ParseError.tag err).toNat},\"name\":\"{name}\"}"

/-- One input → one output line. -/
def evalOne (i : Nat) (input : String) : String :=
  match hexDecode input.trimAscii.toString with
  | none => s!"\{\"i\":{i},\"out\":\"err\",\"tag\":255,\"name\":\"hexDecode\"}"
  | some bs =>
    match parseLfh bs with
    | .ok (lfh, consumed) => emitOk i lfh consumed
    | .error e => emitErr i e

/-- Driver: read every line of stdin, emit one JSON line per input. -/
def main : IO Unit := do
  let stdin ← IO.getStdin
  let stdout ← IO.getStdout
  let mut i : Nat := 0
  let mut buf : Array String := #[]
  -- Buffer all input first so we don't print until we're ready.
  while true do
    let line ← stdin.getLine
    if line.isEmpty then break
    buf := buf.push line
  for line in buf do
    if line.trimAscii.toString.isEmpty then
      i := i + 1
      continue
    stdout.putStrLn (evalOne i line)
    i := i + 1

end Apkaxiom.Tv.LfhEval

def main : IO Unit := Apkaxiom.Tv.LfhEval.main
