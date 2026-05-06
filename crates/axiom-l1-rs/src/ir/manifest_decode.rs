// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! AXML semantic decoder — maps an [`AxmlDoc`] to a [`ManifestModule`].
//!
//! The decoder walks the chunk sequence in order:
//!   1. First `RES_STRING_POOL` chunk → decode all strings.
//!   2. Each `RES_XML_START_ELEMENT` → match element name, extract
//!      attributes, update parse state.
//!   3. Each `RES_XML_END_ELEMENT` → pop state stack.
//!
//! ## Attribute value extraction
//!
//! AXML attributes carry a typed [`ResValue`] alongside an optional raw
//! string reference. For known attributes we read:
//!   - `TYPE_STRING (0x03)`: data field is a string pool index.
//!   - `TYPE_INT_DEC (0x10)` / `TYPE_INT_HEX (0x11)`: data is a 32-bit
//!     integer.
//!   - `TYPE_INT_BOOLEAN (0x12)`: data != 0 → `true`.
//!   - `TYPE_REFERENCE (0x01)`: unresolved resource ref → recorded as
//!     `@0x<id>` for transparency.

use axiom_ir::manifest::{
    Component, ComponentKind, DataAuthority, IntentFilter, ManifestModule, Permission,
    ProtectionLevel,
};
use axiom_ir::core::Tribool;

use super::{axml, strings};

// AXML element chunk type.
const START_ELEMENT: u16 = axml::chunk_type::RES_XML_START_ELEMENT;
const END_ELEMENT: u16 = axml::chunk_type::RES_XML_END_ELEMENT;
const STRING_POOL: u16 = axml::chunk_type::RES_STRING_POOL;

// Attribute data types.
const TYPE_REFERENCE: u8 = 0x01;
const TYPE_STRING: u8 = 0x03;
const TYPE_INT_DEC: u8 = 0x10;
const TYPE_INT_HEX: u8 = 0x11;
const TYPE_INT_BOOLEAN: u8 = 0x12;

/// Decode an [`AxmlDoc`] into a [`ManifestModule`].
///
/// Best-effort: missing or unrecognised elements leave fields at their
/// zero values. Errors in the string pool are non-fatal — the decoder
/// treats them as an empty pool and proceeds.
#[must_use]
pub fn decode(doc: &axml::AxmlDoc) -> ManifestModule {
    // Step 1: decode the string pool.
    let pool = doc
        .chunks
        .iter()
        .find(|c| c.type_id == STRING_POOL)
        .and_then(|c| strings::decode(&c.raw).ok())
        .unwrap_or_default();

    // Step 2: walk element events.
    let mut walker = Walker::new(pool);
    for chunk in &doc.chunks {
        match chunk.type_id {
            START_ELEMENT => walker.on_start(&chunk.raw),
            END_ELEMENT => walker.on_end(&chunk.raw),
            _ => {}
        }
    }
    walker.finish()
}

// ── Walker state machine ──────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
enum Frame {
    Root,
    Manifest,
    UsesSdk,
    Application,
    Component(ComponentKind),
    IntentFilter,
    Permission,
    UsesPermission,
    Other,
}

struct Walker {
    pool: Vec<String>,
    module: ManifestModule,
    stack: Vec<Frame>,
    // Component being assembled (flushed on END_ELEMENT).
    pending_component: Option<Component>,
    // Intent-filter being assembled (added to pending_component on END).
    pending_filter: Option<IntentFilter>,
    // Declared permission being assembled.
    pending_perm: Option<Permission>,
}

impl Walker {
    fn new(pool: Vec<String>) -> Self {
        Self {
            pool,
            module: ManifestModule::default(),
            stack: vec![Frame::Root],
            pending_component: None,
            pending_filter: None,
            pending_perm: None,
        }
    }

    fn finish(self) -> ManifestModule {
        self.module
    }

    // --- attribute helpers ---

    /// Extract all attributes from a start-element chunk as (name, value).
    fn attrs(raw: &[u8], pool: &[String]) -> Vec<(String, AttrValue)> {
        if raw.len() < 36 {
            return Vec::new();
        }
        // attr_start is relative to ResXMLTree_attrExt (which starts at byte 16).
        let attr_start = u16::from_le_bytes([raw[24], raw[25]]) as usize;
        let attr_size = u16::from_le_bytes([raw[26], raw[27]]) as usize;
        let attr_count = u16::from_le_bytes([raw[28], raw[29]]) as usize;
        let attrs_base = 16 + attr_start; // = 36 for standard chunks

        if attr_size < 20 || attrs_base + attr_count * attr_size > raw.len() {
            return Vec::new();
        }

        let mut out = Vec::with_capacity(attr_count);
        for i in 0..attr_count {
            let a = attrs_base + i * attr_size;
            // ns (4), name (4), rawValue (4), size (2), res0 (1), dataType (1), data (4)
            if a + 20 > raw.len() {
                break;
            }
            let name_idx = u32::from_le_bytes([raw[a + 4], raw[a + 5], raw[a + 6], raw[a + 7]]);
            let dtype = raw[a + 15];
            let data = u32::from_le_bytes([raw[a + 16], raw[a + 17], raw[a + 18], raw[a + 19]]);

            let name = if (name_idx as usize) < pool.len() && !pool[name_idx as usize].is_empty() {
                pool[name_idx as usize].clone()
            } else {
                continue; // skip attributes whose name we can't resolve
            };

            let val = match dtype {
                TYPE_STRING => {
                    if (data as usize) < pool.len() {
                        AttrValue::Str(pool[data as usize].clone())
                    } else {
                        continue;
                    }
                }
                TYPE_INT_DEC | TYPE_INT_HEX => AttrValue::Int(data as i64),
                TYPE_INT_BOOLEAN => AttrValue::Bool(data != 0),
                TYPE_REFERENCE => AttrValue::Ref(data),
                _ => continue,
            };
            out.push((name, val));
        }
        out
    }

    fn elem_name(raw: &[u8], pool: &[String]) -> Option<String> {
        if raw.len() < 24 {
            return None;
        }
        let name_idx = u32::from_le_bytes([raw[20], raw[21], raw[22], raw[23]]);
        pool.get(name_idx as usize).cloned()
    }

    // --- event handlers ---

    fn on_start(&mut self, raw: &[u8]) {
        let Some(elem) = Self::elem_name(raw, &self.pool) else {
            self.stack.push(Frame::Other);
            return;
        };
        let attrs = Self::attrs(raw, &self.pool);
        let parent = self.stack.last().cloned().unwrap_or(Frame::Root);

        match elem.as_str() {
            "manifest" => {
                for (k, v) in &attrs {
                    match k.as_str() {
                        "package" => {
                            if let AttrValue::Str(s) = v {
                                self.module.package = s.clone();
                            }
                        }
                        "versionCode" => {
                            // stored as int in the typed value
                            if let AttrValue::Int(_) = v {} // version_code not in ManifestModule v0.1
                        }
                        _ => {}
                    }
                }
                self.stack.push(Frame::Manifest);
            }
            "uses-sdk" => {
                for (k, v) in &attrs {
                    match k.as_str() {
                        "minSdkVersion" => {
                            if let AttrValue::Int(n) = v {
                                self.module.min_sdk = (*n).min(255).max(0) as u8;
                            }
                        }
                        "targetSdkVersion" => {
                            if let AttrValue::Int(n) = v {
                                self.module.target_sdk = (*n).min(255).max(0) as u8;
                            }
                        }
                        _ => {}
                    }
                }
                self.stack.push(Frame::UsesSdk);
            }
            "uses-permission" => {
                for (k, v) in &attrs {
                    if k == "name" {
                        if let AttrValue::Str(s) = v {
                            self.module.uses_permissions.push(s.clone());
                        }
                    }
                }
                self.stack.push(Frame::UsesPermission);
            }
            "permission" => {
                let mut name = String::new();
                let mut level = ProtectionLevel::Normal;
                for (k, v) in &attrs {
                    match k.as_str() {
                        "name" => {
                            if let AttrValue::Str(s) = v { name = s.clone(); }
                        }
                        "protectionLevel" => {
                            if let AttrValue::Int(n) = v {
                                level = protection_level(*n as u32);
                            }
                        }
                        _ => {}
                    }
                }
                if !name.is_empty() {
                    self.pending_perm = Some(Permission { name, protection: level, group: None });
                }
                self.stack.push(Frame::Permission);
            }
            "application" => {
                for (k, v) in &attrs {
                    if k == "label" {
                        match v {
                            AttrValue::Str(s) => {
                                self.module.application_label = Some(s.clone());
                            }
                            AttrValue::Ref(id) => {
                                self.module.application_label = Some(format!("@0x{id:08x}"));
                            }
                            _ => {}
                        }
                    }
                }
                self.stack.push(Frame::Application);
            }
            "activity" => self.start_component(ComponentKind::Activity, &attrs),
            "service" => self.start_component(ComponentKind::Service, &attrs),
            "receiver" => self.start_component(ComponentKind::Receiver, &attrs),
            "provider" => self.start_component(ComponentKind::Provider, &attrs),
            "intent-filter" => {
                // Only valid inside a component.
                let in_component = matches!(parent, Frame::Component(_));
                if in_component {
                    self.pending_filter = Some(IntentFilter::new());
                }
                let prio = attrs
                    .iter()
                    .find(|(k, _)| k == "priority")
                    .and_then(|(_, v)| if let AttrValue::Int(n) = v { Some(*n as i32) } else { None })
                    .unwrap_or(0);
                if let Some(f) = &mut self.pending_filter {
                    f.priority = prio;
                }
                self.stack.push(Frame::IntentFilter);
            }
            "action" => {
                if matches!(parent, Frame::IntentFilter) {
                    for (k, v) in &attrs {
                        if k == "name" {
                            if let AttrValue::Str(s) = v {
                                if let Some(f) = &mut self.pending_filter {
                                    f.actions.push(s.clone());
                                }
                            }
                        }
                    }
                }
                self.stack.push(Frame::Other);
            }
            "category" => {
                if matches!(parent, Frame::IntentFilter) {
                    for (k, v) in &attrs {
                        if k == "name" {
                            if let AttrValue::Str(s) = v {
                                if let Some(f) = &mut self.pending_filter {
                                    f.categories.push(s.clone());
                                }
                            }
                        }
                    }
                }
                self.stack.push(Frame::Other);
            }
            _ => {
                self.stack.push(Frame::Other);
            }
        }
    }

    fn on_end(&mut self, _raw: &[u8]) {
        let frame = self.stack.pop().unwrap_or(Frame::Other);
        match frame {
            Frame::IntentFilter => {
                if let (Some(filter), Some(comp)) =
                    (self.pending_filter.take(), self.pending_component.as_mut())
                {
                    comp.intent_filters.push(filter);
                }
            }
            Frame::Component(_) => {
                if let Some(comp) = self.pending_component.take() {
                    self.module.components.push(comp);
                }
            }
            Frame::Permission => {
                if let Some(perm) = self.pending_perm.take() {
                    self.module.permissions.push(perm);
                }
            }
            _ => {}
        }
    }

    fn start_component(&mut self, kind: ComponentKind, attrs: &[(String, AttrValue)]) {
        let mut name = String::new();
        let mut exported = Tribool::Default;
        let mut enabled = Tribool::Default;
        let mut permission: Option<String> = None;
        let mut authorities: Vec<DataAuthority> = Vec::new();

        for (k, v) in attrs {
            match k.as_str() {
                "name" => {
                    if let AttrValue::Str(s) = v { name = s.clone(); }
                }
                "exported" => {
                    exported = match v {
                        AttrValue::Bool(true) => Tribool::True,
                        AttrValue::Bool(false) => Tribool::False,
                        AttrValue::Int(n) if *n != 0 => Tribool::True,
                        AttrValue::Int(_) => Tribool::False,
                        _ => Tribool::Default,
                    };
                }
                "enabled" => {
                    enabled = match v {
                        AttrValue::Bool(true) | AttrValue::Int(1..) => Tribool::True,
                        AttrValue::Bool(false) => Tribool::False,
                        _ => Tribool::Default,
                    };
                }
                "permission" => {
                    if let AttrValue::Str(s) = v { permission = Some(s.clone()); }
                }
                "authorities" => {
                    if let AttrValue::Str(s) = v {
                        for auth in s.split(';') {
                            let auth = auth.trim();
                            if !auth.is_empty() {
                                authorities.push(DataAuthority { host: auth.to_owned(), port: None });
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        self.pending_component = Some(Component {
            kind,
            name,
            exported,
            enabled,
            permission,
            intent_filters: Vec::new(),
            authorities,
        });
        self.stack.push(Frame::Component(kind));
    }
}

#[derive(Debug, Clone)]
enum AttrValue {
    Str(String),
    Int(i64),
    Bool(bool),
    Ref(u32),
}

fn protection_level(raw: u32) -> ProtectionLevel {
    // Android protection level is a bitmask; the base level is in bits 0-3.
    match raw & 0x0f {
        0 => ProtectionLevel::Normal,
        1 => ProtectionLevel::Dangerous,
        2 => ProtectionLevel::Signature,
        3 => ProtectionLevel::SignatureOrSystem,
        4 => ProtectionLevel::Internal,
        _ => ProtectionLevel::Normal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{axml, emit};
    use crate::apk_data::Manifest;

    fn synthetic_manifest_bytes() -> Vec<u8> {
        // Reuse the synthetic from emit tests — minimal, no elements.
        let mut sp = Vec::new();
        sp.extend_from_slice(&axml::chunk_type::RES_STRING_POOL.to_le_bytes());
        sp.extend_from_slice(&28u16.to_le_bytes());
        sp.extend_from_slice(&28u32.to_le_bytes());
        sp.extend_from_slice(&[0u8; 20]);

        let mut out = Vec::new();
        out.extend_from_slice(&axml::chunk_type::RES_XML.to_le_bytes());
        out.extend_from_slice(&8u16.to_le_bytes());
        let total = (8u32 + sp.len() as u32).to_le_bytes();
        out.extend_from_slice(&total);
        out.extend_from_slice(&sp);
        out
    }

    #[test]
    fn empty_axml_gives_default_module() {
        let raw = synthetic_manifest_bytes();
        let m = Manifest { axml_bytes: raw };
        let ir = emit::emit_manifest(&m).expect("emit");
        let module = decode(&ir.doc);
        assert_eq!(module.package, "");
        assert_eq!(module.min_sdk, 0);
        assert!(module.components.is_empty());
    }
}
