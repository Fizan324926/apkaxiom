#![no_main]
//! libFuzzer target for `axiom_sigblock::scheme::parse_v3`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let a = axiom_sigblock::scheme::parse_v3(data);
    let b = axiom_sigblock::scheme::parse_v3(data);
    assert_eq!(a.is_ok(), b.is_ok());
});
