#![no_main]
//! libFuzzer target for the P1.8 type-state pipeline
//! (`Apk<Unverified>::from_reader → verify_v* → parse_v*`).
//!
//! P1.7 fuzzes the streaming parser
//! ([`fuzz_apk_stream`]). This target raises the bar to the
//! P1.8 wrapper: if `from_reader` succeeds, the harness
//! exhaustively exercises every transition that's typed-allowed
//! from that state, then walks every gated accessor. Pass
//! condition: no panics under arbitrary input — internal
//! invariants on `signature_block()` / `manifest()` /
//! `resources()` must hold for every reachable state.
//!
//! Run protocol:
//!
//!   nix develop --command bash -c \
//!     "cd crates/axiom-l1-rs && \
//!      cargo +nightly fuzz run fuzz_apk_typestate -- -max_total_time=60"
//!
//! Adversarial inputs of interest: oversized DEFLATE-claimed
//! lengths (rejected by [`MAX_INFLATE_BYTES`] bound), corrupted
//! META-INF/<key>.RSA bodies, malformed AXML / ARSC magics,
//! variant-tag mismatch on parse_v* (variant_mismatch_rejected_at_runtime
//! covers that path).

use libfuzzer_sys::fuzz_target;

use axiom_l1_rs::{Apk, Unverified};

fuzz_target!(|data: &[u8]| {
    let apk = match Apk::<Unverified>::from_reader(data) {
        Ok(a) => a,
        Err(_) => return,
    };
    // Walk every entry — establishes that `entries()` is safe
    // for any fuzz-derived state.
    for e in apk.entries() {
        let _ = (
            e.file_name.len(),
            e.compression_method,
            e.compressed_size,
            e.uncompressed_size,
            e.crc32,
            e.general_flags,
        );
    }
    let _ = apk.state_name();

    // Each transition is independent — clone into three branches
    // so a panic in one doesn't shadow the others.
    let v2 = apk.clone().verify_v2();
    let v3 = apk.clone().verify_v3();
    let v4 = apk.verify_v4();

    if let Ok(verified) = v2 {
        // signature_block accessor must not panic under any
        // input the constructor accepted.
        let _ = verified.signature_block();
        let _ = verified.entries().len();
        if let Ok(parsed) = verified.parse_v2() {
            let _ = parsed.manifest();
            let _ = parsed.resources();
            let _ = parsed.signature_block();
            assert_eq!(parsed.signing_variant_tag(), 2);
        }
    }
    if let Ok(verified) = v3 {
        let _ = verified.signature_block();
        if let Ok(parsed) = verified.parse_v3() {
            let _ = parsed.manifest();
            let _ = parsed.resources();
            assert_eq!(parsed.signing_variant_tag(), 3);
        }
    }
    if let Ok(verified) = v4 {
        let _ = verified.signature_block();
        if let Ok(parsed) = verified.parse_v4() {
            let _ = parsed.manifest();
            let _ = parsed.resources();
            assert_eq!(parsed.signing_variant_tag(), 4);
        }
    }
});
