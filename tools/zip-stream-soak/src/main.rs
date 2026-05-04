// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `zip-stream-soak` — P1.7 wire-speed sustained-throughput soak test.
//!
//! Streams a synthetic byte sequence at maximum producer rate
//! through the streaming parser for a configurable duration. Asserts:
//!
//!   - Throughput ≥ a configurable lower bound (default 500 Mbps,
//!     matching the §10 hard-floor).
//!   - No panics, no errors.
//!   - Internal buffer doesn't grow unboundedly.
//!
//! Spec §9 calls for a 60-minute soak run on dedicated hardware
//! (Hetzner AX102 / Helio Edge equivalent). Our default is 60
//! seconds — tunable via `--duration-secs`. The 60-minute run is
//! tracked as a CHECKLIST §C operator one-shot (it requires
//! dedicated benchmark hardware to be meaningful, which the
//! procurement step in §5 explicitly tracks).

#![forbid(unsafe_code)]

use std::io;
use std::time::{Duration, Instant};

use axiom_l1_rs::{ApkParser, ParseEvent, DEFAULT_CHUNK_SIZE, MAX_HEADER_PAYLOAD};

/// Synthetic infinite-stream reader that returns the same archive
/// repeated forever. Models a wire-speed feeder without needing
/// iperf3 / a real network.
struct InfiniteArchive {
    template: Vec<u8>,
    cursor: usize,
}

impl InfiniteArchive {
    fn new(template: Vec<u8>) -> Self {
        Self {
            template,
            cursor: 0,
        }
    }
}

impl io::Read for InfiniteArchive {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // One archive worth of bytes per `Read`-instance lifetime,
        // then `0` (EOF). The soak's outer loop instantiates a fresh
        // reader for each archive, so this still drives sustained
        // wire-speed throughput while keeping the streaming parser's
        // EOF semantics intact.
        if self.cursor >= self.template.len() {
            return Ok(0);
        }
        let n = buf.len().min(self.template.len() - self.cursor);
        buf[..n].copy_from_slice(&self.template[self.cursor..self.cursor + n]);
        self.cursor += n;
        Ok(n)
    }
}

fn synthetic_archive() -> Vec<u8> {
    use axiom_zip_ref::{cdr, eocd, lfh};
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

fn parse_duration_secs() -> u64 {
    std::env::args()
        .skip_while(|a| a != "--duration-secs")
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(60)
}

fn parse_min_mbps() -> u64 {
    std::env::args()
        .skip_while(|a| a != "--min-mbps")
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(500)
}

fn main() {
    let duration_secs = parse_duration_secs();
    let min_mbps = parse_min_mbps();
    let target = Duration::from_secs(duration_secs);
    let archive = synthetic_archive();
    let archive_len = archive.len();
    println!(
        "zip-stream-soak: streaming {} byte archive for {duration_secs} s; gate ≥ {min_mbps} Mbps",
        archive_len
    );

    let start = Instant::now();
    let mut total_bytes = 0u64;
    let mut total_archives = 0u64;
    let mut total_events = 0u64;
    // Memory-growth assertion: track the maximum buffer capacity
    // observed across every parser instance. The streaming parser's
    // internal buffer is fixed-size at construction, so this should
    // be exactly `buf_capacity(DEFAULT_CHUNK_SIZE)`.
    let mut max_buf_cap: usize = 0;
    // Spec §9 bound: buffer never exceeds
    //   `MAX_HEADER_PAYLOAD + DEFAULT_CHUNK_SIZE + LFH_FIXED_SIZE`
    // (the architectural cap from `buf_capacity` in stream.rs).
    let mem_bound = MAX_HEADER_PAYLOAD as usize + DEFAULT_CHUNK_SIZE + 64;

    while start.elapsed() < target {
        let reader = InfiniteArchive::new(archive.clone());
        let mut parser = ApkParser::from_reader(reader);
        loop {
            match parser.next_event() {
                Ok(Some(ev)) => {
                    total_events += 1;
                    let cap = parser.buf_capacity();
                    if cap > max_buf_cap {
                        max_buf_cap = cap;
                    }
                    if cap > mem_bound {
                        eprintln!("::error::soak: buffer grew to {cap} bytes (bound {mem_bound})");
                        std::process::exit(1);
                    }
                    if matches!(ev, ParseEvent::ParseComplete { .. }) {
                        total_bytes += archive_len as u64;
                        total_archives += 1;
                        break;
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    eprintln!("soak: stream error after {total_archives} archives: {e}");
                    std::process::exit(1);
                }
            }
        }
    }

    let elapsed = start.elapsed();
    let total_bits = total_bytes * 8;
    let mbps = total_bits as f64 / elapsed.as_secs_f64() / 1_000_000.0;

    println!(
        "soak: {total_archives} archives, {total_events} events, {} bytes in {:.2} s — {:.1} Mbps",
        total_bytes,
        elapsed.as_secs_f64(),
        mbps
    );
    println!(
        "soak: max buffer capacity observed: {max_buf_cap} bytes (bound {mem_bound}, spec §9: no unbounded growth)"
    );

    if (mbps as u64) < min_mbps {
        eprintln!("::error::soak: throughput {mbps:.1} Mbps < gate {min_mbps} Mbps");
        std::process::exit(1);
    }
    println!("soak: PASS (≥ {min_mbps} Mbps)");
}
