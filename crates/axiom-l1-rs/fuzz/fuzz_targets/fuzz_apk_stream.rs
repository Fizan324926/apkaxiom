#![no_main]
//! libFuzzer target for `axiom_l1_rs::ApkParser::from_reader`.
//!
//! Coverage-guided fuzzing entry point: libFuzzer mutates `data`
//! and feeds it to the streaming parser. Pass condition: no
//! panics; the parser must terminate (return Ok(None) or Err) for
//! every input.
//!
//! Run protocol:
//!
//!   nix develop --command bash -c \
//!     "cd crates/axiom-l1-rs && \
//!      cargo +nightly fuzz run fuzz_apk_stream -- -max_total_time=60"
//!
//! Memory-growth guard: the fuzz harness asserts the parser's
//! internal buffer never exceeds the architectural cap
//! (`buf_capacity = MAX_HEADER_PAYLOAD + DEFAULT_CHUNK_SIZE +
//! LFH_FIXED_SIZE`). Any allocation past that cap means the
//! state machine's compaction strategy regressed.

use libfuzzer_sys::fuzz_target;

use axiom_l1_rs::{ApkParser, DEFAULT_CHUNK_SIZE, MAX_HEADER_PAYLOAD};

fuzz_target!(|data: &[u8]| {
    let mut parser = ApkParser::from_reader(data);
    let cap_bound = MAX_HEADER_PAYLOAD as usize + DEFAULT_CHUNK_SIZE + 64;
    loop {
        match parser.next_event() {
            Ok(Some(_ev)) => {
                // Memory-growth check.
                assert!(
                    parser.buf_capacity() <= cap_bound,
                    "fuzz: buffer grew to {} (bound {cap_bound})",
                    parser.buf_capacity()
                );
            }
            Ok(None) => break,
            Err(_) => break,
        }
    }
});
