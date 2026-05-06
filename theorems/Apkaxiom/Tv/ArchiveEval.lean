/-
P1.12 — translation-validation Lean evaluator for the
Consistency (whole-archive) parser.

Mirrors `tools/archive-eval-rust`. Reads `<hex-blob>\n` lines on
stdin, runs `Apkaxiom.Zip.Consistency.parseArchive` on each.
-/

import Apkaxiom.Zip.Consistency
import Apkaxiom.Tv.Common

namespace Apkaxiom.Tv.ArchiveEval

open Apkaxiom.Zip.Consistency
open Apkaxiom.Tv.Common (hexDecode)

/-- Emit the OK shape. -/
def emitOk (i : Nat) (a : Archive) : String :=
  s!"\{\"i\":{i},\"out\":\"ok\",\"cdrs\":{a.cdrs.length},\"lfhs\":{a.lfhs.length}," ++
  s!"\"total_entries\":{a.eocd.totalEntries.toNat}," ++
  s!"\"cd_offset\":{a.eocd.cdOffset.toNat},\"cd_size\":{a.eocd.cdSize.toNat}}"

/-- Emit the error shape. -/
def emitErr (i : Nat) (err : ArchiveError) : String :=
  let name := match err with
    | .noEocd            => "noEocd"
    | .eocdInvalid       => "eocdInvalid"
    | .cdOutOfRange      => "cdOutOfRange"
    | .cdrInvalid        => "cdrInvalid"
    | .cdrCountMismatch  => "cdrCountMismatch"
    | .lfhOffsetOob      => "lfhOffsetOob"
    | .lfhInvalid        => "lfhInvalid"
    | .filenameMismatch  => "filenameMismatch"
    | .fieldMismatch     => "fieldMismatch"
    | .eocdTooFarFromEof => "eocdTooFarFromEof"
    | .cdAfterEocd       => "cdAfterEocd"
    | .invalidEntryName  => "invalidEntryName"
  s!"\{\"i\":{i},\"out\":\"err\",\"tag\":{(ArchiveError.tag err).toNat},\"name\":\"{name}\"}"

/-- One input → one output line. -/
def evalOne (i : Nat) (input : String) : String :=
  match hexDecode input.trimAscii.toString with
  | none => s!"\{\"i\":{i},\"out\":\"err\",\"tag\":255,\"name\":\"hexDecode\"}"
  | some bs =>
    match parseArchive bs with
    | .ok a => emitOk i a
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

end Apkaxiom.Tv.ArchiveEval

def main : IO Unit := Apkaxiom.Tv.ArchiveEval.main
