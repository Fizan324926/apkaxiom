// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Manifest ↔ resource lowering.
//!
//! In v0.1 the only direction we lower is **manifest → manifest with
//! resource references resolved** — i.e. `application_label` and component
//! permission strings of the form `"@string/<name>"` are replaced with
//! literal strings looked up in the [`ResourceTable`]. The reverse
//! direction (literal → reference) is not part of v0.1.
//!
//! Lowering is **diagnostic-accumulating, not failure-fast**. A symbolic
//! reference that doesn't resolve emits a [`Diagnostic`] but does not
//! abort the pass — downstream phases (P1.15 emitter, P3 symbolic
//! resolver) decide policy.

use crate::core::{Diagnostic, Severity};
use crate::manifest::{Component, ManifestModule};
use crate::resource::{ResourceTable, ResourceType, ResourceValue};

/// Resolved manifest plus diagnostics from the pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveResult {
    /// Manifest with `@string/...` references substituted in.
    pub manifest: ManifestModule,
    /// Diagnostics produced during lowering. Empty on a clean pass.
    pub diagnostics: Vec<Diagnostic>,
}

/// Resolve `@string/...` references in `manifest` against `resources`.
///
/// Reference shape recognised: literal prefix `"@string/"`, followed by
/// the resource name. Unknown names emit a `Severity::Warning`
/// diagnostic and pass the original string through unchanged so
/// downstream consumers can still see what was attempted.
#[must_use]
pub fn resolve(manifest: &ManifestModule, resources: &ResourceTable) -> ResolveResult {
    let mut diagnostics = Vec::new();
    let mut out = manifest.clone();

    if let Some(label) = &manifest.application_label {
        out.application_label = Some(resolve_string(label, resources, &mut diagnostics));
    }

    out.components = manifest
        .components
        .iter()
        .map(|c| resolve_component(c, resources, &mut diagnostics))
        .collect();

    ResolveResult {
        manifest: out,
        diagnostics,
    }
}

fn resolve_component(
    c: &Component,
    resources: &ResourceTable,
    diagnostics: &mut Vec<Diagnostic>,
) -> Component {
    let mut out = c.clone();
    if let Some(name) = &c.name.strip_prefix("@string/") {
        out.name = lookup_string(name, resources).unwrap_or_else(|| {
            diagnostics.push(Diagnostic {
                severity: Severity::Warning,
                message: format!("unresolved component name reference: @string/{name}"),
            });
            c.name.clone()
        });
    }
    if let Some(permission) = &c.permission {
        out.permission = Some(resolve_string(permission, resources, diagnostics));
    }
    out
}

fn resolve_string(
    raw: &str,
    resources: &ResourceTable,
    diagnostics: &mut Vec<Diagnostic>,
) -> String {
    let Some(name) = raw.strip_prefix("@string/") else {
        return raw.to_string();
    };
    lookup_string(name, resources).unwrap_or_else(|| {
        diagnostics.push(Diagnostic {
            severity: Severity::Warning,
            message: format!("unresolved string reference: @string/{name}"),
        });
        raw.to_string()
    })
}

fn lookup_string(name: &str, resources: &ResourceTable) -> Option<String> {
    resources
        .entries
        .iter()
        .find(|e| matches!(e.ref_.r#type, ResourceType::String) && e.ref_.name == name)
        .and_then(|e| match &e.value {
            ResourceValue::String(s) => Some(s.clone()),
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{launcher_activity, Component, ComponentKind};
    use crate::resource::{ResourceEntry, ResourceId, ResourceRef};

    fn label_table() -> ResourceTable {
        ResourceTable::new("com.example").with_entry(ResourceEntry {
            ref_: ResourceRef {
                r#type: ResourceType::String,
                id: ResourceId(0x7f00_0001),
                name: "app_name".into(),
            },
            value: ResourceValue::String("Example App".into()),
        })
    }

    #[test]
    fn known_label_is_substituted() {
        let m = ManifestModule::new("com.example").with_application_label("@string/app_name");
        let r = resolve(&m, &label_table());
        assert_eq!(r.manifest.application_label.as_deref(), Some("Example App"));
        assert!(r.diagnostics.is_empty());
    }

    #[test]
    fn unknown_label_emits_diagnostic() {
        let m = ManifestModule::new("com.example").with_application_label("@string/missing");
        let r = resolve(&m, &label_table());
        assert_eq!(
            r.manifest.application_label.as_deref(),
            Some("@string/missing")
        );
        assert_eq!(r.diagnostics.len(), 1);
        assert_eq!(r.diagnostics[0].severity, Severity::Warning);
    }

    #[test]
    fn literal_label_passthrough() {
        let m = ManifestModule::new("com.example").with_application_label("Literal Label");
        let r = resolve(&m, &label_table());
        assert_eq!(
            r.manifest.application_label.as_deref(),
            Some("Literal Label")
        );
        assert!(r.diagnostics.is_empty());
    }

    #[test]
    fn component_permission_resolves() {
        let mut c = launcher_activity(".Main");
        c.permission = Some("@string/app_name".into());
        let m = ManifestModule::new("com.example").with_component(c);
        let r = resolve(&m, &label_table());
        assert_eq!(
            r.manifest.components[0].permission.as_deref(),
            Some("Example App")
        );
    }

    #[test]
    fn provider_kept_intact() {
        let provider = Component {
            kind: ComponentKind::Provider,
            name: ".P".into(),
            exported: crate::Tribool::Default,
            enabled: crate::Tribool::Default,
            permission: None,
            intent_filters: Vec::new(),
            authorities: Vec::new(),
        };
        let m = ManifestModule::new("com.example").with_component(provider.clone());
        let r = resolve(&m, &label_table());
        assert_eq!(r.manifest.components[0], provider);
    }
}
