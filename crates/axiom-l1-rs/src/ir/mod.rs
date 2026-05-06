// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! P1.15 — AXIOM-IR-v0.1 emitter.
//!
//! Bridges the streaming-parser surface (`Apk<S>` carrying raw
//! `Manifest::axml_bytes` and `Resources::arsc_bytes` in the
//! `FullyParsed` state) to the `axiom-ir` manifest and resource
//! dialects. The emitter parses the on-disk AOSP binary forms
//! (chunked AXML for the manifest, chunked ARSC for resources)
//! into structural representations [`axml::AxmlDoc`] and
//! [`arsc::ArscDoc`], then maps those into the dialect IRs.
//!
//! Two layers of IR:
//!
//! 1. **Structural** (`emit.rs`): the raw chunk trees — preserves every
//!    byte, enables byte-identical reencode. This is the round-trip gate.
//!
//! 2. **Semantic** (`manifest_decode.rs`, `resource_decode.rs`): parsed
//!    element/attribute walks that populate [`axiom_ir::manifest::ManifestModule`]
//!    and [`axiom_ir::resource::ResourceTable`]. These are the dialect
//!    types consumed by `axiom-ir`'s canonical bytes, text, and JSON
//!    wire formats.
//!
//! **Round-trip lemma**: `bytes_to_axml(axml_to_bytes(d)) == d` for
//! every well-formed AXML input. Same for ARSC. HARD gate: ≥ 95 % of
//! Bench-1K APKs round-trip byte-for-byte.

#![allow(clippy::cast_possible_truncation)]

pub mod arsc;
pub mod axml;
pub mod emit;
pub mod manifest_decode;
pub mod resource_decode;
pub(crate) mod strings;
