// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Dialect-agnostic kernel of AXIOM-IR.
//!
//! The kernel pins five primitives that every dialect uses:
//!
//! | Primitive    | Purpose |
//! |--------------|---------|
//! | [`Module`]    | Top-level container; carries dialect tag, attrs, and a region |
//! | [`Region`]    | Ordered sequence of blocks (only one block in v0.1, but the SSA shape is reserved) |
//! | [`Block`]     | Ordered sequence of operations |
//! | [`Operation`] | A named op with operands, results, attributes, and an optional nested region |
//! | [`Value`]     | An SSA value: an [`Type`] plus a [`ValueId`] |
//!
//! Plus two leaf primitives:
//!
//! | Primitive    | Purpose |
//! |--------------|---------|
//! | [`Type`]      | Type expression — either a scalar or a constructor (`List<T>`, `Ref<T>`) |
//! | [`Attribute`] | Compile-time-known attribute on an op (string keys, typed values) |
//!
//! And one Android-domain primitive:
//!
//! | Primitive    | Purpose |
//! |--------------|---------|
//! | [`Tribool`]   | True / False / Default — Android's pervasive "exported" / "enabled" semantics |
//!
//! ## Stability
//!
//! * Variant order is part of canonical bytes — re-ordering is a wire-format
//!   break and requires a schema-version bump.
//! * New variants are appended after a `0xFE` "extension marker" tag: any
//!   v0.1 reader that encounters an extension tag returns
//!   [`IrError::UnknownExtension`] rather than silently truncating, so
//!   forward-incompatible bytes never decode as silently-valid IR.

use std::collections::BTreeMap;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Tribool
// ---------------------------------------------------------------------------

/// Three-valued Android-manifest boolean.
///
/// Android's manifest pervasively distinguishes between an attribute being
/// set to `true`, set to `false`, and *unset* (where the platform decides
/// from context). [`Tribool::Default`] is **not** the same as `False`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Tribool {
    /// Explicitly `true` in the source manifest.
    True,
    /// Explicitly `false`.
    False,
    /// Attribute absent — platform decides per Android's rules.
    Default,
}

impl Tribool {
    /// Stable canonical-byte tag.
    pub(crate) const TAG_TRUE: u8 = 1;
    pub(crate) const TAG_FALSE: u8 = 2;
    pub(crate) const TAG_DEFAULT: u8 = 3;

    /// Returns the canonical tag for canonical-bytes encoding.
    #[must_use]
    pub const fn tag(self) -> u8 {
        match self {
            Self::True => Self::TAG_TRUE,
            Self::False => Self::TAG_FALSE,
            Self::Default => Self::TAG_DEFAULT,
        }
    }

    /// Inverse of [`Self::tag`]. Errors on unknown tags.
    ///
    /// # Errors
    /// Returns [`IrError::BadTag`] if the byte is not a known tribool tag.
    pub const fn from_tag(b: u8) -> Result<Self, IrError> {
        match b {
            Self::TAG_TRUE => Ok(Self::True),
            Self::TAG_FALSE => Ok(Self::False),
            Self::TAG_DEFAULT => Ok(Self::Default),
            _ => Err(IrError::BadTag {
                ctx: "Tribool",
                tag: b,
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Type
// ---------------------------------------------------------------------------

/// Type expression in the IR.
///
/// The set is closed for v0.1; any new constructor requires a schema bump.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Type {
    /// Three-valued boolean (see [`Tribool`]).
    Tribool,
    /// Unsigned 32-bit integer.
    U32,
    /// Signed 32-bit integer.
    I32,
    /// Owned UTF-8 string.
    String,
    /// Owned byte vector.
    Bytes,
    /// Symbolic reference into the resource dialect.
    ResourceRef,
    /// Permission reference (manifest dialect).
    PermissionRef,
    /// Component name (manifest dialect).
    ComponentName,
    /// API level (1..=255).
    ApiLevel,
    /// Homogeneous list — `List<T>`.
    List(Box<Type>),
    /// Optional value — `Option<T>`.
    Option(Box<Type>),
}

impl Type {
    /// Returns `true` if `self` is a scalar (no inner type).
    #[must_use]
    pub const fn is_scalar(&self) -> bool {
        matches!(
            self,
            Self::Tribool
                | Self::U32
                | Self::I32
                | Self::String
                | Self::Bytes
                | Self::ResourceRef
                | Self::PermissionRef
                | Self::ComponentName
                | Self::ApiLevel
        )
    }

    /// Stable canonical tag (single byte).
    #[must_use]
    pub const fn tag(&self) -> u8 {
        match self {
            Self::Tribool => 0x10,
            Self::U32 => 0x11,
            Self::I32 => 0x12,
            Self::String => 0x13,
            Self::Bytes => 0x14,
            Self::ResourceRef => 0x15,
            Self::PermissionRef => 0x16,
            Self::ComponentName => 0x17,
            Self::ApiLevel => 0x18,
            Self::List(_) => 0x80,
            Self::Option(_) => 0x81,
        }
    }
}

// ---------------------------------------------------------------------------
// Attribute
// ---------------------------------------------------------------------------

/// Compile-time-known attribute on an [`Operation`] or a [`Module`].
///
/// Attributes carry constants that aren't dataflow values (which are
/// [`Value`]s). Their storage is sorted by key in [`Operation::attributes`]
/// so canonical bytes are independent of insertion order.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Attribute {
    /// Boolean (two-valued — distinct from [`Tribool`]).
    Bool(bool),
    /// Tribool.
    Tribool(Tribool),
    /// 32-bit unsigned integer.
    U32(u32),
    /// 32-bit signed integer.
    I32(i32),
    /// UTF-8 string.
    String(String),
    /// Raw bytes.
    Bytes(Vec<u8>),
    /// API level (Android L=21 .. U=34 .. V=35 ..).
    ApiLevel(u8),
}

impl Attribute {
    /// Stable canonical tag.
    #[must_use]
    pub const fn tag(&self) -> u8 {
        match self {
            Self::Bool(_) => 0x20,
            Self::Tribool(_) => 0x21,
            Self::U32(_) => 0x22,
            Self::I32(_) => 0x23,
            Self::String(_) => 0x24,
            Self::Bytes(_) => 0x25,
            Self::ApiLevel(_) => 0x26,
        }
    }
}

// ---------------------------------------------------------------------------
// Value
// ---------------------------------------------------------------------------

/// SSA-style identifier for a [`Value`].
///
/// IDs are unique within their owning [`Module`] (allocator is monotonic).
/// They are *not* stable across distinct modules — canonical bytes
/// re-allocate IDs in DFS post-order so structurally-equal modules produce
/// equal canonical bytes regardless of original allocation history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ValueId(pub u32);

/// SSA value: typed identifier produced by an [`Operation`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Value {
    /// Identifier within the owning module.
    pub id: ValueId,
    /// Static type of this value.
    pub ty: Type,
}

// ---------------------------------------------------------------------------
// Operation
// ---------------------------------------------------------------------------

/// A single operation in the IR.
///
/// Operations carry:
/// * a fully-qualified `name` (`"<dialect>.<op>"` — e.g. `"manifest.activity"`),
/// * `operands` — input [`Value`]s by ID,
/// * `results` — output [`Value`]s,
/// * `attributes` — string-keyed compile-time-known constants (sorted),
/// * `regions` — nested [`Region`]s (used by control-flow ops).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Operation {
    /// Fully-qualified name: `"<dialect>.<op>"`.
    pub name: String,
    /// Input value IDs.
    pub operands: Vec<ValueId>,
    /// Output values.
    pub results: Vec<Value>,
    /// Sorted attributes (`BTreeMap` so canonical-bytes order is fixed).
    pub attributes: BTreeMap<String, Attribute>,
    /// Nested regions.
    pub regions: Vec<Region>,
}

impl Operation {
    /// Construct a leaf operation with no regions.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            operands: Vec::new(),
            results: Vec::new(),
            attributes: BTreeMap::new(),
            regions: Vec::new(),
        }
    }

    /// Builder: attach an attribute.
    #[must_use]
    pub fn with_attr(mut self, key: impl Into<String>, value: Attribute) -> Self {
        self.attributes.insert(key.into(), value);
        self
    }

    /// Builder: attach a result value.
    #[must_use]
    pub fn with_result(mut self, v: Value) -> Self {
        self.results.push(v);
        self
    }

    /// Builder: attach a nested region.
    #[must_use]
    pub fn with_region(mut self, r: Region) -> Self {
        self.regions.push(r);
        self
    }

    /// Builder: attach an operand.
    #[must_use]
    pub fn with_operand(mut self, id: ValueId) -> Self {
        self.operands.push(id);
        self
    }
}

// ---------------------------------------------------------------------------
// Block / Region
// ---------------------------------------------------------------------------

/// Block — an ordered sequence of [`Operation`]s.
///
/// Blocks carry a stable `label` (used in MLIR-style text format) and an
/// ordered list of operations.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Block {
    /// Block label (e.g. `"entry"`).
    pub label: String,
    /// Operations in order.
    pub ops: Vec<Operation>,
}

impl Block {
    /// Construct a labelled block.
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            ops: Vec::new(),
        }
    }

    /// Builder: append an operation.
    #[must_use]
    pub fn with_op(mut self, op: Operation) -> Self {
        self.ops.push(op);
        self
    }
}

/// Region — an ordered sequence of [`Block`]s.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Region {
    /// Blocks in order.
    pub blocks: Vec<Block>,
}

impl Region {
    /// Construct a region from a single entry block.
    #[must_use]
    pub fn single(entry: Block) -> Self {
        Self {
            blocks: vec![entry],
        }
    }
}

// ---------------------------------------------------------------------------
// Module
// ---------------------------------------------------------------------------

/// Top-level IR container.
///
/// A module pins a `dialect_tag` (`"manifest"`, `"resource"`, or `"mixed"`),
/// a sorted attribute map, and a single root region.
///
/// Stability: the `producer` field is always [`crate::PRODUCER_TAG`] —
/// downstream consumers may use it for version detection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    /// Producer tag — pinned to [`crate::PRODUCER_TAG`].
    pub producer: String,
    /// Dialect family: `"manifest"`, `"resource"`, or `"mixed"`.
    pub dialect_tag: String,
    /// Top-level attributes (sorted).
    pub attributes: BTreeMap<String, Attribute>,
    /// Root region. Per v0.1, exactly one region with one entry block.
    pub region: Region,
    /// Monotonic value-id allocator state. Persists across canonical bytes
    /// so unmodified roundtrips preserve IDs.
    pub next_value_id: u32,
}

impl Module {
    /// Construct an empty module with the given dialect tag.
    #[must_use]
    pub fn empty(dialect_tag: impl Into<String>) -> Self {
        Self {
            producer: crate::PRODUCER_TAG.to_string(),
            dialect_tag: dialect_tag.into(),
            attributes: BTreeMap::new(),
            region: Region::single(Block::new("entry")),
            next_value_id: 0,
        }
    }

    /// Builder: set a top-level attribute.
    #[must_use]
    pub fn with_attr(mut self, key: impl Into<String>, value: Attribute) -> Self {
        self.attributes.insert(key.into(), value);
        self
    }

    /// Allocate a fresh [`Value`] of the given type.
    ///
    /// # Panics
    /// Panics if the value-id allocator overflows `u32::MAX` — the
    /// module would need >4 billion values, which the canonical-bytes
    /// 4 GiB invariant excludes.
    pub fn fresh(&mut self, ty: Type) -> Value {
        let id = ValueId(self.next_value_id);
        self.next_value_id = self
            .next_value_id
            .checked_add(1)
            .expect("ValueId allocator overflow — module too large");
        Value { id, ty }
    }

    /// Append an op to the entry block.
    ///
    /// # Panics
    /// Only on a malformed module that has no entry block — but
    /// [`Module::empty`] always inserts one and this crate never
    /// removes it, so the panic is unreachable in practice.
    pub fn push(&mut self, op: Operation) {
        self.region
            .blocks
            .first_mut()
            .expect("Module always has at least one block — see Module::empty")
            .ops
            .push(op);
    }

    /// Iterator over all operations in the entry block.
    pub fn ops(&self) -> impl Iterator<Item = &Operation> {
        self.region.blocks.iter().flat_map(|b| b.ops.iter())
    }
}

// ---------------------------------------------------------------------------
// Diagnostic / Error
// ---------------------------------------------------------------------------

/// Diagnostic emitted by IR construction or lowering.
///
/// Diagnostics carry a `severity` and `message`. Lowering passes
/// accumulate diagnostics rather than failing fast — see
/// [`crate::lowering`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Severity level: `error`, `warning`, `info`.
    pub severity: Severity,
    /// Diagnostic message.
    pub message: String,
}

/// Diagnostic severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Severity {
    /// Hard error — the IR is invalid.
    Error,
    /// Recoverable warning.
    Warning,
    /// Informational note.
    Info,
}

/// Errors surfaced by the IR machinery.
#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum IrError {
    /// Canonical-bytes decoder hit a tag it doesn't know.
    #[error("unknown tag 0x{tag:02x} in context {ctx}")]
    BadTag {
        /// Free-text context for the bad tag.
        ctx: &'static str,
        /// The offending byte.
        tag: u8,
    },
    /// Canonical-bytes input ended mid-record.
    #[error("unexpected EOF: needed {needed} byte(s) at offset {offset}")]
    UnexpectedEof {
        /// Bytes still required.
        needed: usize,
        /// Offset at which the buffer ran out.
        offset: usize,
    },
    /// Canonical-bytes magic header didn't match.
    #[error("bad canonical-bytes magic: expected 'AXIR' found {found:?}")]
    BadMagic {
        /// The four bytes seen at offset 0.
        found: [u8; 4],
    },
    /// Wire-format extension marker — IR was produced by a newer schema.
    #[error("unknown extension marker — schema is newer than this reader's v{schema_version}")]
    UnknownExtension {
        /// Reader-side schema version.
        schema_version: &'static str,
    },
    /// Canonical-bytes UTF-8 sub-record was invalid.
    #[error("invalid UTF-8 in canonical bytes at offset {offset}")]
    BadUtf8 {
        /// Offset where the invalid string started.
        offset: usize,
    },
    /// Generic invariant violation reported by lowering or validation.
    #[error("invariant violated: {0}")]
    Invariant(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_alloc_is_monotonic() {
        let mut m = Module::empty("manifest");
        let a = m.fresh(Type::U32);
        let b = m.fresh(Type::U32);
        assert_eq!(a.id.0, 0);
        assert_eq!(b.id.0, 1);
        assert_eq!(m.next_value_id, 2);
    }

    #[test]
    fn tribool_roundtrips_through_tag() {
        for t in [Tribool::True, Tribool::False, Tribool::Default] {
            assert_eq!(Tribool::from_tag(t.tag()).unwrap(), t);
        }
    }

    #[test]
    fn tribool_rejects_bad_tag() {
        assert!(matches!(Tribool::from_tag(99), Err(IrError::BadTag { .. })));
    }

    #[test]
    fn type_tags_are_unique() {
        // Catches accidental duplicate tag assignment if a future PR adds a
        // Type variant without bumping the schema version.
        let scalars = [
            Type::Tribool,
            Type::U32,
            Type::I32,
            Type::String,
            Type::Bytes,
            Type::ResourceRef,
            Type::PermissionRef,
            Type::ComponentName,
            Type::ApiLevel,
        ];
        let constructors = [
            Type::List(Box::new(Type::U32)),
            Type::Option(Box::new(Type::U32)),
        ];
        let mut seen = Vec::new();
        for t in scalars.iter().chain(constructors.iter()) {
            assert!(!seen.contains(&t.tag()), "duplicate tag for {t:?}");
            seen.push(t.tag());
        }
    }

    #[test]
    fn attribute_tags_are_unique() {
        let attrs = [
            Attribute::Bool(true),
            Attribute::Tribool(Tribool::Default),
            Attribute::U32(0),
            Attribute::I32(0),
            Attribute::String(String::new()),
            Attribute::Bytes(Vec::new()),
            Attribute::ApiLevel(0),
        ];
        let mut seen = Vec::new();
        for a in &attrs {
            assert!(!seen.contains(&a.tag()), "duplicate tag for {a:?}");
            seen.push(a.tag());
        }
    }

    #[test]
    fn operation_builder_is_chainable() {
        let mut m = Module::empty("manifest");
        let v = m.fresh(Type::U32);
        let op = Operation::new("manifest.dummy")
            .with_attr("k", Attribute::U32(42))
            .with_result(v);
        assert_eq!(op.name, "manifest.dummy");
        assert_eq!(op.results.len(), 1);
        assert!(op.attributes.contains_key("k"));
    }
}
