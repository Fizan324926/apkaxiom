#![no_main]
//! libFuzzer target for `axiom_sigblock::locate`.
//!
//! Property: never panics on arbitrary byte sequences. The
//! locator either returns `Ok(Some(_))` (well-formed signing
//! block), `Ok(None)` (no block), or `Err(_)` (parse failure).
//! Any panic is a bug.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = axiom_sigblock::locate(data);
});
