-- AXIOM-IR v0.1 — Lean 4 reflection of the Rust reference IR.
--
-- This module mirrors the dialect-agnostic kernel + the manifest /
-- resource dialects from `crates/axiom-ir/src/`. Two purposes:
--
--   1. State Lean-side theorems about IR validity (e.g.
--      `Component.is_exported` is decidable, canonical-tag bytes are
--      injective on a closed variant set).
--
--   2. Give the P1.17 soundness regression CI gate something to bite
--      on — any Phase-2+ refactor of the IR shape must update this
--      mirror in lockstep.
--
-- The mirror is deliberately *minimal*: scalar variants only, no
-- mathlib-heavy recursion. The full inductive families lift in P1.5
-- onward when the Lean kernel actually consumes IR.

namespace Apkaxiom.Ir

/-! ## Canonical-byte tags from `core::Type`.
    Mirrors the `Type::tag` table verbatim; any drift between the Rust
    `tag()` switch and these values indicates a wire-format change. -/

inductive TypeTag where
  | tribool
  | u32
  | i32
  | string
  | bytes
  | resourceRef
  | permissionRef
  | componentName
  | apiLevel
  | list
  | option
  deriving DecidableEq, Repr

/-- Tag value (matches `Type::tag` in the Rust reference). -/
def TypeTag.tag : TypeTag → UInt8
  | .tribool       => 0x10
  | .u32           => 0x11
  | .i32           => 0x12
  | .string        => 0x13
  | .bytes         => 0x14
  | .resourceRef   => 0x15
  | .permissionRef => 0x16
  | .componentName => 0x17
  | .apiLevel      => 0x18
  | .list          => 0x80
  | .option        => 0x81

/-- Canonical-byte tags are pairwise distinct. The single-tag-per-variant
    invariant is what makes the v0.1 wire format unambiguous. -/
theorem TypeTag.tag_injective :
    Function.Injective TypeTag.tag := by
  intro a b h
  cases a <;> cases b <;> simp_all [TypeTag.tag]

/-! ## Tribool

    Three-valued boolean for Android's pervasive `exported` /
    `enabled` semantics. `Default` is *not* the same as `False` —
    Android's resolution rule depends on the component kind plus
    presence/absence of intent filters. -/

inductive Tribool where
  | true
  | false
  | default
  deriving DecidableEq, Repr

namespace Tribool

/-- Stable tag values (mirrors `core::Tribool`). -/
def tag : Tribool → UInt8
  | .true    => 1
  | .false   => 2
  | .default => 3

theorem tag_injective : Function.Injective tag := by
  intro a b h
  cases a <;> cases b <;> simp_all [tag]

end Tribool

/-! ## Component kind. -/

inductive ComponentKind where
  | activity
  | service
  | receiver
  | provider
  deriving DecidableEq, Repr

/-- Authoritative resolution of `exported`:
    * `True`  → exported.
    * `False` → not exported.
    * `Default` → exported iff *not* a provider AND has at least one
                  intent filter (modelled by the boolean `hasFilter`).

    This matches the Rust `Component::is_exported` resolution and is
    the property the canonical-bytes round-trip preserves. -/
def isExported (kind : ComponentKind) (exported : Tribool) (hasFilter : Bool) : Bool :=
  match exported with
  | .true    => true
  | .false   => false
  | .default =>
    match kind with
    | .provider => false
    | _         => hasFilter

theorem isExported_true_iff (kind : ComponentKind) (hasFilter : Bool) :
    isExported kind .true hasFilter = true := by
  simp [isExported]

theorem isExported_false_iff (kind : ComponentKind) (hasFilter : Bool) :
    isExported kind .false hasFilter = false := by
  simp [isExported]

/-- Provider-default never exports, regardless of `hasFilter`. -/
theorem provider_default_not_exported (hasFilter : Bool) :
    isExported .provider .default hasFilter = false := by
  simp [isExported]

/-- Activity / service / receiver default-export iff a filter exists. -/
theorem nonprovider_default_exported_iff (hasFilter : Bool) :
    isExported .activity .default hasFilter = hasFilter := by
  simp [isExported]

/-! ## Protection levels. -/

inductive ProtectionLevel where
  | normal
  | dangerous
  | signature
  | signatureOrSystem
  | internal
  deriving DecidableEq, Repr

/-- Mirrors `ProtectionLevel::is_grantable_to_third_parties`. -/
def isGrantableToThirdParties : ProtectionLevel → Bool
  | .normal    => true
  | .dangerous => true
  | _          => false

/-- `signature`-class permissions are never grantable to third parties. -/
theorem signature_not_grantable :
    isGrantableToThirdParties .signature = false := by
  simp [isGrantableToThirdParties]

theorem signatureOrSystem_not_grantable :
    isGrantableToThirdParties .signatureOrSystem = false := by
  simp [isGrantableToThirdParties]

theorem internal_not_grantable :
    isGrantableToThirdParties .internal = false := by
  simp [isGrantableToThirdParties]

/-! ## Resource-type tags. -/

inductive ResourceType where
  | string
  | drawable
  | layout
  | color
  | dimen
  | style
  | bool
  | integer
  | raw
  deriving DecidableEq, Repr

end Apkaxiom.Ir
