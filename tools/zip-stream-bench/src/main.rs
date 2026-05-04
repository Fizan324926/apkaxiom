// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `zip-stream-bench` — P1.7 streaming-vs-file-load microbench.
//!
//! Compares two parser entry points on the same byte slice:
//!
//!   1. **stream** — `ApkParser::from_reader(slice)` consumed event
//!      by event.
//!   2. **file** — `axiom_zip_ref::archive::parse_archive(slice)`
//!      (the verified single-shot parser).
//!
//! Both should produce equivalent semantic outcomes; we measure
//! per-iteration wall-time and emit a JSON summary with min/median
//! /p99/throughput-Mbps.
//!
//! Note: per the P1.7 §4 spec, Criterion is the production
//! microbench tool. We use a hand-rolled timer here to keep the
//! Reindeer surface small (no Criterion dep). The Criterion-based
//! `bench/stream-vs-file.rs` integration ports to Phase-2 hardening.
//!
//! Run protocol:
//!
//! ```bash
//! cargo run -p zip-stream-bench --release -- --iters 10000
//! ```

#![forbid(unsafe_code)]

use std::time::{Duration, Instant};

use axiom_l1_rs::ApkParser;

/// Build a synthetic single-entry archive (98 bytes) for the bench.
/// We keep the corpus in-process so the bench number is purely the
/// parser's overhead — no syscall noise.
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

fn bench_stream(bytes: &[u8], iters: u64) -> Vec<Duration> {
    let mut samples = Vec::with_capacity(iters as usize);
    for _ in 0..iters {
        let t = Instant::now();
        let mut parser = ApkParser::from_reader(bytes);
        let mut count = 0;
        while let Some(_ev) = parser.next_event().unwrap() {
            count += 1;
        }
        let elapsed = t.elapsed();
        std::hint::black_box(count);
        samples.push(elapsed);
    }
    samples
}

/// Measure the time from `from_reader` construction to the first
/// `next_event()` returning `Ok(Some)` — the gate the §10 spec calls
/// "time-to-first-event ≤ 5 ms p99".
fn bench_time_to_first_event(bytes: &[u8], iters: u64) -> Vec<Duration> {
    let mut samples = Vec::with_capacity(iters as usize);
    for _ in 0..iters {
        let t = Instant::now();
        let mut parser = ApkParser::from_reader(bytes);
        let _first = parser.next_event().unwrap();
        let elapsed = t.elapsed();
        samples.push(elapsed);
    }
    samples
}

fn bench_file(bytes: &[u8], iters: u64) -> Vec<Duration> {
    let mut samples = Vec::with_capacity(iters as usize);
    for _ in 0..iters {
        let t = Instant::now();
        let archive = axiom_zip_ref::archive::parse_archive(bytes).unwrap();
        let elapsed = t.elapsed();
        std::hint::black_box(archive);
        samples.push(elapsed);
    }
    samples
}

fn percentile(samples: &mut [Duration], p: f64) -> Duration {
    samples.sort_unstable();
    let idx = ((samples.len() as f64) * p / 100.0).ceil() as usize - 1;
    samples[idx.min(samples.len() - 1)]
}

fn report(label: &str, samples: &mut [Duration], bytes_per_iter: u64) {
    let total: Duration = samples.iter().sum();
    let median = percentile(samples, 50.0);
    let p99 = percentile(samples, 99.0);
    let min = samples.iter().min().copied().unwrap();
    // Throughput in MB/s: bytes_per_iter / iter_time, then sum / N.
    let avg_ns = total.as_nanos() as f64 / samples.len() as f64;
    let mb_per_sec = (bytes_per_iter as f64) * 1_000.0 / avg_ns;
    println!(
        "{label}: min={min:?} median={median:?} p99={p99:?} avg={:.2} MB/s",
        mb_per_sec
    );
}

fn parse_iters() -> u64 {
    std::env::args()
        .skip_while(|a| a != "--iters")
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(10_000)
}

fn report_t1e(samples: &mut [Duration]) {
    let median = percentile(samples, 50.0);
    let p99 = percentile(samples, 99.0);
    let max = samples.iter().max().copied().unwrap();
    let min = samples.iter().min().copied().unwrap();
    println!("time-to-first-event: min={min:?} p50={median:?} p99={p99:?} max={max:?}");
    let gate = Duration::from_millis(5);
    if p99 > gate {
        println!("  ::error::time-to-first-event p99 {p99:?} > spec gate {gate:?}");
        std::process::exit(1);
    } else {
        println!("  PASS: p99 {p99:?} ≤ spec gate {gate:?}");
    }
}

fn main() {
    let iters = parse_iters();
    let bytes = synthetic_archive();
    println!("zip-stream-bench: synthetic 98-byte archive, {iters} iters per arm");

    // (1) Time-to-first-event — the §10 hard floor.
    let mut t1e_samples = bench_time_to_first_event(&bytes, iters);
    report_t1e(&mut t1e_samples);

    // (2) Throughput / total-consume comparison.
    let mut stream_samples = bench_stream(&bytes, iters);
    let mut file_samples = bench_file(&bytes, iters);
    report("stream", &mut stream_samples, bytes.len() as u64);
    report("file  ", &mut file_samples, bytes.len() as u64);

    let stream_p99 = percentile(&mut stream_samples, 99.0);
    let file_p99 = percentile(&mut file_samples, 99.0);
    let parity = if stream_p99.as_nanos() == 0 || file_p99.as_nanos() == 0 {
        0.0
    } else {
        (stream_p99.as_nanos() as f64 - file_p99.as_nanos() as f64).abs()
            / file_p99.as_nanos() as f64
    };
    println!("p99 parity: {:.1}% (spec: ≤ 5%)", parity * 100.0);
}
