#![no_main]
//! libFuzzer target for `axiom_l1_rs::commit_chain::parse_with_commit_chain`.
//!
//! Properties asserted on every input:
//!
//!   1. **No panics.** `parse_with_commit_chain` must return either
//!      `Ok((events, chain))` or `Err(_)` for arbitrary byte
//!      sequences. Any panic is a bug.
//!   2. **Determinism.** If the chain succeeds on input `data`,
//!      running it again on the same bytes produces a byte-identical
//!      Merkle root + leaf list.
//!   3. **Acceptance parity with bare streaming.** If the bare
//!      streaming parser (`ApkParser::next_event` until None or Err)
//!      accepts an input, `parse_with_commit_chain` must also accept
//!      it. The chain wrapper must not narrow the parser's accept set.
//!
//! Run protocol:
//!
//!   nix develop --command bash -c \
//!     "cd crates/axiom-l1-rs && \
//!      cargo +nightly fuzz run fuzz_commit_chain -- -max_total_time=60"
//!
//! libFuzzer mutates `data` in a coverage-guided loop. The harness
//! exits non-zero on any property violation.

use libfuzzer_sys::fuzz_target;

use axiom_l1_rs::commit_chain::parse_with_commit_chain;
use axiom_l1_rs::stream::ApkParser;

fuzz_target!(|data: &[u8]| {
    // Property 3: acceptance parity. Run the bare parser first.
    let bare_accepts = bare_streaming_accepts(data);

    // Properties 1 + 2: chain must not panic, and must be
    // deterministic on accepted inputs.
    match parse_with_commit_chain(data) {
        Ok((events_a, chain_a)) => {
            // Re-run on identical input.
            let (events_b, chain_b) = parse_with_commit_chain(data)
                .expect("chain accepted then rejected the same bytes");
            assert_eq!(
                chain_a.root, chain_b.root,
                "merkle root non-deterministic (input len {})",
                data.len()
            );
            assert_eq!(
                chain_a.leaves.len(),
                chain_b.leaves.len(),
                "leaf count non-deterministic"
            );
            for (i, (la, lb)) in chain_a.leaves.iter().zip(&chain_b.leaves).enumerate() {
                assert_eq!(la.hash, lb.hash, "leaf #{i} hash non-deterministic");
                assert_eq!(la.tag, lb.tag, "leaf #{i} tag non-deterministic");
                assert_eq!(la.offset, lb.offset, "leaf #{i} offset non-deterministic");
                assert_eq!(la.length, lb.length, "leaf #{i} length non-deterministic");
            }
            // Acceptance parity: chain accepts → bare must too
            // (bare is a strict subset of chain's work).
            assert!(
                bare_accepts,
                "chain accepted input that bare parser rejected"
            );
            // Sanity: events match between runs.
            assert_eq!(events_a.len(), events_b.len(), "event count");
        }
        Err(_) => {
            // Chain rejected. Bare parser may also reject (fine).
            // What we forbid is the *opposite* — chain rejecting
            // an input bare accepts — covered by property 3 below.
            if bare_accepts {
                panic!(
                    "chain rejected input that bare parser accepted (input len {})",
                    data.len()
                );
            }
        }
    }
});

fn bare_streaming_accepts(data: &[u8]) -> bool {
    let mut parser = ApkParser::from_reader(data);
    loop {
        match parser.next_event() {
            Ok(Some(_)) => {}
            Ok(None) => return true,
            Err(_) => return false,
        }
    }
}
