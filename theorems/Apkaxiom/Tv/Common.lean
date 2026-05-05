/-
P1.9 §IV — shared TV utilities (hex encode/decode, JSON escaping).

Pulled out of `Apkaxiom.Tv.LfhEval` so the EOCD / CDR evaluators
can `import Apkaxiom.Tv.Common` without inheriting LfhEval's
top-level `def main` (Lake `lean_exe` would pick the imported
module's `main` over the importing module's, breaking the entry
point).
-/

import Std

namespace Apkaxiom.Tv.Common

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

end Apkaxiom.Tv.Common
