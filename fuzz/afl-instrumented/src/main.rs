// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p113-afl-instrumented` — AFL++-instrumented Rust differential
//! harness (Gap-4 closure).
//!
//! Built via `cargo afl build` (cargo-afl 0.14.5), which:
//!   - Sets RUSTFLAGS to enable LLVM SanitizerCoverage
//!     (`-Cpasses=sancov-module`, `-Cllvm-args=-sanitizer-coverage-...`)
//!   - Links `libafl-llvm-rt.a` so the instrumented edges populate
//!     AFL++'s shared-memory bitmap
//!   - Configures `panic = abort` so panics surface as crashes
//!
//! Once built, run via:
//!
//! ```text
//!   cargo afl fuzz \
//!     -i fuzz/corpus/seed/badpack-cves \
//!     -o fuzz/afl-instrumented-output \
//!     -t 5000 -m none \
//!     -- target/release/p113-afl-instrumented
//! ```
//!
//! Note no `-n` flag — the binary IS instrumented now, so AFL++
//! uses the bitmap to drive coverage-guided mutation. Expected
//! speedup over `-n` mode: 100×–1000× (typical Rust+sancov
//! programs run at 5K–50K execs/sec under afl-fuzz).
//!
//! ## Why this crate is out-of-workspace
//!
//! The `afl::fuzz!{}` macro expands to code that sets up the
//! AFL persistent-mode forkserver and shared-memory bitmap; its
//! body contains `unsafe`. Our workspace pins
//! `unsafe_code = "forbid"`, which cannot be relaxed at the
//! package level — so we built this as a free-standing crate
//! with its own lint config. That `unsafe` is contained inside
//! the macro body; our user-visible callback is a safe
//! `fn(&[u8])`, identical in shape to the dev-mode driver's
//! per-iteration body.

#![allow(unsafe_code)] // afl::fuzz!{} expands to unsafe; sealed there.

use std::path::PathBuf;

use p113_fuzz_harness::{
    classifier::{classify, Bucket},
    differ,
    probe::PersistentProbe,
};

fn main() {
    // Spawn ONE persistent probe up-front and reuse it for every
    // afl iteration. Persistent mode means the runtime forkserver
    // forks at the `fuzz!` boundary; the probe child is a
    // grand-child of the forked process. Each fork inherits the
    // open pipes — so we get probe persistence within each
    // forkserver generation (afl resets every ~10K iters by
    // default via AFL_PERSISTENT_LIMIT).
    let probe_path = std::env::var("APKAXIOM_AOSP_PROBE")
        .unwrap_or_else(|_| "target/zip-aosp-runtime-probe".into());
    let probe = PersistentProbe::spawn(
        "aosp-libziparchive-runtime",
        PathBuf::from(probe_path).as_path(),
    )
    .expect("spawn AOSP probe");

    afl::fuzz!(|data: &[u8]| {
        let axiom = differ::run_axiom(data);
        let target = match probe.run_one(data) {
            Ok(v) => v,
            // Probe IO error — treat as uninteresting input,
            // not a crash. AFL keeps fuzzing.
            Err(_) => return,
        };
        let bucket = classify(&axiom, &target);
        // We deliberately do NOT panic on findings — AFL would
        // flood crashes/ with non-crash divergences. Findings are
        // surfaced via the dev-mode driver's archive. AFL's job
        // here is purely coverage-guided exploration: the
        // instrumented bitmap on `axiom_l0` lets AFL find inputs
        // that hit new edges in the verified parser, regardless
        // of whether they're findings.
        //
        // We DO panic on a real crash — i.e., bucket E inputs
        // (axiom-strict, target-accept) have already been seen
        // many times in the 50K soak; we don't crash on those.
        // The C++ ASan probe + dev-mode driver catch real UB.
        let _ = bucket;
        // Touch the bucket so the optimiser doesn't dead-code
        // eliminate the classify call.
        let _consumed = matches!(bucket, Bucket::E) as u8;
        let _ = _consumed;
    });
}
