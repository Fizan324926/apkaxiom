// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
//
// P1.9 §V item 8 — coverage-bounded TV gate.
//
// Drives `axiom_zip_ref::lfh::parse_lfh` against every input in
// the LFH corpus (1500 files, ~1499 non-empty), so that
// `cargo llvm-cov` can measure line/branch/region coverage on
// the parser. The test alone doesn't enforce a coverage threshold
// — that's `make coverage-gate`'s job, which extracts the metric
// from llvm-cov's JSON output and asserts it ≥ 95%.

#![allow(
    clippy::needless_range_loop,
    clippy::redundant_closure_for_method_calls,
    clippy::similar_names
)]

use std::path::PathBuf;

use axiom_zip_ref::lfh::parse_lfh;

fn corpus_dirs() -> Vec<PathBuf> {
    vec![
        PathBuf::from("../../corpus/zip/lfh-valid"),
        PathBuf::from("../../corpus/zip/lfh-adversarial"),
    ]
}

#[test]
fn drive_corpus_for_coverage() {
    let mut driven = 0u32;
    for dir in corpus_dirs() {
        if !dir.exists() {
            continue;
        }
        let mut entries: Vec<_> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("read_dir {dir:?}: {e}"))
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("bin"))
            .collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let bytes = std::fs::read(entry.path()).unwrap_or_default();
            // We do not care about the result — we care that the
            // parser was *executed*. Any panic is a real bug.
            let _ = parse_lfh(&bytes);
            driven += 1;
        }
    }
    assert!(
        driven >= 1000,
        "corpus too small for coverage gate ({driven} < 1000)"
    );
}
