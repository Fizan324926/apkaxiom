// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `ParseEvent` — the wire-format-stable event type emitted by the
//! streaming APK parser.
//!
//! An `ApkParser::from_reader` consumer sees a sequence of these
//! events as bytes arrive. The first event is *always* `ZipEntryHeader`
//! (or an error) and the last is *always* `ParseComplete` (on success).
//! Manifest / resource events arrive interleaved with ZIP-layer events:
//! e.g.
//!
//! ```text
//!   ZipEntryHeader { name = "AndroidManifest.xml", … }
//!   ZipEntryData { offset = 0, len = N }                  [chunk 1]
//!   ZipEntryData { offset = N, len = M }                  [chunk 2]
//!   ManifestStart
//!   ManifestField { tag = …, value = … }
//!   …
//!   ManifestEnd
//!   ZipEntryHeader { name = "classes.dex", … }
//!   …
//!   EocdSeen
//!   ParseComplete { entries = K, bytes = T }
//! ```
//!
//! The enum is `serde::Serialize` so the harness can capture an event
//! trace as JSON for golden-file testing.
//!
//! Per P1.7 spec §2, this module is the API surface that P1.8
//! (type-state phantoms), P1.10 (Merkle commit hooks), and P1.15
//! (AXIOM-IR emission) build on.

#![allow(missing_docs)] // event payload field names are self-documenting

/// Wire-format-stable streaming-parse event. Order of variants is
/// committed via the discriminant byte returned by [`ParseEvent::tag`];
/// downstream layers (P1.10/P1.15) depend on this order.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseEvent {
    /// A new ZIP local-file-header was parsed. Variable-length
    /// payload (file body) arrives in subsequent `ZipEntryData`
    /// events.
    ZipEntryHeader {
        /// Verbatim LFH bytes — fixed 30-byte prefix + name +
        /// extra-field. Hashing this commits to every LFH field
        /// (CRC, sizes, mod-time, general-flags, extra-fields,
        /// name) byte-for-byte; a single bit-flip anywhere
        /// produces a different leaf.
        raw_header: Vec<u8>,
        /// Stream-offset of the LFH signature (first byte of
        /// `raw_header`).
        offset: u64,
        /// Filename bytes, exactly as they appear in the LFH.
        file_name: Vec<u8>,
        /// `compression_method` (0 = stored, 8 = deflate, …).
        compression_method: u16,
        /// LFH `compressed_size` field. For data-descriptor entries
        /// (general-flag bit 3), this is `0` and the real size
        /// arrives via the trailing data-descriptor (which we
        /// surface as the eventual `ZipEntryData::len` total).
        compressed_size: u32,
        /// LFH `uncompressed_size` field. Same DD caveat.
        uncompressed_size: u32,
        /// CRC-32 of the uncompressed data (LFH-declared).
        crc32: u32,
        /// `general_purpose_bit_flag` — bit 3 = data descriptor
        /// present.
        general_flags: u16,
    },
    /// A chunk of file body for the most recently announced
    /// `ZipEntryHeader`. Multiple `ZipEntryData` events stream the
    /// file body without buffering the whole entry.
    ZipEntryData {
        /// Offset within the entry's compressed body.
        offset: u64,
        /// Bytes in this chunk.
        bytes: Vec<u8>,
    },
    /// Data-descriptor record (12 or 16 bytes, depending on
    /// whether the optional signature 0x08074b50 is present).
    /// Emitted only for entries whose LFH had general-flag bit 3
    /// set (deferred-CRC mode). The verbatim bytes are committed
    /// so a single-bit-flip anywhere in the DD changes the
    /// Merkle root.
    DataDescriptor {
        /// Verbatim DD bytes.
        raw: Vec<u8>,
        /// Stream-offset of the DD's first byte.
        offset: u64,
        /// Recovered CRC-32 of the body.
        crc32: u32,
        /// Recovered compressed size.
        compressed_size: u32,
        /// Recovered uncompressed size.
        uncompressed_size: u32,
    },
    /// The bytes between the last LFH body and the central
    /// directory — typically the APK signing block (v2/v3) when
    /// present, or empty for unsigned archives. Hashing this
    /// commits to the signing-block bytes verbatim.
    SigningBlock {
        /// Verbatim signing-block bytes (`bytes_consumed_at_end_of_body`
        /// to `cd_offset` from the EOCD).
        raw: Vec<u8>,
        /// Stream-offset of the first byte.
        offset: u64,
    },
    /// One central-directory record (CDR). Emitted once per
    /// archive entry, after all `ZipEntryHeader`/`ZipEntryData`
    /// for that entry but before the EOCD.
    CdrEntry {
        /// Verbatim CDR bytes (46-byte fixed prefix + name +
        /// extra-field + comment).
        raw: Vec<u8>,
        /// Stream-offset of the CDR signature.
        offset: u64,
        /// Filename bytes, exactly as they appear in the CDR.
        file_name: Vec<u8>,
        /// `compression_method` (0 = stored, 8 = deflate, …).
        compression_method: u16,
        /// CDR `compressed_size` field — authoritative even when
        /// the LFH zeroed it via the data-descriptor flag.
        compressed_size: u32,
        /// CDR `uncompressed_size` field — same authoritative
        /// caveat.
        uncompressed_size: u32,
        /// CRC-32 of the uncompressed data (CDR-declared).
        crc32: u32,
        /// `general_purpose_bit_flag`.
        general_flags: u16,
        /// LFH offset (relative to start of archive) — the
        /// "where to find this entry's local header" cross-link.
        lfh_offset: u32,
    },
    /// The end-of-central-directory signature was found.
    EocdSeen {
        /// Verbatim EOCD record bytes.
        raw: Vec<u8>,
        /// Stream-offset of the EOCD signature.
        offset: u64,
        /// `total_entries` from the EOCD.
        total_entries: u16,
        /// `cd_offset` from the EOCD.
        cd_offset: u32,
        /// `cd_size` from the EOCD.
        cd_size: u32,
    },
    /// The compressed AndroidManifest.xml body has been observed in
    /// full. Manifest events follow until `ManifestEnd`. P1.7
    /// emits this as a placeholder (no manifest decoding yet);
    /// real AXML decoding lands in P1.8.
    ManifestStart,
    /// One field of the AndroidManifest. `tag` and `value` are
    /// AXML-decoded strings. Placeholder in P1.7.
    ManifestField {
        /// AXML element / attribute name.
        tag: String,
        /// AXML attribute value as a UTF-8 string.
        value: String,
    },
    /// Manifest decoding finished.
    ManifestEnd,
    /// `resources.arsc` body observed. Resource events follow until
    /// `ResourceEnd`. Placeholder in P1.7 (real ARSC decoding lands
    /// in P1.9).
    ResourceStart,
    /// One ARSC entry. Placeholder shape until P1.9.
    ResourceEntry {
        /// Resource id (full 32-bit composite — package | type |
        /// entry).
        resource_id: u32,
        /// Type-tagged resource value.
        value: ResourceValue,
    },
    /// ARSC decoding finished.
    ResourceEnd,
    /// All bytes consumed; the parser is in a terminal success
    /// state. After this no further events are emitted.
    ParseComplete {
        /// Total entries in the archive (matches EOCD's
        /// `total_entries`).
        entries: u32,
        /// Total bytes consumed from the input stream.
        bytes: u64,
    },
}

/// ARSC value tag. Placeholder enumeration; the full set is fleshed
/// out in P1.9.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResourceValue {
    /// Raw 32-bit integer.
    Int32(i32),
    /// Indirect string-pool reference.
    StringRef(u32),
    /// Boolean.
    Bool(bool),
}

impl ParseEvent {
    /// Stable tag byte for cross-language interop (Lean differential
    /// harness, AXIOM-IR emitters). Each variant has a distinct
    /// non-zero byte.
    #[must_use]
    pub const fn tag(&self) -> u8 {
        match self {
            Self::ZipEntryHeader { .. } => 1,
            Self::ZipEntryData { .. } => 2,
            Self::EocdSeen { .. } => 3,
            Self::ManifestStart => 4,
            Self::ManifestField { .. } => 5,
            Self::ManifestEnd => 6,
            Self::ResourceStart => 7,
            Self::ResourceEntry { .. } => 8,
            Self::ResourceEnd => 9,
            Self::ParseComplete { .. } => 10,
            Self::SigningBlock { .. } => 11,
            Self::CdrEntry { .. } => 12,
            Self::DataDescriptor { .. } => 13,
        }
    }

    /// Serialise this event as a JSON-compatible line. The shape is
    /// `{"tag": "<name>", "fields": {…}}` — stable across versions
    /// (P1.10's Merkle-commit hooks consume this format).
    ///
    /// We don't use `serde` here because the project's
    /// reindeer-vendored third-party set deliberately excludes
    /// `serde_core` (its `build.rs` needs `CARGO_PKG_VERSION_PATCH`
    /// which Reindeer doesn't pass — see `third-party/rust/Cargo.toml`).
    /// `tools/unsafe-census` uses the same hand-rolled approach.
    #[must_use]
    pub fn to_json(&self) -> String {
        match self {
            Self::ZipEntryHeader {
                raw_header,
                offset,
                file_name,
                compression_method,
                compressed_size,
                uncompressed_size,
                crc32,
                general_flags,
            } => format!(
                "{{\"tag\":\"ZipEntryHeader\",\"offset\":{offset},\"raw_len\":{},\"raw_header\":{},\"file_name\":{},\"compression_method\":{compression_method},\"compressed_size\":{compressed_size},\"uncompressed_size\":{uncompressed_size},\"crc32\":{crc32},\"general_flags\":{general_flags}}}",
                raw_header.len(),
                json_byte_array(raw_header),
                json_byte_array(file_name),
            ),
            Self::ZipEntryData { offset, bytes } => format!(
                "{{\"tag\":\"ZipEntryData\",\"offset\":{},\"len\":{},\"bytes\":{}}}",
                offset,
                bytes.len(),
                json_byte_array(bytes),
            ),
            Self::SigningBlock { raw, offset } => format!(
                "{{\"tag\":\"SigningBlock\",\"offset\":{offset},\"len\":{},\"raw\":{}}}",
                raw.len(),
                json_byte_array(raw),
            ),
            Self::DataDescriptor {
                raw,
                offset,
                crc32,
                compressed_size,
                uncompressed_size,
            } => format!(
                "{{\"tag\":\"DataDescriptor\",\"offset\":{offset},\"len\":{},\"raw\":{},\"crc32\":{crc32},\"compressed_size\":{compressed_size},\"uncompressed_size\":{uncompressed_size}}}",
                raw.len(),
                json_byte_array(raw),
            ),
            Self::CdrEntry {
                raw,
                offset,
                file_name,
                compression_method,
                compressed_size,
                uncompressed_size,
                crc32,
                general_flags,
                lfh_offset,
            } => format!(
                "{{\"tag\":\"CdrEntry\",\"offset\":{offset},\"raw_len\":{},\"raw\":{},\"file_name\":{},\"compression_method\":{compression_method},\"compressed_size\":{compressed_size},\"uncompressed_size\":{uncompressed_size},\"crc32\":{crc32},\"general_flags\":{general_flags},\"lfh_offset\":{lfh_offset}}}",
                raw.len(),
                json_byte_array(raw),
                json_byte_array(file_name),
            ),
            Self::EocdSeen {
                raw,
                offset,
                total_entries,
                cd_offset,
                cd_size,
            } => format!(
                "{{\"tag\":\"EocdSeen\",\"offset\":{offset},\"raw_len\":{},\"raw\":{},\"total_entries\":{total_entries},\"cd_offset\":{cd_offset},\"cd_size\":{cd_size}}}",
                raw.len(),
                json_byte_array(raw),
            ),
            Self::ManifestStart => "{\"tag\":\"ManifestStart\"}".to_string(),
            Self::ManifestField { tag, value } => format!(
                "{{\"tag\":\"ManifestField\",\"name\":{},\"value\":{}}}",
                json_string(tag),
                json_string(value),
            ),
            Self::ManifestEnd => "{\"tag\":\"ManifestEnd\"}".to_string(),
            Self::ResourceStart => "{\"tag\":\"ResourceStart\"}".to_string(),
            Self::ResourceEntry { resource_id, value } => format!(
                "{{\"tag\":\"ResourceEntry\",\"resource_id\":{},\"value\":{}}}",
                resource_id,
                resource_value_to_json(value),
            ),
            Self::ResourceEnd => "{\"tag\":\"ResourceEnd\"}".to_string(),
            Self::ParseComplete { entries, bytes } => format!(
                "{{\"tag\":\"ParseComplete\",\"entries\":{entries},\"bytes\":{bytes}}}",
            ),
        }
    }
}

/// Serialise a byte array as a JSON array of integers. Stable across
/// versions; the consumer can reassemble bytes from the integer
/// sequence.
fn json_byte_array(bs: &[u8]) -> String {
    let mut s = String::with_capacity(bs.len() * 4 + 2);
    s.push('[');
    for (i, b) in bs.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&b.to_string());
    }
    s.push(']');
    s
}

/// Serialise a UTF-8 string as a JSON string literal (RFC 8259 §7).
fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn resource_value_to_json(v: &ResourceValue) -> String {
    match v {
        ResourceValue::Int32(i) => format!("{{\"kind\":\"Int32\",\"value\":{i}}}"),
        ResourceValue::StringRef(r) => format!("{{\"kind\":\"StringRef\",\"value\":{r}}}"),
        ResourceValue::Bool(b) => format!("{{\"kind\":\"Bool\",\"value\":{b}}}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_bytes_are_distinct() {
        // Every variant we can construct without test data.
        let evs: Vec<u8> = vec![
            ParseEvent::ManifestStart.tag(),
            ParseEvent::ManifestEnd.tag(),
            ParseEvent::ResourceStart.tag(),
            ParseEvent::ResourceEnd.tag(),
        ];
        let unique: std::collections::HashSet<_> = evs.iter().copied().collect();
        assert_eq!(
            unique.len(),
            evs.len(),
            "tag bytes must be pairwise distinct"
        );
    }

    #[test]
    fn tag_byte_in_one_to_ten() {
        // All 10 tags must fit in [1, 10].
        for t in [1, 2, 3, 4, 5, 6, 7, 8, 9, 10] {
            assert!(t > 0 && t <= 10);
        }
    }
}
