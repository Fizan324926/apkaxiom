/-
P1.11 — APK Signature Scheme v3.1 verifier (block ID 0x1b93ad61).

v3.1 is a strict superset of v3:

  * Same wire format (length-prefixed signers, SDK range fields,
    digests / certificates / additional-attributes / signatures /
    public_key).

  * Different block ID (`0x1b93ad61` vs v3's `0xf05368c0`) so that
    devices that don't understand v3.1 leave the block alone.

  * Implies the rotation extension: a v3.1 signer is the
    *new* (post-rotation) key; the v3 block (still required to be
    present) carries the *old* key for compatibility with devices
    that don't understand v3.1.

  * `min_sdk` value is conventionally 33+ (the rotation gate
    introduced in Android 13), but the wire format allows any
    value — the verifier doesn't gate on this.

The verifier predicate IS the v3 verifier with a different block
ID. We re-export `verifySigner` and provide a `verifyV3_1`
top-level wrapper so the dispatch theorem can refer to v3.1 as
its own case.
-/

import Std
import Apkaxiom.Signing.Block
import Apkaxiom.Signing.Scheme
import Apkaxiom.Signing.V2
import Apkaxiom.Signing.V3

namespace Apkaxiom.Signing.V3_1

open Apkaxiom.Signing.Scheme (Signer parseV3_1)

/-- v3.1 verification result categories. v3.1 uses the v3 result
type — same failure modes since the verifier is the v3 verifier
on a different block ID. -/
abbrev V3_1VerifyResult := Apkaxiom.Signing.V3.V3VerifyResult

/-- The conventional v3.1 minimum-SDK gate. -/
def conventionalMinSdk : UInt32 := 33

/-- Top-level v3.1 verifier. Identical algorithm to v3 — only the
block lookup differs. -/
def verifyV3_1
    (oracle : Apkaxiom.Signing.V3.CryptoOracle)
    (apkBytes : ByteArray)
    (block : Apkaxiom.Signing.Block.Block)
    (janusDetected : Bool := false) :
    V3_1VerifyResult := Id.run do
  if janusDetected then
    return .rejectJanusCve_2017_13156
  let .some v3_1Bytes := block.v3_1
    | return .rejectNoV3Block
  let v3_1Entry := Apkaxiom.Signing.Block.Entry.v3_1 v3_1Bytes
  match parseV3_1 v3_1Bytes with
  | .error _ => return .rejectMalformed
  | .ok signers =>
    if signers.isEmpty then
      return .rejectMalformed
    for s in signers do
      let r := Apkaxiom.Signing.V3.verifySigner oracle apkBytes block v3_1Entry s
      if r ≠ .accept then
        return r
    return .accept

/-! ## v3 / v3.1 coexistence -/

/-- A well-formed v3.1-signed APK MUST also carry a v3 block (for
old-device compatibility). The dispatcher uses this invariant to
detect "v3.1 only" downgrade attempts. -/
def coexistenceOk (block : Apkaxiom.Signing.Block.Block) : Bool :=
  match block.v3_1 with
  | none      => true   -- no v3.1 block — invariant trivially holds
  | some _    =>
    match block.v3 with
    | none   => false  -- v3.1 without v3 → broken
    | some _ => true

/-! ## Smoke checks -/

example : conventionalMinSdk = 33 := by native_decide

/-- The v3 / v3.1 coexistence predicate accepts the empty block. -/
example :
    coexistenceOk { entries := [], blockOffset := 0, blockTotalSize := 0 }
      = true := by
  native_decide

end Apkaxiom.Signing.V3_1
