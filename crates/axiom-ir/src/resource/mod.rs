// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Resource dialect of AXIOM-IR v0.1.
//!
//! Models `resources.arsc`: a [`StringPool`], a list of
//! [`Configuration`]s (density / locale / orientation / sdk), and a list
//! of [`ResourceEntry`] records keyed by [`ResourceRef`].
//!
//! Like the manifest dialect, [`ResourceTable`]s wrap into the kernel
//! [`crate::Module`] shell via [`wrap_module`] / [`unwrap_module`].
//!
//! ## Out of scope (v0.1)
//!
//! * **Bit-perfect re-encoding** of `resources.arsc` is *not* a v0.1 goal
//!   — see `docs/phase-1/P1.4/CHECKLIST.md` §C-3. The dialect captures the
//!   *semantic* shape; round-tripping is checked at the AXIOM-IR level
//!   (canonical bytes), not at the AOSP-binary level. P1.15's emitter is
//!   what bridges to actual `.arsc` decoding.
//! * Complex configurations (UI mode, screen layout flags) are reserved
//!   for v0.2.

// Counts fit in `u32`; see canonical-bytes 4 GiB invariant.
#![allow(clippy::cast_possible_truncation)]

use std::collections::BTreeMap;

use crate::core::{Attribute, IrError, Module};

mod config;
mod string_pool;
mod table;

pub use config::Configuration;
pub use string_pool::StringPool;
pub use table::{ResourceEntry, ResourceId, ResourceRef, ResourceType, ResourceValue};

/// Top-level resource module.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResourceTable {
    /// Owning Java package (matches the manifest's package, almost
    /// always — but resource overlays can ship in a different package).
    pub package: String,
    /// Indexed string pool. All resource string-typed values are
    /// references *into* this pool.
    pub string_pool: StringPool,
    /// Configurations covered by this resource table.
    pub configurations: Vec<Configuration>,
    /// Resource entries.
    pub entries: Vec<ResourceEntry>,
}

impl ResourceTable {
    /// Empty resource table.
    #[must_use]
    pub fn new(package: impl Into<String>) -> Self {
        Self {
            package: package.into(),
            ..Default::default()
        }
    }

    /// Builder: append a configuration.
    #[must_use]
    pub fn with_configuration(mut self, c: Configuration) -> Self {
        self.configurations.push(c);
        self
    }

    /// Builder: append a resource entry.
    #[must_use]
    pub fn with_entry(mut self, e: ResourceEntry) -> Self {
        self.entries.push(e);
        self
    }

    /// Look up an entry by [`ResourceRef`]. Linear scan — fine for v0.1
    /// where tables are small; P1.15's emitter introduces a `HashMap`
    /// index when scale matters.
    #[must_use]
    pub fn lookup(&self, key: &ResourceRef) -> Option<&ResourceEntry> {
        self.entries.iter().find(|e| &e.ref_ == key)
    }
}

/// Wrap a [`ResourceTable`] into a kernel [`Module`].
#[must_use]
pub fn wrap_module(t: &ResourceTable) -> Module {
    let mut payload = Vec::with_capacity(256);
    crate::canonical::write_resource_payload(&mut payload, t);

    let mut attrs: BTreeMap<String, Attribute> = BTreeMap::new();
    attrs.insert("resource.payload".into(), Attribute::Bytes(payload));
    attrs.insert(
        "resource.package".into(),
        Attribute::String(t.package.clone()),
    );
    attrs.insert(
        "resource.string_pool_size".into(),
        Attribute::U32(t.string_pool.strings.len() as u32),
    );
    attrs.insert(
        "resource.entry_count".into(),
        Attribute::U32(t.entries.len() as u32),
    );
    attrs.insert(
        "resource.configuration_count".into(),
        Attribute::U32(t.configurations.len() as u32),
    );

    Module {
        producer: crate::PRODUCER_TAG.to_string(),
        dialect_tag: "resource".into(),
        attributes: attrs,
        region: crate::core::Region::single(crate::core::Block::new("entry")),
        next_value_id: 0,
    }
}

/// Inverse of [`wrap_module`].
///
/// # Errors
/// Returns [`IrError::Invariant`] if the shell does not carry a
/// `resource.payload` attribute.
pub fn unwrap_module(m: &Module) -> Result<ResourceTable, IrError> {
    let Some(Attribute::Bytes(bytes)) = m.attributes.get("resource.payload") else {
        return Err(IrError::Invariant(
            "resource module missing 'resource.payload' attribute".into(),
        ));
    };
    crate::canonical::read_resource_payload(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_unwrap_round_trip() {
        let t = ResourceTable::new("com.example")
            .with_configuration(Configuration::default_for_sdk(21).with_qualifier("default"))
            .with_entry(ResourceEntry {
                ref_: ResourceRef {
                    r#type: ResourceType::String,
                    id: ResourceId(0x7f00_0001),
                    name: "app_name".into(),
                },
                value: ResourceValue::String("Example".into()),
            });
        let m = wrap_module(&t);
        let back = unwrap_module(&m).unwrap();
        assert_eq!(back, t);
    }

    #[test]
    fn lookup_returns_known_entry() {
        let key = ResourceRef {
            r#type: ResourceType::String,
            id: ResourceId(0x7f00_0001),
            name: "app_name".into(),
        };
        let t = ResourceTable::new("com.example").with_entry(ResourceEntry {
            ref_: key.clone(),
            value: ResourceValue::String("Example".into()),
        });
        assert!(t.lookup(&key).is_some());
    }
}
