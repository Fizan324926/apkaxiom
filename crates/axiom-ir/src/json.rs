// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Stable JSON serialiser for AXIOM-IR.
//!
//! Hand-rolled, no `serde_json`. The output is byte-deterministic given
//! the same input — keys always emerge from a [`std::collections::BTreeMap`]
//! (already sorted by core), no whitespace, no trailing newline.
//!
//! Used by `tools/ir-corpus` to emit drift-stable summary JSONs that the
//! P1.4 CI gate compares byte-for-byte.

// Counts fit in `u32`; SDK levels in `u8`; both bounded by domain.
#![allow(clippy::cast_possible_truncation, clippy::cast_lossless)]

use std::fmt::Write as _;

use crate::core::{Attribute, Module, Tribool, Type};
use crate::manifest::{
    Component, ComponentKind, DataAuthority, DataFilter, IntentFilter, ManifestModule, Permission,
    ProtectionLevel,
};
use crate::resource::{
    Configuration, ResourceEntry, ResourceRef, ResourceTable, ResourceType, ResourceValue,
};

/// Encode any module to stable JSON.
#[must_use]
pub fn encode_module(m: &Module) -> String {
    let mut out = String::with_capacity(256);
    write_module(&mut out, m);
    out
}

/// Encode a manifest to stable JSON.
#[must_use]
pub fn encode_manifest(m: &ManifestModule) -> String {
    let mut out = String::with_capacity(256);
    write_manifest(&mut out, m);
    out
}

/// Encode a resource table to stable JSON.
#[must_use]
pub fn encode_resource(t: &ResourceTable) -> String {
    let mut out = String::with_capacity(256);
    write_resource(&mut out, t);
    out
}

// ---------------------------------------------------------------------------
// Module
// ---------------------------------------------------------------------------

fn write_module(out: &mut String, m: &Module) {
    out.push('{');
    write_kv_str(out, "producer", &m.producer);
    out.push(',');
    write_kv_str(out, "dialect", &m.dialect_tag);
    out.push(',');
    out.push_str("\"attributes\":");
    write_attr_map(out, &m.attributes);
    out.push(',');
    out.push_str("\"next_value_id\":");
    let _ = write!(out, "{}", m.next_value_id);
    out.push('}');
}

fn write_attr_map(out: &mut String, map: &std::collections::BTreeMap<String, Attribute>) {
    out.push('{');
    let mut first = true;
    for (k, v) in map {
        if !first {
            out.push(',');
        }
        first = false;
        write_kv_attr(out, k, v);
    }
    out.push('}');
}

fn write_kv_attr(out: &mut String, key: &str, attr: &Attribute) {
    write_string(out, key);
    out.push(':');
    write_attr(out, attr);
}

fn write_attr(out: &mut String, a: &Attribute) {
    out.push('{');
    match a {
        Attribute::Bool(b) => {
            out.push_str("\"kind\":\"bool\",\"value\":");
            out.push_str(if *b { "true" } else { "false" });
        }
        Attribute::Tribool(t) => {
            out.push_str("\"kind\":\"tribool\",\"value\":");
            write_string(out, tribool_str(*t));
        }
        Attribute::U32(n) => {
            out.push_str("\"kind\":\"u32\",\"value\":");
            let _ = write!(out, "{n}");
        }
        Attribute::I32(n) => {
            out.push_str("\"kind\":\"i32\",\"value\":");
            let _ = write!(out, "{n}");
        }
        Attribute::String(s) => {
            out.push_str("\"kind\":\"string\",\"value\":");
            write_string(out, s);
        }
        Attribute::Bytes(b) => {
            out.push_str("\"kind\":\"bytes\",\"len\":");
            let _ = write!(out, "{}", b.len());
            out.push_str(",\"sha256\":");
            write_string(out, &crate::hash::hex(&crate::hash::sha256(b)));
        }
        Attribute::ApiLevel(n) => {
            out.push_str("\"kind\":\"api_level\",\"value\":");
            let _ = write!(out, "{n}");
        }
    }
    out.push('}');
}

// ---------------------------------------------------------------------------
// Manifest
// ---------------------------------------------------------------------------

fn write_manifest(out: &mut String, m: &ManifestModule) {
    out.push('{');
    write_kv_str(out, "package", &m.package);
    out.push(',');
    write_kv_u32(out, "target_sdk", u32::from(m.target_sdk));
    out.push(',');
    write_kv_u32(out, "min_sdk", u32::from(m.min_sdk));
    out.push(',');
    out.push_str("\"application_label\":");
    match &m.application_label {
        Some(s) => write_string(out, s),
        None => out.push_str("null"),
    }
    out.push(',');
    out.push_str("\"components\":[");
    for (i, c) in m.components.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write_component(out, c);
    }
    out.push(']');
    out.push(',');
    out.push_str("\"permissions\":[");
    for (i, p) in m.permissions.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write_permission(out, p);
    }
    out.push(']');
    out.push(',');
    out.push_str("\"uses_permissions\":[");
    for (i, p) in m.uses_permissions.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write_string(out, p);
    }
    out.push(']');
    out.push('}');
}

fn write_component(out: &mut String, c: &Component) {
    out.push('{');
    write_kv_str(out, "kind", component_kind_str(c.kind));
    out.push(',');
    write_kv_str(out, "name", &c.name);
    out.push(',');
    write_kv_str(out, "exported", tribool_str(c.exported));
    out.push(',');
    write_kv_str(out, "enabled", tribool_str(c.enabled));
    out.push(',');
    out.push_str("\"is_exported\":");
    out.push_str(if c.is_exported() { "true" } else { "false" });
    out.push(',');
    out.push_str("\"permission\":");
    match &c.permission {
        Some(s) => write_string(out, s),
        None => out.push_str("null"),
    }
    out.push(',');
    out.push_str("\"intent_filters\":[");
    for (i, f) in c.intent_filters.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write_intent_filter(out, f);
    }
    out.push(']');
    out.push(',');
    out.push_str("\"authorities\":[");
    for (i, a) in c.authorities.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write_authority(out, a);
    }
    out.push(']');
    out.push('}');
}

fn write_intent_filter(out: &mut String, f: &IntentFilter) {
    out.push('{');
    out.push_str("\"actions\":");
    write_str_array(out, &f.actions);
    out.push(',');
    out.push_str("\"categories\":");
    write_str_array(out, &f.categories);
    out.push(',');
    out.push_str("\"data\":[");
    for (i, d) in f.data.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write_data_filter(out, d);
    }
    out.push(']');
    out.push(',');
    write_kv_i32(out, "priority", f.priority);
    out.push('}');
}

fn write_data_filter(out: &mut String, d: &DataFilter) {
    out.push('{');
    let pairs: [(&str, Option<&String>); 7] = [
        ("scheme", d.scheme.as_ref()),
        ("host", d.host.as_ref()),
        ("port", d.port.as_ref()),
        ("path", d.path.as_ref()),
        ("path_prefix", d.path_prefix.as_ref()),
        ("path_pattern", d.path_pattern.as_ref()),
        ("mime_type", d.mime_type.as_ref()),
    ];
    let mut first = true;
    for (k, v) in &pairs {
        if !first {
            out.push(',');
        }
        first = false;
        out.push('"');
        out.push_str(k);
        out.push_str("\":");
        match v {
            Some(s) => write_string(out, s),
            None => out.push_str("null"),
        }
    }
    out.push('}');
}

fn write_authority(out: &mut String, a: &DataAuthority) {
    out.push('{');
    write_kv_str(out, "host", &a.host);
    out.push(',');
    out.push_str("\"port\":");
    match &a.port {
        Some(p) => write_string(out, p),
        None => out.push_str("null"),
    }
    out.push('}');
}

fn write_permission(out: &mut String, p: &Permission) {
    out.push('{');
    write_kv_str(out, "name", &p.name);
    out.push(',');
    write_kv_str(out, "protection", protection_str(p.protection));
    out.push(',');
    out.push_str("\"group\":");
    match &p.group {
        Some(g) => write_string(out, g),
        None => out.push_str("null"),
    }
    out.push('}');
}

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

fn write_resource(out: &mut String, t: &ResourceTable) {
    out.push('{');
    write_kv_str(out, "package", &t.package);
    out.push(',');
    write_kv_u32(out, "string_pool_size", t.string_pool.strings.len() as u32);
    out.push(',');
    out.push_str("\"configurations\":[");
    for (i, c) in t.configurations.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write_configuration(out, c);
    }
    out.push(']');
    out.push(',');
    out.push_str("\"entries\":[");
    for (i, e) in t.entries.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write_resource_entry(out, e);
    }
    out.push(']');
    out.push('}');
}

fn write_configuration(out: &mut String, c: &Configuration) {
    out.push('{');
    write_kv_str(out, "qualifier", &c.qualifier);
    out.push(',');
    write_kv_u32(out, "density_dpi", c.density_dpi);
    out.push(',');
    out.push_str("\"locale\":");
    match &c.locale {
        Some(s) => write_string(out, s),
        None => out.push_str("null"),
    }
    out.push(',');
    out.push_str("\"orientation\":");
    match &c.orientation {
        Some(s) => write_string(out, s),
        None => out.push_str("null"),
    }
    out.push(',');
    write_kv_u32(out, "min_sdk", u32::from(c.min_sdk));
    out.push('}');
}

fn write_resource_entry(out: &mut String, e: &ResourceEntry) {
    out.push('{');
    out.push_str("\"ref\":");
    write_resource_ref(out, &e.ref_);
    out.push(',');
    out.push_str("\"value\":");
    write_resource_value(out, &e.value);
    out.push('}');
}

fn write_resource_ref(out: &mut String, r: &ResourceRef) {
    out.push('{');
    write_kv_str(out, "type", resource_type_str(r.r#type));
    out.push(',');
    out.push_str("\"id\":");
    let _ = write!(out, "{}", r.id.0);
    out.push(',');
    write_kv_str(out, "name", &r.name);
    out.push('}');
}

fn write_resource_value(out: &mut String, v: &ResourceValue) {
    match v {
        ResourceValue::String(s) => {
            out.push_str("{\"kind\":\"string\",\"value\":");
            write_string(out, s);
            out.push('}');
        }
        ResourceValue::Int(n) => {
            out.push_str("{\"kind\":\"int\",\"value\":");
            let _ = write!(out, "{n}");
            out.push('}');
        }
        ResourceValue::Bool(b) => {
            out.push_str("{\"kind\":\"bool\",\"value\":");
            out.push_str(if *b { "true" } else { "false" });
            out.push('}');
        }
        ResourceValue::Ref(r) => {
            out.push_str("{\"kind\":\"ref\",\"value\":");
            write_resource_ref(out, r);
            out.push('}');
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn write_kv_str(out: &mut String, key: &str, value: &str) {
    write_string(out, key);
    out.push(':');
    write_string(out, value);
}

fn write_kv_u32(out: &mut String, key: &str, value: u32) {
    write_string(out, key);
    out.push(':');
    let _ = write!(out, "{value}");
}

fn write_kv_i32(out: &mut String, key: &str, value: i32) {
    write_string(out, key);
    out.push(':');
    let _ = write!(out, "{value}");
}

fn write_str_array(out: &mut String, xs: &[String]) {
    out.push('[');
    for (i, s) in xs.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write_string(out, s);
    }
    out.push(']');
}

fn write_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

const fn tribool_str(t: Tribool) -> &'static str {
    match t {
        Tribool::True => "true",
        Tribool::False => "false",
        Tribool::Default => "default",
    }
}

const fn component_kind_str(k: ComponentKind) -> &'static str {
    match k {
        ComponentKind::Activity => "activity",
        ComponentKind::Service => "service",
        ComponentKind::Receiver => "receiver",
        ComponentKind::Provider => "provider",
    }
}

const fn protection_str(p: ProtectionLevel) -> &'static str {
    match p {
        ProtectionLevel::Normal => "normal",
        ProtectionLevel::Dangerous => "dangerous",
        ProtectionLevel::Signature => "signature",
        ProtectionLevel::SignatureOrSystem => "signatureOrSystem",
        ProtectionLevel::Internal => "internal",
    }
}

const fn resource_type_str(r: ResourceType) -> &'static str {
    match r {
        ResourceType::String => "string",
        ResourceType::Drawable => "drawable",
        ResourceType::Layout => "layout",
        ResourceType::Color => "color",
        ResourceType::Dimen => "dimen",
        ResourceType::Style => "style",
        ResourceType::Bool => "bool",
        ResourceType::Integer => "integer",
        ResourceType::Raw => "raw",
    }
}

/// Type → JSON type-tag. Exposed for the corpus harness.
#[must_use]
pub fn type_str(t: &Type) -> String {
    match t {
        Type::Tribool => "tribool".into(),
        Type::U32 => "u32".into(),
        Type::I32 => "i32".into(),
        Type::String => "string".into(),
        Type::Bytes => "bytes".into(),
        Type::ResourceRef => "resource_ref".into(),
        Type::PermissionRef => "permission_ref".into(),
        Type::ComponentName => "component_name".into(),
        Type::ApiLevel => "api_level".into(),
        Type::List(inner) => format!("list<{}>", type_str(inner)),
        Type::Option(inner) => format!("option<{}>", type_str(inner)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{launcher_activity, ManifestModule};

    #[test]
    fn manifest_json_is_deterministic() {
        let m = ManifestModule::new("com.example")
            .with_target_sdk(34)
            .with_min_sdk(21)
            .with_component(launcher_activity(".MainActivity"))
            .with_uses_permission("android.permission.INTERNET");
        let a = encode_manifest(&m);
        let b = encode_manifest(&m);
        assert_eq!(a, b);
        assert!(a.contains("\"package\":\"com.example\""));
        assert!(a.contains("\"target_sdk\":34"));
    }

    #[test]
    fn json_escapes_special_chars() {
        let mut out = String::new();
        write_string(&mut out, "a\"b\\c\nd");
        assert_eq!(out, r#""a\"b\\c\nd""#);
    }

    #[test]
    fn type_str_recursive() {
        let t = Type::List(Box::new(Type::Option(Box::new(Type::U32))));
        assert_eq!(type_str(&t), "list<option<u32>>");
    }
}
