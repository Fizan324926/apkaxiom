#![no_main]
//! libFuzzer target for `axiom_zip_ref::archive::parse_archive`. See
//! `fuzz_parse_lfh.rs` for the run protocol.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = axiom_zip_ref::archive::parse_archive(data);
});
