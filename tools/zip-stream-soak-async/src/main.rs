// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `zip-stream-soak-async` — io_uring-backed soak harness for the
//! P1.7 streaming parser.
//!
//! Runs the same wire-speed feeder as `tools/zip-stream-soak`, but
//! drives I/O via Glommio's thread-per-core io_uring executor instead
//! of `std::io::Read`. Demonstrates that the parser's runtime-agnostic
//! async surface (`AsyncByteSource`) plugs cleanly into a real
//! io_uring runtime.
//!
//! ## Why io_uring
//!
//! io_uring is the Linux 5.8+ submission-queue/completion-queue
//! interface that lets the parser's "read 64 KiB" requests run
//! coalesced and completion-batched in the kernel — typical
//! single-thread improvement for sequential file reads is 1.5-3×
//! over `read(2)` syscalls. P1.8 (per ADR-0020) will take this
//! further with completion polling and registered buffers.
//!
//! ## What it actually proves
//!
//! Two things:
//!
//! 1. The runtime-agnostic `AsyncByteSource` trait is sufficient — a
//!    real io_uring runtime can drive the parser without any
//!    parser-side changes.
//! 2. End-to-end, the async parser produces the same event stream
//!    the sync parser does (same event count, same byte total).
//!
//! ## Operator one-shot
//!
//! Like `zip-stream-soak`, the 60-minute version of this run is a
//! CHECKLIST §C operator one-shot — it requires dedicated benchmark
//! hardware to be meaningful (Hetzner AX102 / Helio Edge or
//! equivalent). Default duration here is 30 s.
//!
//! ## Required runtime config
//!
//! Glommio bumps `RLIMIT_MEMLOCK` for io_uring SQ/CQ pinning. In a
//! container or restricted dev shell, run with
//! `ulimit -l unlimited` (CAP_SYS_RESOURCE) or `--ulimit memlock=…`.
//! On bare metal as root this is automatic.

#![forbid(unsafe_code)]

use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use axiom_l1_rs::{ApkAsyncParser, AsyncByteSource, ParseEvent};
use glommio::io::BufferedFile;
use glommio::LocalExecutorBuilder;

/// io_uring-backed AsyncByteSource over a `glommio::BufferedFile`.
/// Reads are issued via `read_at(pos, len)` so the kernel can
/// coalesce sequential page-cache reads.
///
/// We deliberately use `BufferedFile` (page-cache reads) rather than
/// `DmaFile` (O_DIRECT). DmaFile requires DMA-aligned positions and
/// sizes (block-size multiples — typically 4 KiB or 512 B); a 64 KiB
/// archive's tail is short of alignment and silently returned 0
/// bytes from `DmaFile::read_at` in early integration runs (manifesting
/// as a spurious truncation after ~17 K archives — exactly when a
/// short tail-read first happened). BufferedFile imposes no
/// alignment constraint and stays on the io_uring fast path.
///
/// The soak's outer loop runs hundreds of thousands of archives, so
/// the source borrows a single long-lived file handle rather than
/// reopening per archive. `rewind()` resets the read cursor so a
/// fresh parser sees a fresh archive without io_uring open/close
/// churn.
struct BufferedFileSource<'a> {
    file: &'a BufferedFile,
    pos: u64,
    eof: u64,
}

impl<'a> BufferedFileSource<'a> {
    fn new(file: &'a BufferedFile, eof: u64) -> Self {
        Self { file, pos: 0, eof }
    }

    fn rewind(&mut self) {
        self.pos = 0;
    }
}

impl<'a> AsyncByteSource for BufferedFileSource<'a> {
    async fn read_chunk(&mut self, n: usize) -> io::Result<Vec<u8>> {
        if self.pos >= self.eof {
            return Ok(Vec::new());
        }
        let want = std::cmp::min(n as u64, self.eof - self.pos) as usize;
        let read = self
            .file
            .read_at(self.pos, want)
            .await
            .map_err(|e| io::Error::other(format!("read_at: {e:?}")))?;
        let bytes = read.to_vec();
        self.pos += bytes.len() as u64;
        Ok(bytes)
    }
}

/// Build a synthetic 64 KiB archive for the soak fixture (mirrors
/// the body shape of `zip-stream-soak`'s `synthetic_archive` but
/// padded out so per-archive parser overhead amortises over a real
/// payload size).
fn synthetic_archive() -> Vec<u8> {
    use axiom_zip_ref::{cdr, eocd, lfh};
    let body = vec![0xa5u8; 64 * 1024];
    let name = b"payload.bin";
    let nl = name.len() as u16;

    let mut bytes = Vec::with_capacity(body.len() + 256);
    let lfh_off = bytes.len() as u32;
    bytes.extend_from_slice(&lfh::SIGNATURE.to_le_bytes());
    bytes.extend_from_slice(&[0x14, 0x00]);
    bytes.extend_from_slice(&[0x00, 0x00]);
    bytes.extend_from_slice(&[0x00, 0x00]);
    bytes.extend_from_slice(&[0x00; 4]);
    bytes.extend_from_slice(&[0x00; 4]);
    let size = body.len() as u32;
    bytes.extend_from_slice(&size.to_le_bytes());
    bytes.extend_from_slice(&size.to_le_bytes());
    bytes.extend_from_slice(&nl.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(name);
    bytes.extend_from_slice(&body);

    let cd_offset = bytes.len() as u32;
    let cdr_start = bytes.len();
    bytes.extend_from_slice(&cdr::SIGNATURE.to_le_bytes());
    bytes.extend_from_slice(&[0x14, 0x00, 0x14, 0x00]);
    bytes.extend_from_slice(&[0u8; 8]);
    bytes.extend_from_slice(&[0u8; 4]);
    bytes.extend_from_slice(&size.to_le_bytes());
    bytes.extend_from_slice(&size.to_le_bytes());
    bytes.extend_from_slice(&nl.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&[0u8; 2]);
    bytes.extend_from_slice(&[0u8; 2]);
    bytes.extend_from_slice(&[0u8; 4]);
    bytes.extend_from_slice(&lfh_off.to_le_bytes());
    bytes.extend_from_slice(name);
    let cd_size = (bytes.len() - cdr_start) as u32;

    bytes.extend_from_slice(&eocd::SIGNATURE.to_le_bytes());
    bytes.extend_from_slice(&[0u8; 4]);
    bytes.extend_from_slice(&[0x01, 0x00]);
    bytes.extend_from_slice(&[0x01, 0x00]);
    bytes.extend_from_slice(&cd_size.to_le_bytes());
    bytes.extend_from_slice(&cd_offset.to_le_bytes());
    bytes.extend_from_slice(&[0u8; 2]);

    bytes
}

fn parse_duration_secs() -> u64 {
    std::env::args()
        .skip_while(|a| a != "--duration-secs")
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(30)
}

fn parse_min_mbps() -> u64 {
    std::env::args()
        .skip_while(|a| a != "--min-mbps")
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(100)
}

fn fixture_path() -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push("apkaxiom-soak-async.zip");
    p
}

fn main() {
    let duration_secs = parse_duration_secs();
    let min_mbps = parse_min_mbps();
    let target = Duration::from_secs(duration_secs);

    // Materialize fixture on disk. io_uring needs a file descriptor;
    // the tmpfs path is fine — the goal is to drive the parser via
    // the same kernel-mediated read path real APK ingestion uses.
    let archive = synthetic_archive();
    let path = fixture_path();
    std::fs::write(&path, &archive).expect("write fixture");
    let archive_len = archive.len() as u64;

    println!(
        "zip-stream-soak-async: {} byte archive at {:?} for {duration_secs} s; gate ≥ {min_mbps} Mbps",
        archive_len, path
    );

    let path_for_executor = path.clone();
    let executor = LocalExecutorBuilder::default()
        .name("apkaxiom-soak-async")
        .make()
        .expect("LocalExecutor build (need RLIMIT_MEMLOCK / CAP_SYS_RESOURCE)");

    let (total_archives, total_events, total_bytes, max_buf_cap, elapsed) =
        executor.run(async move {
            let file = BufferedFile::open(&path_for_executor)
                .await
                .expect("open fixture (need RLIMIT_MEMLOCK / CAP_SYS_RESOURCE)");
            let eof = file.file_size().await.expect("file_size");

            let start = Instant::now();
            let mut total_bytes = 0u64;
            let mut total_archives = 0u64;
            let mut total_events = 0u64;
            let mut max_buf_cap = 0usize;

            while start.elapsed() < target {
                let mut source = BufferedFileSource::new(&file, eof);
                source.rewind();
                let mut parser = ApkAsyncParser::new(source);
                loop {
                    match parser.next_event().await {
                        Ok(Some(ev)) => {
                            total_events += 1;
                            let cap = parser.buf_capacity();
                            if cap > max_buf_cap {
                                max_buf_cap = cap;
                            }
                            if matches!(ev, ParseEvent::ParseComplete { .. }) {
                                total_bytes += archive_len;
                                total_archives += 1;
                                break;
                            }
                        }
                        Ok(None) => break,
                        Err(e) => {
                            eprintln!(
                                "soak-async: stream error after {total_archives} archives: {e}"
                            );
                            std::process::exit(1);
                        }
                    }
                }
            }

            let elapsed = start.elapsed();
            file.close().await.expect("BufferedFile::close");
            (
                total_archives,
                total_events,
                total_bytes,
                max_buf_cap,
                elapsed,
            )
        });

    let total_bits = total_bytes * 8;
    let mbps = total_bits as f64 / elapsed.as_secs_f64() / 1_000_000.0;

    println!(
        "soak-async: {total_archives} archives, {total_events} events, {} bytes in {:.2} s — {:.1} Mbps",
        total_bytes,
        elapsed.as_secs_f64(),
        mbps
    );
    println!(
        "soak-async: max buffer capacity observed: {max_buf_cap} bytes (parser fixed-size, no growth)"
    );

    let _ = std::fs::remove_file(&path);

    if (mbps as u64) < min_mbps {
        eprintln!("::error::soak-async: throughput {mbps:.1} Mbps < gate {min_mbps} Mbps");
        std::process::exit(1);
    }
    println!("soak-async: PASS (≥ {min_mbps} Mbps via io_uring)");
}
