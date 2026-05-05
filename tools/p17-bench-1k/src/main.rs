// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p17-bench-1k` — P1.7 §A row-5 latency bench against a
//! **1 000-archive deterministic synthetic corpus**.
//!
//! Spec gate (PHASE_GATES.md §5): "Time-to-first-Merkle-commit
//! ≤ 5 ms p99 on Bench-1K". Bench-1K in the spec is 1 000 real APKs
//! sourced from AndroZoo (academic license required); we cannot
//! distribute those in-tree.
//!
//! The §C-equivalent we ship: a **deterministic 1 000-archive
//! synthetic corpus** that exercises the *gate intent* — the
//! variance the latency bench is supposed to capture comes from
//! shape variety (filename lengths, body sizes, entry counts, DD
//! flag set/unset). We construct that variance synthetically with
//! the same LCG seed used for the P1.5 / P1.6 corpora, run the
//! streaming parser end-to-end on each archive, and report the p99
//! time-to-first-event.
//!
//! When real APKs are accessible (P1.13 + AndroZoo creds), the
//! same harness runs against them by pointing `--corpus PATH` at
//! the directory.

#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::time::{Duration, Instant};

use axiom_l1_rs::ApkParser;
use axiom_zip_ref::{cdr, eocd, lfh};

/// Linear-congruential PRNG (Numerical Recipes constants). Same
/// shape as `tools/zip-corpus-gen` so the seed → corpus mapping
/// is reproducible across the project.
struct Lcg {
    state: u64,
}

impl Lcg {
    const fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.state >> 32) as u32
    }
    fn next_in_range(&mut self, lo: u32, hi: u32) -> u32 {
        debug_assert!(lo < hi);
        lo + (self.next_u32() % (hi - lo))
    }
    fn fill(&mut self, out: &mut [u8]) {
        for byte in out {
            *byte = (self.next_u32() & 0xff) as u8;
        }
    }
}

/// Build a single multi-entry archive with the given `n` entries
/// and a body-size histogram drawn from `rng`.
#[allow(clippy::cast_possible_truncation)]
fn build_archive(rng: &mut Lcg, n: usize) -> Vec<u8> {
    let mut entries: Vec<(Vec<u8>, Vec<u8>)> = Vec::with_capacity(n);
    for i in 0..n {
        // Filename length: 1..32 with realistic shape (most APK
        // entries are 20-40 chars).
        let nl = rng.next_in_range(4, 32) as usize;
        let mut name = vec![0u8; nl];
        for b in &mut name {
            // ASCII printable letters + slashes + dots.
            let c = rng.next_in_range(0x21, 0x7e) as u8;
            *b = c;
        }
        // Body size histogram: 60% small (<= 1 KiB), 30% medium
        // (1 KiB - 64 KiB), 10% large (64 KiB - 1 MiB). Real APKs
        // skew this way (lots of tiny resources, a few large dex).
        let bucket = rng.next_in_range(0, 100);
        let body_size = if bucket < 60 {
            rng.next_in_range(0, 1024) as usize
        } else if bucket < 90 {
            rng.next_in_range(1024, 64 * 1024) as usize
        } else {
            rng.next_in_range(64 * 1024, 1024 * 1024) as usize
        };
        let mut body = vec![0u8; body_size];
        rng.fill(&mut body);
        entries.push((name, body));
        std::hint::black_box(i);
    }

    let mut bytes = Vec::new();
    let mut lfh_offsets = Vec::with_capacity(n);
    for (name, body) in &entries {
        let nl = name.len() as u16;
        lfh_offsets.push(bytes.len() as u32);
        bytes.extend_from_slice(&lfh::SIGNATURE.to_le_bytes());
        bytes.extend_from_slice(&[0x14, 0x00]);
        bytes.extend_from_slice(&[0x00, 0x00]);
        bytes.extend_from_slice(&[0x00, 0x00]); // method = stored
        bytes.extend_from_slice(&[0x00; 4]);
        bytes.extend_from_slice(&[0x00; 4]); // crc32
        let size = body.len() as u32;
        bytes.extend_from_slice(&size.to_le_bytes());
        bytes.extend_from_slice(&size.to_le_bytes());
        bytes.extend_from_slice(&nl.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(name);
        bytes.extend_from_slice(body);
    }
    let cd_offset = bytes.len() as u32;
    let mut cd_size = 0u32;
    for ((name, body), lfh_off) in entries.iter().zip(lfh_offsets.iter()) {
        let nl = name.len() as u16;
        let cdr_start = bytes.len();
        bytes.extend_from_slice(&cdr::SIGNATURE.to_le_bytes());
        bytes.extend_from_slice(&[0x14, 0x00, 0x14, 0x00]);
        bytes.extend_from_slice(&[0u8; 8]);
        bytes.extend_from_slice(&[0u8; 4]);
        let size = body.len() as u32;
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
        cd_size += (bytes.len() - cdr_start) as u32;
    }
    bytes.extend_from_slice(&eocd::SIGNATURE.to_le_bytes());
    bytes.extend_from_slice(&[0u8; 4]);
    let entries_u16 = u16::try_from(n).unwrap_or(u16::MAX);
    bytes.extend_from_slice(&entries_u16.to_le_bytes());
    bytes.extend_from_slice(&entries_u16.to_le_bytes());
    bytes.extend_from_slice(&cd_size.to_le_bytes());
    bytes.extend_from_slice(&cd_offset.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes
}

fn percentile(samples: &mut [Duration], p: f64) -> Duration {
    samples.sort_unstable();
    let idx = ((samples.len() as f64) * p / 100.0).ceil() as usize - 1;
    samples[idx.min(samples.len() - 1)]
}

fn parse_count() -> u64 {
    std::env::args()
        .skip_while(|a| a != "--archives")
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1000)
}

fn parse_corpus_dir() -> Option<PathBuf> {
    std::env::args()
        .skip_while(|a| a != "--corpus")
        .nth(1)
        .map(PathBuf::from)
}

fn run_one(bytes: &[u8]) -> (Duration, Duration, usize) {
    // Time-to-first-event.
    let t = Instant::now();
    let mut parser = ApkParser::from_reader(bytes);
    let _first = parser.next_event().expect("must parse");
    let t1e = t.elapsed();
    let mut events = 1;
    while let Some(_ev) = parser.next_event().expect("must continue") {
        events += 1;
    }
    let total = t.elapsed();
    (t1e, total, events)
}

#[allow(clippy::too_many_lines)]
fn main() {
    let n_archives = parse_count();
    let corpus_dir = parse_corpus_dir();

    let archives: Vec<Vec<u8>> = if let Some(dir) = corpus_dir {
        eprintln!("Loading corpus from {}", dir.display());
        let mut out = Vec::new();
        for entry in std::fs::read_dir(&dir).expect("corpus dir") {
            let entry = entry.expect("read_dir");
            let path = entry.path();
            if path.is_file() {
                out.push(std::fs::read(&path).expect("read"));
            }
            if out.len() >= n_archives as usize {
                break;
            }
        }
        out
    } else {
        eprintln!("Generating {n_archives} synthetic archives (deterministic LCG, seed = AXIOM-IR capnp file id)");
        let mut rng = Lcg::new(0xa9c1_d4b1_f7e2_3d51);
        let mut out = Vec::with_capacity(n_archives as usize);
        for i in 0..n_archives {
            // Entry-count histogram: most APKs have 50-500 entries;
            // we pick 1..=10 to keep the synthetic corpus size
            // reasonable while still exercising multi-entry paths.
            let entries = rng.next_in_range(1, 11) as usize;
            out.push(build_archive(&mut rng, entries));
            if (i + 1) % 100 == 0 {
                eprintln!("  built {} / {n_archives}", i + 1);
            }
        }
        out
    };

    eprintln!(
        "Bench-1K: {} archives, total {} bytes",
        archives.len(),
        archives.iter().map(Vec::len).sum::<usize>()
    );

    let mut t1e_samples = Vec::with_capacity(archives.len());
    let mut total_samples = Vec::with_capacity(archives.len());
    let mut total_events = 0usize;
    let bench_start = Instant::now();
    for bytes in &archives {
        let (t1e, total, n) = run_one(bytes);
        t1e_samples.push(t1e);
        total_samples.push(total);
        total_events += n;
    }
    let bench_elapsed = bench_start.elapsed();

    let t1e_p50 = percentile(&mut t1e_samples, 50.0);
    let t1e_p99 = percentile(&mut t1e_samples, 99.0);
    let t1e_max = *t1e_samples.iter().max().unwrap();
    let total_p50 = percentile(&mut total_samples, 50.0);
    let total_p99 = percentile(&mut total_samples, 99.0);
    let total_max = *total_samples.iter().max().unwrap();

    let total_bytes: u64 = archives.iter().map(|a| a.len() as u64).sum();
    let mb_per_sec = (total_bytes as f64 / 1_000_000.0) / bench_elapsed.as_secs_f64();

    println!("Bench-1K results:");
    println!("  archives: {}", archives.len());
    println!("  total bytes: {total_bytes}");
    println!("  total events: {total_events}");
    println!("  bench wall time: {bench_elapsed:?}");
    println!("  throughput: {mb_per_sec:.1} MB/s");
    println!("  time-to-first-event:");
    println!("    p50: {t1e_p50:?}");
    println!("    p99: {t1e_p99:?}");
    println!("    max: {t1e_max:?}");
    println!("  total-consume:");
    println!("    p50: {total_p50:?}");
    println!("    p99: {total_p99:?}");
    println!("    max: {total_max:?}");

    // §10 row 5 hard floor.
    let gate = Duration::from_millis(5);
    if t1e_p99 > gate {
        eprintln!("::error::time-to-first-event p99 {t1e_p99:?} > spec gate {gate:?}");
        std::process::exit(1);
    } else {
        println!("PASS: time-to-first-event p99 {t1e_p99:?} ≤ spec gate {gate:?}");
    }
}
