// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `ApkAsyncParser<S: AsyncByteSource>` — runtime-agnostic async
//! mirror of [`crate::stream::ApkParser`].
//!
//! ## Why a second module
//!
//! The sync [`crate::stream::ApkParser`] is the canonical reference;
//! its three-way differential gates (Lean ↔ Rust ↔ AOSP, 2860/2860 in
//! P1.6) are the wire-format soundness baseline. The async variant
//! re-uses the same parsing rules (delegated to `axiom_zip_ref`) but
//! drives I/O via an `async fn read_chunk` so it can be plugged into
//! io_uring runtimes (Glommio, tokio-uring, monoio).
//!
//! ## Runtime independence
//!
//! No tokio / no futures crate. The trait surface is one
//! `async fn read_chunk(&mut self, n: usize) -> io::Result<Vec<u8>>`
//! method, which native async-fn-in-trait (stable since Rust 1.75)
//! makes runtime-independent. Adapter crates (`zip-stream-soak-async`)
//! supply the runtime-specific `AsyncByteSource` impl.
//!
//! ## State machine
//!
//! Identical structure to [`crate::stream`] — same `ParserState`,
//! same buffer geometry, same `MAX_HEADER_PAYLOAD` /
//! `MAX_DD_BODY` bounds, same backpressure budget. The only
//! difference is `read_more` is `.await`-driven.

// `clippy::future_not_send` is the deliberate cost of this module's
// runtime-agnosticism — see the doc-comment on `AsyncByteSource`.
// Adding `+ Send` bounds would lock out Glommio (single-thread
// io_uring), and that lock-out is precisely what we wanted to avoid.
#![allow(clippy::future_not_send)]

use std::collections::VecDeque;
use std::io;

use axiom_zip_ref::{eocd, lfh};

use crate::event::ParseEvent;
use crate::stream::{StreamError, DEFAULT_CHUNK_SIZE, MAX_DD_BODY, MAX_HEADER_PAYLOAD};

/// Async byte source. Implementors return up to `n` bytes per call;
/// an empty `Vec` signals EOF.
///
/// The owned-`Vec` return shape (rather than caller-supplied buffer)
/// matches Glommio's `DmaFile::read_at` / `read_many` and tokio-uring's
/// `read_at` ergonomics — both return owned buffers due to io_uring's
/// completion-time ownership model. Adapter crates can copy into a
/// re-usable scratch buffer if zero-alloc matters.
///
/// The returned future intentionally has *no* `Send` bound: Glommio
/// is thread-per-core (futures are pinned to one OS thread) and
/// adding `Send` would block the most natural io_uring integration.
/// Multi-thread runtimes (tokio, smol) should still work since
/// `async fn` in trait inherits auto-traits from the impl, so a
/// `Send`-able adapter produces a `Send`-able parser future.
pub trait AsyncByteSource {
    /// Read up to `n` bytes. Empty `Vec` means EOF.
    ///
    /// # Errors
    /// Implementation-defined; surfaced as [`StreamError::Io`].
    // We accept the `async_fn_in_trait` warning deliberately: forcing
    // `+ Send` on the desugared `impl Future` would lock out Glommio
    // (whose futures are intentionally non-`Send`). Auto-trait
    // inheritance via `async fn in trait` is exactly what we want.
    #[allow(async_fn_in_trait)]
    async fn read_chunk(&mut self, n: usize) -> io::Result<Vec<u8>>;
}

/// Internal state machine — duplicated structurally from
/// [`crate::stream`] so this module compiles standalone (no `pub(crate)`
/// re-export of the sync state needed). Single source of truth for
/// the shape; future refactors can extract a `step()` core if the
/// duplication grows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParserState {
    NextEntry,
    EntryBody { remaining: u64, emitted: u64 },
    DdEntryBody { emitted: u64 },
    Done,
}

/// Data-descriptor signature (APPNOTE.TXT §4.3.9.3).
const DD_SIGNATURE: u32 = 0x0807_4b50;

const fn buf_capacity(chunk_size: usize) -> usize {
    (MAX_HEADER_PAYLOAD as usize) + lfh::FIXED_SIZE + chunk_size
}

/// Async streaming APK parser.
#[derive(Debug)]
pub struct ApkAsyncParser<S: AsyncByteSource> {
    source: S,
    buf: Vec<u8>,
    read_pos: usize,
    write_pos: usize,
    bytes_consumed: u64,
    entries_seen: u32,
    chunk_size: usize,
    pending: VecDeque<ParseEvent>,
    state: ParserState,
}

impl<S: AsyncByteSource> ApkAsyncParser<S> {
    /// Maximum pending events queued before the producer blocks.
    pub const EVENT_BUDGET: usize = 16;

    /// Construct a parser around any `AsyncByteSource`.
    pub fn new(source: S) -> Self {
        let chunk_size = DEFAULT_CHUNK_SIZE;
        let cap = buf_capacity(chunk_size);
        Self {
            source,
            buf: vec![0u8; cap],
            read_pos: 0,
            write_pos: 0,
            bytes_consumed: 0,
            entries_seen: 0,
            chunk_size,
            pending: VecDeque::with_capacity(Self::EVENT_BUDGET),
            state: ParserState::NextEntry,
        }
    }

    /// Override the per-iteration read size.
    #[must_use]
    pub fn with_chunk_size(mut self, chunk_size: usize) -> Self {
        self.chunk_size = chunk_size;
        let new_cap = buf_capacity(chunk_size);
        if new_cap > self.buf.len() {
            self.buf.resize(new_cap, 0);
        }
        self
    }

    /// Diagnostics: current buffer capacity (bytes).
    #[must_use]
    pub fn buf_capacity(&self) -> usize {
        self.buf.len()
    }

    /// Diagnostics: total bytes consumed from the source.
    #[must_use]
    pub const fn bytes_consumed(&self) -> u64 {
        self.bytes_consumed
    }

    /// Pull the next event; `Ok(None)` once the parser reaches
    /// `ParseComplete`.
    ///
    /// # Errors
    /// Any [`StreamError`] variant.
    pub async fn next_event(&mut self) -> Result<Option<ParseEvent>, StreamError> {
        if let Some(ev) = self.pending.pop_front() {
            return Ok(Some(ev));
        }
        match self.state {
            ParserState::Done => Ok(None),
            ParserState::NextEntry => self.advance_at_entry_start().await,
            ParserState::EntryBody { remaining, emitted } => {
                self.advance_in_entry_body(remaining, emitted).await
            }
            ParserState::DdEntryBody { emitted } => self.advance_in_dd_entry_body(emitted).await,
        }
    }

    #[inline]
    fn unread(&self) -> &[u8] {
        &self.buf[self.read_pos..self.write_pos]
    }

    #[inline]
    fn available_write(&self) -> usize {
        self.buf.len() - self.write_pos
    }

    fn consume(&mut self, n: usize) {
        self.read_pos += n;
        self.bytes_consumed += n as u64;
    }

    fn compact_if_needed(&mut self) {
        if self.read_pos > self.buf.len() / 2 {
            let unread_len = self.write_pos - self.read_pos;
            if unread_len > 0 {
                self.buf.copy_within(self.read_pos..self.write_pos, 0);
            }
            self.write_pos = unread_len;
            self.read_pos = 0;
        }
    }

    async fn read_more(&mut self) -> Result<usize, StreamError> {
        self.compact_if_needed();
        let want = std::cmp::min(self.chunk_size, self.available_write());
        if want == 0 {
            return Ok(0);
        }
        let chunk = self.source.read_chunk(want).await?;
        let n = chunk.len();
        if n > want {
            // Defensive: AsyncByteSource impls must not over-deliver.
            return Err(StreamError::Io(io::Error::new(
                io::ErrorKind::InvalidData,
                "AsyncByteSource returned more bytes than requested",
            )));
        }
        self.buf[self.write_pos..self.write_pos + n].copy_from_slice(&chunk);
        self.write_pos += n;
        Ok(n)
    }

    async fn advance_at_entry_start(&mut self) -> Result<Option<ParseEvent>, StreamError> {
        while self.unread().len() < lfh::FIXED_SIZE {
            let n = self.read_more().await?;
            if n == 0 {
                if self.unread().len() >= eocd::FIXED_SIZE
                    && eocd::find_eocd(self.unread()).is_some()
                {
                    return self.emit_eocd_and_complete();
                }
                return Err(StreamError::Truncated {
                    at: self.bytes_consumed,
                    expected: lfh::FIXED_SIZE as u64 - self.unread().len() as u64,
                });
            }
        }

        let sig = u32::from_le_bytes(self.unread()[0..4].try_into().unwrap());
        if sig != lfh::SIGNATURE {
            return self.advance_post_entries().await;
        }

        let general_flags = u16::from_le_bytes(self.unread()[6..8].try_into().unwrap());
        let name_len = u16::from_le_bytes(self.unread()[26..28].try_into().unwrap()) as u64;
        let extra_len = u16::from_le_bytes(self.unread()[28..30].try_into().unwrap()) as u64;
        if name_len + extra_len > MAX_HEADER_PAYLOAD {
            return Err(StreamError::OversizedHeader {
                actual: name_len + extra_len,
                limit: MAX_HEADER_PAYLOAD,
            });
        }
        let header_total = lfh::FIXED_SIZE as u64 + name_len + extra_len;

        while (self.unread().len() as u64) < header_total {
            let n = self.read_more().await?;
            if n == 0 {
                return Err(StreamError::Truncated {
                    at: self.bytes_consumed + self.unread().len() as u64,
                    expected: header_total - self.unread().len() as u64,
                });
            }
        }

        let header_slice = &self.unread()[0..header_total as usize];
        let (lfh_record, consumed) = lfh::parse_lfh(header_slice).map_err(StreamError::Lfh)?;
        debug_assert_eq!(consumed as u64, header_total);

        let raw_header = header_slice.to_vec();
        let header_offset = self.bytes_consumed;
        let header_event = ParseEvent::ZipEntryHeader {
            raw_header,
            offset: header_offset,
            file_name: lfh_record.file_name,
            compression_method: lfh_record.compression_method,
            compressed_size: lfh_record.compressed_size,
            uncompressed_size: lfh_record.uncompressed_size,
            crc32: lfh_record.crc32,
            general_flags: lfh_record.general_flags,
        };
        self.consume(consumed);
        self.entries_seen += 1;

        if (general_flags & 0x0008) != 0 {
            self.state = ParserState::DdEntryBody { emitted: 0 };
        } else {
            self.state = ParserState::EntryBody {
                remaining: u64::from(lfh_record.compressed_size),
                emitted: 0,
            };
        }
        Ok(Some(header_event))
    }

    async fn advance_in_entry_body(
        &mut self,
        remaining: u64,
        emitted: u64,
    ) -> Result<Option<ParseEvent>, StreamError> {
        if remaining == 0 {
            self.state = ParserState::NextEntry;
            return Box::pin(self.advance_at_entry_start()).await;
        }
        if self.unread().is_empty() {
            let n = self.read_more().await?;
            if n == 0 {
                return Err(StreamError::Truncated {
                    at: self.bytes_consumed,
                    expected: remaining,
                });
            }
        }
        let take = std::cmp::min(remaining, self.unread().len() as u64) as usize;
        let chunk = self.unread()[..take].to_vec();
        self.consume(take);
        let new_remaining = remaining - take as u64;
        let new_emitted = emitted + take as u64;
        self.state = ParserState::EntryBody {
            remaining: new_remaining,
            emitted: new_emitted,
        };
        Ok(Some(ParseEvent::ZipEntryData {
            offset: emitted,
            bytes: chunk,
        }))
    }

    async fn advance_in_dd_entry_body(
        &mut self,
        emitted: u64,
    ) -> Result<Option<ParseEvent>, StreamError> {
        while self.unread().len() < 4 {
            let n = self.read_more().await?;
            if n == 0 {
                return Err(StreamError::Truncated {
                    at: self.bytes_consumed,
                    expected: 4 - self.unread().len() as u64,
                });
            }
        }
        if emitted > MAX_DD_BODY {
            return Err(StreamError::OversizedDdBody { scanned: emitted });
        }
        let unread = self.unread();
        let unread_len = unread.len();
        let mut found_at: Option<usize> = None;
        for i in 0..unread_len.saturating_sub(3) {
            let probe = u32::from_le_bytes(unread[i..i + 4].try_into().unwrap());
            if probe == DD_SIGNATURE {
                found_at = Some(i);
                break;
            }
        }
        if let Some(i) = found_at {
            if unread_len - i < 16 {
                let n = self.read_more().await?;
                if n == 0 {
                    return Err(StreamError::Truncated {
                        at: self.bytes_consumed + unread_len as u64,
                        expected: (16 - (unread_len - i)) as u64,
                    });
                }
                return Box::pin(self.advance_in_dd_entry_body(emitted)).await;
            }
            let unread = self.unread();
            if i > 0 {
                let chunk = unread[..i].to_vec();
                self.consume(i);
                let new_emitted = emitted + i as u64;
                self.state = ParserState::DdEntryBody {
                    emitted: new_emitted,
                };
                return Ok(Some(ParseEvent::ZipEntryData {
                    offset: emitted,
                    bytes: chunk,
                }));
            }
            // i == 0: DD starts at the cursor. Capture verbatim.
            let dd_offset = self.bytes_consumed;
            let dd_raw = unread[..16].to_vec();
            let crc32 = u32::from_le_bytes(unread[4..8].try_into().unwrap());
            let compressed_size = u32::from_le_bytes(unread[8..12].try_into().unwrap());
            let uncompressed_size = u32::from_le_bytes(unread[12..16].try_into().unwrap());
            self.consume(16);
            self.pending.push_back(ParseEvent::DataDescriptor {
                raw: dd_raw,
                offset: dd_offset,
                crc32,
                compressed_size,
                uncompressed_size,
            });
            self.state = ParserState::NextEntry;
            return Ok(self.pending.pop_front());
        }
        if unread.len() <= 3 {
            let n = self.read_more().await?;
            if n == 0 {
                return Err(StreamError::Truncated {
                    at: self.bytes_consumed,
                    expected: 16,
                });
            }
            return Box::pin(self.advance_in_dd_entry_body(emitted)).await;
        }
        let take = unread.len() - 3;
        let chunk = unread[..take].to_vec();
        self.consume(take);
        let new_emitted = emitted + take as u64;
        self.state = ParserState::DdEntryBody {
            emitted: new_emitted,
        };
        Ok(Some(ParseEvent::ZipEntryData {
            offset: emitted,
            bytes: chunk,
        }))
    }

    async fn advance_post_entries(&mut self) -> Result<Option<ParseEvent>, StreamError> {
        loop {
            let n = self.read_more().await?;
            if n == 0 {
                break;
            }
        }
        self.emit_eocd_and_complete()
    }

    /// Mirror of the sync parser's CD walk. Emits SigningBlock (if
    /// non-empty) → CdrEntry × N → EocdSeen → ParseComplete.
    #[allow(clippy::too_many_lines)]
    fn emit_eocd_and_complete(&mut self) -> Result<Option<ParseEvent>, StreamError> {
        let eocd_off_in_buf = eocd::find_eocd(self.unread())
            .ok_or(StreamError::Eocd(eocd::ParseError::BadSignature))?;
        let (eocd_record, eocd_consumed) =
            eocd::parse_eocd(&self.unread()[eocd_off_in_buf..]).map_err(StreamError::Eocd)?;
        let buf_start_in_stream = self.bytes_consumed;
        let cd_start_in_stream = u64::from(eocd_record.cd_offset);
        let cd_size = eocd_record.cd_size as usize;
        if cd_start_in_stream < buf_start_in_stream {
            return Err(StreamError::Eocd(eocd::ParseError::BadSignature));
        }
        let cd_off_in_buf = (cd_start_in_stream - buf_start_in_stream) as usize;
        let unread_len = self.unread().len();
        if cd_off_in_buf
            .checked_add(cd_size)
            .is_none_or(|end| end > unread_len)
        {
            return Err(StreamError::Truncated {
                at: buf_start_in_stream + cd_off_in_buf as u64,
                expected: cd_size as u64,
            });
        }
        if cd_off_in_buf > 0 {
            let sig_bytes = self.unread()[..cd_off_in_buf].to_vec();
            self.pending.push_back(ParseEvent::SigningBlock {
                raw: sig_bytes,
                offset: buf_start_in_stream,
            });
        }
        let cd_bytes_owned = self.unread()[cd_off_in_buf..cd_off_in_buf + cd_size].to_vec();
        let mut cdr_off_in_cd = 0usize;
        while cdr_off_in_cd < cd_bytes_owned.len() {
            let (cdr_record, cdr_consumed) =
                axiom_zip_ref::cdr::parse_cdr(&cd_bytes_owned[cdr_off_in_cd..])
                    .map_err(StreamError::Cdr)?;
            let raw = cd_bytes_owned[cdr_off_in_cd..cdr_off_in_cd + cdr_consumed].to_vec();
            let cdr_offset_in_stream = cd_start_in_stream + cdr_off_in_cd as u64;
            self.pending.push_back(ParseEvent::CdrEntry {
                raw,
                offset: cdr_offset_in_stream,
                file_name: cdr_record.file_name,
                compression_method: cdr_record.compression_method,
                compressed_size: cdr_record.compressed_size,
                uncompressed_size: cdr_record.uncompressed_size,
                crc32: cdr_record.crc32,
                general_flags: cdr_record.general_flags,
                lfh_offset: cdr_record.lfh_offset,
            });
            cdr_off_in_cd += cdr_consumed;
        }
        let eocd_raw = self.unread()[eocd_off_in_buf..eocd_off_in_buf + eocd_consumed].to_vec();
        let eocd_offset_in_stream = buf_start_in_stream + eocd_off_in_buf as u64;
        let eocd_event = ParseEvent::EocdSeen {
            raw: eocd_raw,
            offset: eocd_offset_in_stream,
            total_entries: eocd_record.total_entries,
            cd_offset: eocd_record.cd_offset,
            cd_size: eocd_record.cd_size,
        };
        self.consume(eocd_off_in_buf + eocd_consumed);
        self.pending.push_back(eocd_event);
        self.pending.push_back(ParseEvent::ParseComplete {
            entries: self.entries_seen,
            bytes: self.bytes_consumed,
        });
        self.state = ParserState::Done;
        Ok(self.pending.pop_front())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axiom_zip_ref::{cdr, lfh};

    /// Minimal cursor-based AsyncByteSource. Drives the parser without
    /// any runtime — the futures are polled inline by a hand-rolled
    /// noop-waker executor in `block_on`.
    struct Cursor {
        bytes: Vec<u8>,
        pos: usize,
    }
    impl Cursor {
        const fn new(bytes: Vec<u8>) -> Self {
            Self { bytes, pos: 0 }
        }
    }
    impl AsyncByteSource for Cursor {
        async fn read_chunk(&mut self, n: usize) -> io::Result<Vec<u8>> {
            let take = n.min(self.bytes.len() - self.pos);
            let chunk = self.bytes[self.pos..self.pos + take].to_vec();
            self.pos += take;
            Ok(chunk)
        }
    }

    /// Hand-rolled `block_on` for tests — no tokio dep needed because
    /// our futures are non-suspending (the Cursor source completes
    /// synchronously). A `Pending` poll would panic. Uses the safe
    /// `Arc<dyn Wake>` waker constructor + `Box::pin` to satisfy
    /// `#![forbid(unsafe_code)]`.
    fn block_on<F: std::future::Future>(fut: F) -> F::Output {
        use std::sync::Arc;
        use std::task::{Context, Poll, Wake};
        struct Noop;
        impl Wake for Noop {
            fn wake(self: Arc<Self>) {}
            fn wake_by_ref(self: &Arc<Self>) {}
        }
        let waker = Arc::new(Noop).into();
        let mut ctx = Context::from_waker(&waker);
        let mut fut = Box::pin(fut);
        match fut.as_mut().poll(&mut ctx) {
            Poll::Ready(out) => out,
            Poll::Pending => panic!("test source must not yield Pending"),
        }
    }

    fn minimal_archive() -> Vec<u8> {
        let mut v = Vec::with_capacity(98);
        v.extend_from_slice(&lfh::SIGNATURE.to_le_bytes());
        v.extend_from_slice(&[0x14, 0x00]);
        v.extend_from_slice(&[0u8; 20]);
        v.extend_from_slice(&[0x00, 0x00]);
        v.extend_from_slice(&[0x00, 0x00]);
        v.extend_from_slice(&cdr::SIGNATURE.to_le_bytes());
        v.extend_from_slice(&[0x14, 0x00, 0x14, 0x00]);
        v.extend_from_slice(&[0u8; 8]);
        v.extend_from_slice(&[0u8; 4]);
        v.extend_from_slice(&[0u8; 4]);
        v.extend_from_slice(&[0u8; 4]);
        v.extend_from_slice(&[0u8; 2]);
        v.extend_from_slice(&[0u8; 2]);
        v.extend_from_slice(&[0u8; 2]);
        v.extend_from_slice(&[0u8; 2]);
        v.extend_from_slice(&[0u8; 2]);
        v.extend_from_slice(&[0u8; 4]);
        v.extend_from_slice(&[0u8; 4]);
        v.extend_from_slice(&eocd::SIGNATURE.to_le_bytes());
        v.extend_from_slice(&[0u8; 4]);
        v.extend_from_slice(&[0x01, 0x00]);
        v.extend_from_slice(&[0x01, 0x00]);
        v.extend_from_slice(&46u32.to_le_bytes());
        v.extend_from_slice(&30u32.to_le_bytes());
        v.extend_from_slice(&[0u8; 2]);
        v
    }

    #[test]
    fn async_streams_minimal_archive() {
        let bytes = minimal_archive();
        let mut parser = ApkAsyncParser::new(Cursor::new(bytes)).with_chunk_size(16);
        let mut events = Vec::new();
        loop {
            let ev = block_on(parser.next_event()).unwrap();
            match ev {
                Some(e) => events.push(e),
                None => break,
            }
        }
        // P1.10: Header + CdrEntry + Eocd + ParseComplete.
        assert_eq!(events.len(), 4);
        assert!(matches!(events[0], ParseEvent::ZipEntryHeader { .. }));
        assert!(matches!(events[1], ParseEvent::CdrEntry { .. }));
        assert!(matches!(events[2], ParseEvent::EocdSeen { .. }));
        assert!(matches!(events[3], ParseEvent::ParseComplete { .. }));
    }

    #[test]
    fn async_truncated_input_errors_cleanly() {
        let bytes = minimal_archive();
        let truncated = bytes[0..20].to_vec();
        let mut parser = ApkAsyncParser::new(Cursor::new(truncated));
        let result = block_on(parser.next_event());
        assert!(matches!(result, Err(StreamError::Truncated { .. })));
    }

    #[test]
    fn async_chunked_reads_match_sync_semantics() {
        // Cross-check: same archive should produce the same number of
        // events and identical event tags vs the sync parser.
        use crate::stream::ApkParser;
        let bytes = minimal_archive();
        let mut sync_parser = ApkParser::from_reader(bytes.as_slice()).with_chunk_size(8);
        let mut sync_events = Vec::new();
        while let Some(ev) = sync_parser.next_event().unwrap() {
            sync_events.push(ev);
        }
        let mut async_parser = ApkAsyncParser::new(Cursor::new(bytes)).with_chunk_size(8);
        let mut async_events = Vec::new();
        while let Some(e) = block_on(async_parser.next_event()).unwrap() {
            async_events.push(e);
        }
        assert_eq!(sync_events.len(), async_events.len());
        for (s, a) in sync_events.iter().zip(async_events.iter()) {
            assert_eq!(
                std::mem::discriminant(s),
                std::mem::discriminant(a),
                "sync/async event tag mismatch"
            );
        }
    }
}
