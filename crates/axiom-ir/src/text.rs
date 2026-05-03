// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! MLIR-style text format for AXIOM-IR v0.1.
//!
//! Used for diagnostics and the spec — not consumed at the security
//! boundary (canonical bytes own that). The printer is best-effort
//! readable; there is no parser yet (round-trip-via-text is not a v0.1
//! requirement).
//!
//! Example output:
//!
//! ```text
//! axir.module @manifest producer="apkaxiom::ir/0.1.0" {
//!   #manifest.package = "com.example"
//!   #manifest.target_sdk = 34
//!   ^entry:
//!     manifest.activity ".MainActivity" exported=true {
//!       intent_filter actions=["android.intent.action.MAIN"]
//!                     categories=["android.intent.category.LAUNCHER"]
//!     }
//! }
//! ```

use std::fmt::Write as _;

use crate::core::{Attribute, Module, Tribool};
use crate::manifest::{Component, ComponentKind, IntentFilter, ManifestModule};

/// Print a kernel module to MLIR-style text.
#[must_use]
pub fn print_module(m: &Module) -> String {
    let mut out = String::with_capacity(256);
    writeln!(
        &mut out,
        "axir.module @{} producer={:?} {{",
        m.dialect_tag, m.producer
    )
    .unwrap();
    for (k, v) in &m.attributes {
        writeln!(&mut out, "  #{} = {}", k, attr_text(v)).unwrap();
    }
    if let Some(block) = m.region.blocks.first() {
        writeln!(&mut out, "  ^{}:", block.label).unwrap();
        for op in &block.ops {
            writeln!(&mut out, "    {}", op.name).unwrap();
        }
    }
    out.push('}');
    out.push('\n');
    out
}

/// Human-friendly print of a manifest module.
#[must_use]
pub fn print_manifest(m: &ManifestModule) -> String {
    let mut out = String::with_capacity(256);
    writeln!(
        &mut out,
        "manifest {{ package={:?} min_sdk={} target_sdk={} }}",
        m.package, m.min_sdk, m.target_sdk
    )
    .unwrap();
    if let Some(label) = &m.application_label {
        writeln!(&mut out, "  application_label={label:?}").unwrap();
    }
    for c in &m.components {
        write_component(&mut out, c);
    }
    for p in &m.permissions {
        writeln!(
            &mut out,
            "  permission {{ name={:?} protection={:?} }}",
            p.name, p.protection
        )
        .unwrap();
    }
    for u in &m.uses_permissions {
        writeln!(&mut out, "  uses_permission {u:?}").unwrap();
    }
    out
}

fn write_component(out: &mut String, c: &Component) {
    writeln!(
        out,
        "  {kind} {name:?} exported={exported} enabled={enabled}",
        kind = component_kind_kw(c.kind),
        name = c.name,
        exported = tribool_text(c.exported),
        enabled = tribool_text(c.enabled),
    )
    .unwrap();
    if let Some(p) = &c.permission {
        writeln!(out, "    permission={p:?}").unwrap();
    }
    for f in &c.intent_filters {
        write_intent_filter(out, f);
    }
    for a in &c.authorities {
        match &a.port {
            Some(port) => writeln!(out, "    authority host={:?} port={port:?}", a.host).unwrap(),
            None => writeln!(out, "    authority host={:?}", a.host).unwrap(),
        }
    }
}

fn write_intent_filter(out: &mut String, f: &IntentFilter) {
    writeln!(
        out,
        "    intent_filter actions={:?} categories={:?} priority={}",
        f.actions, f.categories, f.priority
    )
    .unwrap();
    for d in &f.data {
        writeln!(out, "      data {d:?}").unwrap();
    }
}

const fn component_kind_kw(k: ComponentKind) -> &'static str {
    match k {
        ComponentKind::Activity => "activity",
        ComponentKind::Service => "service",
        ComponentKind::Receiver => "receiver",
        ComponentKind::Provider => "provider",
    }
}

const fn tribool_text(t: Tribool) -> &'static str {
    match t {
        Tribool::True => "true",
        Tribool::False => "false",
        Tribool::Default => "default",
    }
}

fn attr_text(a: &Attribute) -> String {
    match a {
        Attribute::Bool(b) => b.to_string(),
        Attribute::Tribool(t) => tribool_text(*t).to_string(),
        Attribute::U32(n) => n.to_string(),
        Attribute::I32(n) => n.to_string(),
        Attribute::String(s) => format!("{s:?}"),
        Attribute::Bytes(b) => format!("<bytes len={}>", b.len()),
        Attribute::ApiLevel(n) => format!("api{n}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{launcher_activity, ManifestModule};

    #[test]
    fn print_module_contains_dialect_tag() {
        let m = Module::empty("manifest");
        let s = print_module(&m);
        assert!(s.contains("axir.module @manifest"));
    }

    #[test]
    fn print_manifest_contains_components() {
        let m =
            ManifestModule::new("com.example").with_component(launcher_activity(".MainActivity"));
        let s = print_manifest(&m);
        assert!(s.contains("activity"));
        assert!(s.contains(".MainActivity"));
    }
}
