// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p110-tamper-fuzz` — P1.10 §B item 5 (HARD).
//!
//! Tamper-detection gate. For each of the four real F-Droid APK
//! fixtures, drive `parse_with_commit_chain` on:
//!
//!   1. The original bytes — record the canonical Merkle root.
//!   2. `--runs` (default 10 000) deterministic single-bit-flip
//!      mutations at LCG-chosen offsets.
//!
//! For every mutation:
//!
//!   - **kill = parse error** — the parser refused to consume
//!     the tampered input. Counts as a tamper-detection win
//!     (downstream consumer never sees the bad bytes).
//!   - **kill = root changed** — parser accepted the tampered
//!     input but the Merkle root differs from the canonical one.
//!     Downstream verifier comparing roots detects the tamper.
//!   - **miss = root identical** — parser accepted AND root
//!     unchanged. The mutation is invisible to the chain.
//!     Reported per-component for review.
//!
//! Every mutated byte is classified by which structural region
//! it lands in — LFH-header / LFH-body / signing-block / CDR /
//! EOCD / EOCD-comment — so we can report tamper kill rate per
//! component. The kill-rate gate is ≥ 99 % per non-comment
//! component (default; tunable via `--gate`).
//!
//! Misses are typically:
//!
//!   - Inside the EOCD comment region (which we deliberately do
//!     not commit; ZIP allows arbitrary trailing comment bytes).
//!   - Inside an LFH or CDR "extra field" of length zero
//!     (out-of-band bytes that the parser skips entirely).
//!
//! The tool exits non-zero if any non-comment component falls
//! below the kill-rate gate.

#![forbid(unsafe_code)]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_lossless,
    clippy::cast_possible_truncation
)]

use std::collections::BTreeMap;

use axiom_l1_rs::commit_chain::{parse_with_commit_chain, CommitChain};
use axiom_l1_rs::event::ParseEvent;
use axiom_l1_rs::stream::ApkParser;

const FIXTURES: &[&str] = &[
    "crates/axiom-l1-rs/tests/fixtures/fdroid-privileged-2050.apk",
    "crates/axiom-l1-rs/tests/fixtures/clipboard.apk",
    "crates/axiom-l1-rs/tests/fixtures/tickytacky-mirror.apk",
    "crates/axiom-l1-rs/tests/fixtures/wifiautoff.apk",
];

#[derive(Default, Debug, Clone)]
struct ComponentStats {
    mutations: u64,
    kill_parse_error: u64,
    kill_root_changed: u64,
    miss_identical: u64,
}

impl ComponentStats {
    fn record(&mut self, outcome: Outcome) {
        self.mutations += 1;
        match outcome {
            Outcome::ParseError => self.kill_parse_error += 1,
            Outcome::RootChanged => self.kill_root_changed += 1,
            Outcome::Identical => self.miss_identical += 1,
        }
    }

    fn kill_rate_pct(&self) -> f64 {
        if self.mutations == 0 {
            return 100.0;
        }
        let killed = self.kill_parse_error + self.kill_root_changed;
        (killed as f64 / self.mutations as f64) * 100.0
    }
}

#[derive(Copy, Clone, Debug)]
enum Outcome {
    ParseError,
    RootChanged,
    Identical,
}

/// Map a stream-offset to the structural component the byte
/// belongs to. Built from the canonical `CommitChain` leaves
/// + the EOCD's record offset/size to identify the comment
///   region.
fn classify_offset(
    off: u64,
    leaves: &[axiom_l1_rs::commit_chain::CommitLeaf],
    total_len: u64,
) -> &'static str {
    for leaf in leaves {
        let start = leaf.offset;
        let end = leaf.offset + leaf.length;
        if off >= start && off < end {
            return leaf.tag;
        }
    }
    // Anything past the last leaf is the EOCD comment region (or
    // garbage padding the parser skipped). Inputs typically have
    // a zero-length comment, but the spec allows up to 64 KiB.
    if off >= leaves.last().map_or(0, |l| l.offset + l.length) && off < total_len {
        "eocd-comment"
    } else {
        "out-of-bounds"
    }
}

fn run_one_fixture(
    path: &str,
    runs: u64,
    seed: u64,
) -> Result<BTreeMap<&'static str, ComponentStats>, Box<dyn std::error::Error>> {
    let bytes = std::fs::read(path).map_err(|e| format!("read {path}: {e}"))?;
    let total_len = bytes.len() as u64;

    // Canonical root.
    let (_, canonical) = parse_with_commit_chain(bytes.as_slice())?;
    let canonical_root = canonical.root;
    let canonical_leaves = canonical.leaves;

    let mut stats: BTreeMap<&'static str, ComponentStats> = BTreeMap::new();
    let mut s: u64 = seed;
    for _ in 0..runs {
        // LCG step.
        s = s
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        // Pick a byte offset and a bit within it.
        let off = (s >> 32) % total_len;
        s = s
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let bit = ((s >> 56) & 0x07) as u8;
        let component = classify_offset(off, &canonical_leaves, total_len);

        // Apply mutation, run the chain, record outcome, restore.
        let mut mutated = bytes.clone();
        mutated[off as usize] ^= 1 << bit;
        let outcome = match parse_with_commit_chain(mutated.as_slice()) {
            Ok((_, CommitChain { root, .. })) => {
                if root == canonical_root {
                    Outcome::Identical
                } else {
                    Outcome::RootChanged
                }
            }
            Err(_) => Outcome::ParseError,
        };
        stats.entry(component).or_default().record(outcome);
    }
    Ok(stats)
}

/// Walk the parser to confirm we have the byte ranges before fuzzing.
/// Used as a sanity check + summary print.
fn dump_components(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read(path)?;
    let mut parser = ApkParser::from_reader(bytes.as_slice());
    let mut summary: BTreeMap<&'static str, (u64, u64)> = BTreeMap::new();
    while let Some(ev) = parser.next_event()? {
        let (tag, len) = match &ev {
            ParseEvent::ZipEntryHeader { raw_header, .. } => {
                ("lfh-header", raw_header.len() as u64)
            }
            ParseEvent::ZipEntryData { bytes, .. } => ("lfh-body", bytes.len() as u64),
            ParseEvent::SigningBlock { raw, .. } => ("signing-block", raw.len() as u64),
            ParseEvent::CdrEntry { raw, .. } => ("cdr-entry", raw.len() as u64),
            ParseEvent::EocdSeen { raw, .. } => ("eocd", raw.len() as u64),
            _ => continue,
        };
        let entry = summary.entry(tag).or_insert((0, 0));
        entry.0 += 1;
        entry.1 += len;
    }
    let basename = path.rsplit('/').next().unwrap_or(path);
    println!(
        "  {basename}: {} bytes, regions: {:?}",
        bytes.len(),
        summary
    );
    Ok(())
}

fn parse_arg<T: std::str::FromStr>(name: &str, default: T) -> T {
    std::env::args()
        .skip_while(|a| a != name)
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn main() {
    let runs: u64 = parse_arg("--runs", 10_000);
    let seed: u64 = parse_arg("--seed", 0xdead_beef_cafe_babe);
    let gate: f64 = parse_arg("--gate", 99.0);
    println!(
        "p110-tamper-fuzz: runs/fixture={runs} seed=0x{seed:016x} kill-rate gate ≥ {gate:.1} % per non-comment component"
    );
    println!();
    println!("Component summary:");
    for path in FIXTURES {
        if let Err(e) = dump_components(path) {
            eprintln!("  ERROR {path}: {e}");
        }
    }
    println!();

    let mut overall: BTreeMap<&'static str, ComponentStats> = BTreeMap::new();
    for path in FIXTURES {
        let basename = path.rsplit('/').next().unwrap_or(path);
        println!("=== {basename} ({runs} mutations) ===");
        let stats = match run_one_fixture(path, runs, seed) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("  ERROR: {e}");
                std::process::exit(2);
            }
        };
        for (component, s) in &stats {
            println!(
                "  {component:<14} mutations={:>6} kill_parse={:>6} kill_root={:>6} miss={:>6} kill_rate={:>5.1} %",
                s.mutations, s.kill_parse_error, s.kill_root_changed, s.miss_identical, s.kill_rate_pct()
            );
            // Aggregate.
            let agg = overall.entry(component).or_default();
            agg.mutations += s.mutations;
            agg.kill_parse_error += s.kill_parse_error;
            agg.kill_root_changed += s.kill_root_changed;
            agg.miss_identical += s.miss_identical;
        }
    }

    println!();
    println!("=== Aggregate kill rates (all 4 fixtures) ===");
    let mut any_fail = false;
    for (component, s) in &overall {
        let kr = s.kill_rate_pct();
        let is_comment = component.contains("comment");
        let pass_marker = if is_comment {
            "(comment, ungated)".to_string()
        } else if kr >= gate {
            "PASS".to_string()
        } else {
            any_fail = true;
            format!("FAIL (< {gate:.1} %)")
        };
        println!(
            "  {component:<14} mutations={:>6} kill_parse={:>6} kill_root={:>6} miss={:>6} kill_rate={:>5.1} %  {pass_marker}",
            s.mutations, s.kill_parse_error, s.kill_root_changed, s.miss_identical, kr
        );
    }
    if any_fail {
        eprintln!();
        eprintln!(
            "::error::p110-tamper-fuzz: at least one non-comment component fell below kill rate {gate:.1} %"
        );
        std::process::exit(1);
    }
}
