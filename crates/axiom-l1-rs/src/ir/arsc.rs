// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Chunked binary `resources.arsc` parser + emitter.
//!
//! ARSC is the Android Resource Container — same chunk format
//! family as AXML. The top-level chunk is `RES_TABLE_TYPE`
//! (0x0002); inside it lives a global string pool (chunk type
//! 0x0001) followed by per-package chunks (chunk type 0x0200,
//! `RES_TABLE_PACKAGE_TYPE`).
//!
//! ## Round-trip strategy
//!
//! Same as [`super::axml`]: keep raw chunk bytes alongside the
//! parsed structural form. Re-emission writes the raw bytes
//! back, guaranteeing byte-identical round-trip on every chunk
//! we parse and on opaque chunks we don't.
//!
//! v0.1 scope: structural decode of the chunk tree (table header,
//! string pool, package chunks). Per-resource value decoding
//! (TypeSpec / TypeEntry interior) is **not** semantically
//! decoded by this module — it stays as opaque payload bytes.
//! Resource-value-level decoding lands in v0.2; the v0.1 dialect
//! is sufficient for the round-trip gate because the chunk-level
//! representation already captures every byte of the original.

#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]

use std::convert::TryFrom;

/// ARSC chunk type IDs.
pub mod chunk_type {
    /// `RES_STRING_POOL_TYPE`.
    pub const RES_STRING_POOL: u16 = 0x0001;
    /// `RES_TABLE_TYPE` — outer resource table wrapper.
    pub const RES_TABLE: u16 = 0x0002;
    /// `RES_TABLE_PACKAGE_TYPE`.
    pub const RES_TABLE_PACKAGE: u16 = 0x0200;
    /// `RES_TABLE_TYPE_SPEC_TYPE`.
    pub const RES_TABLE_TYPE_SPEC: u16 = 0x0202;
    /// `RES_TABLE_TYPE_TYPE`.
    pub const RES_TABLE_TYPE: u16 = 0x0201;
}

/// Errors surfaced by the parser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArscError {
    /// Buffer is shorter than a chunk header demands.
    Truncated {
        /// Byte offset where truncation was detected.
        offset: usize,
        /// How many bytes were needed.
        need: usize,
        /// How many bytes were available.
        have: usize,
    },
    /// Top-level chunk is not `RES_TABLE_TYPE`.
    NotArsc {
        /// The type id that was actually seen.
        type_seen: u16,
    },
    /// A chunk's declared size overflows the buffer.
    ChunkOverflow {
        /// Byte offset of the offending chunk.
        offset: usize,
        /// The declared size.
        declared: u32,
        /// Bytes remaining from that offset.
        remaining: u32,
    },
    /// Header size shorter than 8 bytes.
    HeaderTooSmall {
        /// Byte offset of the offending chunk.
        offset: usize,
        /// The header size that was declared.
        header_size: u16,
    },
    /// Type/header_size pair makes no sense (e.g., zero-sized chunk).
    BadHeader {
        /// Byte offset of the offending chunk.
        offset: usize,
        /// The chunk type id.
        type_id: u16,
        /// The header size that was declared.
        header_size: u16,
    },
}

impl std::fmt::Display for ArscError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArscError::Truncated { offset, need, have } => write!(
                f, "truncated ARSC at offset {offset}: needed {need} bytes, had {have}"
            ),
            ArscError::NotArsc { type_seen } => write!(
                f, "expected ARSC table (type=0x0002), got 0x{type_seen:04x}"
            ),
            ArscError::ChunkOverflow { offset, declared, remaining } => write!(
                f, "chunk at offset {offset} declares {declared} bytes but only {remaining} remain"
            ),
            ArscError::HeaderTooSmall { offset, header_size } => write!(
                f, "chunk header at offset {offset} too small ({header_size} < 8)"
            ),
            ArscError::BadHeader { offset, type_id, header_size } => write!(
                f, "bad header at offset {offset}: type=0x{type_id:04x} header_size={header_size}"
            ),
        }
    }
}

impl std::error::Error for ArscError {}

/// Inner chunk inside the ARSC table wrapper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    /// ARSC chunk type ID.
    pub type_id: u16,
    /// Byte offset of this chunk within the outer table.
    pub offset: usize,
    /// Raw bytes of the entire chunk (header + payload).
    pub raw: Vec<u8>,
}

/// Parsed ARSC table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArscDoc {
    /// `package_count` from the table header (informational; we
    /// preserve the raw header bytes for round-trip).
    pub package_count: u32,
    /// Inner chunks, in original order. First is always the
    /// global string pool; remainder are per-package chunks
    /// (also chunked binary internally).
    pub chunks: Vec<Chunk>,
    /// Outer table-header size (12 bytes in known emitters: u16
    /// type + u16 header_size + u32 chunk_size + u32 package_count).
    pub outer_header_size: u16,
    /// Outer chunk size from the table header.
    pub declared_size: u32,
    /// Padding/trailer bytes after the last inner chunk and before
    /// the end of the declared size. Some aapt2 builds emit a few
    /// alignment bytes here; we preserve them verbatim so round-
    /// trip stays byte-identical.
    pub trailer: Vec<u8>,
}

/// Parse an ARSC byte buffer.
pub fn parse(bytes: &[u8]) -> Result<ArscDoc, ArscError> {
    if bytes.len() < 12 {
        return Err(ArscError::Truncated {
            offset: 0,
            need: 12,
            have: bytes.len(),
        });
    }
    let outer_type = u16::from_le_bytes([bytes[0], bytes[1]]);
    let outer_header_size = u16::from_le_bytes([bytes[2], bytes[3]]);
    let outer_chunk_size = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    let package_count = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
    if outer_type != chunk_type::RES_TABLE {
        return Err(ArscError::NotArsc { type_seen: outer_type });
    }
    if outer_header_size < 12 {
        return Err(ArscError::HeaderTooSmall {
            offset: 0,
            header_size: outer_header_size,
        });
    }
    let total = bytes.len();
    if (outer_chunk_size as usize) > total {
        return Err(ArscError::ChunkOverflow {
            offset: 0,
            declared: outer_chunk_size,
            remaining: total as u32,
        });
    }
    let end = outer_chunk_size as usize;

    let mut chunks: Vec<Chunk> = Vec::new();
    let mut i = outer_header_size as usize;
    while i + 8 <= end {
        let type_id = u16::from_le_bytes([bytes[i], bytes[i + 1]]);
        let header_size = u16::from_le_bytes([bytes[i + 2], bytes[i + 3]]);
        let chunk_size = u32::from_le_bytes([bytes[i + 4], bytes[i + 5], bytes[i + 6], bytes[i + 7]]);
        if header_size < 8 {
            return Err(ArscError::HeaderTooSmall {
                offset: i,
                header_size,
            });
        }
        let cs_us = chunk_size as usize;
        if cs_us == 0 {
            // A zero-sized chunk would loop forever — surface as
            // bad header so the caller knows the input is broken.
            return Err(ArscError::BadHeader {
                offset: i,
                type_id,
                header_size,
            });
        }
        if i + cs_us > end {
            return Err(ArscError::ChunkOverflow {
                offset: i,
                declared: chunk_size,
                remaining: u32::try_from(end - i).unwrap_or(u32::MAX),
            });
        }
        chunks.push(Chunk {
            type_id,
            offset: i,
            raw: bytes[i..i + cs_us].to_vec(),
        });
        i += cs_us;
    }

    // Anything between the last chunk and the end of declared_size
    // is padding/trailer — preserve verbatim.
    let trailer = if i < end {
        bytes[i..end].to_vec()
    } else {
        Vec::new()
    };

    Ok(ArscDoc {
        package_count,
        chunks,
        outer_header_size,
        declared_size: outer_chunk_size,
        trailer,
    })
}

/// Re-emit an [`ArscDoc`] back to bytes. Round-trips byte-
/// identically with [`parse`] for every chunk we recognise.
#[must_use]
pub fn emit(doc: &ArscDoc) -> Vec<u8> {
    let inner_total: usize = doc.chunks.iter().map(|c| c.raw.len()).sum();
    let outer_total = (doc.outer_header_size as usize) + inner_total + doc.trailer.len();
    let mut out = Vec::with_capacity(outer_total);
    out.extend_from_slice(&chunk_type::RES_TABLE.to_le_bytes());
    out.extend_from_slice(&doc.outer_header_size.to_le_bytes());
    let total_u32 = u32::try_from(outer_total).unwrap_or(u32::MAX);
    out.extend_from_slice(&total_u32.to_le_bytes());
    out.extend_from_slice(&doc.package_count.to_le_bytes());
    if (doc.outer_header_size as usize) > 12 {
        out.extend(std::iter::repeat(0u8).take((doc.outer_header_size as usize) - 12));
    }
    for c in &doc.chunks {
        out.extend_from_slice(&c.raw);
    }
    out.extend_from_slice(&doc.trailer);
    out
}

/// Convenience — parse + immediately re-emit.
pub fn round_trip(bytes: &[u8]) -> Result<Vec<u8>, ArscError> {
    let d = parse(bytes)?;
    Ok(emit(&d))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_arsc() -> Vec<u8> {
        // Outer table header: 12 bytes + one zero-sized stub chunk
        // (cooperative — real files always have payload).
        // We'll place a single empty string-pool chunk inside.
        let mut inner = Vec::new();
        inner.extend_from_slice(&chunk_type::RES_STRING_POOL.to_le_bytes());
        inner.extend_from_slice(&28u16.to_le_bytes());
        inner.extend_from_slice(&28u32.to_le_bytes());
        inner.extend_from_slice(&[0u8; 20]);

        let mut out = Vec::new();
        out.extend_from_slice(&chunk_type::RES_TABLE.to_le_bytes());
        out.extend_from_slice(&12u16.to_le_bytes()); // header_size
        let total = (12 + inner.len()) as u32;
        out.extend_from_slice(&total.to_le_bytes()); // chunk_size
        out.extend_from_slice(&1u32.to_le_bytes()); // package_count
        out.extend_from_slice(&inner);
        out
    }

    #[test]
    fn parse_synthetic() {
        let bytes = synthetic_arsc();
        let doc = parse(&bytes).expect("parse");
        assert_eq!(doc.chunks.len(), 1);
        assert_eq!(doc.chunks[0].type_id, chunk_type::RES_STRING_POOL);
        assert_eq!(doc.package_count, 1);
    }

    #[test]
    fn round_trip_synthetic_byte_identical() {
        let bytes = synthetic_arsc();
        let out = round_trip(&bytes).expect("round-trip");
        assert_eq!(bytes, out);
    }

    #[test]
    fn rejects_non_arsc() {
        let bad = vec![0u8; 16];
        assert!(matches!(parse(&bad), Err(ArscError::NotArsc { .. })));
    }
}
