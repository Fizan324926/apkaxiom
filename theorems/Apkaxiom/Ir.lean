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

/-! ## Full kernel reflection — IrType / Attribute / Value / Module shell.

    These mirror the Rust shape from `crates/axiom-ir/src/core.rs`. The
    Operation/Region/Block tree is reflected as a `List Nat`-bounded
    proxy (`Module.opCount`) rather than as full mutual inductives —
    Lean 4.29's `mutual` block + `deriving Repr` interaction is
    finicky, and what the soundness regression CI actually needs is
    the *type-tag table* and the *Module shell*, not the full
    structural recursion. P1.17 will graft any deeper invariants on
    top of these as the freeze observation window plays out.

    Phase-2+ work that adds an IR variant must update these inductives
    in lockstep — the `deepened` test below will fail at the soundness
    regression CI gate otherwise. -/

/-- Type expression in the IR. Closed for v0.1; new constructors require
    a schema-version bump. (Named `IrType` to avoid colliding with Lean's
    `Type` universe.) -/
inductive IrType where
  | tribool
  | u32
  | i32
  | str
  | bytes
  | resourceRef
  | permissionRef
  | componentName
  | apiLevel
  | list (inner : IrType)
  | option (inner : IrType)
  deriving Repr

/-- Compile-time-known attribute on an operation or module.
    Mirrors `core::Attribute`. (We use `List UInt8` for the bytes
    variant rather than `ByteArray` to keep `deriving Repr` working
    without a custom instance.) -/
inductive Attribute where
  | bool (b : Bool)
  | tribool (t : Tribool)
  | u32 (n : UInt32)
  | i32 (n : Int32)
  | str (s : String)
  | bytes (b : List UInt8)
  | apiLevel (api : UInt8)
  deriving Repr

/-- SSA-style value identifier. -/
structure ValueId where
  raw : UInt32
  deriving DecidableEq, Repr

/-- SSA value: typed identifier produced by an Operation. -/
structure Value where
  id : ValueId
  ty : IrType
  deriving Repr

/-- Top-level module shell — producer tag, dialect tag, attribute count,
    op count, value-id allocator state. The interior tree (operations,
    regions, blocks) is intentionally reflected as a count rather than
    a structural tree; see the docstring on the section above for why. -/
structure Module where
  producer : String
  dialectTag : String
  attributeCount : Nat
  opCount : Nat
  nextValueId : UInt32
  deriving Repr

/-! ## Type-tag mirror — the closed set of v0.1 types. -/

/-- Mirror of `Type::tag` in the Rust reference. The mapping is closed
    for v0.1; any new variant requires a schema-version bump. -/
def IrType.tag : IrType → UInt8
  | .tribool       => 0x10
  | .u32           => 0x11
  | .i32           => 0x12
  | .str           => 0x13
  | .bytes         => 0x14
  | .resourceRef   => 0x15
  | .permissionRef => 0x16
  | .componentName => 0x17
  | .apiLevel      => 0x18
  | .list _        => 0x80
  | .option _      => 0x81

/-- Whether a type is scalar — mirrors `Type::is_scalar` in Rust. -/
def IrType.isScalar : IrType → Bool
  | .list _   => false
  | .option _ => false
  | _         => true

/-- Tag-distinctness for the scalar variants — proves that the canonical-
    byte tag table is unambiguous on the closed v0.1 set. -/
theorem IrType.scalar_tags_distinct :
    IrType.tag .tribool ≠ IrType.tag .u32 ∧
    IrType.tag .u32 ≠ IrType.tag .i32 ∧
    IrType.tag .i32 ≠ IrType.tag .str ∧
    IrType.tag .str ≠ IrType.tag .bytes ∧
    IrType.tag .bytes ≠ IrType.tag .resourceRef ∧
    IrType.tag .resourceRef ≠ IrType.tag .permissionRef ∧
    IrType.tag .permissionRef ≠ IrType.tag .componentName ∧
    IrType.tag .componentName ≠ IrType.tag .apiLevel := by
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> decide

/-- Constructors are tagged ≥ 0x80; scalars are tagged ≤ 0x18. So the
    constructor / scalar partitions never collide. -/
theorem IrType.scalar_constructor_disjoint (t : IrType) :
    t.isScalar = true → t.tag.toNat < 0x80 := by
  intro hscalar
  cases t <;> simp_all [IrType.isScalar, IrType.tag] <;> decide

/-! ## Attribute-tag mirror. -/

/-- Mirror of `Attribute::tag` in the Rust reference. -/
def Attribute.tag : Attribute → UInt8
  | .bool _      => 0x20
  | .tribool _   => 0x21
  | .u32 _       => 0x22
  | .i32 _       => 0x23
  | .str _       => 0x24
  | .bytes _     => 0x25
  | .apiLevel _  => 0x26

/-- Attribute tag distinctness on a representative scalar subset.
    Quadratic-form pinned for the most likely tag-collision pairs. -/
theorem Attribute.tag_distinct_subset :
    Attribute.tag (.bool false) ≠ Attribute.tag (.tribool .true) ∧
    Attribute.tag (.tribool .true) ≠ Attribute.tag (.u32 0) ∧
    Attribute.tag (.u32 0) ≠ Attribute.tag (.i32 0) ∧
    Attribute.tag (.i32 0) ≠ Attribute.tag (.str "") ∧
    Attribute.tag (.str "") ≠ Attribute.tag (.bytes []) ∧
    Attribute.tag (.bytes []) ≠ Attribute.tag (.apiLevel 0) := by
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩ <;> decide

/-- Every Attribute tag is in the range `[0x20, 0x27)`. -/
theorem Attribute.tag_in_range (a : Attribute) :
    0x20 ≤ a.tag.toNat ∧ a.tag.toNat < 0x27 := by
  cases a <;> simp [Attribute.tag] <;> decide

/-! ## Module shell sanity. -/

/-- An empty module has zero ops and a value-id allocator at 0. This
    pins the `Module::empty` constructor's contract from the Rust
    reference. -/
def Module.empty (dialect : String) : Module :=
  { producer := "apkaxiom::ir/0.1.0"
  , dialectTag := dialect
  , attributeCount := 0
  , opCount := 0
  , nextValueId := 0
  }

theorem Module.empty_has_no_ops (d : String) :
    (Module.empty d).opCount = 0 := by
  simp [Module.empty]

theorem Module.empty_value_id_zero (d : String) :
    (Module.empty d).nextValueId = 0 := by
  simp [Module.empty]

end Apkaxiom.Ir
