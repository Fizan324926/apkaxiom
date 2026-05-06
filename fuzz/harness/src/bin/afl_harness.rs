// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p113-afl-harness` — AFL++ persistent-mode driver.
//!
//! Compiled with `afl-clang-fast++`-equivalent instrumentation
//! and run via `afl-fuzz -i seeds -o findings -- p113-afl-harness`.
//! Each iteration: `afl-fuzz` writes one input to `@@`, we read
//! it, run the differential, return non-zero exit if axiom-l0
//! and the AOSP probe disagree (that's how AFL++ identifies
//! "interesting" inputs and adds them to the queue).
//!
//! AFL++ provides:
//!   - **coverage-guided** mutation (Gap-14): the queue grows
//!     when a new input hits a new edge in the verified parser
//!   - **bit-flip dictionaries** + havoc + splice — far better
//!     mutator distribution than our LCG
//!   - **crash detection** via signal handlers — surfaces real
//!     C++ UB in the libziparchive probe
//!
//! Run protocol (any host with afl-fuzz installed; no KVM):
//!
//! ```text
//!   make p113-afl-harness        # compile this binary
//!   afl-fuzz -i fuzz/corpus/seed/badpack-cves \
//!            -o fuzz/afl-output \
//!            -t 5000 -m none \
//!            -- target/release/p113-afl-harness @@
//! ```
//!
//! Note: AFL++ persistent mode requires `__AFL_LOOP` which is a
//! C macro; without afl-clang-fast++ Rust integration we run in
//! "fork mode" — each iteration is a fresh fork of this binary.
//! That's slower than persistent (~1K iter/s vs ~100K) but works
//! out of the box on stable rustc + afl-fuzz from the system
//! package.

#![forbid(unsafe_code)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::uninlined_format_args,
    clippy::items_after_statements
)]

use std::path::PathBuf;

use p113_fuzz_harness::{classifier::classify, differ, probe::PersistentProbe};

fn main() -> std::io::Result<()> {
    // AFL++ default: pass the input file path as argv[1]
    let argv: Vec<String> = std::env::args().collect();
    let input_path: PathBuf = if argv.len() >= 2 {
        PathBuf::from(&argv[1])
    } else {
        // Fallback: read stdin (afl-fuzz also supports this).
        let mut buf = Vec::new();
        use std::io::Read as _;
        std::io::stdin().read_to_end(&mut buf)?;
        let bytes = buf;
        return run_one(&bytes);
    };
    let bytes = std::fs::read(&input_path)?;
    run_one(&bytes)
}

fn run_one(input: &[u8]) -> std::io::Result<()> {
    let probe_path = std::env::var("APKAXIOM_AOSP_PROBE")
        .unwrap_or_else(|_| "target/zip-aosp-runtime-probe".into());
    let probe = PersistentProbe::spawn(
        "aosp-libziparchive-runtime",
        PathBuf::from(probe_path).as_path(),
    )?;

    let axiom = differ::run_axiom(input);
    let target = probe.run_one(input)?;
    let bucket = classify(&axiom, &target);

    // AFL++ semantics:
    //   - exit(0)   = uninteresting (just continues)
    //   - non-zero  = "interesting" — input added to queue
    //   - signal/crash = crash — saved to crashes/
    //
    // We exit non-zero on D + E + C buckets. AFL++ then queues
    // these inputs and tries to find more inputs that hit similar
    // coverage edges, dramatically increasing the differential
    // surface area covered per CPU-second.
    if bucket.is_finding() {
        std::process::exit(1);
    }
    Ok(())
}
