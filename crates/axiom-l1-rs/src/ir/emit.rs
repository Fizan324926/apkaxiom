// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Glue from `Apk<FullyParsed<V>>` → AXIOM-IR-v0.1 dialects.
//!
//! Two paths:
//!   - manifest: `Manifest::axml_bytes` → `axml::AxmlDoc` →
//!     [`ManifestIr`].
//!   - resources: `Resources::arsc_bytes` → `arsc::ArscDoc` →
//!     [`ResourceIr`].
//!
//! The IR carriers preserve the structural form (the parsed
//! [`super::axml::AxmlDoc`] / [`super::arsc::ArscDoc`]) so the
//! reverse path back to bytes is byte-identical for every chunk
//! we recognise.

use axiom_ir::manifest::ManifestModule;
use axiom_ir::resource::ResourceTable;

use crate::apk_data::{Manifest, Resources};
use super::{arsc, axml, manifest_decode, resource_decode};

/// Manifest IR — carries both the structural chunk tree (for byte-
/// identical reencode) and the decoded [`ManifestModule`] semantic view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestIr {
    /// Parsed AXML chunk tree — write back with [`reencode_manifest`] for
    /// byte-identical output.
    pub doc: axml::AxmlDoc,
    /// Semantic manifest view — package name, components, permissions, SDK
    /// bounds. Populated by [`manifest_decode::decode`].
    pub module: ManifestModule,
}

/// Resource IR — carries both the structural ARSC chunk tree and the
/// decoded [`ResourceTable`] semantic view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceIr {
    /// Parsed ARSC chunk tree — write back with [`reencode_resources`] for
    /// byte-identical output.
    pub doc: arsc::ArscDoc,
    /// Semantic resource view — package name, global string pool, type/key
    /// names. Populated by [`resource_decode::decode`].
    pub table: ResourceTable,
}

/// Emit a manifest IR from the raw AXML bytes wrapped by
/// [`Manifest`]. Errors propagate from the chunk parser.
///
/// Both the structural chunk tree and the [`ManifestModule`] semantic
/// view are populated in one pass.
pub fn emit_manifest(manifest: &Manifest) -> Result<ManifestIr, axml::AxmlError> {
    let doc = axml::parse(&manifest.axml_bytes)?;
    let module = manifest_decode::decode(&doc);
    Ok(ManifestIr { doc, module })
}

/// Reverse path: produce AXML bytes from a [`ManifestIr`].
#[must_use]
pub fn reencode_manifest(ir: &ManifestIr) -> Vec<u8> {
    axml::emit(&ir.doc)
}

/// Emit a resource IR from the raw ARSC bytes wrapped by
/// [`Resources`].
///
/// Both the structural chunk tree and the [`ResourceTable`] semantic
/// view are populated in one pass.
pub fn emit_resources(resources: &Resources) -> Result<ResourceIr, arsc::ArscError> {
    let doc = arsc::parse(&resources.arsc_bytes)?;
    let table = resource_decode::decode(&doc);
    Ok(ResourceIr { doc, table })
}

/// Reverse path: produce ARSC bytes from a [`ResourceIr`].
#[must_use]
pub fn reencode_resources(ir: &ResourceIr) -> Vec<u8> {
    arsc::emit(&ir.doc)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Synthetic minimal AXML — same payload as the unit test in
    /// the `axml` module. Verifies the glue layer doesn't lose
    /// bytes.
    fn synthetic_axml_manifest() -> Manifest {
        let mut s = Vec::new();
        s.extend_from_slice(&axml::chunk_type::RES_STRING_POOL.to_le_bytes());
        s.extend_from_slice(&28u16.to_le_bytes());
        s.extend_from_slice(&28u32.to_le_bytes());
        s.extend_from_slice(&[0u8; 20]);
        let mut out = Vec::new();
        out.extend_from_slice(&axml::chunk_type::RES_XML.to_le_bytes());
        out.extend_from_slice(&8u16.to_le_bytes());
        let total = (8 + s.len()) as u32;
        out.extend_from_slice(&total.to_le_bytes());
        out.extend_from_slice(&s);
        Manifest { axml_bytes: out }
    }

    #[test]
    fn manifest_round_trip_byte_identical() {
        let m = synthetic_axml_manifest();
        let ir = emit_manifest(&m).expect("emit");
        let out = reencode_manifest(&ir);
        assert_eq!(m.axml_bytes, out);
    }
}
