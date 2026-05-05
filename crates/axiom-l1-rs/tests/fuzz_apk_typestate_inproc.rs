// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
//
// P1.8 §F-4 — in-process mutation-fuzz of the type-state.

#![allow(
    clippy::cast_possible_truncation,
    clippy::single_match_else,
    clippy::needless_pass_by_value,
    clippy::unreadable_literal
)]
// pipeline. Mirrors the libFuzzer target
// `crates/axiom-l1-rs/fuzz/fuzz_targets/fuzz_apk_typestate.rs`
// but runs as a stock `cargo test` integration test (no nightly
// toolchain required, no libFuzzer linkage). Pass condition:
// **no panics across 10 000 mutated inputs**.
//
// The mutation stream:
//
//   - takes the F-Droid Privileged Extension fixture as the seed,
//   - produces 10 000 mutants by flipping random bytes (LCG-seeded
//     so the run is reproducible),
//   - feeds each mutant through the full pipeline (`from_reader →
//     verify_v{2,3,4} → parse_v{2,3,4}`),
//   - asserts the wrapper either returns `Ok(_)` or `Err(_)` —
//     never panics.
//
// The LCG seed (`0xa9c1_d4b1_f7e2_3d51`) matches the P1.5/P1.6
// corpus seed for cross-phase reproducibility.

use axiom_l1_rs::{Apk, Unverified};

const FIXTURE_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/fdroid-privileged-2050.apk"
);

const LCG_SEED: u64 = 0xa9c1_d4b1_f7e2_3d51;
const ITERATIONS: usize = 10_000;
/// Per-mutant flip count (bytes). Empirically a small flip count
/// generates the most "interestingly-broken" archives — most
/// mutations either keep the structure intact (covered already by
/// the green-path test) or trash the LFH signature (rejected at
/// the streaming layer). 4 flips lands in the middle.
const FLIPS_PER_MUTANT: usize = 4;

fn seed_bytes() -> Vec<u8> {
    std::fs::read(FIXTURE_PATH).expect("fixture present")
}

#[derive(Debug, Clone, Copy)]
struct Lcg(u64);
impl Lcg {
    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }
}

fn mutate(buf: &mut [u8], rng: &mut Lcg) {
    for _ in 0..FLIPS_PER_MUTANT {
        let idx = (rng.next_u64() as usize) % buf.len();
        let xor = (rng.next_u64() & 0xff) as u8;
        buf[idx] ^= xor;
    }
}

#[test]
fn typestate_fuzz_10k_mutations_no_panic() {
    let seed = seed_bytes();
    let mut rng = Lcg(LCG_SEED);

    let mut accepted = 0u64;
    let mut rejected = 0u64;
    let mut full_pipeline = 0u64;
    for _ in 0..ITERATIONS {
        let mut buf = seed.clone();
        mutate(&mut buf, &mut rng);

        // Step 1: structural parse + body capture.
        let apk = match Apk::<Unverified>::from_reader(buf.as_slice()) {
            Ok(a) => {
                accepted += 1;
                a
            }
            Err(_) => {
                rejected += 1;
                continue;
            }
        };

        // Step 2: every legitimate transition. We exercise all
        // three verify variants so libFuzzer-equivalent coverage
        // stays balanced.
        for verify_fn in [
            Apk::<Unverified>::verify_v2,
            Apk::<Unverified>::verify_v3,
            Apk::<Unverified>::verify_v4,
        ] {
            let Ok(verified) = verify_fn(apk.clone()) else {
                continue;
            };
            let _ = verified.entries().len();
            let _ = verified.signature_block();
            // parse_v* — choose the matching variant so the
            // runtime cross-bind passes; we want to reach
            // FullyParsed<V> and walk its accessors.
            let parsed = match verified.signature_block().variant_tag {
                2 => verified.parse_v2().map(|p| {
                    let _ = p.manifest();
                    let _ = p.resources();
                    let _ = p.signing_variant_tag();
                    let _ = p.signature_block();
                }),
                3 => verified.parse_v3().map(|p| {
                    let _ = p.manifest();
                    let _ = p.resources();
                    let _ = p.signing_variant_tag();
                }),
                4 => verified.parse_v4().map(|p| {
                    let _ = p.manifest();
                    let _ = p.resources();
                    let _ = p.signing_variant_tag();
                }),
                _ => unreachable!("variant tag came from our own SigVariant set"),
            };
            if parsed.is_ok() {
                full_pipeline += 1;
            }
        }
    }

    eprintln!(
        "typestate-fuzz: iters={ITERATIONS} accepted={accepted} rejected={rejected} \
         full-pipeline-success={full_pipeline}"
    );
    // Sanity: at least *some* mutants survived structural parse,
    // otherwise the fuzz isn't actually exercising the wrapper.
    assert!(
        accepted >= 100,
        "fuzz: too few mutants reached the wrapper ({accepted}/{ITERATIONS})"
    );
}
