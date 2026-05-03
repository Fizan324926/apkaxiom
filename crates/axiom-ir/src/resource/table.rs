// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Resource table entries — types, IDs, refs, values.

/// AOSP resource type space — closed set for v0.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ResourceType {
    /// `R.string.*`
    String,
    /// `R.drawable.*`
    Drawable,
    /// `R.layout.*`
    Layout,
    /// `R.color.*`
    Color,
    /// `R.dimen.*`
    Dimen,
    /// `R.style.*`
    Style,
    /// `R.bool.*`
    Bool,
    /// `R.integer.*`
    Integer,
    /// `R.raw.*`
    Raw,
}

/// AOSP resource ID — typically `0x7f00_0000 | (type << 16) | index`.
///
/// We keep it as an opaque `u32` here; downstream consumers can decode
/// the bits if they need to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ResourceId(pub u32);

/// Canonical reference to a resource.
///
/// The triple `(type, id, name)` is *redundant* on purpose: it lets a
/// reader use whichever the producer-side has confidence in. Equality is
/// structural — two refs match only if all three fields agree.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceRef {
    /// Type (`String`, `Drawable`, …).
    pub r#type: ResourceType,
    /// Numeric ID.
    pub id: ResourceId,
    /// Symbolic name (e.g. `"app_name"`).
    pub name: String,
}

/// One row in the resource table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceEntry {
    /// Reference to this entry.
    pub ref_: ResourceRef,
    /// Value held by this entry.
    pub value: ResourceValue,
}

/// Inhabitant of a [`ResourceEntry`].
///
/// Closed for v0.1 — bitmaps, colours-with-alpha, complex types arrive
/// in v0.2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceValue {
    /// Plain string.
    String(String),
    /// Signed 32-bit integer (covers `R.integer`, `R.color` packed RGBA,
    /// dimension fixed-point — the consumer disambiguates from the
    /// owning [`ResourceType`]).
    Int(i32),
    /// Boolean.
    Bool(bool),
    /// Reference to another resource.
    Ref(ResourceRef),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ref_equality_is_structural() {
        let a = ResourceRef {
            r#type: ResourceType::String,
            id: ResourceId(0x7f00_0001),
            name: "app_name".into(),
        };
        let b = ResourceRef {
            r#type: ResourceType::String,
            id: ResourceId(0x7f00_0001),
            name: "app_name".into(),
        };
        assert_eq!(a, b);
        let c = ResourceRef {
            r#type: ResourceType::String,
            id: ResourceId(0x7f00_0002),
            name: "app_name".into(),
        };
        assert_ne!(a, c);
    }
}
