/-
P1.9 §IV — translation-validation Lean evaluator for the EOCD parser.

Mirrors `Apkaxiom.Tv.LfhEval` (the LFH evaluator). Reads
`<hex-blob>\n` lines on stdin, runs
`Apkaxiom.Zip.Eocd.parseEocd` on each, emits one stable JSON-line
per input on stdout. The `tools/translation-validator` Rust binary
diffs the output against `tools/eocd-eval-rust` to assert byte-
identical agreement on the EOCD corpus.

Output line shape (stable field order):

  {"i":<index>,"out":"ok","sig":<u32>,"disk":<u16>,"cd_disk":<u16>,
   "entries_on_disk":<u16>,"total_entries":<u16>,"cd_size":<u32>,
   "cd_offset":<u32>,"clen":<u16>,"comment":"<hex>","consumed":<usize>}

  {"i":<index>,"out":"err","tag":<u8>,"name":"<short-name>"}
-/

import Apkaxiom.Zip.Eocd
import Apkaxiom.Tv.Common

namespace Apkaxiom.Tv.EocdEval

open Apkaxiom.Zip.Eocd
open Apkaxiom.Tv.Common (hexEncode hexDecode escString)

/-- Emit the OK shape. -/
def emitOk (i : Nat) (eocd : Eocd) (consumed : Nat) : String :=
  let c := escString (hexEncode eocd.comment)
  s!"\{\"i\":{i},\"out\":\"ok\",\"sig\":{eocdSignature.toNat},\"disk\":{eocd.diskNumber.toNat}," ++
  s!"\"cd_disk\":{eocd.cdStartDisk.toNat},\"entries_on_disk\":{eocd.entriesOnThisDisk.toNat}," ++
  s!"\"total_entries\":{eocd.totalEntries.toNat},\"cd_size\":{eocd.cdSize.toNat}," ++
  s!"\"cd_offset\":{eocd.cdOffset.toNat},\"clen\":{eocd.comment.size}," ++
  s!"\"comment\":\"{c}\",\"consumed\":{consumed}}"

/-- Emit the error shape. -/
def emitErr (i : Nat) (err : ParseError) : String :=
  let name := match err with
    | .shortFixed         => "shortFixed"
    | .badSignature       => "badSignature"
    | .shortComment       => "shortComment"
    | .inconsistentDisks  => "inconsistentDisks"
  s!"\{\"i\":{i},\"out\":\"err\",\"tag\":{(ParseError.tag err).toNat},\"name\":\"{name}\"}"

/-- One input → one output line. -/
def evalOne (i : Nat) (input : String) : String :=
  match hexDecode input.trimAscii.toString with
  | none => s!"\{\"i\":{i},\"out\":\"err\",\"tag\":255,\"name\":\"hexDecode\"}"
  | some bs =>
    match parseEocd bs with
    | .ok (eocd, consumed) => emitOk i eocd consumed
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

end Apkaxiom.Tv.EocdEval

def main : IO Unit := Apkaxiom.Tv.EocdEval.main
