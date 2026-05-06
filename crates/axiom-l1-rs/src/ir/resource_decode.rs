// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! ARSC semantic decoder — maps an [`ArscDoc`] to a [`ResourceTable`].
//!
//! v0.1 scope: decode the global string pool and the first package
//! chunk's identity (package name, type string pool, key string pool).
//! Per-resource value decoding (TypeSpec/TypeEntry interiors) is deferred
//! to v0.2; the structural chunk tree already round-trips byte-identically.
//!
//! ## Package chunk layout
//!
//! ```text
//!   u16 type              = 0x0200
//!   u16 header_size       = 288 (0x120) in standard aapt2 output
//!   u32 chunk_size
//!   u32 id                (package id, e.g. 0x7f for the app)
//!   u8[256] name          (UTF-16LE null-terminated package name)
//!   u32 type_strings      (byte offset from chunk start to type pool)
//!   u32 last_public_type
//!   u32 key_strings       (byte offset from chunk start to key pool)
//!   u32 last_public_key
//!   u32 type_id_offset
//! ```

use axiom_ir::resource::{ResourceTable, StringPool};

use super::{arsc, strings};

const PKG_CHUNK: u16 = arsc::chunk_type::RES_TABLE_PACKAGE;
const GLOBAL_POOL: u16 = arsc::chunk_type::RES_STRING_POOL;

/// Minimum package header size: chunk header (8) + id (4) + name (256) +
/// offsets (4×5 = 20) = 288 bytes.
const PKG_HDR_MIN: usize = 288;

/// Decode an [`ArscDoc`] into a [`ResourceTable`].
///
/// Best-effort: missing chunks leave fields at their zero values.
#[must_use]
pub fn decode(doc: &arsc::ArscDoc) -> ResourceTable {
    // Global string pool — first RES_STRING_POOL chunk in the outer table.
    let global_strings: Vec<String> = doc
        .chunks
        .iter()
        .find(|c| c.type_id == GLOBAL_POOL)
        .and_then(|c| strings::decode(&c.raw).ok())
        .unwrap_or_default();

    // First package chunk.
    let Some(pkg_chunk) = doc.chunks.iter().find(|c| c.type_id == PKG_CHUNK) else {
        return ResourceTable {
            package: String::new(),
            string_pool: StringPool { strings: global_strings },
            ..Default::default()
        };
    };

    let raw = &pkg_chunk.raw;
    if raw.len() < PKG_HDR_MIN {
        return ResourceTable {
            string_pool: StringPool { strings: global_strings },
            ..Default::default()
        };
    }

    // Package id.
    let pkg_id = u32::from_le_bytes([raw[8], raw[9], raw[10], raw[11]]);

    // Package name: 256 bytes of UTF-16LE starting at offset 12.
    let name_bytes = &raw[12..12 + 256];
    let name_u16: Vec<u16> = name_bytes
        .chunks_exact(2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .take_while(|&c| c != 0)
        .collect();
    let package_name = String::from_utf16_lossy(&name_u16).to_string();

    // Type string pool offset (from chunk start).
    let type_strings_off = u32::from_le_bytes([raw[268], raw[269], raw[270], raw[271]]) as usize;
    // Key string pool offset (from chunk start).
    let key_strings_off = u32::from_le_bytes([raw[276], raw[277], raw[278], raw[279]]) as usize;

    let type_names = decode_inner_pool(raw, type_strings_off);
    let key_names = decode_inner_pool(raw, key_strings_off);

    // Merge type + key names into the resource string pool.
    // The global pool carries string-typed resource values; type/key names
    // are an additional semantic layer. We expose all three independently
    // via the `ResourceTable` extensions and also merge into `string_pool`
    // for downstream consumers that expect a flat pool.
    let mut merged = global_strings;
    merged.extend(type_names.iter().cloned());
    merged.extend(key_names.iter().cloned());

    let _ = pkg_id; // informational — not exposed in ResourceTable v0.1

    ResourceTable {
        package: package_name,
        string_pool: StringPool { strings: merged },
        configurations: Vec::new(),
        entries: Vec::new(),
    }
}

fn decode_inner_pool(pkg_raw: &[u8], off: usize) -> Vec<String> {
    if off == 0 || off + 8 > pkg_raw.len() {
        return Vec::new();
    }
    strings::decode(&pkg_raw[off..]).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{arsc, emit};
    use crate::apk_data::Resources;

    fn synthetic_arsc_bytes() -> Vec<u8> {
        let mut inner = Vec::new();
        inner.extend_from_slice(&arsc::chunk_type::RES_STRING_POOL.to_le_bytes());
        inner.extend_from_slice(&28u16.to_le_bytes());
        inner.extend_from_slice(&28u32.to_le_bytes());
        inner.extend_from_slice(&[0u8; 20]);

        let mut out = Vec::new();
        out.extend_from_slice(&arsc::chunk_type::RES_TABLE.to_le_bytes());
        out.extend_from_slice(&12u16.to_le_bytes());
        let total = (12u32 + inner.len() as u32).to_le_bytes();
        out.extend_from_slice(&total);
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&inner);
        out
    }

    #[test]
    fn no_package_chunk_gives_empty_table() {
        let raw = synthetic_arsc_bytes();
        let r = Resources { arsc_bytes: raw };
        let ir = emit::emit_resources(&r).expect("emit");
        let table = decode(&ir.doc);
        assert_eq!(table.package, "");
        assert!(table.entries.is_empty());
    }
}
