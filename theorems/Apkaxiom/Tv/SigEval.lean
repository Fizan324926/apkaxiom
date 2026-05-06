/-
P1.11 — translation-validation Lean evaluator for the APK
signing-block parser.

The evaluator reads `<hex-blob>\n` records on stdin (one per line),
each blob a complete APK file's bytes; for every input, runs
`Apkaxiom.Signing.Block.locate` + the v2 / v3 / v3.1 internal
parsers, and emits one stable-shape JSON line on stdout.

The Rust mirror at `tools/sig-eval-rust` consumes the SAME stdin
shape and produces byte-identical output. The differential
harness diffs both side-by-side over `corpus/signing/` (real
F-Droid + apksigner-resigned multi-scheme APKs).

Run protocol:

  for f in corpus/signing/**/*.apk; do
    xxd -p -c 9999 < "$f"
  done | lake exe sig-eval

Output line shape (one JSON object per input):

  {"i":<index>,"out":"unsigned"}                                 -- no signing block
  {"i":<index>,"out":"err","tag":<u8>,"name":"<short-name>"}      -- block parse error
  {"i":<index>,"out":"ok","blockOffset":<nat>,"blockTotalSize":<nat>,
   "entries":[{"id":<u32>,"len":<nat>}, …],
   "v2":[<signer>, …]?, "v3":[<signer>, …]?, "v3_1":[<signer>, …]?}
-/

import Apkaxiom.Signing.Block
import Apkaxiom.Signing.Scheme
import Apkaxiom.Tv.Common

namespace Apkaxiom.Tv.SigEval

open Apkaxiom.Signing.Block
open Apkaxiom.Signing.Scheme
open Apkaxiom.Tv.Common (hexDecode)

/-! ## JSON encoders -/

/-- Encode one entry summary. -/
def encodeEntry (e : Entry) : String :=
  "{\"id\":" ++ toString e.id.toNat ++ ",\"len\":" ++ toString e.value.size ++ "}"

/-- Encode the entries list. -/
def encodeEntries (es : List Entry) : String := Id.run do
  let mut s : String := "["
  let mut first : Bool := true
  for e in es do
    if !first then s := s ++ ","
    first := false
    s := s ++ encodeEntry e
  return s ++ "]"

/-- Encode a single signer summary. -/
def encodeSigner (s : Signer) (variantTag : String) : String :=
  let digsLen := s.digests.length
  let sigsLen := s.signatures.length
  let certsLen := s.certificates.length
  let sdkPart : String :=
    match s.sdkRange with
    | none => ""
    | some (a, b) => ",\"sdk_min\":" ++ toString a.toNat ++ ",\"sdk_max\":" ++ toString b.toNat
  "{\"variant\":\"" ++ variantTag
    ++ "\",\"signed_data_len\":" ++ toString s.signedData.size
    ++ ",\"public_key_len\":" ++ toString s.publicKey.size
    ++ ",\"certs\":" ++ toString certsLen
    ++ ",\"digests\":" ++ toString digsLen
    ++ ",\"signatures\":" ++ toString sigsLen
    ++ sdkPart ++ "}"

/-- Encode a list of signers. -/
def encodeSigners (signers : List Signer) (variantTag : String) : String := Id.run do
  let mut s : String := "["
  let mut first : Bool := true
  for sg in signers do
    if !first then s := s ++ ","
    first := false
    s := s ++ encodeSigner sg variantTag
  return s ++ "]"

/-- Try to parse and encode the v2/v3/v3.1 internal layer. -/
def encodeSchemeIfPresent (block : Block) : String := Id.run do
  let mut detail : String := ""
  match block.v2 with
  | none => pure ()
  | some v2bs =>
    match parseV2 v2bs with
    | .ok signers =>
        let encoded := encodeSigners signers "v2"
        detail := detail ++ ",\"v2\":" ++ encoded
    | .error _ => pure ()
  match block.v3 with
  | none => pure ()
  | some v3bs =>
    match parseV3 v3bs with
    | .ok signers =>
        let encoded := encodeSigners signers "v3"
        detail := detail ++ ",\"v3\":" ++ encoded
    | .error _ => pure ()
  match block.v3_1 with
  | none => pure ()
  | some v3_1bs =>
    match parseV3_1 v3_1bs with
    | .ok signers =>
        let encoded := encodeSigners signers "v3_1"
        detail := detail ++ ",\"v3_1\":" ++ encoded
    | .error _ => pure ()
  return detail

/-- Eval one APK's bytes → one JSON line. -/
def evalOne (i : Nat) (input : String) : String :=
  match hexDecode input.trimAscii.toString with
  | none =>
    "{\"i\":" ++ toString i ++ ",\"out\":\"err\",\"tag\":255,\"name\":\"hex-decode\"}"
  | some bs =>
    match locate bs with
    | .ok none =>
      "{\"i\":" ++ toString i ++ ",\"out\":\"unsigned\"}"
    | .ok (some block) =>
      "{\"i\":" ++ toString i
        ++ ",\"out\":\"ok\",\"blockOffset\":" ++ toString block.blockOffset
        ++ ",\"blockTotalSize\":" ++ toString block.blockTotalSize
        ++ ",\"entries\":" ++ encodeEntries block.entries
        ++ encodeSchemeIfPresent block ++ "}"
    | .error e =>
      "{\"i\":" ++ toString i ++ ",\"out\":\"err\",\"tag\":"
        ++ toString e.tag.toNat ++ ",\"name\":\"" ++ toString e ++ "\"}"

/-- Driver. -/
def main : IO Unit := do
  let stdin ← IO.getStdin
  let stdout ← IO.getStdout
  let mut i : Nat := 0
  let mut buf : Array String := #[]
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

end Apkaxiom.Tv.SigEval

def main : IO Unit := Apkaxiom.Tv.SigEval.main
