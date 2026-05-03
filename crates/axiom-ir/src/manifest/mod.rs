// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Manifest dialect of AXIOM-IR v0.1.
//!
//! Models the Android `AndroidManifest.xml` namespace: package identity,
//! components ([`Component`] / [`ComponentKind`]), [`IntentFilter`]s,
//! [`Permission`]s, and uses-permissions.
//!
//! ## Wrapping into the IR shell
//!
//! The kernel [`crate::Module`] is dialect-agnostic. To round-trip a
//! [`ManifestModule`] through canonical bytes we **wrap** the manifest
//! payload as a single binary attribute on the kernel module — see
//! [`wrap_module`] and [`unwrap_module`]. This keeps the IR kernel uniform
//! while still letting the manifest dialect carry domain-specific
//! structure.
//!
//! Components carry [`Tribool`] for `exported` and `enabled` to match
//! Android's three-valued semantics — `Default` is meaningfully different
//! from `False` (the platform decides based on whether intent filters
//! exist; the manifest spec is explicit about this).

// Counts fit in `u32` because canonical bytes never exceed 4 GiB
// (corpus harness enforces). SDK levels are 1-byte-bounded by Android's
// own type system.
#![allow(clippy::cast_possible_truncation)]

use std::collections::BTreeMap;

use crate::core::{Attribute, IrError, Module, Tribool};

mod components;
mod intent_filter;
mod permission;

pub use components::{Component, ComponentKind, ComponentName, DataAuthority};
pub use intent_filter::{DataFilter, IntentFilter};
pub use permission::{Permission, PermissionRef, ProtectionLevel};

/// Top-level manifest module — the dialect-specific data carried by an
/// IR module of `dialect_tag = "manifest"`.
///
/// Stable for v0.1. Field names map directly to canonical-bytes layout
/// in [`crate::canonical::encode_manifest`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ManifestModule {
    /// Java package — `package="com.example.app"`.
    pub package: String,
    /// `targetSdkVersion`. 0 if unset.
    pub target_sdk: u8,
    /// `minSdkVersion`. 0 if unset.
    pub min_sdk: u8,
    /// Optional application label — either a literal string or a
    /// `@string/...` resource reference (both encoded as a string here;
    /// resolution to the resource dialect happens in
    /// [`crate::lowering::resolve`]).
    pub application_label: Option<String>,
    /// Activities, services, receivers, providers (in source order).
    pub components: Vec<Component>,
    /// Permissions *declared* by this app.
    pub permissions: Vec<Permission>,
    /// Permissions *requested* by this app.
    pub uses_permissions: Vec<String>,
}

impl ManifestModule {
    /// Construct an empty manifest with the given package name.
    #[must_use]
    pub fn new(package: impl Into<String>) -> Self {
        Self {
            package: package.into(),
            ..Default::default()
        }
    }

    /// Builder: append a component.
    #[must_use]
    pub fn with_component(mut self, c: Component) -> Self {
        self.components.push(c);
        self
    }

    /// Builder: append a declared permission.
    #[must_use]
    pub fn with_permission(mut self, p: Permission) -> Self {
        self.permissions.push(p);
        self
    }

    /// Builder: append a uses-permission.
    #[must_use]
    pub fn with_uses_permission(mut self, p: impl Into<String>) -> Self {
        self.uses_permissions.push(p.into());
        self
    }

    /// Builder: set target SDK.
    #[must_use]
    pub const fn with_target_sdk(mut self, v: u8) -> Self {
        self.target_sdk = v;
        self
    }

    /// Builder: set min SDK.
    #[must_use]
    pub const fn with_min_sdk(mut self, v: u8) -> Self {
        self.min_sdk = v;
        self
    }

    /// Builder: set application label.
    #[must_use]
    pub fn with_application_label(mut self, v: impl Into<String>) -> Self {
        self.application_label = Some(v.into());
        self
    }

    /// Iterator over components of a given kind (e.g. only activities).
    pub fn components_of(&self, kind: ComponentKind) -> impl Iterator<Item = &Component> {
        self.components.iter().filter(move |c| c.kind == kind)
    }

    /// Number of *exported* (or default-when-filters-present) components.
    /// Used as a fast surface measure for security review.
    #[must_use]
    pub fn exported_count(&self) -> usize {
        self.components.iter().filter(|c| c.is_exported()).count()
    }
}

/// Wrap a [`ManifestModule`] into the kernel [`Module`] shell.
///
/// The shell carries a single binary attribute `manifest.payload`
/// containing the canonical-bytes encoding of the dialect-specific data.
/// The shell also publishes a few well-known top-level attrs for fast
/// inspection without decoding the payload.
#[must_use]
pub fn wrap_module(m: &ManifestModule) -> Module {
    let mut payload = Vec::with_capacity(256);
    crate::canonical::write_manifest_payload(&mut payload, m);

    let mut attrs: BTreeMap<String, Attribute> = BTreeMap::new();
    attrs.insert("manifest.payload".into(), Attribute::Bytes(payload));
    attrs.insert(
        "manifest.package".into(),
        Attribute::String(m.package.clone()),
    );
    attrs.insert(
        "manifest.target_sdk".into(),
        Attribute::ApiLevel(m.target_sdk),
    );
    attrs.insert("manifest.min_sdk".into(), Attribute::ApiLevel(m.min_sdk));
    attrs.insert(
        "manifest.component_count".into(),
        Attribute::U32(m.components.len() as u32),
    );
    attrs.insert(
        "manifest.exported_count".into(),
        Attribute::U32(m.exported_count() as u32),
    );

    Module {
        producer: crate::PRODUCER_TAG.to_string(),
        dialect_tag: "manifest".into(),
        attributes: attrs,
        region: crate::core::Region::single(crate::core::Block::new("entry")),
        next_value_id: 0,
    }
}

/// Inverse of [`wrap_module`]. Decodes the `manifest.payload` attribute.
///
/// # Errors
/// Returns [`IrError::Invariant`] if the shell does not carry a
/// `manifest.payload` attribute, and propagates any
/// [`crate::canonical`] decoder errors otherwise.
pub fn unwrap_module(m: &Module) -> Result<ManifestModule, IrError> {
    let Some(Attribute::Bytes(bytes)) = m.attributes.get("manifest.payload") else {
        return Err(IrError::Invariant(
            "manifest module missing 'manifest.payload' attribute".into(),
        ));
    };
    crate::canonical::read_manifest_payload(bytes)
}

/// Convenience constructor: an exported activity with one `ACTION_MAIN` +
/// `CATEGORY_LAUNCHER` intent filter.
#[must_use]
pub fn launcher_activity(component_name: impl Into<String>) -> Component {
    Component {
        kind: ComponentKind::Activity,
        name: component_name.into(),
        exported: Tribool::True,
        enabled: Tribool::Default,
        permission: None,
        intent_filters: vec![IntentFilter {
            actions: vec!["android.intent.action.MAIN".into()],
            categories: vec!["android.intent.category.LAUNCHER".into()],
            data: Vec::new(),
            priority: 0,
        }],
        authorities: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launcher_activity_is_exported() {
        let m =
            ManifestModule::new("com.example").with_component(launcher_activity(".MainActivity"));
        assert_eq!(m.exported_count(), 1);
    }

    #[test]
    fn wrap_unwrap_round_trip() {
        let m = ManifestModule::new("com.example")
            .with_target_sdk(34)
            .with_min_sdk(21)
            .with_component(launcher_activity(".MainActivity"))
            .with_uses_permission("android.permission.INTERNET");
        let wrapped = wrap_module(&m);
        let back = unwrap_module(&wrapped).unwrap();
        assert_eq!(back, m);
    }

    #[test]
    fn unwrap_missing_payload_errors() {
        let m = Module::empty("manifest");
        assert!(matches!(unwrap_module(&m), Err(IrError::Invariant(_))));
    }
}
