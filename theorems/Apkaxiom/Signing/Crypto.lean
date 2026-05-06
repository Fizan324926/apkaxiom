/-
P1.11 — HACL\* binding surface for APK signature verification.

This module declares the cryptographic-primitive interface that
every scheme verifier (V1/V2/V3/V3.1) is parameterised over.
Rationale (per ADR-0028 and ADR-0029):

  * HACL\*'s F\*-verified primitives — SHA-256, SHA-512, RSA-PKCS1,
    RSA-PSS, ECDSA, Ed25519 — are the *target* implementations.
  * HACL\* C build is a 30-minute cold operation requiring F\* +
    OCaml + opam infrastructure that lives outside the dev-shell.
    Per repo policy "operator one-shots are not gaps" the actual
    C-binding wiring is `P111-OP-1` in CHECKLIST §C.
  * In-session, the verifier is *fully specified* against an
    abstract oracle. The Rust mirror at
    `tools/sig-eval-rust` plugs in the audited Rust crates
    (`sha2`, `rsa`, `p256`, `ed25519-dalek`); the differential
    harness asserts byte-equivalence against AOSP `apksigner`.

When HACL\* lands, the abstract oracle is replaced with the
HACL\*-backed oracle, the structural Lean verifier is unchanged,
and CHECKLIST §B row 6 turns ✅.
-/

import Std

namespace Apkaxiom.Signing.Crypto

/-! ## Primitive identifiers -/

/-- A hash function the APK signing schemes use. -/
inductive HashAlgorithm : Type where
  /-- SHA-1 — accepted only by v1 (legacy). -/
  | sha1
  /-- SHA-256 — primary digest used by v2/v3/v3.1 chunked digest. -/
  | sha256
  /-- SHA-512 — used by the `0x0102` and `0x0104` algorithm IDs. -/
  | sha512
deriving Repr, DecidableEq, Inhabited

/-- A signature scheme that signs a hash digest. -/
inductive SignatureScheme : Type where
  /-- RSA-PKCS1-v1.5 over a hash digest. -/
  | rsaPkcs1 (hash : HashAlgorithm)
  /-- RSA-PSS over a hash digest. -/
  | rsaPss (hash : HashAlgorithm)
  /-- ECDSA over the NIST P-256 curve, hash digest. -/
  | ecdsaP256 (hash : HashAlgorithm)
  /-- DSA over a hash digest (legacy). -/
  | dsa (hash : HashAlgorithm)
  /-- Ed25519 — pure-EdDSA, no hash parameter. -/
  | ed25519
deriving Repr, DecidableEq, Inhabited

/-- Map a `SignatureScheme` to the underlying hash. Ed25519
returns SHA-512 internally but exposes `none` here because the
APK chunked-digest path doesn't use it directly. -/
def SignatureScheme.hashOf : SignatureScheme → Option HashAlgorithm
  | .rsaPkcs1 h     => some h
  | .rsaPss h       => some h
  | .ecdsaP256 h    => some h
  | .dsa h          => some h
  | .ed25519        => none

/-! ## HACL\* binding-surface oracle -/

/-- The HACL\*-verified-crypto oracle. Every scheme verifier
takes one of these as a parameter. The fields are spelled as
arrows so the verifier predicate is independent of the
implementation strategy (real HACL\* C, audited Rust, mock).

Tests parameterise this with a *mock* oracle that always
accepts; production verifiers parameterise with the
HACL\*-backed oracle wired by `tools/sig-eval-rust`. -/
structure Oracle where
  /-- Cryptographic hash. -/
  hash : HashAlgorithm → ByteArray → ByteArray
  /-- Signature verification: returns `true` iff `signature` is
  valid over `data` under `(scheme, publicKeyDer)`. -/
  verify : SignatureScheme → (publicKeyDer : ByteArray) →
            (data : ByteArray) → (signature : ByteArray) → Bool
  /-- Extract the SPKI DER from an X.509 leaf cert. The full
  X.509 parser sits behind this. -/
  extractSpki : (leafCertDer : ByteArray) → Option ByteArray

/-! ## Mock oracle -/

/-- A *mock* oracle that accepts every signature and returns
zero-hashes. Useful for property tests that need to walk the
verifier control-flow without a real crypto stack. NEVER use in
production. -/
def mockOracle : Oracle where
  hash _ _ := ByteArray.empty
  verify _ _ _ _ := true
  extractSpki _ := some ByteArray.empty

/-- A *deny-all* mock oracle. Verifies fail; useful for testing
reject paths. -/
def denyAllOracle : Oracle where
  hash _ _ := ByteArray.empty
  verify _ _ _ _ := false
  extractSpki _ := some ByteArray.empty

/-! ## Smoke checks -/

example : SignatureScheme.hashOf (.rsaPkcs1 .sha256) = some .sha256 := by native_decide
example : SignatureScheme.hashOf .ed25519 = none := by native_decide

example : (mockOracle.verify .ed25519 ByteArray.empty ByteArray.empty ByteArray.empty) = true := by
  native_decide
example : (denyAllOracle.verify .ed25519 ByteArray.empty ByteArray.empty ByteArray.empty) = false := by
  native_decide

/-! ## Algorithm identification -/

/-- v2/v3 wire `algorithm_id` → underlying signature scheme.
Matches the `Apkaxiom.Signing.Scheme.SignatureAlgorithmId` →
`Apkaxiom.Signing.Crypto.SignatureScheme` lift, via the wire IDs
assigned by AOSP. -/
def schemeFromWireAlgorithm (id : UInt32) : Option SignatureScheme :=
  if id = 0x0101 then some (.rsaPss .sha256)
  else if id = 0x0102 then some (.rsaPss .sha512)
  else if id = 0x0103 then some (.rsaPkcs1 .sha256)
  else if id = 0x0104 then some (.rsaPkcs1 .sha512)
  else if id = 0x0201 then some (.ecdsaP256 .sha256)
  else if id = 0x0202 then some (.ecdsaP256 .sha512)
  else if id = 0x0301 then some (.dsa .sha256)
  else if id = 0x0421 then some (.rsaPkcs1 .sha256)
  else if id = 0x0423 then some (.ecdsaP256 .sha256)
  else if id = 0x0425 then some (.dsa .sha256)
  else none

/-- All ten known wire IDs lift to a non-`none` scheme. -/
example :
    schemeFromWireAlgorithm 0x0101 = some (.rsaPss .sha256)
  ∧ schemeFromWireAlgorithm 0x0102 = some (.rsaPss .sha512)
  ∧ schemeFromWireAlgorithm 0x0103 = some (.rsaPkcs1 .sha256)
  ∧ schemeFromWireAlgorithm 0x0104 = some (.rsaPkcs1 .sha512)
  ∧ schemeFromWireAlgorithm 0x0201 = some (.ecdsaP256 .sha256)
  ∧ schemeFromWireAlgorithm 0x0202 = some (.ecdsaP256 .sha512)
  ∧ schemeFromWireAlgorithm 0x0301 = some (.dsa .sha256)
  ∧ schemeFromWireAlgorithm 0x0421 = some (.rsaPkcs1 .sha256)
  ∧ schemeFromWireAlgorithm 0x0423 = some (.ecdsaP256 .sha256)
  ∧ schemeFromWireAlgorithm 0x0425 = some (.dsa .sha256) := by
  native_decide

/-- Unknown wire IDs lift to `none`. -/
example : schemeFromWireAlgorithm 0xdeadbeef = none := by native_decide

/-! ## Output-length contracts -/

/-- Output bytes per hash. -/
def HashAlgorithm.outLen : HashAlgorithm → Nat
  | .sha1   => 20
  | .sha256 => 32
  | .sha512 => 64

example : HashAlgorithm.sha1.outLen = 20 := by native_decide
example : HashAlgorithm.sha256.outLen = 32 := by native_decide
example : HashAlgorithm.sha512.outLen = 64 := by native_decide

/-! ## Chunked-digest helper -/

/-- Build the chunked-digest input for one chunk per APK
Signature Scheme v2: `0xa5 || u32_le(chunk_len) || chunk_bytes`.
The leaf-level input fed to `Oracle.hash`. -/
def chunkInput (chunk : ByteArray) : ByteArray := Id.run do
  let len : UInt32 := UInt32.ofNat chunk.size
  let mut out : ByteArray := ByteArray.empty
  out := out.push 0xa5
  out := out.push (len.toUInt8)
  out := out.push ((len >>> 8).toUInt8)
  out := out.push ((len >>> 16).toUInt8)
  out := out.push ((len >>> 24).toUInt8)
  return out ++ chunk

/-- Build the root-level chunked-digest input: `0x5a ||
u32_le(num_chunks) || concat(chunk_digests)`. -/
def rootInput (numChunks : UInt32) (chunkDigests : ByteArray) :
    ByteArray := Id.run do
  let mut out : ByteArray := ByteArray.empty
  out := out.push 0x5a
  out := out.push numChunks.toUInt8
  out := out.push ((numChunks >>> 8).toUInt8)
  out := out.push ((numChunks >>> 16).toUInt8)
  out := out.push ((numChunks >>> 24).toUInt8)
  return out ++ chunkDigests

/-- The 1 MiB chunk size hardcoded by AOSP `tools/apksig`. -/
def chunkSize : Nat := 1 <<< 20  -- 1 MiB

example : chunkSize = 1048576 := by native_decide

end Apkaxiom.Signing.Crypto
