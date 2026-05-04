#![no_main]
//! libFuzzer target for `axiom_zip_ref::lfh::parse_lfh`.
//!
//! Coverage-guided fuzzing entry point: libFuzzer mutates `data` and
//! invokes `parse_lfh` on every iteration. The pass condition is
//! "no panic" — `parse_lfh` is `forbid(unsafe_code)` and the
//! `ParseError` enum closes the failure space, so any panic indicates
//! a parser bug. The differential harness covers verdict equality;
//! this target catches the residual class libFuzzer is good at:
//! coverage-driven divergence into untested branches.
//!
//! Run protocol:
//!
//!   nix develop --command bash -c \
//!     "cd crates/axiom-zip-ref && cargo +nightly fuzz run fuzz_parse_lfh -- -max_total_time=60"

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = axiom_zip_ref::lfh::parse_lfh(data);
});
