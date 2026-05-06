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
//! **Round-trip lemma**: `bytes_to_axml(axml_to_bytes(d)) == d` for
//! every well-formed AXML input. The emitter preserves chunk
//! order, header sizes, and string-pool encoding so the reverse
//! path is bit-identical. Same for ARSC. The HARD gate at
//! P1.15 §10 row 3 is ≥ 95 % of Bench-1K APKs round-tripping
//! byte-for-byte; documented exceptions are inputs whose
//! original bytes carried structural anomalies (over-aligned
//! chunks, trailing padding) that the parser normalises away.

#![allow(clippy::cast_possible_truncation)]

pub mod arsc;
pub mod axml;
pub mod emit;
