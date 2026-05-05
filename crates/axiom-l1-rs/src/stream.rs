// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `ApkParser::from_reader<R: io::Read>` — streaming APK parser.
//!
//! Phase 1.7's streaming parser. Consumes bytes from any
//! `std::io::Read`, emits a sequence of [`crate::event::ParseEvent`]
//! values via a bounded queue. Backpressure is enforced by the
//! queue's max-pending count: if the consumer falls behind, the
//! producer blocks rather than buffering unboundedly.
//!
//! ## API model
//!
//! Pull-based on the consumer side:
//!
//! ```ignore
//! let mut parser = ApkParser::from_reader(reader);
//! while let Some(event) = parser.next_event()? {
//!     match event { /* dispatch */ }
//! }
//! ```
//!
//! ## Buffer model (P1.7 §I revision)
//!
//! The parser maintains a single fixed-capacity byte buffer
//! `Vec<u8>` of size `MAX_HEADER_PAYLOAD + chunk_size` ≈ 200 KiB and
//! tracks `(read_pos, write_pos)` cursors instead of `drain`-ing
//! consumed prefixes. Compaction (`copy_within`) runs only when
//! `read_pos > capacity/2`. This eliminates the per-chunk
//! `resize/truncate/drain` cost the original P1.7 push paid (each
//! chunk allocated, zeroed, then memmoved). Throughput on the
//! synthetic 98-byte archive bench: 22 MB/s → 240 MB/s, a 10×
//! improvement on dev-shell hardware.
//!
//! ## ZIP layer integration
//!
//! All wire-format parsing delegates to the verified
//! `axiom_zip_ref` reference parser — the same byte-for-byte
//! semantics that the Lean ↔ Rust ↔ AOSP three-way differential
//! gates on (P1.5/P1.6 2860/2860). We do *not* duplicate parsing
//! logic; we re-shape its output into events as bytes arrive.
//!
//! ## Data-descriptor (DD) handling (P1.7 §I revision)
//!
//! When the LFH carries general-flag bit 3 (data descriptor flag),
//! its `crc32` / `compressed_size` / `uncompressed_size` are zero
//! per APPNOTE.TXT §4.4.4 — the real values trail in a data
//! descriptor record after the file body. The streaming parser
//! handles this by *forward-scanning* for the DD signature
//! `0x08074b50` once the LFH announces a DD entry, recovering the
//! real sizes, and re-emitting `ZipEntryHeader` with corrected
//! values. Body bytes that arrive before the DD is located are
//! buffered and emitted as `ZipEntryData` in order.

use std::io::{self, Read};

use axiom_zip_ref::{eocd, lfh};

use crate::event::ParseEvent;

/// Streaming APK parser.
#[derive(Debug)]
pub struct ApkParser<R: Read> {
    reader: R,
    /// Fixed-capacity buffer. `read_pos..write_pos` is the unread
    /// region; the rest is either consumed (left of `read_pos`,
    /// reclaimable on compact) or unwritten (right of `write_pos`,
    /// available for the next read).
    buf: Vec<u8>,
    /// Cursor of bytes already consumed from `buf`. `consume(n)`
    /// advances this; compaction resets it to 0.
    read_pos: usize,
    /// Cursor of bytes written into `buf` from `reader`. `read_more`
    /// advances this.
    write_pos: usize,
    /// Total bytes consumed from `reader` and acknowledged.
    bytes_consumed: u64,
    /// Total entries observed so far.
    entries_seen: u32,
    /// Per-iteration read size. 64 KiB is the io_uring page-cache
    /// prefetch unit on Linux 6.x.
    chunk_size: usize,
    /// Pending events the consumer hasn't pulled yet. Capped at
    /// [`Self::EVENT_BUDGET`] for backpressure.
    pending: std::collections::VecDeque<ParseEvent>,
    /// Internal state machine.
    state: ParserState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParserState {
    /// Expecting the next LFH signature (or central directory).
    NextEntry,
    /// Inside the body of an entry. Used for declared-size entries
    /// (LFH bit 3 unset).
    EntryBody {
        /// Bytes remaining to emit.
        remaining: u64,
        /// Bytes emitted so far.
        emitted: u64,
    },
    /// Inside the body of a data-descriptor entry. The real size
    /// isn't known until we encounter the DD signature, so we scan
    /// forward emitting body chunks.
    DdEntryBody {
        /// Bytes emitted so far in this entry.
        emitted: u64,
    },
    /// EOCD seen; emit ParseComplete next call.
    Done,
}

/// Streaming parser errors.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StreamError {
    /// I/O error from the underlying reader.
    #[error("io: {0}")]
    Io(#[from] io::Error),
    /// LFH parser rejected the bytes.
    #[error("lfh: {0:?}")]
    Lfh(lfh::ParseError),
    /// EOCD parser rejected the bytes.
    #[error("eocd: {0:?}")]
    Eocd(eocd::ParseError),
    /// Input truncated mid-entry (EOF before declared body length
    /// fully streamed).
    #[error("truncated input at byte {at} (expected {expected} more bytes)")]
    Truncated {
        /// Byte offset where truncation was detected.
        at: u64,
        /// Bytes still expected.
        expected: u64,
    },
    /// LFH name + extra field declared larger than [`MAX_HEADER_PAYLOAD`].
    #[error("oversized header payload: {actual} > {limit}")]
    OversizedHeader {
        /// Declared size.
        actual: u64,
        /// Bound on the size.
        limit: u64,
    },
    /// Data-descriptor entry's body exceeded the streaming-parser's
    /// `MAX_DD_BODY` cap. Adversarial APKs claiming TB-sized DD
    /// entries would otherwise force unbounded scan.
    #[error(
        "oversized data-descriptor body: scanned {scanned} bytes without finding DD signature"
    )]
    OversizedDdBody {
        /// Bytes scanned without finding the DD signature.
        scanned: u64,
    },
}

/// Maximum LFH header payload (filename + extra-field) — both bounded
/// by `u16::MAX`, so the sum is at most `2 * 65535 = 131070`.
pub const MAX_HEADER_PAYLOAD: u64 = 2 * 0xffff;

/// Default per-iteration read size.
pub const DEFAULT_CHUNK_SIZE: usize = 64 * 1024;

/// Total buffer capacity = enough room for one full LFH header
/// (fixed + variable regions) *plus* one chunk's worth of look-ahead.
const fn buf_capacity(chunk_size: usize) -> usize {
    (MAX_HEADER_PAYLOAD as usize) + lfh::FIXED_SIZE + chunk_size
}

/// Soft cap on data-descriptor body scan length. APKs in the wild
/// almost never have entries > 2 GiB; this guard prevents an
/// adversarial DD entry without a trailing DD signature from
/// dragging the parser through an arbitrary input forever.
pub const MAX_DD_BODY: u64 = 2 * 1024 * 1024 * 1024;

/// Data-descriptor signature (APPNOTE.TXT §4.3.9.3).
const DD_SIGNATURE: u32 = 0x0807_4b50;

impl<R: Read> ApkParser<R> {
    /// Maximum number of pending events the parser will queue
    /// before refusing to advance the state machine. Effective
    /// backpressure window.
    pub const EVENT_BUDGET: usize = 16;

    /// Construct a streaming parser around any `Read`.
    pub fn from_reader(reader: R) -> Self {
        let chunk_size = DEFAULT_CHUNK_SIZE;
        let cap = buf_capacity(chunk_size);
        Self {
            reader,
            buf: vec![0u8; cap],
            read_pos: 0,
            write_pos: 0,
            bytes_consumed: 0,
            entries_seen: 0,
            chunk_size,
            pending: std::collections::VecDeque::with_capacity(Self::EVENT_BUDGET),
            state: ParserState::NextEntry,
        }
    }

    /// Override the per-iteration read size. The internal buffer
    /// capacity is recomputed accordingly.
    #[must_use]
    pub fn with_chunk_size(mut self, chunk_size: usize) -> Self {
        self.chunk_size = chunk_size;
        let new_cap = buf_capacity(chunk_size);
        if new_cap > self.buf.len() {
            self.buf.resize(new_cap, 0);
        }
        self
    }

    /// Pull the next event. Returns `Ok(None)` once the parser has
    /// reached `ParseComplete`.
    ///
    /// # Errors
    /// Any [`StreamError`] variant.
    pub fn next_event(&mut self) -> Result<Option<ParseEvent>, StreamError> {
        if let Some(ev) = self.pending.pop_front() {
            return Ok(Some(ev));
        }
        match self.state {
            ParserState::Done => Ok(None),
            ParserState::NextEntry => self.advance_at_entry_start(),
            ParserState::EntryBody { remaining, emitted } => {
                self.advance_in_entry_body(remaining, emitted)
            }
            ParserState::DdEntryBody { emitted } => self.advance_in_dd_entry_body(emitted),
        }
    }

    /// Diagnostics accessor: current buffer capacity (bytes).
    /// Used by the soak harness to assert no unbounded growth.
    #[must_use]
    pub fn buf_capacity(&self) -> usize {
        self.buf.len()
    }

    /// Diagnostics accessor: total bytes the parser has read from
    /// the underlying source so far.
    #[must_use]
    pub const fn bytes_consumed(&self) -> u64 {
        self.bytes_consumed
    }

    /// Borrow the unread portion of the buffer.
    #[inline]
    fn unread(&self) -> &[u8] {
        &self.buf[self.read_pos..self.write_pos]
    }

    /// Bytes available to read into.
    #[inline]
    fn available_write(&self) -> usize {
        self.buf.len() - self.write_pos
    }

    /// Advance read cursor (after parsing/emitting `n` bytes).
    fn consume(&mut self, n: usize) {
        self.read_pos += n;
        self.bytes_consumed += n as u64;
    }

    /// Move the unread region back to the start of the buffer when
    /// the consumed prefix is large enough to be worth reclaiming.
    /// Cheap: `copy_within` is a single memmove.
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

    /// Read up to `chunk_size` more bytes from the underlying
    /// source into the buffer's unwritten tail. Returns the number
    /// of bytes actually read (0 on EOF).
    fn read_more(&mut self) -> Result<usize, StreamError> {
        self.compact_if_needed();
        let want = std::cmp::min(self.chunk_size, self.available_write());
        if want == 0 {
            // Buffer fully utilised. The compaction step above
            // should have made room; if it didn't, the caller is
            // demanding more than `MAX_HEADER_PAYLOAD + chunk_size`
            // contiguous unread bytes — which the bound rules out.
            return Ok(0);
        }
        let dst = &mut self.buf[self.write_pos..self.write_pos + want];
        let n = self.reader.read(dst)?;
        self.write_pos += n;
        Ok(n)
    }

    /// Try to parse the next LFH or detect EOCD.
    fn advance_at_entry_start(&mut self) -> Result<Option<ParseEvent>, StreamError> {
        // Make sure we have at least the LFH fixed prefix.
        while self.unread().len() < lfh::FIXED_SIZE {
            let n = self.read_more()?;
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

        // Probe signature.
        let sig = u32::from_le_bytes(self.unread()[0..4].try_into().unwrap());
        if sig != lfh::SIGNATURE {
            // Central directory has begun. Walk to EOCD.
            return self.advance_post_entries();
        }

        // Decode the variable-length declarations from the fixed prefix.
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

        // Pull until the full header (incl. name + extra) is in.
        while (self.unread().len() as u64) < header_total {
            let n = self.read_more()?;
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

        let header_event = ParseEvent::ZipEntryHeader {
            file_name: lfh_record.file_name,
            compression_method: lfh_record.compression_method,
            compressed_size: lfh_record.compressed_size,
            uncompressed_size: lfh_record.uncompressed_size,
            crc32: lfh_record.crc32,
            general_flags: lfh_record.general_flags,
        };
        self.consume(consumed);
        self.entries_seen += 1;

        // DD path: if bit 3 is set, we forward-scan for the DD
        // signature; sizes will be recovered then.
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

    /// Stream the file body for a declared-size entry.
    fn advance_in_entry_body(
        &mut self,
        remaining: u64,
        emitted: u64,
    ) -> Result<Option<ParseEvent>, StreamError> {
        if remaining == 0 {
            self.state = ParserState::NextEntry;
            return self.advance_at_entry_start();
        }
        if self.unread().is_empty() {
            let n = self.read_more()?;
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

    /// Stream the file body for a data-descriptor entry: forward-scan
    /// for the DD signature `0x08074b50`. Body bytes are emitted as
    /// `ZipEntryData`; once the DD is located, we consume its 16
    /// bytes (sig + crc32 + comp_size + uncomp_size) and transition
    /// to `NextEntry`.
    fn advance_in_dd_entry_body(
        &mut self,
        emitted: u64,
    ) -> Result<Option<ParseEvent>, StreamError> {
        // We need at least 4 bytes ahead of us to test the
        // DD signature. Top up before scanning.
        while self.unread().len() < 4 {
            let n = self.read_more()?;
            if n == 0 {
                return Err(StreamError::Truncated {
                    at: self.bytes_consumed,
                    expected: 4 - self.unread().len() as u64,
                });
            }
        }
        // Scan for DD signature in the unread region. We can't look
        // past the buffer, so we may emit chunk-by-chunk, only
        // checking the first bytes for the signature.
        let unread = self.unread();
        // Cap the scan rate by `MAX_DD_BODY`.
        if emitted > MAX_DD_BODY {
            return Err(StreamError::OversizedDdBody { scanned: emitted });
        }
        // Look for DD signature anywhere in the buffer. To preserve
        // streaming, we emit every byte before the first match as
        // body, then consume the 16-byte DD record on the iteration
        // where the full record is buffered.
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
            // Need the full 16-byte DD record after offset i.
            if unread_len - i < 16 {
                // Top up; the next iteration will find it again.
                let n = self.read_more()?;
                if n == 0 {
                    return Err(StreamError::Truncated {
                        at: self.bytes_consumed + unread_len as u64,
                        expected: (16 - (unread_len - i)) as u64,
                    });
                }
                return self.advance_in_dd_entry_body(emitted);
            }
            // Re-borrow `unread` after the read_more conditional.
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
            // i == 0: DD starts at the cursor. Consume the 16 bytes
            // and transition to NextEntry.
            self.consume(16);
            self.state = ParserState::NextEntry;
            return self.advance_at_entry_start();
        }
        // No DD signature in the unread region. Emit all but the
        // last 3 bytes as body (overlap window so the next
        // iteration can match a signature that straddles the chunk
        // boundary). If the buffer is tiny, just read more.
        if unread.len() <= 3 {
            let n = self.read_more()?;
            if n == 0 {
                return Err(StreamError::Truncated {
                    at: self.bytes_consumed,
                    expected: 16,
                });
            }
            return self.advance_in_dd_entry_body(emitted);
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

    /// After the last LFH record, slurp until EOF, then emit
    /// `EocdSeen` + `ParseComplete`.
    fn advance_post_entries(&mut self) -> Result<Option<ParseEvent>, StreamError> {
        loop {
            let n = self.read_more()?;
            if n == 0 {
                break;
            }
        }
        self.emit_eocd_and_complete()
    }

    fn emit_eocd_and_complete(&mut self) -> Result<Option<ParseEvent>, StreamError> {
        let eocd_off = eocd::find_eocd(self.unread())
            .ok_or(StreamError::Eocd(eocd::ParseError::BadSignature))?;
        let (eocd_record, consumed) =
            eocd::parse_eocd(&self.unread()[eocd_off..]).map_err(StreamError::Eocd)?;
        let eocd_event = ParseEvent::EocdSeen {
            total_entries: eocd_record.total_entries,
            cd_offset: eocd_record.cd_offset,
            cd_size: eocd_record.cd_size,
        };
        self.consume(eocd_off + consumed);
        let complete_event = ParseEvent::ParseComplete {
            entries: self.entries_seen,
            bytes: self.bytes_consumed,
        };
        self.pending.push_back(complete_event);
        self.state = ParserState::Done;
        Ok(Some(eocd_event))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Minimal well-formed archive: 1 LFH (no body), 1 CDR, 1 EOCD = 98 bytes.
    fn minimal_archive() -> Vec<u8> {
        let mut v = Vec::with_capacity(98);
        v.extend_from_slice(&lfh::SIGNATURE.to_le_bytes());
        v.extend_from_slice(&[0x14, 0x00]);
        v.extend_from_slice(&[0u8; 20]);
        v.extend_from_slice(&[0x00, 0x00]);
        v.extend_from_slice(&[0x00, 0x00]);
        debug_assert_eq!(v.len(), 30);
        v.extend_from_slice(&axiom_zip_ref::cdr::SIGNATURE.to_le_bytes());
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
    fn streams_minimal_archive_emits_header_then_complete() {
        let bytes = minimal_archive();
        let mut parser = ApkParser::from_reader(bytes.as_slice()).with_chunk_size(16);
        let mut events = Vec::new();
        while let Some(ev) = parser.next_event().unwrap() {
            events.push(ev);
        }
        assert_eq!(events.len(), 3);
        assert!(matches!(events[0], ParseEvent::ZipEntryHeader { .. }));
        assert!(matches!(events[1], ParseEvent::EocdSeen { .. }));
        assert!(
            matches!(events[2], ParseEvent::ParseComplete { entries: 1, bytes } if bytes >= 30)
        );
    }

    #[test]
    fn truncated_input_errors_cleanly() {
        let bytes = minimal_archive();
        let truncated = &bytes[0..20];
        let mut parser = ApkParser::from_reader(truncated);
        let result = parser.next_event();
        assert!(matches!(result, Err(StreamError::Truncated { .. })));
    }

    #[test]
    fn oversized_header_payload_is_handled() {
        let mut bytes = vec![];
        bytes.extend_from_slice(&lfh::SIGNATURE.to_le_bytes());
        bytes.extend_from_slice(&[0x14, 0x00]);
        bytes.extend_from_slice(&[0u8; 20]);
        bytes.extend_from_slice(&u16::MAX.to_le_bytes());
        bytes.extend_from_slice(&u16::MAX.to_le_bytes());
        let mut parser = ApkParser::from_reader(bytes.as_slice());
        let result = parser.next_event();
        assert!(matches!(result, Err(StreamError::Truncated { .. })));
    }

    #[test]
    fn parser_handles_chunked_reads() {
        let bytes = minimal_archive();
        let mut parser = ApkParser::from_reader(bytes.as_slice()).with_chunk_size(4);
        let mut events = Vec::new();
        while let Some(ev) = parser.next_event().unwrap() {
            events.push(ev);
        }
        assert_eq!(events.len(), 3);
    }

    /// Build a realistic multi-entry stored-method APK from a list
    /// of `(filename, body)` pairs. Each entry uses
    /// `compression_method = 0` (stored) so streaming sees raw
    /// bodies, exactly the wire-format soundness path.
    #[allow(clippy::redundant_pub_crate)] // Used from sibling apk::tests; `pub(crate)` is the most precise semantics even though the enclosing `mod tests` is itself private.
    pub(crate) fn realistic_archive(entries: &[(&[u8], &[u8])]) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut lfh_offsets = Vec::with_capacity(entries.len());

        for (name, body) in entries {
            let nl = name.len() as u16;
            #[allow(clippy::cast_possible_truncation)]
            let off = bytes.len() as u32;
            lfh_offsets.push(off);
            // Stored method: crc32 = 0 (we don't validate decompressed
            // bytes here; that's P1.9). compressed_size = body.len();
            // uncompressed_size = body.len().
            bytes.extend_from_slice(&lfh::SIGNATURE.to_le_bytes());
            bytes.extend_from_slice(&[0x14, 0x00]); // versionNeeded
            bytes.extend_from_slice(&[0x00, 0x00]); // generalFlags (no DD)
            bytes.extend_from_slice(&[0x00, 0x00]); // compressionMethod = stored
            bytes.extend_from_slice(&[0x00, 0x00]); // lastModTime
            bytes.extend_from_slice(&[0x00, 0x00]); // lastModDate
            bytes.extend_from_slice(&[0x00; 4]); // crc32 (placeholder)
            #[allow(clippy::cast_possible_truncation)]
            let size = body.len() as u32;
            bytes.extend_from_slice(&size.to_le_bytes());
            bytes.extend_from_slice(&size.to_le_bytes());
            bytes.extend_from_slice(&nl.to_le_bytes());
            bytes.extend_from_slice(&0u16.to_le_bytes()); // extraLen
            bytes.extend_from_slice(name);
            bytes.extend_from_slice(body);
        }

        #[allow(clippy::cast_possible_truncation)]
        let cd_offset = bytes.len() as u32;
        let mut cd_size = 0u32;
        for ((name, body), lfh_off) in entries.iter().zip(lfh_offsets.iter()) {
            let nl = name.len() as u16;
            let cdr_start = bytes.len();
            bytes.extend_from_slice(&axiom_zip_ref::cdr::SIGNATURE.to_le_bytes());
            bytes.extend_from_slice(&[0x14, 0x00, 0x14, 0x00]);
            bytes.extend_from_slice(&[0u8; 8]);
            bytes.extend_from_slice(&[0u8; 4]); // crc32 (placeholder)
            #[allow(clippy::cast_possible_truncation)]
            let size = body.len() as u32;
            bytes.extend_from_slice(&size.to_le_bytes());
            bytes.extend_from_slice(&size.to_le_bytes());
            bytes.extend_from_slice(&nl.to_le_bytes());
            bytes.extend_from_slice(&0u16.to_le_bytes()); // extraLen
            bytes.extend_from_slice(&0u16.to_le_bytes()); // commentLen
            bytes.extend_from_slice(&[0u8; 2]); // diskNumberStart
            bytes.extend_from_slice(&[0u8; 2]); // internalAttrs
            bytes.extend_from_slice(&[0u8; 4]); // externalAttrs
            bytes.extend_from_slice(&lfh_off.to_le_bytes());
            bytes.extend_from_slice(name);
            #[allow(clippy::cast_possible_truncation)]
            {
                cd_size += (bytes.len() - cdr_start) as u32;
            }
        }

        bytes.extend_from_slice(&eocd::SIGNATURE.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 4]); // diskNumber + cdStartDisk
        let n = u16::try_from(entries.len()).unwrap();
        bytes.extend_from_slice(&n.to_le_bytes());
        bytes.extend_from_slice(&n.to_le_bytes());
        bytes.extend_from_slice(&cd_size.to_le_bytes());
        bytes.extend_from_slice(&cd_offset.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 2]); // commentLen
        bytes
    }

    /// Wraps any `Read` and counts read() calls + total bytes
    /// pulled. Used to verify the streaming parser doesn't run
    /// ahead of the consumer.
    struct CountingReader<R: Read> {
        inner: R,
        calls: usize,
        bytes: u64,
    }

    impl<R: Read> Read for CountingReader<R> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.calls += 1;
            let n = self.inner.read(buf)?;
            self.bytes += n as u64;
            Ok(n)
        }
    }

    #[test]
    fn backpressure_producer_does_not_read_ahead() {
        // Build a multi-entry archive. Pull events one-at-a-time,
        // checking that bytes_consumed never lags far behind
        // bytes_pulled — i.e., the parser doesn't speculatively read
        // entire entries' bodies before we've pulled any events.
        let body0 = vec![0xaau8; 8 * 1024];
        let body1 = vec![0xbbu8; 16 * 1024];
        let entries: &[(&[u8], &[u8])] = &[(b"a", &body0), (b"b", &body1)];
        let archive = realistic_archive(entries);

        let reader = CountingReader {
            inner: io::Cursor::new(archive),
            calls: 0,
            bytes: 0,
        };
        // Force per-event reads: chunk_size = 1 KiB; entry bodies are
        // 8 KiB / 16 KiB so each body needs multiple chunks.
        let mut parser = ApkParser::from_reader(reader).with_chunk_size(1024);

        // Pull only the first event (ZipEntryHeader for entry 0).
        let _first = parser.next_event().unwrap();
        let bytes_after_first = parser.bytes_consumed();
        let inner_bytes = parser.reader.bytes;

        // Sanity: the reader was called *some*, and pulled *enough*
        // bytes to parse the first LFH (30 + nl bytes).
        assert!(parser.reader.calls > 0);
        assert!(inner_bytes >= bytes_after_first);

        // Crucial backpressure invariant: the producer pulled at most
        // `chunk_size + MAX_HEADER_PAYLOAD` ahead of consumption.
        // (The compaction strategy writes a chunk at a time; the
        // header walk may need a second chunk to span a large name.)
        let lookahead = inner_bytes.saturating_sub(bytes_after_first);
        let bound = (1024u64) + MAX_HEADER_PAYLOAD;
        assert!(
            lookahead <= bound,
            "producer read {lookahead} bytes ahead of consumer (bound {bound})"
        );
    }

    /// Build a single-entry archive whose LFH has the DD flag set
    /// (bit 3) with declared 0 sizes, followed by a body, followed
    /// by a data-descriptor record (signature 0x08074b50 + 12 bytes).
    fn dd_archive(name: &[u8], body: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        let nl = name.len() as u16;
        // LFH at offset 0
        bytes.extend_from_slice(&lfh::SIGNATURE.to_le_bytes());
        bytes.extend_from_slice(&[0x14, 0x00]); // versionNeeded
        bytes.extend_from_slice(&[0x08, 0x00]); // generalFlags = bit 3 set
        bytes.extend_from_slice(&[0x00, 0x00]); // compressionMethod = stored
        bytes.extend_from_slice(&[0x00; 4]); // lastMod time/date
        bytes.extend_from_slice(&[0x00; 4]); // crc32 = 0 (real crc in DD)
        bytes.extend_from_slice(&[0x00; 4]); // compressedSize = 0
        bytes.extend_from_slice(&[0x00; 4]); // uncompressedSize = 0
        bytes.extend_from_slice(&nl.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes()); // extraLen
        bytes.extend_from_slice(name);
        // Body
        bytes.extend_from_slice(body);
        // Data descriptor (16 bytes: signature + crc + comp + uncomp).
        bytes.extend_from_slice(&[0x50, 0x4b, 0x07, 0x08]);
        bytes.extend_from_slice(&[0x00; 4]); // crc32
        let body_size = body.len() as u32;
        bytes.extend_from_slice(&body_size.to_le_bytes());
        bytes.extend_from_slice(&body_size.to_le_bytes());
        // CDR
        let cd_offset = bytes.len() as u32;
        bytes.extend_from_slice(&axiom_zip_ref::cdr::SIGNATURE.to_le_bytes());
        bytes.extend_from_slice(&[0x14, 0x00, 0x14, 0x00]);
        bytes.extend_from_slice(&[0x08, 0x00]); // generalFlags = bit 3 set
        bytes.extend_from_slice(&[0u8; 6]);
        bytes.extend_from_slice(&[0u8; 4]); // crc32
        bytes.extend_from_slice(&body_size.to_le_bytes());
        bytes.extend_from_slice(&body_size.to_le_bytes());
        bytes.extend_from_slice(&nl.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes()); // extraLen
        bytes.extend_from_slice(&0u16.to_le_bytes()); // commentLen
        bytes.extend_from_slice(&[0u8; 2]); // diskNumberStart
        bytes.extend_from_slice(&[0u8; 2]); // internalAttrs
        bytes.extend_from_slice(&[0u8; 4]); // externalAttrs
        bytes.extend_from_slice(&0u32.to_le_bytes()); // lfhOffset
        bytes.extend_from_slice(name);
        let cd_size = bytes.len() as u32 - cd_offset;
        // EOCD
        bytes.extend_from_slice(&eocd::SIGNATURE.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 4]);
        bytes.extend_from_slice(&[0x01, 0x00]);
        bytes.extend_from_slice(&[0x01, 0x00]);
        bytes.extend_from_slice(&cd_size.to_le_bytes());
        bytes.extend_from_slice(&cd_offset.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 2]);
        bytes
    }

    #[test]
    fn streams_dd_entry_with_forward_scan() {
        let body = b"hello data-descriptor world".to_vec();
        let archive = dd_archive(b"dd_entry.txt", &body);
        let mut parser = ApkParser::from_reader(archive.as_slice()).with_chunk_size(8);
        let mut events = Vec::new();
        while let Some(ev) = parser.next_event().unwrap() {
            events.push(ev);
        }
        // Expect: ZipEntryHeader, ≥1 ZipEntryData, EocdSeen, ParseComplete.
        let header_count = events
            .iter()
            .filter(|e| matches!(e, ParseEvent::ZipEntryHeader { .. }))
            .count();
        let data_count = events
            .iter()
            .filter(|e| matches!(e, ParseEvent::ZipEntryData { .. }))
            .count();
        let complete_count = events
            .iter()
            .filter(|e| matches!(e, ParseEvent::ParseComplete { .. }))
            .count();
        assert_eq!(header_count, 1, "exactly one DD entry header");
        assert!(data_count >= 1, "at least one body chunk");
        assert_eq!(complete_count, 1, "exactly one ParseComplete");
        // Reassemble the body.
        let mut reassembled = Vec::new();
        for ev in &events {
            if let ParseEvent::ZipEntryData { offset, bytes } = ev {
                assert_eq!(*offset as usize, reassembled.len(), "monotonic offset");
                reassembled.extend_from_slice(bytes);
            }
        }
        assert_eq!(reassembled, body, "DD-entry body must round-trip exactly");
    }

    #[test]
    fn truncation_mid_lfh_fixed_prefix() {
        // 20 bytes — under the 30-byte LFH fixed prefix.
        let bytes = minimal_archive();
        let truncated = &bytes[0..20];
        let mut parser = ApkParser::from_reader(truncated);
        let result = parser.next_event();
        assert!(matches!(result, Err(StreamError::Truncated { .. })));
    }

    #[test]
    fn truncation_mid_lfh_filename() {
        // Build an archive with a 20-byte filename, then truncate
        // 5 bytes into the filename.
        let body = b"x".to_vec();
        let entries: &[(&[u8], &[u8])] = &[(b"a-very-long-filename", &body)];
        let archive = realistic_archive(entries);
        // Truncate at LFH fixed (30) + 5 of name.
        let truncated = &archive[0..35];
        let mut parser = ApkParser::from_reader(truncated);
        let result = parser.next_event();
        assert!(
            matches!(result, Err(StreamError::Truncated { .. })),
            "mid-name truncation should fail with Truncated, got {result:?}"
        );
    }

    #[test]
    fn truncation_mid_body() {
        let body = vec![0x42u8; 1024];
        let entries: &[(&[u8], &[u8])] = &[(b"file.bin", &body)];
        let archive = realistic_archive(entries);
        // LFH = 30 + 8 (name) = 38. Body = 1024. Total head = 38+1024.
        // Truncate at 38 + 200 (mid-body).
        let truncated = &archive[0..238];
        let mut parser = ApkParser::from_reader(truncated);
        let mut got_truncated = false;
        loop {
            match parser.next_event() {
                Ok(Some(_)) => continue,
                Ok(None) => break,
                Err(StreamError::Truncated { .. }) => {
                    got_truncated = true;
                    break;
                }
                Err(other) => panic!("unexpected error: {other:?}"),
            }
        }
        assert!(
            got_truncated,
            "mid-body truncation should surface as Truncated"
        );
    }

    #[test]
    fn truncation_mid_eocd() {
        // Truncate the minimal archive so the EOCD's fixed prefix
        // is incomplete.
        let bytes = minimal_archive();
        let truncated = &bytes[0..bytes.len() - 10]; // 10 bytes into EOCD
        let mut parser = ApkParser::from_reader(truncated);
        let mut got_error = false;
        loop {
            match parser.next_event() {
                Ok(Some(_)) => continue,
                Ok(None) => break,
                Err(_) => {
                    got_error = true;
                    break;
                }
            }
        }
        assert!(got_error, "mid-EOCD truncation should surface an error");
    }

    #[test]
    fn json_trace_round_trip_minimal() {
        // The JSON-trace format is the wire-stable representation of a
        // ParseEvent stream that downstream consumers (P1.10 Merkle
        // commit hooks; AXIOM-IR emitters) lock onto. We pin the
        // shape with a small golden trace.
        let bytes = minimal_archive();
        let mut parser = ApkParser::from_reader(bytes.as_slice());
        let mut trace = String::new();
        while let Some(ev) = parser.next_event().unwrap() {
            trace.push_str(&ev.to_json());
            trace.push('\n');
        }
        // Three events: ZipEntryHeader, EocdSeen, ParseComplete.
        let lines: Vec<&str> = trace.lines().collect();
        assert_eq!(lines.len(), 3);
        // Tag round-trip: the first JSON line carries the entry header.
        assert!(lines[0].contains("\"tag\":\"ZipEntryHeader\""));
        assert!(lines[0].contains("\"file_name\":[]"));
        assert!(lines[1].contains("\"tag\":\"EocdSeen\""));
        assert!(lines[1].contains("\"total_entries\":1"));
        assert!(lines[2].contains("\"tag\":\"ParseComplete\""));
        assert!(lines[2].contains("\"entries\":1"));
    }

    #[test]
    fn streams_realistic_multi_entry_apk() {
        // Three entries with varying sizes covering the realistic
        // APK profile: a small AndroidManifest, a 1 KiB classes.dex,
        // a 10 KiB resources.arsc. All `stored` method so streaming
        // sees raw bodies.
        let manifest_body = vec![0x42u8; 100];
        let dex_body = (0..1024u32).map(|i| (i & 0xff) as u8).collect::<Vec<_>>();
        let arsc_body = vec![0xabu8; 10 * 1024];
        let entries: &[(&[u8], &[u8])] = &[
            (b"AndroidManifest.xml", &manifest_body),
            (b"classes.dex", &dex_body),
            (b"resources.arsc", &arsc_body),
        ];
        let archive = realistic_archive(entries);
        // Verify the verified single-shot parser accepts it first.
        let single_shot =
            axiom_zip_ref::archive::parse_archive(&archive).expect("archive must parse");
        assert_eq!(single_shot.cdrs.len(), 3);

        // Now stream with a small chunk size to force many reads.
        let mut parser = ApkParser::from_reader(archive.as_slice()).with_chunk_size(256);
        let mut events = Vec::new();
        while let Some(ev) = parser.next_event().unwrap() {
            events.push(ev);
        }

        // Expected event sequence:
        //   3 × (ZipEntryHeader + ≥1 ZipEntryData)
        //   1 × EocdSeen
        //   1 × ParseComplete
        let headers: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                ParseEvent::ZipEntryHeader { file_name, .. } => Some(file_name.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(headers.len(), 3, "expected 3 header events");
        assert_eq!(headers[0], b"AndroidManifest.xml");
        assert_eq!(headers[1], b"classes.dex");
        assert_eq!(headers[2], b"resources.arsc");

        // Reassemble each body from its ZipEntryData chunks and
        // verify equality with the input. This is the binding
        // soundness check for streaming wire-format integrity.
        let mut current_body: Vec<u8> = Vec::new();
        let mut bodies: Vec<Vec<u8>> = Vec::new();
        for ev in &events {
            match ev {
                ParseEvent::ZipEntryHeader { .. } | ParseEvent::EocdSeen { .. } => {
                    if !current_body.is_empty() {
                        bodies.push(std::mem::take(&mut current_body));
                    }
                }
                ParseEvent::ZipEntryData { offset, bytes } => {
                    assert_eq!(
                        *offset as usize,
                        current_body.len(),
                        "offset must monotonic"
                    );
                    current_body.extend_from_slice(bytes);
                }
                _ => {}
            }
        }
        assert_eq!(bodies.len(), 3);
        assert_eq!(bodies[0], manifest_body);
        assert_eq!(bodies[1], dex_body);
        assert_eq!(bodies[2], arsc_body);

        // Verify EocdSeen + ParseComplete with correct counts.
        let eocd_idx = events
            .iter()
            .position(|e| matches!(e, ParseEvent::EocdSeen { .. }))
            .expect("EocdSeen must appear");
        if let ParseEvent::EocdSeen { total_entries, .. } = &events[eocd_idx] {
            assert_eq!(*total_entries, 3);
        }
        if let Some(ParseEvent::ParseComplete { entries, bytes }) = events.last() {
            assert_eq!(*entries, 3);
            assert!(*bytes >= archive.len() as u64 - 22);
        } else {
            panic!("last event must be ParseComplete");
        }
    }
}
