// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `axiom-l1-rs` — APKAXIOM L1 untrusted-shell parser surface.
//!
//! P1.1 introduced this crate as a build-graph liveness probe. P1.7
//! lands the streaming-reader trait — `ApkParser::from_reader<R:
//! io::Read>` produces a `ParseEvent` stream as bytes arrive without
//! buffering the whole file. The wire-format-stable event taxonomy
//! is in `event.rs`; the producer state machine is in `stream.rs`;
//! both delegate all *parsing-soundness* questions to the verified
//! `axiom_zip_ref` crate (which the Lean ↔ Rust ↔ AOSP three-way
//! differential gates on).

#![forbid(unsafe_code)]
#![warn(missing_docs, unreachable_pub)]
// Stream module unfolds many u16→u64 / u64→usize casts that read more
// naturally as `as` casts than as `From` / `try_into` chains. The
// values are bounded by construction (LFH lengths fit u16, sample
// sizes ≤ a few MB).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::doc_markdown,
    clippy::too_long_first_doc_paragraph
)]

pub mod event;
pub mod stream;
pub mod stream_async;

pub use event::{ParseEvent, ResourceValue};
pub use stream::{ApkParser, StreamError, DEFAULT_CHUNK_SIZE, MAX_HEADER_PAYLOAD};
pub use stream_async::{ApkAsyncParser, AsyncByteSource};

/// Build-graph liveness probe.
///
/// Returns a value derived from L0's probe. If L0's constant changes, this
/// changes — by design, so a single L0 edit flushes downstream
/// reproducibility hashes.
#[must_use]
pub const fn placeholder() -> u32 {
    axiom_l0::placeholder() ^ 0x0000_00A1
}

/// Crate identifier baked into the binary.
pub const CRATE_ID: &str = "apkaxiom::l1-rs";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_xors_l0() {
        assert_eq!(placeholder(), 0xA710_0000 ^ 0x0000_00A1);
    }
}
