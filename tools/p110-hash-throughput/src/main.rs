// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p110-hash-throughput` — P1.10 §10 row 2 hash-throughput gate.
//!
//! Measures BLAKE3 single-core throughput on a 256 MiB random
//! buffer (in-memory, no I/O). The spec gate is **≥ 1.5 GB/s**.
//! Running on dev-shell hardware; reference EPYC 9354 reports
//! 4-6 GB/s for `blake3 = 1.5` `pure` (no SIMD); spec EPYC numbers
//! assume the SIMD-enabled `blake3` build, which is gated behind
//! `RUSTFLAGS="-C target-cpu=native"` plus the `blake3` crate's
//! default-features.
//!
//! Output:
//!
//! ```text
//! p110-hash-throughput: 256 MiB in 0.045s — 5.6 GB/s
//! ```

#![forbid(unsafe_code)]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_lossless,
    clippy::cast_possible_truncation
)]

use std::time::Instant;

use axiom_blake3_hacl::{Blake3, Hasher};

const PAYLOAD_BYTES: usize = 256 * 1024 * 1024;
const GATE_GB_PER_SEC_DEFAULT: f64 = 1.5;

fn parse_gate() -> f64 {
    std::env::args()
        .skip_while(|a| a != "--gate")
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(GATE_GB_PER_SEC_DEFAULT)
}

fn main() {
    let gate = parse_gate();
    // LCG-seeded 256 MiB buffer.
    let mut payload = vec![0u8; PAYLOAD_BYTES];
    let mut s: u64 = 0x1357_9bdf_2468_ace0;
    for chunk in payload.chunks_mut(8) {
        s = s
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let bytes = s.to_le_bytes();
        for (i, b) in chunk.iter_mut().enumerate() {
            *b = bytes[i];
        }
    }
    // Warm.
    let _ = Blake3::hash_oneshot(&payload[..1024]);
    // Measure.
    let runs = 5usize;
    let mut elapsed_total = std::time::Duration::ZERO;
    for _ in 0..runs {
        let start = Instant::now();
        let h = Blake3::hash_oneshot(&payload);
        elapsed_total += start.elapsed();
        std::hint::black_box(h);
    }
    let elapsed_avg = elapsed_total / runs as u32;
    let secs = elapsed_avg.as_secs_f64();
    let bytes = PAYLOAD_BYTES as f64;
    let gb_per_sec = (bytes / 1e9) / secs;
    println!(
        "p110-hash-throughput: {:.0} MiB in {:.4}s avg over {runs} runs — {:.2} GB/s",
        bytes / (1024.0 * 1024.0),
        secs,
        gb_per_sec,
    );
    println!("  gate: ≥ {gate:.2} GB/s");
    if gb_per_sec >= gate {
        println!("  PASS");
        std::process::exit(0);
    }
    eprintln!(
        "  FAIL: {gb_per_sec:.2} GB/s < {gate:.2} GB/s — switch to SIMD blake3 (default-features=true) or run on reference HW"
    );
    std::process::exit(1);
}
