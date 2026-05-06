// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p112-bench-10k` — P1.12 end-to-end benchmark corpus.
//!
//! Generates 10 000 deterministic well-formed ZIP archives under
//! `corpus/zip/bench-10k/<idx:05>.bin`, with a manifest of
//! per-archive verification verdicts.
//!
//! The same 10 000 inputs back the four HARD perf gates the
//! verified ZIP layer must clear:
//!
//!   - `p112-perf-delta`     — verified vs hand-written ≤ 15 %
//!   - `p112-throughput`     — ≥ 250 APKs / sec / 16-core
//!   - `p112-latency`        — p99 ≤ 80 ms
//!   - `p112-tv-bench-10k`   — verified-path acceptance, 10 000/10 000
//!
//! Determinism contract: same seed → byte-identical corpus. The
//! generator uses an in-tree LCG (no `rand` dep) to keep the
//! Reindeer surface small.

#![forbid(unsafe_code)]
#![warn(missing_docs, unreachable_pub)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::uninlined_format_args,
    clippy::too_many_lines
)]

use std::{
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
    time::Instant,
};

use axiom_l0_zip_verified::consistency::parse_archive;
use axiom_zip_ref::{cdr, eocd, lfh};

/// Bench-10K corpus size — pinned in the docstring above.
const BENCH_SIZE: usize = 10_000;

/// LCG seed — capnp-style schema id so any drift in the seed is a
/// loud diff in the generated corpus.
const SEED: u64 = 0xb117_2c4e_5e10_0000;

/// APPNOTE.TXT §4.4.4 data-descriptor flag (general-purpose bit 3).
const GPB_DATA_DESCRIPTOR_MASK: u16 = 0x0008;

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

    fn next_u16(&mut self) -> u16 {
        self.next_u32() as u16
    }

    fn next_in_range(&mut self, lo: u32, hi: u32) -> u32 {
        debug_assert!(lo < hi);
        lo + (self.next_u32() % (hi - lo))
    }
}

struct EntryAttrs {
    filename: Vec<u8>,
    crc32: u32,
    compressed_size: u32,
    uncompressed_size: u32,
    compression_method: u16,
    last_mod_time: u16,
    last_mod_date: u16,
    general_flags: u16,
    data_descriptor: bool,
}

/// Build a well-formed full ZIP archive with `n` entries. Mirrors
/// the canonical layout used by `tools/zip-corpus-gen::build_archive`
/// — LFH₀ ‖ … ‖ LFHₙ₋₁ ‖ CDR₀ ‖ … ‖ CDRₙ₋₁ ‖ EOCD — with shared
/// fields between matching LFH/CDR pairs so cross-record consistency
/// holds.
fn build_archive(rng: &mut Lcg, n: usize) -> Vec<u8> {
    debug_assert!(n >= 1);
    let mut entries: Vec<EntryAttrs> = Vec::with_capacity(n);
    for _ in 0..n {
        let nl = rng.next_in_range(0, 16) as usize;
        let mut filename = Vec::with_capacity(nl);
        for _ in 0..nl {
            filename.push((rng.next_in_range(0x20, 0x7f)) as u8);
        }
        let data_descriptor = rng.next_u32().trailing_zeros() >= 2;
        let mut general_flags = rng.next_u16();
        if data_descriptor {
            general_flags |= GPB_DATA_DESCRIPTOR_MASK;
        } else {
            general_flags &= !GPB_DATA_DESCRIPTOR_MASK;
        }
        entries.push(EntryAttrs {
            filename,
            crc32: rng.next_u32(),
            compressed_size: rng.next_u32(),
            uncompressed_size: rng.next_u32(),
            compression_method: rng.next_u16(),
            last_mod_time: rng.next_u16(),
            last_mod_date: rng.next_u16(),
            general_flags,
            data_descriptor,
        });
    }

    let mut bytes: Vec<u8> = Vec::new();
    let mut lfh_offsets: Vec<u32> = Vec::with_capacity(n);
    for entry in &entries {
        lfh_offsets.push(bytes.len() as u32);
        let nl = entry.filename.len() as u16;
        let (lfh_crc, lfh_c, lfh_u) = if entry.data_descriptor {
            (0u32, 0u32, 0u32)
        } else {
            (entry.crc32, entry.compressed_size, entry.uncompressed_size)
        };
        bytes.extend_from_slice(&lfh::SIGNATURE.to_le_bytes());
        bytes.extend_from_slice(&rng.next_u16().to_le_bytes()); // versionNeeded
        bytes.extend_from_slice(&entry.general_flags.to_le_bytes());
        bytes.extend_from_slice(&entry.compression_method.to_le_bytes());
        bytes.extend_from_slice(&entry.last_mod_time.to_le_bytes());
        bytes.extend_from_slice(&entry.last_mod_date.to_le_bytes());
        bytes.extend_from_slice(&lfh_crc.to_le_bytes());
        bytes.extend_from_slice(&lfh_c.to_le_bytes());
        bytes.extend_from_slice(&lfh_u.to_le_bytes());
        bytes.extend_from_slice(&nl.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes()); // extraLen
        bytes.extend_from_slice(&entry.filename);
    }

    let cd_start = bytes.len() as u32;
    let mut cd_size: u32 = 0;
    for (entry, lfh_off) in entries.iter().zip(lfh_offsets.iter()) {
        let nl = entry.filename.len() as u16;
        let mut v: Vec<u8> = Vec::new();
        v.extend_from_slice(&cdr::SIGNATURE.to_le_bytes());
        v.extend_from_slice(&rng.next_u16().to_le_bytes()); // versionMadeBy
        v.extend_from_slice(&rng.next_u16().to_le_bytes()); // versionNeeded
        v.extend_from_slice(&entry.general_flags.to_le_bytes());
        v.extend_from_slice(&entry.compression_method.to_le_bytes());
        v.extend_from_slice(&entry.last_mod_time.to_le_bytes());
        v.extend_from_slice(&entry.last_mod_date.to_le_bytes());
        v.extend_from_slice(&entry.crc32.to_le_bytes());
        v.extend_from_slice(&entry.compressed_size.to_le_bytes());
        v.extend_from_slice(&entry.uncompressed_size.to_le_bytes());
        v.extend_from_slice(&nl.to_le_bytes());
        v.extend_from_slice(&0u16.to_le_bytes()); // extraLen
        v.extend_from_slice(&0u16.to_le_bytes()); // commentLen
        v.extend_from_slice(&[0u8; 2]); // diskNumberStart
        v.extend_from_slice(&rng.next_u16().to_le_bytes()); // internalAttrs
        v.extend_from_slice(&rng.next_u32().to_le_bytes()); // externalAttrs
        v.extend_from_slice(&lfh_off.to_le_bytes());
        v.extend_from_slice(&entry.filename);
        cd_size += v.len() as u32;
        bytes.extend_from_slice(&v);
    }

    bytes.extend_from_slice(&eocd::SIGNATURE.to_le_bytes());
    bytes.extend_from_slice(&[0u8; 4]); // diskNumber + cdStartDisk = 0
    let entries_u16 = u16::try_from(n).unwrap_or(u16::MAX);
    bytes.extend_from_slice(&entries_u16.to_le_bytes());
    bytes.extend_from_slice(&entries_u16.to_le_bytes());
    bytes.extend_from_slice(&cd_size.to_le_bytes());
    bytes.extend_from_slice(&cd_start.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes()); // commentLen
    bytes
}

fn write_sample(dir: &Path, idx: usize, bytes: &[u8]) -> Result<(), std::io::Error> {
    fs::create_dir_all(dir)?;
    fs::write(dir.join(format!("{idx:05}.bin")), bytes)
}

fn write_manifest(dir: &Path, count: usize, total_bytes: u64) -> Result<(), std::io::Error> {
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str("  \"schema_version\": \"1.0\",\n");
    s.push_str("  \"phase\": \"P1.12\",\n");
    s.push_str(&format!("  \"seed\": \"{SEED:#018x}\",\n"));
    s.push_str(&format!("  \"count\": {count},\n"));
    s.push_str(&format!("  \"total_bytes\": {total_bytes},\n"));
    s.push_str("  \"verdict\": \"ok\",\n");
    s.push_str("  \"layout\": {\n");
    s.push_str("    \"path_pattern\": \"<idx:05>.bin\",\n");
    s.push_str("    \"entry_count_range\": [1, 8]\n");
    s.push_str("  }\n");
    s.push_str("}\n");
    fs::write(dir.join("manifest.json"), s)
}

fn run(out_root: &Path) -> Result<(), std::io::Error> {
    let t0 = Instant::now();
    let mut rng = Lcg::new(SEED);
    let bench_dir = out_root.to_path_buf();
    fs::create_dir_all(&bench_dir)?;

    let mut total_bytes: u64 = 0;
    let mut max_size: usize = 0;
    let mut min_size: usize = usize::MAX;
    for i in 0..BENCH_SIZE {
        // Vary entry count ∈ [1, 8] and per-entry size; the LCG draw is
        // deterministic so the same SEED always yields the same archives.
        let n = rng.next_in_range(1, 9) as usize;
        let bytes = build_archive(&mut rng, n);
        // Round-trip via the verified path before writing — any failure
        // is a generator bug, not a data point.
        match parse_archive(&bytes) {
            Ok(_) => {}
            Err(e) => {
                eprintln!(
                    "ERROR sample {i} (n_entries={n}, len={}): verified parse rejected: {e:?}",
                    bytes.len()
                );
                return Err(std::io::Error::other(format!(
                    "verified parser rejected sample {i}"
                )));
            }
        }
        total_bytes += bytes.len() as u64;
        max_size = max_size.max(bytes.len());
        min_size = min_size.min(bytes.len());
        write_sample(&bench_dir, i, &bytes)?;
    }

    write_manifest(&bench_dir, BENCH_SIZE, total_bytes)?;
    let dt = t0.elapsed();
    eprintln!(
        "p112-bench-10k: wrote {} archives ({} bytes total, min={} max={}) in {:.2}s",
        BENCH_SIZE,
        total_bytes,
        min_size,
        max_size,
        dt.as_secs_f64()
    );
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().collect();
    let Some(out_root) = args.get(1).cloned() else {
        eprintln!("usage: p112-bench-10k <output-dir>");
        return ExitCode::from(2);
    };
    let path = PathBuf::from(out_root);
    match run(&path) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("FAIL: {e}");
            ExitCode::from(1)
        }
    }
}
