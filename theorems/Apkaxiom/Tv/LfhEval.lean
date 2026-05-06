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
import Apkaxiom.Tv.Common
import Apkaxiom.Tv.Encode

namespace Apkaxiom.Tv.LfhEval

open Apkaxiom.Zip.LocalHeader
open Apkaxiom.Tv.Common (hexDecode)

/-- One input → one output line. Single source of truth: the
encoder lives in `Apkaxiom.Tv.Encode`. -/
def evalOne (i : Nat) (input : String) : String :=
  match hexDecode input.trimAscii.toString with
  | none    => Apkaxiom.Tv.Encode.encodeHexDecodeError i
  | some bs => Apkaxiom.Tv.Encode.encodeLfhResult i (parseLfh bs)

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
