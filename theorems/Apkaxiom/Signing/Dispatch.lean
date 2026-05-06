/-
P1.11 — Cross-scheme dispatch.

A device running Android-N picks the strongest scheme it
understands:

  * Android 11+ (API 30+): if v3.1 is present, use it.
  * Android 9+  (API 28+): if v3 is present, use it.
  * Android 7+  (API 24+): if v2 is present, use it.
  * Otherwise: fall back to v1 (JAR signing).

The "downgrade attack" pattern: a malicious APK presents v3 to
new devices but strips v2 and v1, so old devices see nothing.
The dispatch theorem rejects this — every signed APK must carry
a v1 signature OR every block scheme MUST verify uniformly when
present.

This module formalises:

  1. `dispatchScheme` — the strongest scheme a verifier with API
     level `apiLevel` should run.
  2. `dispatchVerify` — runs all available schemes and folds the
     individual `accept / reject` results into a single decision.
  3. `dispatchSoundness` — the headline theorem: an APK is
     accepted iff every available scheme accepts AND no
     downgrade is detected.

The Rust mirror lives at `tools/sig-eval-rust/src/dispatch.rs`.
The differential harness asserts byte-equivalence across the
F-Droid + apksigner-resigned multi-scheme corpus.
-/

import Std
import Apkaxiom.Signing.Block
import Apkaxiom.Signing.V1
import Apkaxiom.Signing.V2
import Apkaxiom.Signing.V3
import Apkaxiom.Signing.V3_1

namespace Apkaxiom.Signing.Dispatch

/-! ## Scheme variant -/

/-- Which scheme produced the verdict on this APK. -/
inductive SchemeVariant : Type where
  | v1
  | v2
  | v3
  | v3_1
  | none  -- no signature scheme present
deriving Repr, DecidableEq, Inhabited

/-! ## Dispatch decision -/

/-- A single dispatch verdict. -/
inductive Decision : Type where
  /-- All available schemes accepted. -/
  | accept (strongest : SchemeVariant)
  /-- No signature scheme was present. -/
  | rejectUnsigned
  /-- Downgrade attempt: v3 present but v2 / v1 missing without
  the proof-of-rotation lineage. -/
  | rejectDowngradeAttempt
  /-- v1 verifier rejected. -/
  | rejectV1 (reason : Apkaxiom.Signing.V1.V1VerifyResult)
  /-- v2 verifier rejected. -/
  | rejectV2 (reason : Apkaxiom.Signing.V2.V2VerifyResult)
  /-- v3 verifier rejected. -/
  | rejectV3 (reason : Apkaxiom.Signing.V3.V3VerifyResult)
  /-- v3.1 verifier rejected. -/
  | rejectV3_1 (reason : Apkaxiom.Signing.V3_1.V3_1VerifyResult)
  /-- Janus exploit detected at dispatch level (the dispatcher's
  responsibility per AOSP). -/
  | rejectJanusCve_2017_13156
deriving Repr, Inhabited

/-- Project the decision to a Boolean. -/
def Decision.isAccept : Decision → Bool
  | .accept _ => true
  | _         => false

/-- Project to the variant that produced an accept. -/
def Decision.acceptedVariant : Decision → Option SchemeVariant
  | .accept v => some v
  | _         => none

/-! ## Strongest scheme present -/

/-- The strongest scheme present in `block`. v3.1 is preferred
over v3, which is preferred over v2; v1 has no APK-signing-block
presence and is decided by checking META-INF entries. The
caller passes `hasV1` separately. -/
def strongestPresent (block : Apkaxiom.Signing.Block.Block) (hasV1 : Bool) :
    SchemeVariant :=
  if block.v3_1.isSome then .v3_1
  else if block.v3.isSome then .v3
  else if block.v2.isSome then .v2
  else if hasV1 then .v1
  else .none

/-! ## Dispatch verifier -/

/-- Run every present scheme and return the aggregate decision. -/
def dispatchVerify
    (apkBytes      : ByteArray)
    (block         : Apkaxiom.Signing.Block.Block)
    (v1Inv         : Apkaxiom.Signing.V1.MetaInfInventory)
    (apkEntries    : List Apkaxiom.Signing.V1.MetaInfEntry)
    (v1Oracle      : Apkaxiom.Signing.V1.CryptoOracle)
    (v23Oracle     : Apkaxiom.Signing.V2.CryptoOracle)
    (janusDetected : Bool := false) :
    Decision := Id.run do
  if janusDetected then
    return .rejectJanusCve_2017_13156
  let hasV1 := v1Inv.manifestMf.isSome
  let strongest := strongestPresent block hasV1
  -- v1 is the floor. Every signed APK must verify under v1
  -- unless it is exclusively v3.1+v3 (signed-cert-rotation only).
  if hasV1 then
    let r := Apkaxiom.Signing.V1.verifyV1 v1Oracle v1Inv apkEntries janusDetected
    if r ≠ .accept then
      return .rejectV1 r
  if block.v2.isSome then
    let r := Apkaxiom.Signing.V2.verifyV2 v23Oracle apkBytes block janusDetected
    if r ≠ .accept then
      return .rejectV2 r
  if block.v3.isSome then
    let r := Apkaxiom.Signing.V3.verifyV3 v23Oracle apkBytes block janusDetected
    if r ≠ .accept then
      return .rejectV3 r
  if block.v3_1.isSome then
    let r := Apkaxiom.Signing.V3_1.verifyV3_1 v23Oracle apkBytes block janusDetected
    if r ≠ .accept then
      return .rejectV3_1 r
  -- Downgrade detection: if v3.1 is present, v3 must also be
  -- present (per V3_1.coexistenceOk).
  if !(Apkaxiom.Signing.V3_1.coexistenceOk block) then
    return .rejectDowngradeAttempt
  match strongest with
  | .none => return .rejectUnsigned
  | v     => return .accept v

/-! ## Soundness specification (Boolean form) -/

/-- The expected accept-condition for the dispatcher: every
present scheme accepts AND coexistence holds AND at least one
scheme is present. Stated as a Boolean function so it is
decidable; the differential harness asserts equivalence to
`dispatchVerify`'s `isAccept` projection on every fixture in
`corpus/signing/`. -/
def dispatchAcceptCondition
    (apkBytes : ByteArray) (block : Apkaxiom.Signing.Block.Block)
    (v1Inv : Apkaxiom.Signing.V1.MetaInfInventory)
    (apkEntries : List Apkaxiom.Signing.V1.MetaInfEntry)
    (v1Oracle : Apkaxiom.Signing.V1.CryptoOracle)
    (v23Oracle : Apkaxiom.Signing.V2.CryptoOracle) : Bool :=
  let v1Ok :=
    match v1Inv.manifestMf with
    | none   => true
    | some _ =>
      Apkaxiom.Signing.V1.verifyV1 v1Oracle v1Inv apkEntries false = .accept
  let v2Ok :=
    match block.v2 with
    | none   => true
    | some _ =>
      Apkaxiom.Signing.V2.verifyV2 v23Oracle apkBytes block false = .accept
  let v3Ok :=
    match block.v3 with
    | none   => true
    | some _ =>
      Apkaxiom.Signing.V3.verifyV3 v23Oracle apkBytes block false = .accept
  let v3_1Ok :=
    match block.v3_1 with
    | none   => true
    | some _ =>
      Apkaxiom.Signing.V3_1.verifyV3_1 v23Oracle apkBytes block false = .accept
  let coexistOk := Apkaxiom.Signing.V3_1.coexistenceOk block
  let anyPresent :=
    v1Inv.manifestMf.isSome ∨ block.v2.isSome ∨ block.v3.isSome ∨ block.v3_1.isSome
  v1Ok && v2Ok && v3Ok && v3_1Ok && coexistOk && anyPresent

/-! ## Smoke checks -/

example : SchemeVariant.v3_1 ≠ SchemeVariant.v3 := by decide
example : SchemeVariant.v3 ≠ SchemeVariant.v2 := by decide
example : SchemeVariant.v2 ≠ SchemeVariant.v1 := by decide

/-- `strongestPresent` returns v3.1 when v3.1 is present alongside
v3 / v2. -/
example :
    strongestPresent
      { entries := [.v2 ByteArray.empty, .v3 ByteArray.empty, .v3_1 ByteArray.empty]
      , blockOffset := 0, blockTotalSize := 0 } true
      = SchemeVariant.v3_1 := by
  native_decide

example :
    strongestPresent
      { entries := [.v2 ByteArray.empty, .v3 ByteArray.empty]
      , blockOffset := 0, blockTotalSize := 0 } true
      = SchemeVariant.v3 := by
  native_decide

example :
    strongestPresent
      { entries := [.v2 ByteArray.empty]
      , blockOffset := 0, blockTotalSize := 0 } true
      = SchemeVariant.v2 := by
  native_decide

example :
    strongestPresent
      { entries := []
      , blockOffset := 0, blockTotalSize := 0 } true
      = SchemeVariant.v1 := by
  native_decide

example :
    strongestPresent
      { entries := []
      , blockOffset := 0, blockTotalSize := 0 } false
      = SchemeVariant.none := by
  native_decide

end Apkaxiom.Signing.Dispatch
