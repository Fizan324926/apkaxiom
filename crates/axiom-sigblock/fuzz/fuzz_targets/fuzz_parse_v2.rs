#![no_main]
//! libFuzzer target for `axiom_sigblock::scheme::parse_v2`.
//! Property: total — every byte slice produces either Ok or Err
//! without panicking; deterministic across two runs.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let a = axiom_sigblock::scheme::parse_v2(data);
    let b = axiom_sigblock::scheme::parse_v2(data);
    // Determinism: two runs on the same bytes give the same Result-shape.
    assert_eq!(a.is_ok(), b.is_ok());
});
