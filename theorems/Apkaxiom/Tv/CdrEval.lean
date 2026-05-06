/-
P1.12 — translation-validation Lean evaluator for the CDR parser.

Mirrors `tools/cdr-eval-rust`. Reads `<hex-blob>\n` lines on
stdin, runs `Apkaxiom.Zip.CentralDirectory.parseCdr` on each,
emits one stable JSON-line per input on stdout.

Output line shape (stable field order):

  {"i":<index>,"out":"ok","sig":<u32>,"vmade":<u16>,"vneed":<u16>,
   "flags":<u16>,"method":<u16>,"time":<u16>,"date":<u16>,"crc":<u32>,
   "csize":<u32>,"usize":<u32>,"nlen":<u16>,"elen":<u16>,"clen":<u16>,
   "disk":<u16>,"iattr":<u16>,"eattr":<u32>,"lfh_off":<u32>,
   "name":"<hex>","extra":"<hex>","comment":"<hex>","consumed":<usize>}

  {"i":<index>,"out":"err","tag":<u8>,"name":"<short-name>"}
-/

import Apkaxiom.Zip.CentralDirectory
import Apkaxiom.Tv.Common

namespace Apkaxiom.Tv.CdrEval

open Apkaxiom.Zip.CentralDirectory
open Apkaxiom.Tv.Common (hexEncode hexDecode escString)

/-- Emit the OK shape. -/
def emitOk (i : Nat) (c : Cdr) (consumed : Nat) : String :=
  let n := escString (hexEncode c.fileName)
  let e := escString (hexEncode c.extraField)
  let co := escString (hexEncode c.fileComment)
  s!"\{\"i\":{i},\"out\":\"ok\",\"sig\":{cdrSignature.toNat},\"vmade\":{c.versionMadeBy.toNat}," ++
  s!"\"vneed\":{c.versionNeeded.toNat},\"flags\":{c.generalFlags.toNat}," ++
  s!"\"method\":{c.compressionMethod.toNat},\"time\":{c.lastModTime.toNat}," ++
  s!"\"date\":{c.lastModDate.toNat},\"crc\":{c.crc32.toNat}," ++
  s!"\"csize\":{c.compressedSize.toNat},\"usize\":{c.uncompressedSize.toNat}," ++
  s!"\"nlen\":{c.fileName.size},\"elen\":{c.extraField.size}," ++
  s!"\"clen\":{c.fileComment.size},\"disk\":{c.diskNumberStart.toNat}," ++
  s!"\"iattr\":{c.internalFileAttributes.toNat}," ++
  s!"\"eattr\":{c.externalFileAttributes.toNat}," ++
  s!"\"lfh_off\":{c.lfhOffset.toNat}," ++
  s!"\"name\":\"{n}\",\"extra\":\"{e}\",\"comment\":\"{co}\",\"consumed\":{consumed}}"

/-- Emit the error shape. -/
def emitErr (i : Nat) (err : ParseError) : String :=
  let name := match err with
    | .shortHeader   => "shortHeader"
    | .badSignature  => "badSignature"
    | .shortName     => "shortName"
    | .shortExtra    => "shortExtra"
    | .shortComment  => "shortComment"
  s!"\{\"i\":{i},\"out\":\"err\",\"tag\":{(ParseError.tag err).toNat},\"name\":\"{name}\"}"

/-- One input → one output line. -/
def evalOne (i : Nat) (input : String) : String :=
  match hexDecode input.trimAscii.toString with
  | none => s!"\{\"i\":{i},\"out\":\"err\",\"tag\":255,\"name\":\"hexDecode\"}"
  | some bs =>
    match parseCdr bs with
    | .ok (c, consumed) => emitOk i c consumed
    | .error e => emitErr i e

/-- Driver. -/
def main : IO Unit := do
  let stdin ← IO.getStdin
  let stdout ← IO.getStdout
  let mut buf : Array String := #[]
  while true do
    let line ← stdin.getLine
    if line.isEmpty then break
    buf := buf.push line
  let mut i : Nat := 0
  for line in buf do
    if line.trimAscii.toString.isEmpty then
      i := i + 1
      continue
    stdout.putStrLn (evalOne i line)
    i := i + 1

end Apkaxiom.Tv.CdrEval

def main : IO Unit := Apkaxiom.Tv.CdrEval.main
