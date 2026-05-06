#![no_main]
//! P1.9 §V item 9 — differential libFuzzer harness for the LFH
//! parser. Compares `axiom_zip_ref::lfh::parse_lfh` (hand-Rust)
//! against `axiom_l0_zip_lfh_extracted::parse_lfh` (auto-extracted
//! from Lean) and asserts they agree on every input.
//!
//! Run protocol:
//!
//!   cd crates/axiom-zip-ref
//!   cargo +nightly fuzz run fuzz_parse_lfh_differential -- -max_total_time=3600
//!
//! Pass condition: zero divergences across the corpus-guided
//! libFuzzer run. A divergence means either:
//!
//!   - the extractor introduced a semantic bug, OR
//!   - the hand-Rust parser drifted from the Lean reference.
//!
//! Both fail the gate.

use libfuzzer_sys::fuzz_target;

use axiom_l0_zip_lfh_extracted::parse_lfh as extracted_parse_lfh;
use axiom_zip_ref::lfh::parse_lfh as verified_parse_lfh;

fuzz_target!(|data: &[u8]| {
    let v = verified_parse_lfh(data);
    let e = extracted_parse_lfh(data);

    match (&v, &e) {
        (Ok((lv, nv)), Ok((le, ne))) => {
            assert_eq!(lv.version_needed, le.version_needed, "version_needed mismatch");
            assert_eq!(lv.general_flags, le.general_flags, "general_flags mismatch");
            assert_eq!(lv.compression_method, le.compression_method, "compression_method mismatch");
            assert_eq!(lv.last_mod_time, le.last_mod_time, "last_mod_time mismatch");
            assert_eq!(lv.last_mod_date, le.last_mod_date, "last_mod_date mismatch");
            assert_eq!(lv.crc32, le.crc32, "crc32 mismatch");
            assert_eq!(lv.compressed_size, le.compressed_size, "compressed_size mismatch");
            assert_eq!(lv.uncompressed_size, le.uncompressed_size, "uncompressed_size mismatch");
            assert_eq!(lv.file_name, le.file_name, "file_name mismatch");
            assert_eq!(lv.extra_field, le.extra_field, "extra_field mismatch");
            assert_eq!(nv, ne, "consumed mismatch");
        }
        (Err(ev), Err(ee)) => {
            // Map both error variants to their canonical tag and
            // compare. The extracted enum and the hand-Rust enum
            // are nominally distinct types but structurally
            // identical.
            let tv = ev.tag();
            let te = match ee {
                axiom_l0_zip_lfh_extracted::ParseError::ShortHeader => 1,
                axiom_l0_zip_lfh_extracted::ParseError::BadSignature => 2,
                axiom_l0_zip_lfh_extracted::ParseError::ShortName => 3,
                axiom_l0_zip_lfh_extracted::ParseError::ShortExtra => 4,
            };
            assert_eq!(tv, te, "error tag mismatch: hand={tv} extracted={te}");
        }
        (Ok(_), Err(_)) | (Err(_), Ok(_)) => {
            panic!("verified ↔ extracted result-shape divergence: hand={v:?} extracted={e:?}");
        }
    }
});
