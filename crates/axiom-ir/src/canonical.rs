// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Canonical-bytes wire format for AXIOM-IR v0.1.
//!
//! ## Wire shape
//!
//! Every encoded buffer starts with a 16-byte header:
//!
//! | Off | Len | Field           | Notes                                                |
//! |----:|----:|-----------------|------------------------------------------------------|
//! |   0 |   4 | magic           | ASCII `"AXIR"` (`0x41 0x58 0x49 0x52`)               |
//! |   4 |   2 | schema-major    | u16 BE — `0x0000` for v0.x                            |
//! |   6 |   2 | schema-minor    | u16 BE — `0x0001` for v0.1                            |
//! |   8 |   8 | payload-length  | u64 BE — bytes of payload following the header        |
//!
//! After the header comes the [`Module`] payload. Every variable-length
//! field is preceded by a `varint` (1-9 bytes) length, then the raw bytes.
//! Variants are distinguished by a single byte tag from the [`crate::core`]
//! tag tables.
//!
//! ## Determinism rules
//!
//! 1. Maps are emitted in sorted-key order (`BTreeMap` enforces this).
//! 2. Variants encode their tag *before* their inner payload — never after.
//! 3. Integers are big-endian. (No, this isn't accidental: BE is the
//!    de-facto standard for hash-friendly canonical formats and matches
//!    what every Lean test will produce on any host.)
//! 4. Strings are UTF-8 with a length prefix in bytes (not chars).
//! 5. There is no `#[non_exhaustive]` extension marker yet — when v0.2
//!    arrives, additional variants will be encoded behind tag `0xFE`,
//!    which v0.1 readers reject with [`IrError::UnknownExtension`].
//!
//! ## Why not serde / bincode
//!
//! Two reasons. First, the workspace deliberately avoids serde-family deps
//! (see `third-party/rust/Cargo.toml` — Reindeer's build-script runner
//! does not pass `CARGO_PKG_VERSION_PATCH`, which `serde_core`'s `build.rs`
//! requires). Second, we want the wire format to be exactly what we
//! specify — no derive-macro fragility and no surprise format changes
//! across serde versions.

// Varint encoding deliberately reads `u64` and projects to `usize` /
// `u32` / `u8` per the field type. Bounds are documented per call site:
// counts fit in `u32` because canonical bytes never exceed 4 GiB (an
// invariant the corpus harness enforces); SDK levels and tribool tags
// are 1-byte-bounded by construction.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions
)]

use crate::core::{
    Attribute, Block, IrError, Module, Operation, Region, Tribool, Type, Value, ValueId,
};
use crate::manifest::{
    self, Component, ComponentKind, DataAuthority, DataFilter, IntentFilter, ManifestModule,
    Permission, ProtectionLevel,
};
use crate::resource::{
    self, Configuration, ResourceEntry, ResourceId, ResourceRef, ResourceTable, ResourceType,
    StringPool,
};
use std::collections::BTreeMap;

const MAGIC: &[u8; 4] = b"AXIR";
const SCHEMA_MAJOR: u16 = 0;
const SCHEMA_MINOR: u16 = 1;
const HEADER_LEN: usize = 16;

const TAG_DIALECT_MANIFEST: u8 = 0x40;
const TAG_DIALECT_RESOURCE: u8 = 0x41;
const TAG_DIALECT_MIXED: u8 = 0x42;

const TAG_COMPONENT_ACTIVITY: u8 = 0x50;
const TAG_COMPONENT_SERVICE: u8 = 0x51;
const TAG_COMPONENT_RECEIVER: u8 = 0x52;
const TAG_COMPONENT_PROVIDER: u8 = 0x53;

const TAG_PROTECTION_NORMAL: u8 = 0x60;
const TAG_PROTECTION_DANGEROUS: u8 = 0x61;
const TAG_PROTECTION_SIGNATURE: u8 = 0x62;
const TAG_PROTECTION_SIGNATURE_OR_SYSTEM: u8 = 0x63;
const TAG_PROTECTION_INTERNAL: u8 = 0x64;

const TAG_RESOURCE_TYPE_STRING: u8 = 0x70;
const TAG_RESOURCE_TYPE_DRAWABLE: u8 = 0x71;
const TAG_RESOURCE_TYPE_LAYOUT: u8 = 0x72;
const TAG_RESOURCE_TYPE_COLOR: u8 = 0x73;
const TAG_RESOURCE_TYPE_DIMEN: u8 = 0x74;
const TAG_RESOURCE_TYPE_STYLE: u8 = 0x75;
const TAG_RESOURCE_TYPE_BOOL: u8 = 0x76;
const TAG_RESOURCE_TYPE_INTEGER: u8 = 0x77;
const TAG_RESOURCE_TYPE_RAW: u8 = 0x78;

const TAG_RESOURCE_ENTRY_STRING: u8 = 0xA0;
const TAG_RESOURCE_ENTRY_INT: u8 = 0xA1;
const TAG_RESOURCE_ENTRY_BOOL: u8 = 0xA2;
const TAG_RESOURCE_ENTRY_REF: u8 = 0xA3;

const TAG_EXTENSION: u8 = 0xFE;

// ---------------------------------------------------------------------------
// Encode
// ---------------------------------------------------------------------------

/// Encode a [`Module`] into canonical bytes.
///
/// The output is byte-deterministic for a given input — equal modules
/// always produce equal byte strings. This is the input to the IR
/// commitment hash.
#[must_use]
pub fn encode(module: &Module) -> Vec<u8> {
    let mut payload = Vec::with_capacity(256);
    write_module(&mut payload, module);

    let mut out = Vec::with_capacity(HEADER_LEN + payload.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&SCHEMA_MAJOR.to_be_bytes());
    out.extend_from_slice(&SCHEMA_MINOR.to_be_bytes());
    out.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    out.extend_from_slice(&payload);
    out
}

/// Encode + SHA-256 — the IR commitment hash.
///
/// Identical to running `sha256sum` over the output of [`encode`].
#[must_use]
pub fn commitment_hash(module: &Module) -> [u8; 32] {
    crate::hash::sha256(&encode(module))
}

fn write_module(out: &mut Vec<u8>, m: &Module) {
    write_str(out, &m.producer);
    out.push(dialect_tag(&m.dialect_tag));
    write_str(out, &m.dialect_tag);
    write_attr_map(out, &m.attributes);
    write_region(out, &m.region);
    write_varint(out, u64::from(m.next_value_id));
    // Manifest / resource dialect-specific payloads live in module
    // attributes (encoded above). Pure-IR construction does not produce
    // a dialect-specific tail; manifest::encode_into and resource::encode_into
    // hooks plug into write_attr_map via well-known keys.
}

fn write_region(out: &mut Vec<u8>, r: &Region) {
    write_varint(out, r.blocks.len() as u64);
    for b in &r.blocks {
        write_block(out, b);
    }
}

fn write_block(out: &mut Vec<u8>, b: &Block) {
    write_str(out, &b.label);
    write_varint(out, b.ops.len() as u64);
    for op in &b.ops {
        write_op(out, op);
    }
}

fn write_op(out: &mut Vec<u8>, op: &Operation) {
    write_str(out, &op.name);
    write_varint(out, op.operands.len() as u64);
    for id in &op.operands {
        write_varint(out, u64::from(id.0));
    }
    write_varint(out, op.results.len() as u64);
    for v in &op.results {
        write_value(out, v);
    }
    write_attr_map(out, &op.attributes);
    write_varint(out, op.regions.len() as u64);
    for r in &op.regions {
        write_region(out, r);
    }
}

fn write_value(out: &mut Vec<u8>, v: &Value) {
    write_varint(out, u64::from(v.id.0));
    write_type(out, &v.ty);
}

fn write_type(out: &mut Vec<u8>, t: &Type) {
    out.push(t.tag());
    match t {
        Type::List(inner) | Type::Option(inner) => write_type(out, inner),
        _ => {}
    }
}

fn write_attr_map(out: &mut Vec<u8>, m: &BTreeMap<String, Attribute>) {
    write_varint(out, m.len() as u64);
    for (k, v) in m {
        write_str(out, k);
        write_attr(out, v);
    }
}

fn write_attr(out: &mut Vec<u8>, a: &Attribute) {
    out.push(a.tag());
    match a {
        Attribute::Bool(b) => out.push(u8::from(*b)),
        Attribute::Tribool(t) => out.push(t.tag()),
        Attribute::U32(n) => out.extend_from_slice(&n.to_be_bytes()),
        Attribute::I32(n) => out.extend_from_slice(&n.to_be_bytes()),
        Attribute::String(s) => write_str(out, s),
        Attribute::Bytes(b) => write_bytes(out, b),
        Attribute::ApiLevel(api) => out.push(*api),
    }
}

fn write_str(out: &mut Vec<u8>, s: &str) {
    write_bytes(out, s.as_bytes());
}

fn write_bytes(out: &mut Vec<u8>, b: &[u8]) {
    write_varint(out, b.len() as u64);
    out.extend_from_slice(b);
}

/// Unsigned varint, big-endian, 1-9 bytes.
///
/// Encoding: top bit of each byte except the last is `1`; the last byte
/// has top bit `0`. Up to 9 bytes (8 * 7 = 56 bits + 1 special — enough
/// for any `u64`).
fn write_varint(out: &mut Vec<u8>, mut n: u64) {
    while n >= 0x80 {
        out.push(((n & 0x7f) as u8) | 0x80);
        n >>= 7;
    }
    out.push(n as u8);
}

const fn dialect_tag(tag: &str) -> u8 {
    if str_eq(tag, "manifest") {
        TAG_DIALECT_MANIFEST
    } else if str_eq(tag, "resource") {
        TAG_DIALECT_RESOURCE
    } else {
        // Default: any unknown dialect tag (incl. "mixed") encodes as
        // TAG_DIALECT_MIXED — readers preserve the raw string in the
        // module's `dialect_tag` field.
        TAG_DIALECT_MIXED
    }
}

const fn str_eq(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

// ---------------------------------------------------------------------------
// Decode
// ---------------------------------------------------------------------------

/// Decode canonical bytes into a [`Module`].
///
/// # Errors
/// Returns one of [`IrError`]'s wire-format variants on any malformed
/// input. Specifically:
/// * [`IrError::BadMagic`] — header magic isn't `AXIR`.
/// * [`IrError::UnknownExtension`] — byte stream encodes a v0.2+ schema.
/// * [`IrError::BadTag`] / [`IrError::BadUtf8`] / [`IrError::UnexpectedEof`]
///   — generic structural errors.
pub fn decode(bytes: &[u8]) -> Result<Module, IrError> {
    let mut r = Reader::new(bytes);
    let magic = r.take(4)?;
    if magic != MAGIC {
        let mut found = [0u8; 4];
        found.copy_from_slice(magic);
        return Err(IrError::BadMagic { found });
    }
    let major = r.read_u16()?;
    let minor = r.read_u16()?;
    if major != SCHEMA_MAJOR || minor != SCHEMA_MINOR {
        return Err(IrError::UnknownExtension {
            schema_version: crate::SCHEMA_VERSION,
        });
    }
    let _payload_len = r.read_u64()?;
    read_module(&mut r)
}

struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    const fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], IrError> {
        if self.pos + n > self.buf.len() {
            return Err(IrError::UnexpectedEof {
                needed: n,
                offset: self.pos,
            });
        }
        let s = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    fn read_u8(&mut self) -> Result<u8, IrError> {
        Ok(self.take(1)?[0])
    }

    fn read_u16(&mut self) -> Result<u16, IrError> {
        let b = self.take(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }

    fn read_u32(&mut self) -> Result<u32, IrError> {
        let b = self.take(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn read_i32(&mut self) -> Result<i32, IrError> {
        let b = self.take(4)?;
        Ok(i32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn read_u64(&mut self) -> Result<u64, IrError> {
        let b = self.take(8)?;
        Ok(u64::from_be_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    fn read_varint(&mut self) -> Result<u64, IrError> {
        let mut acc: u64 = 0;
        for shift in (0..64).step_by(7) {
            let b = self.read_u8()?;
            acc |= u64::from(b & 0x7f) << shift;
            if b & 0x80 == 0 {
                return Ok(acc);
            }
        }
        Err(IrError::Invariant("varint > 9 bytes".into()))
    }

    fn read_str(&mut self) -> Result<String, IrError> {
        let off = self.pos;
        let bytes = self.read_bytes()?;
        String::from_utf8(bytes).map_err(|_| IrError::BadUtf8 { offset: off })
    }

    fn read_bytes(&mut self) -> Result<Vec<u8>, IrError> {
        let len = self.read_varint()? as usize;
        Ok(self.take(len)?.to_vec())
    }
}

fn read_module(r: &mut Reader<'_>) -> Result<Module, IrError> {
    let producer = r.read_str()?;
    let dialect_byte = r.read_u8()?;
    if dialect_byte == TAG_EXTENSION {
        return Err(IrError::UnknownExtension {
            schema_version: crate::SCHEMA_VERSION,
        });
    }
    if !matches!(
        dialect_byte,
        TAG_DIALECT_MANIFEST | TAG_DIALECT_RESOURCE | TAG_DIALECT_MIXED
    ) {
        return Err(IrError::BadTag {
            ctx: "Module::dialect_tag",
            tag: dialect_byte,
        });
    }
    let dialect_tag = r.read_str()?;
    let attributes = read_attr_map(r)?;
    let region = read_region(r)?;
    let next_value_id = r.read_varint()?;
    Ok(Module {
        producer,
        dialect_tag,
        attributes,
        region,
        next_value_id: next_value_id as u32,
    })
}

fn read_region(r: &mut Reader<'_>) -> Result<Region, IrError> {
    let count = r.read_varint()? as usize;
    let mut blocks = Vec::with_capacity(count);
    for _ in 0..count {
        blocks.push(read_block(r)?);
    }
    Ok(Region { blocks })
}

fn read_block(r: &mut Reader<'_>) -> Result<Block, IrError> {
    let label = r.read_str()?;
    let count = r.read_varint()? as usize;
    let mut ops = Vec::with_capacity(count);
    for _ in 0..count {
        ops.push(read_op(r)?);
    }
    Ok(Block { label, ops })
}

fn read_op(r: &mut Reader<'_>) -> Result<Operation, IrError> {
    let name = r.read_str()?;
    let n_operands = r.read_varint()? as usize;
    let mut operands = Vec::with_capacity(n_operands);
    for _ in 0..n_operands {
        operands.push(ValueId(r.read_varint()? as u32));
    }
    let n_results = r.read_varint()? as usize;
    let mut results = Vec::with_capacity(n_results);
    for _ in 0..n_results {
        results.push(read_value(r)?);
    }
    let attributes = read_attr_map(r)?;
    let n_regions = r.read_varint()? as usize;
    let mut regions = Vec::with_capacity(n_regions);
    for _ in 0..n_regions {
        regions.push(read_region(r)?);
    }
    Ok(Operation {
        name,
        operands,
        results,
        attributes,
        regions,
    })
}

fn read_value(r: &mut Reader<'_>) -> Result<Value, IrError> {
    let id = ValueId(r.read_varint()? as u32);
    let ty = read_type(r)?;
    Ok(Value { id, ty })
}

fn read_type(r: &mut Reader<'_>) -> Result<Type, IrError> {
    let tag = r.read_u8()?;
    Ok(match tag {
        0x10 => Type::Tribool,
        0x11 => Type::U32,
        0x12 => Type::I32,
        0x13 => Type::String,
        0x14 => Type::Bytes,
        0x15 => Type::ResourceRef,
        0x16 => Type::PermissionRef,
        0x17 => Type::ComponentName,
        0x18 => Type::ApiLevel,
        0x80 => Type::List(Box::new(read_type(r)?)),
        0x81 => Type::Option(Box::new(read_type(r)?)),
        TAG_EXTENSION => {
            return Err(IrError::UnknownExtension {
                schema_version: crate::SCHEMA_VERSION,
            })
        }
        _ => return Err(IrError::BadTag { ctx: "Type", tag }),
    })
}

fn read_attr_map(r: &mut Reader<'_>) -> Result<BTreeMap<String, Attribute>, IrError> {
    let count = r.read_varint()? as usize;
    let mut m = BTreeMap::new();
    for _ in 0..count {
        let k = r.read_str()?;
        let v = read_attr(r)?;
        m.insert(k, v);
    }
    Ok(m)
}

fn read_attr(r: &mut Reader<'_>) -> Result<Attribute, IrError> {
    let tag = r.read_u8()?;
    Ok(match tag {
        0x20 => Attribute::Bool(r.read_u8()? != 0),
        0x21 => Attribute::Tribool(Tribool::from_tag(r.read_u8()?)?),
        0x22 => Attribute::U32(r.read_u32()?),
        0x23 => Attribute::I32(r.read_i32()?),
        0x24 => Attribute::String(r.read_str()?),
        0x25 => Attribute::Bytes(r.read_bytes()?),
        0x26 => Attribute::ApiLevel(r.read_u8()?),
        TAG_EXTENSION => {
            return Err(IrError::UnknownExtension {
                schema_version: crate::SCHEMA_VERSION,
            })
        }
        _ => {
            return Err(IrError::BadTag {
                ctx: "Attribute",
                tag,
            })
        }
    })
}

// ---------------------------------------------------------------------------
// Manifest dialect canonical bytes
// ---------------------------------------------------------------------------

/// Encode a [`ManifestModule`] into canonical bytes.
///
/// Manifests serialise both a [`ManifestModule`] payload (the
/// dialect-specific data) *and* the IR shell. The shell encodes the
/// dialect-specific data into a single `Bytes` attribute under the
/// well-known key `"manifest.payload"`. Decoding reverses this.
#[must_use]
pub fn encode_manifest(m: &ManifestModule) -> Vec<u8> {
    encode(&manifest::wrap_module(m))
}

/// Decode a [`ManifestModule`] from canonical bytes.
///
/// # Errors
/// Returns [`IrError`] on a malformed shell or missing payload attribute.
pub fn decode_manifest(bytes: &[u8]) -> Result<ManifestModule, IrError> {
    let module = decode(bytes)?;
    manifest::unwrap_module(&module)
}

/// Encode a [`ResourceTable`] into canonical bytes (resource dialect).
#[must_use]
pub fn encode_resource(t: &ResourceTable) -> Vec<u8> {
    encode(&resource::wrap_module(t))
}

/// Decode a [`ResourceTable`] from canonical bytes.
///
/// # Errors
/// Returns [`IrError`] on a malformed shell or missing payload attribute.
pub fn decode_resource(bytes: &[u8]) -> Result<ResourceTable, IrError> {
    let module = decode(bytes)?;
    resource::unwrap_module(&module)
}

// ---------------------------------------------------------------------------
// Manifest payload (dialect-private)
// ---------------------------------------------------------------------------

pub(crate) fn write_manifest_payload(out: &mut Vec<u8>, m: &ManifestModule) {
    write_str(out, &m.package);
    write_varint(out, u64::from(m.target_sdk));
    write_varint(out, u64::from(m.min_sdk));
    write_optional_str(out, m.application_label.as_deref());
    write_varint(out, m.components.len() as u64);
    for c in &m.components {
        write_component(out, c);
    }
    write_varint(out, m.permissions.len() as u64);
    for p in &m.permissions {
        write_permission(out, p);
    }
    write_varint(out, m.uses_permissions.len() as u64);
    for p in &m.uses_permissions {
        write_str(out, p);
    }
}

pub(crate) fn read_manifest_payload(bytes: &[u8]) -> Result<ManifestModule, IrError> {
    let mut r = Reader::new(bytes);
    let package = r.read_str()?;
    let target_sdk = r.read_varint()? as u8;
    let min_sdk = r.read_varint()? as u8;
    let application_label = read_optional_str(&mut r)?;
    let n_components = r.read_varint()? as usize;
    let mut components = Vec::with_capacity(n_components);
    for _ in 0..n_components {
        components.push(read_component(&mut r)?);
    }
    let n_permissions = r.read_varint()? as usize;
    let mut permissions = Vec::with_capacity(n_permissions);
    for _ in 0..n_permissions {
        permissions.push(read_permission(&mut r)?);
    }
    let n_uses = r.read_varint()? as usize;
    let mut uses_permissions = Vec::with_capacity(n_uses);
    for _ in 0..n_uses {
        uses_permissions.push(r.read_str()?);
    }
    Ok(ManifestModule {
        package,
        target_sdk,
        min_sdk,
        application_label,
        components,
        permissions,
        uses_permissions,
    })
}

fn write_component(out: &mut Vec<u8>, c: &Component) {
    out.push(match c.kind {
        ComponentKind::Activity => TAG_COMPONENT_ACTIVITY,
        ComponentKind::Service => TAG_COMPONENT_SERVICE,
        ComponentKind::Receiver => TAG_COMPONENT_RECEIVER,
        ComponentKind::Provider => TAG_COMPONENT_PROVIDER,
    });
    write_str(out, &c.name);
    out.push(c.exported.tag());
    out.push(c.enabled.tag());
    write_optional_str(out, c.permission.as_deref());
    write_varint(out, c.intent_filters.len() as u64);
    for f in &c.intent_filters {
        write_intent_filter(out, f);
    }
    write_varint(out, c.authorities.len() as u64);
    for a in &c.authorities {
        write_data_authority(out, a);
    }
}

fn read_component(r: &mut Reader<'_>) -> Result<Component, IrError> {
    let kind = match r.read_u8()? {
        TAG_COMPONENT_ACTIVITY => ComponentKind::Activity,
        TAG_COMPONENT_SERVICE => ComponentKind::Service,
        TAG_COMPONENT_RECEIVER => ComponentKind::Receiver,
        TAG_COMPONENT_PROVIDER => ComponentKind::Provider,
        tag => {
            return Err(IrError::BadTag {
                ctx: "ComponentKind",
                tag,
            })
        }
    };
    let name = r.read_str()?;
    let exported = Tribool::from_tag(r.read_u8()?)?;
    let enabled = Tribool::from_tag(r.read_u8()?)?;
    let permission = read_optional_str(r)?;
    let n_filters = r.read_varint()? as usize;
    let mut intent_filters = Vec::with_capacity(n_filters);
    for _ in 0..n_filters {
        intent_filters.push(read_intent_filter(r)?);
    }
    let n_authorities = r.read_varint()? as usize;
    let mut authorities = Vec::with_capacity(n_authorities);
    for _ in 0..n_authorities {
        authorities.push(read_data_authority(r)?);
    }
    Ok(Component {
        kind,
        name,
        exported,
        enabled,
        permission,
        intent_filters,
        authorities,
    })
}

fn write_intent_filter(out: &mut Vec<u8>, f: &IntentFilter) {
    write_str_list(out, &f.actions);
    write_str_list(out, &f.categories);
    write_varint(out, f.data.len() as u64);
    for d in &f.data {
        write_data_filter(out, d);
    }
    out.extend_from_slice(&f.priority.to_be_bytes());
}

fn read_intent_filter(r: &mut Reader<'_>) -> Result<IntentFilter, IrError> {
    let actions = read_str_list(r)?;
    let categories = read_str_list(r)?;
    let n_data = r.read_varint()? as usize;
    let mut data = Vec::with_capacity(n_data);
    for _ in 0..n_data {
        data.push(read_data_filter(r)?);
    }
    let priority = r.read_i32()?;
    Ok(IntentFilter {
        actions,
        categories,
        data,
        priority,
    })
}

fn write_data_filter(out: &mut Vec<u8>, d: &DataFilter) {
    write_optional_str(out, d.scheme.as_deref());
    write_optional_str(out, d.host.as_deref());
    write_optional_str(out, d.port.as_deref());
    write_optional_str(out, d.path.as_deref());
    write_optional_str(out, d.path_prefix.as_deref());
    write_optional_str(out, d.path_pattern.as_deref());
    write_optional_str(out, d.mime_type.as_deref());
}

fn read_data_filter(r: &mut Reader<'_>) -> Result<DataFilter, IrError> {
    Ok(DataFilter {
        scheme: read_optional_str(r)?,
        host: read_optional_str(r)?,
        port: read_optional_str(r)?,
        path: read_optional_str(r)?,
        path_prefix: read_optional_str(r)?,
        path_pattern: read_optional_str(r)?,
        mime_type: read_optional_str(r)?,
    })
}

fn write_data_authority(out: &mut Vec<u8>, a: &DataAuthority) {
    write_str(out, &a.host);
    write_optional_str(out, a.port.as_deref());
}

fn read_data_authority(r: &mut Reader<'_>) -> Result<DataAuthority, IrError> {
    Ok(DataAuthority {
        host: r.read_str()?,
        port: read_optional_str(r)?,
    })
}

fn write_permission(out: &mut Vec<u8>, p: &Permission) {
    write_str(out, &p.name);
    out.push(match p.protection {
        ProtectionLevel::Normal => TAG_PROTECTION_NORMAL,
        ProtectionLevel::Dangerous => TAG_PROTECTION_DANGEROUS,
        ProtectionLevel::Signature => TAG_PROTECTION_SIGNATURE,
        ProtectionLevel::SignatureOrSystem => TAG_PROTECTION_SIGNATURE_OR_SYSTEM,
        ProtectionLevel::Internal => TAG_PROTECTION_INTERNAL,
    });
    write_optional_str(out, p.group.as_deref());
}

fn read_permission(r: &mut Reader<'_>) -> Result<Permission, IrError> {
    let name = r.read_str()?;
    let protection = match r.read_u8()? {
        TAG_PROTECTION_NORMAL => ProtectionLevel::Normal,
        TAG_PROTECTION_DANGEROUS => ProtectionLevel::Dangerous,
        TAG_PROTECTION_SIGNATURE => ProtectionLevel::Signature,
        TAG_PROTECTION_SIGNATURE_OR_SYSTEM => ProtectionLevel::SignatureOrSystem,
        TAG_PROTECTION_INTERNAL => ProtectionLevel::Internal,
        tag => {
            return Err(IrError::BadTag {
                ctx: "ProtectionLevel",
                tag,
            })
        }
    };
    let group = read_optional_str(r)?;
    Ok(Permission {
        name,
        protection,
        group,
    })
}

fn write_str_list(out: &mut Vec<u8>, xs: &[String]) {
    write_varint(out, xs.len() as u64);
    for s in xs {
        write_str(out, s);
    }
}

fn read_str_list(r: &mut Reader<'_>) -> Result<Vec<String>, IrError> {
    let n = r.read_varint()? as usize;
    let mut xs = Vec::with_capacity(n);
    for _ in 0..n {
        xs.push(r.read_str()?);
    }
    Ok(xs)
}

fn write_optional_str(out: &mut Vec<u8>, s: Option<&str>) {
    match s {
        None => out.push(0),
        Some(v) => {
            out.push(1);
            write_str(out, v);
        }
    }
}

fn read_optional_str(r: &mut Reader<'_>) -> Result<Option<String>, IrError> {
    match r.read_u8()? {
        0 => Ok(None),
        1 => Ok(Some(r.read_str()?)),
        tag => Err(IrError::BadTag {
            ctx: "Option<String>",
            tag,
        }),
    }
}

// ---------------------------------------------------------------------------
// Resource payload (dialect-private)
// ---------------------------------------------------------------------------

pub(crate) fn write_resource_payload(out: &mut Vec<u8>, t: &ResourceTable) {
    write_str(out, &t.package);
    write_string_pool(out, &t.string_pool);
    write_varint(out, t.configurations.len() as u64);
    for c in &t.configurations {
        write_configuration(out, c);
    }
    write_varint(out, t.entries.len() as u64);
    for e in &t.entries {
        write_resource_entry(out, e);
    }
}

pub(crate) fn read_resource_payload(bytes: &[u8]) -> Result<ResourceTable, IrError> {
    let mut r = Reader::new(bytes);
    let package = r.read_str()?;
    let string_pool = read_string_pool(&mut r)?;
    let n_configs = r.read_varint()? as usize;
    let mut configurations = Vec::with_capacity(n_configs);
    for _ in 0..n_configs {
        configurations.push(read_configuration(&mut r)?);
    }
    let n_entries = r.read_varint()? as usize;
    let mut entries = Vec::with_capacity(n_entries);
    for _ in 0..n_entries {
        entries.push(read_resource_entry(&mut r)?);
    }
    Ok(ResourceTable {
        package,
        string_pool,
        configurations,
        entries,
    })
}

fn write_string_pool(out: &mut Vec<u8>, p: &StringPool) {
    write_str_list(out, &p.strings);
}

fn read_string_pool(r: &mut Reader<'_>) -> Result<StringPool, IrError> {
    Ok(StringPool {
        strings: read_str_list(r)?,
    })
}

fn write_configuration(out: &mut Vec<u8>, c: &Configuration) {
    write_str(out, &c.qualifier);
    out.extend_from_slice(&c.density_dpi.to_be_bytes());
    write_optional_str(out, c.locale.as_deref());
    write_optional_str(out, c.orientation.as_deref());
    out.push(c.min_sdk);
}

fn read_configuration(r: &mut Reader<'_>) -> Result<Configuration, IrError> {
    Ok(Configuration {
        qualifier: r.read_str()?,
        density_dpi: r.read_u32()?,
        locale: read_optional_str(r)?,
        orientation: read_optional_str(r)?,
        min_sdk: r.read_u8()?,
    })
}

fn write_resource_entry(out: &mut Vec<u8>, e: &ResourceEntry) {
    out.push(resource_type_tag(e.ref_.r#type));
    out.extend_from_slice(&e.ref_.id.0.to_be_bytes());
    write_str(out, &e.ref_.name);
    match &e.value {
        resource::ResourceValue::String(s) => {
            out.push(TAG_RESOURCE_ENTRY_STRING);
            write_str(out, s);
        }
        resource::ResourceValue::Int(n) => {
            out.push(TAG_RESOURCE_ENTRY_INT);
            out.extend_from_slice(&n.to_be_bytes());
        }
        resource::ResourceValue::Bool(b) => {
            out.push(TAG_RESOURCE_ENTRY_BOOL);
            out.push(u8::from(*b));
        }
        resource::ResourceValue::Ref(r) => {
            out.push(TAG_RESOURCE_ENTRY_REF);
            out.push(resource_type_tag(r.r#type));
            out.extend_from_slice(&r.id.0.to_be_bytes());
            write_str(out, &r.name);
        }
    }
}

fn read_resource_entry(r: &mut Reader<'_>) -> Result<ResourceEntry, IrError> {
    let ty = read_resource_type(r)?;
    let id = ResourceId(r.read_u32()?);
    let name = r.read_str()?;
    let ref_ = ResourceRef {
        r#type: ty,
        id,
        name,
    };
    let value = match r.read_u8()? {
        TAG_RESOURCE_ENTRY_STRING => resource::ResourceValue::String(r.read_str()?),
        TAG_RESOURCE_ENTRY_INT => resource::ResourceValue::Int(r.read_i32()?),
        TAG_RESOURCE_ENTRY_BOOL => resource::ResourceValue::Bool(r.read_u8()? != 0),
        TAG_RESOURCE_ENTRY_REF => {
            let ty = read_resource_type(r)?;
            let id = ResourceId(r.read_u32()?);
            let name = r.read_str()?;
            resource::ResourceValue::Ref(ResourceRef {
                r#type: ty,
                id,
                name,
            })
        }
        tag => {
            return Err(IrError::BadTag {
                ctx: "ResourceValue",
                tag,
            })
        }
    };
    Ok(ResourceEntry { ref_, value })
}

const fn resource_type_tag(t: ResourceType) -> u8 {
    match t {
        ResourceType::String => TAG_RESOURCE_TYPE_STRING,
        ResourceType::Drawable => TAG_RESOURCE_TYPE_DRAWABLE,
        ResourceType::Layout => TAG_RESOURCE_TYPE_LAYOUT,
        ResourceType::Color => TAG_RESOURCE_TYPE_COLOR,
        ResourceType::Dimen => TAG_RESOURCE_TYPE_DIMEN,
        ResourceType::Style => TAG_RESOURCE_TYPE_STYLE,
        ResourceType::Bool => TAG_RESOURCE_TYPE_BOOL,
        ResourceType::Integer => TAG_RESOURCE_TYPE_INTEGER,
        ResourceType::Raw => TAG_RESOURCE_TYPE_RAW,
    }
}

fn read_resource_type(r: &mut Reader<'_>) -> Result<ResourceType, IrError> {
    Ok(match r.read_u8()? {
        TAG_RESOURCE_TYPE_STRING => ResourceType::String,
        TAG_RESOURCE_TYPE_DRAWABLE => ResourceType::Drawable,
        TAG_RESOURCE_TYPE_LAYOUT => ResourceType::Layout,
        TAG_RESOURCE_TYPE_COLOR => ResourceType::Color,
        TAG_RESOURCE_TYPE_DIMEN => ResourceType::Dimen,
        TAG_RESOURCE_TYPE_STYLE => ResourceType::Style,
        TAG_RESOURCE_TYPE_BOOL => ResourceType::Bool,
        TAG_RESOURCE_TYPE_INTEGER => ResourceType::Integer,
        TAG_RESOURCE_TYPE_RAW => ResourceType::Raw,
        tag => {
            return Err(IrError::BadTag {
                ctx: "ResourceType",
                tag,
            })
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Module;

    #[test]
    fn varint_round_trip_boundary() {
        for n in [
            0u64,
            1,
            0x7f,
            0x80,
            0x3fff,
            0x4000,
            u32::MAX as u64,
            u64::MAX,
        ] {
            let mut buf = Vec::new();
            write_varint(&mut buf, n);
            let mut r = Reader::new(&buf);
            assert_eq!(r.read_varint().unwrap(), n);
        }
    }

    #[test]
    fn empty_module_decode_round_trip() {
        let m = Module::empty("manifest");
        let bytes = encode(&m);
        let m2 = decode(&bytes).unwrap();
        assert_eq!(encode(&m2), bytes);
        assert_eq!(m2.dialect_tag, "manifest");
    }

    #[test]
    fn bad_magic_rejected() {
        let mut bytes = encode(&Module::empty("manifest"));
        bytes[0] = b'X';
        assert!(matches!(decode(&bytes), Err(IrError::BadMagic { .. })));
    }

    #[test]
    fn bad_schema_rejected() {
        let mut bytes = encode(&Module::empty("manifest"));
        bytes[7] = 0xff; // schema_minor low byte → not v0.1
        assert!(matches!(
            decode(&bytes),
            Err(IrError::UnknownExtension { .. })
        ));
    }

    #[test]
    fn truncated_input_rejected() {
        let bytes = encode(&Module::empty("manifest"));
        assert!(matches!(
            decode(&bytes[..10]),
            Err(IrError::UnexpectedEof { .. })
        ));
    }

    #[test]
    fn extension_tag_is_distinct() {
        // The TAG_EXTENSION value (0xFE) must not collide with any
        // production tag — re-verify the few we've issued so far.
        for forbidden in [
            TAG_DIALECT_MANIFEST,
            TAG_DIALECT_RESOURCE,
            TAG_DIALECT_MIXED,
            TAG_COMPONENT_ACTIVITY,
            TAG_COMPONENT_PROVIDER,
            TAG_PROTECTION_NORMAL,
            TAG_PROTECTION_INTERNAL,
            TAG_RESOURCE_TYPE_STRING,
            TAG_RESOURCE_TYPE_RAW,
            TAG_RESOURCE_ENTRY_STRING,
            TAG_RESOURCE_ENTRY_REF,
        ] {
            assert_ne!(forbidden, TAG_EXTENSION);
        }
    }

    #[test]
    fn commitment_hash_is_deterministic() {
        let m = Module::empty("manifest");
        assert_eq!(commitment_hash(&m), commitment_hash(&m));
    }
}
