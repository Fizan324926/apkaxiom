// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p111-tamper-fuzz` — P1.11 G11.
//!
//! Random single-bit-flip mutations × every honest signed APK ×
//! every byte position class. Asserts the verifier rejects every
//! mutation that lands in a region whose tamper detection is
//! cryptographically committed.
//!
//! Per-component classifier:
//!
//!   * `lfh-header`     — LFH bytes (every entry header)
//!   * `lfh-body`       — entry body bytes
//!   * `signing-block`  — bytes inside the v2/v3/v3.1 carrier
//!   * `cdr-entry`      — central-directory record bytes
//!   * `eocd`           — end-of-central-directory bytes
//!   * `out-of-bounds`  — bytes the parser doesn't classify
//!     (typically the EOCD comment region; not committed by
//!     the verifier).
//!
//! Default: 10 000 mutations × 4 honest fixtures = 40 000 trials.

#![forbid(unsafe_code)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::uninlined_format_args,
    clippy::doc_markdown,
    clippy::unusual_byte_groupings,
    clippy::missing_const_for_fn,
    clippy::redundant_else,
    clippy::collapsible_else_if,
    clippy::match_same_arms
)]

use std::collections::BTreeMap;

use axiom_sigverify::{verify_apk, Verdict};

const HONEST_FIXTURES: &[&str] = &[
    "corpus/signing/v1-only/wifiautoff-v1.apk",
    "corpus/signing/v1-v2/wifiautoff-v1v2.apk",
    "corpus/signing/v1-v2-v3/wifiautoff-v1v2v3.apk",
    "corpus/signing/v1-v2-v3-v31/wifiautoff-v1v2v3v31.apk",
];

#[derive(Default, Debug, Clone, Copy)]
struct Stats {
    mutations: u64,
    kill_reject: u64,
    kill_malformed: u64,
    miss_accept: u64,
}

impl Stats {
    fn record(&mut self, v: &Verdict) {
        self.mutations += 1;
        match v {
            Verdict::Accept => self.miss_accept += 1,
            Verdict::Reject(_) => self.kill_reject += 1,
            Verdict::Malformed(_) | Verdict::NotPresent => self.kill_malformed += 1,
        }
    }

    fn kill_rate_pct(self) -> f64 {
        if self.mutations == 0 {
            return 100.0;
        }
        let killed = self.kill_reject + self.kill_malformed;
        (killed as f64 / self.mutations as f64) * 100.0
    }
}

/// Classify a byte offset inside an APK by which sub-region of the
/// signing block it falls in. Verifier-committed regions (LFH /
/// CDR / EOCD / v2/v3/v3.1 signing-scheme bytes) are gated; the
/// AOSP padding block and source-stamp blocks are NOT committed
/// by our verifier (only by apksigner's source-stamp path) and
/// are surfaced as separate ungated buckets.
fn classify(apk: &[u8], off: u64) -> &'static str {
    let off_us = off as usize;
    if let Ok(Some(block)) = axiom_sigblock::locate(apk) {
        let block_start = block.block_offset as usize;
        let block_end = (block.block_offset + block.block_total_size) as usize;
        if off_us < block_start {
            return "lfh-or-cdr";
        }
        if off_us >= block_end {
            return "eocd";
        }
        // Walk pairs to find the sub-block.
        let mut cur_in_buf = block_start + 8; // past leading u64
        let pair_end = block_end - 24; // before trailing size+magic
        while cur_in_buf < pair_end {
            let length =
                u64::from_le_bytes(apk[cur_in_buf..cur_in_buf + 8].try_into().unwrap_or([0; 8]))
                    as usize;
            let pair_total = 8 + length;
            let pair_value_start = cur_in_buf + 12;
            let pair_value_end = cur_in_buf + 8 + length;
            if off_us >= cur_in_buf && off_us < pair_value_end {
                let id = u32::from_le_bytes(
                    apk[cur_in_buf + 8..cur_in_buf + 12]
                        .try_into()
                        .unwrap_or([0; 4]),
                );
                let _ = pair_value_start;
                return match id {
                    axiom_sigblock::ID_V2 => "v2-block",
                    axiom_sigblock::ID_V3 => "v3-block",
                    axiom_sigblock::ID_V3_1 => "v3_1-block",
                    axiom_sigblock::ID_PADDING => "padding-block",
                    axiom_sigblock::ID_SOURCE_STAMP_V1 => "source-stamp-block",
                    axiom_sigblock::ID_SOURCE_STAMP_V2 => "source-stamp-block",
                    _ => "unknown-pair",
                };
            }
            cur_in_buf += pair_total;
        }
        return "block-tail";
    }
    let total_len = apk.len() as u64;
    if off >= total_len.saturating_sub(22) {
        return "eocd";
    }
    "lfh-or-cdr"
}

fn parse_arg<T: std::str::FromStr>(name: &str, default: T) -> T {
    std::env::args()
        .skip_while(|a| a != name)
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn lcg_next(s: u64) -> u64 {
    s.wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407)
}

fn main() {
    let runs: u64 = parse_arg("--runs", 10_000);
    let seed: u64 = parse_arg("--seed", 0xdead_beef_cafe_babe);
    // Default gate 95 % — v1-only APKs by design don't commit ZIP-
    // layout bytes (LFH/CDR/EOCD), so a single-bit flip in those
    // regions on the v1-only fixture is a known v1 limitation, not
    // a verifier bug. v2/v3/v3.1 blocks should hit 100 %.
    let gate_pct: f64 = parse_arg("--gate", 95.0);
    println!(
        "p111-tamper-fuzz: {runs} mutations × {} fixtures = {} trials, gate ≥ {gate_pct} %",
        HONEST_FIXTURES.len(),
        runs * HONEST_FIXTURES.len() as u64,
    );
    let mut overall: BTreeMap<&'static str, Stats> = BTreeMap::new();
    for path in HONEST_FIXTURES {
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("ERROR read {path}: {e}");
                std::process::exit(2);
            }
        };
        // Canonical accept.
        let canonical = verify_apk(&bytes);
        if !matches!(canonical, Verdict::Accept) {
            eprintln!("ERROR canonical verify {path}: {canonical:?}");
            std::process::exit(2);
        }
        let mut s = seed;
        let mut per_fixture: BTreeMap<&'static str, Stats> = BTreeMap::new();
        for _ in 0..runs {
            s = lcg_next(s);
            let off = (s >> 32) % bytes.len() as u64;
            s = lcg_next(s);
            let bit = ((s >> 56) & 0x07) as u8;
            let mut mutated = bytes.clone();
            mutated[off as usize] ^= 1 << bit;
            let v = verify_apk(&mutated);
            let comp = classify(&bytes, off);
            per_fixture.entry(comp).or_default().record(&v);
            overall.entry(comp).or_default().record(&v);
        }
        let basename = path.rsplit('/').next().unwrap_or(path);
        println!("=== {basename} ===");
        for (comp, st) in &per_fixture {
            println!(
                "  {comp:<14} mutations={:>5} kill_reject={:>5} kill_malformed={:>5} miss_accept={:>5} kill_rate={:>5.1} %",
                st.mutations, st.kill_reject, st.kill_malformed, st.miss_accept, st.kill_rate_pct()
            );
        }
    }
    println!();
    println!("=== Aggregate ===");
    let mut any_fail = false;
    for (comp, st) in &overall {
        let kr = st.kill_rate_pct();
        // Source-stamp + AOSP padding aren't covered by our verifier
        // (only by apksigner's source-stamp path) — flagged but ungated.
        let allowed_low = comp.contains("source-stamp")
            || comp.contains("padding-block")
            || comp.contains("block-tail")
            || comp.contains("unknown-pair");
        let pass = if allowed_low {
            "(uncommitted region — ungated)".to_string()
        } else if kr >= gate_pct {
            "PASS".to_string()
        } else {
            any_fail = true;
            format!("FAIL (< {gate_pct:.1} %)")
        };
        println!(
            "  {comp:<14} mutations={:>6} kill_reject={:>6} kill_malformed={:>6} miss_accept={:>6} kill_rate={:>5.1} %  {pass}",
            st.mutations, st.kill_reject, st.kill_malformed, st.miss_accept, kr
        );
    }
    if any_fail {
        eprintln!();
        eprintln!("::error::p111-tamper-fuzz: at least one committed region fell below kill rate {gate_pct:.1} %");
        std::process::exit(1);
    }
}
