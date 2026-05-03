// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Manifest components: activity / service / receiver / provider.

use super::{IntentFilter, PermissionRef};
use crate::core::Tribool;

/// Kind of an Android component.
///
/// Variant order is part of canonical bytes — DO NOT reorder. New kinds
/// require a schema-version bump.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ComponentKind {
    /// `<activity>` — a screen the user can navigate to.
    Activity,
    /// `<service>` — a long-running background component.
    Service,
    /// `<receiver>` — a broadcast receiver.
    Receiver,
    /// `<provider>` — a content provider (note: the only kind with
    /// **authorities** rather than intent filters as its primary lookup
    /// surface).
    Provider,
}

/// Java-class component name. Either fully-qualified
/// (`"com.example.MainActivity"`) or shorthand starting with `'.'`
/// (`".MainActivity"`).
pub type ComponentName = String;

/// A single component declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    /// Component kind.
    pub kind: ComponentKind,
    /// Class name.
    pub name: ComponentName,
    /// Exported flag — three-valued, `Default` is *not* equivalent to
    /// `False`. See [`Component::is_exported`] for the authoritative
    /// resolution.
    pub exported: Tribool,
    /// Enabled flag — three-valued.
    pub enabled: Tribool,
    /// Optional gating permission name (downstream
    /// [`crate::lowering::resolve`] turns this into a
    /// [`PermissionRef`]).
    pub permission: Option<String>,
    /// Intent filters declared on this component.
    pub intent_filters: Vec<IntentFilter>,
    /// `<provider>`-specific: the authorities this component matches.
    /// Empty for non-providers.
    pub authorities: Vec<DataAuthority>,
}

/// Provider authority — `(host, optional port)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataAuthority {
    /// Authority host.
    pub host: String,
    /// Optional authority port (string-typed because Android allows
    /// symbolic port references).
    pub port: Option<String>,
}

impl Component {
    /// Authoritative resolution of the `exported` tribool.
    ///
    /// Per Android docs:
    /// * `Tribool::True`  → exported
    /// * `Tribool::False` → not exported
    /// * `Tribool::Default` → exported iff at least one intent filter is
    ///   declared (and the component is an activity, service, or
    ///   receiver — providers default to *not* exported on API ≥ 17).
    #[must_use]
    pub fn is_exported(&self) -> bool {
        match self.exported {
            Tribool::True => true,
            Tribool::False => false,
            Tribool::Default => match self.kind {
                ComponentKind::Provider => false,
                ComponentKind::Activity | ComponentKind::Service | ComponentKind::Receiver => {
                    !self.intent_filters.is_empty()
                }
            },
        }
    }

    /// Returns the gating permission as a [`PermissionRef`], if any.
    #[must_use]
    pub fn permission_ref(&self) -> Option<PermissionRef> {
        self.permission.clone().map(PermissionRef::Symbolic)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Tribool;

    fn act_with_filter(name: &str) -> Component {
        Component {
            kind: ComponentKind::Activity,
            name: name.into(),
            exported: Tribool::Default,
            enabled: Tribool::Default,
            permission: None,
            intent_filters: vec![IntentFilter::default()],
            authorities: Vec::new(),
        }
    }

    #[test]
    fn is_exported_default_with_filter_is_true() {
        let c = act_with_filter(".X");
        assert!(c.is_exported());
    }

    #[test]
    fn is_exported_default_no_filter_is_false() {
        let mut c = act_with_filter(".X");
        c.intent_filters.clear();
        assert!(!c.is_exported());
    }

    #[test]
    fn provider_default_is_not_exported() {
        let c = Component {
            kind: ComponentKind::Provider,
            name: ".P".into(),
            exported: Tribool::Default,
            enabled: Tribool::Default,
            permission: None,
            intent_filters: Vec::new(),
            authorities: vec![DataAuthority {
                host: "com.example".into(),
                port: None,
            }],
        };
        assert!(!c.is_exported());
    }

    #[test]
    fn explicit_false_overrides_filter() {
        let mut c = act_with_filter(".X");
        c.exported = Tribool::False;
        assert!(!c.is_exported());
    }
}
