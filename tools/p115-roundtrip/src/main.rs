// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p115-roundtrip` — AXIOM-IR-v0.1 round-trip gate.
//!
//! For each APK in the corpus directory, opens it via the streaming
//! parser (which captures raw AXML + ARSC bytes without signature
//! verification), parses both through the IR layer, re-encodes, and
//! checks byte-identity.
//!
//! Exit code: 0 iff ≥ 95 % of APKs pass both AXML and ARSC gates.
//!
//! Usage:
//!   p115-roundtrip [--corpus DIR] [--verbose]
//!
//! Default corpus dir: `fuzz/corpus/real-apks` relative to the
//! workspace root (detected via `CARGO_MANIFEST_DIR` or CWD).

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::time::Instant;

use axiom_l1_rs::{Apk, Unverified};
use axiom_l1_rs::ir::{axml, arsc};

fn main() {
    let mut args = std::env::args().skip(1).peekable();
    let mut corpus: Option<PathBuf> = None;
    let mut verbose = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--corpus" => {
                corpus = args.next().map(PathBuf::from);
            }
            "--verbose" => verbose = true,
            other => {
                eprintln!("unknown arg: {other}");
                std::process::exit(2);
            }
        }
    }

    let corpus_dir = corpus.unwrap_or_else(|| {
        // Walk up from CWD to find the workspace root (contains Cargo.toml
        // with [workspace]).
        let cwd = std::env::current_dir().expect("cwd");
        find_workspace_root(&cwd)
            .map(|r| r.join("fuzz/corpus/real-apks"))
            .unwrap_or_else(|| cwd.join("fuzz/corpus/real-apks"))
    });

    if !corpus_dir.exists() {
        eprintln!("corpus dir not found: {}", corpus_dir.display());
        std::process::exit(2);
    }

    let apks: Vec<PathBuf> = {
        let mut v: Vec<PathBuf> = std::fs::read_dir(&corpus_dir)
            .expect("read corpus dir")
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension().and_then(|s| s.to_str()) == Some("apk")
            })
            .collect();
        v.sort();
        v
    };

    if apks.is_empty() {
        eprintln!("no .apk files in {}", corpus_dir.display());
        std::process::exit(2);
    }

    println!("p115-roundtrip: {} APKs in {}", apks.len(), corpus_dir.display());

    let total = apks.len();
    let mut manifest_pass = 0usize;
    let mut manifest_skip = 0usize; // no manifest bytes in this APK
    let mut manifest_fail = 0usize;
    let mut resources_pass = 0usize;
    let mut resources_skip = 0usize;
    let mut resources_fail = 0usize;

    let t0 = Instant::now();

    for path in &apks {
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("  OPEN ERROR {name}: {e}");
                manifest_fail += 1;
                resources_fail += 1;
                continue;
            }
        };

        let apk: Apk<Unverified> = match Apk::from_reader(file) {
            Ok(a) => a,
            Err(e) => {
                eprintln!("  PARSE ERROR {name}: {e}");
                manifest_fail += 1;
                resources_fail += 1;
                continue;
            }
        };

        // — Manifest round-trip —
        match apk.manifest_bytes() {
            None => {
                if verbose {
                    println!("  SKIP-manifest {name}: no AndroidManifest.xml captured");
                }
                manifest_skip += 1;
            }
            Some(raw) => match axml::round_trip(raw) {
                Err(e) => {
                    if verbose {
                        println!("  FAIL-manifest {name}: parse error: {e}");
                    }
                    manifest_fail += 1;
                }
                Ok(reencoded) => {
                    if reencoded == raw {
                        if verbose {
                            println!("  PASS-manifest {name}");
                        }
                        manifest_pass += 1;
                    } else {
                        if verbose {
                            println!(
                                "  DIFF-manifest {name}: {} input bytes, {} reencoded bytes",
                                raw.len(),
                                reencoded.len()
                            );
                        }
                        manifest_fail += 1;
                    }
                }
            },
        }

        // — Resources round-trip —
        match apk.resources_bytes() {
            None => {
                if verbose {
                    println!("  SKIP-resources {name}: no resources.arsc captured");
                }
                resources_skip += 1;
            }
            Some(raw) => match arsc::round_trip(raw) {
                Err(e) => {
                    if verbose {
                        println!("  FAIL-resources {name}: parse error: {e}");
                    }
                    resources_fail += 1;
                }
                Ok(reencoded) => {
                    if reencoded == raw {
                        if verbose {
                            println!("  PASS-resources {name}");
                        }
                        resources_pass += 1;
                    } else {
                        if verbose {
                            println!(
                                "  DIFF-resources {name}: {} input bytes, {} reencoded bytes",
                                raw.len(),
                                reencoded.len()
                            );
                        }
                        resources_fail += 1;
                    }
                }
            },
        }
    }

    let elapsed = t0.elapsed();

    println!();
    println!("=== AXML manifest round-trip ===");
    let manifest_eligible = total - manifest_skip;
    let manifest_rate = if manifest_eligible > 0 {
        manifest_pass as f64 / manifest_eligible as f64 * 100.0
    } else {
        100.0
    };
    println!(
        "  total={total}  skip={manifest_skip}  eligible={manifest_eligible}  pass={manifest_pass}  fail={manifest_fail}"
    );
    println!("  byte-identical rate: {manifest_rate:.1}%");

    println!();
    println!("=== ARSC resources round-trip ===");
    let resources_eligible = total - resources_skip;
    let resources_rate = if resources_eligible > 0 {
        resources_pass as f64 / resources_eligible as f64 * 100.0
    } else {
        100.0
    };
    println!(
        "  total={total}  skip={resources_skip}  eligible={resources_eligible}  pass={resources_pass}  fail={resources_fail}"
    );
    println!("  byte-identical rate: {resources_rate:.1}%");

    println!();
    println!("elapsed: {:.2}s", elapsed.as_secs_f64());

    let gate = 95.0_f64;
    let manifest_ok = manifest_eligible == 0 || manifest_rate >= gate;
    let resources_ok = resources_eligible == 0 || resources_rate >= gate;

    if manifest_ok && resources_ok {
        println!("GATE PASS — both channels ≥ {gate:.0}% byte-identical");
        std::process::exit(0);
    } else {
        if !manifest_ok {
            eprintln!("GATE FAIL — AXML rate {manifest_rate:.1}% < {gate:.0}%");
        }
        if !resources_ok {
            eprintln!("GATE FAIL — ARSC rate {resources_rate:.1}% < {gate:.0}%");
        }
        std::process::exit(1);
    }
}

fn find_workspace_root(start: &Path) -> Option<PathBuf> {
    let mut dir = start.to_path_buf();
    loop {
        let candidate = dir.join("Cargo.toml");
        if candidate.exists() {
            let content = std::fs::read_to_string(&candidate).unwrap_or_default();
            if content.contains("[workspace]") {
                return Some(dir);
            }
        }
        if !dir.pop() {
            return None;
        }
    }
}

// ── Determinism test ──────────────────────────────────────────────────────────
// Verifies that parsing the same bytes twice produces identical IR.
// This runs as a binary-level integration test (not `cargo test`) so
// it uses real APKs from the corpus instead of synthetic payloads.

#[cfg(test)]
mod tests {
    use super::*;
    use axiom_l1_rs::ir::emit::emit_manifest;
    use axiom_l1_rs::apk_data::Manifest;

    #[test]
    fn axml_determinism_synthetic() {
        // Two parses of the same bytes must produce identical docs.
        let mut raw: Vec<u8> = Vec::new();
        // Outer RES_XML header
        raw.extend_from_slice(&axml::chunk_type::RES_XML.to_le_bytes());
        raw.extend_from_slice(&8u16.to_le_bytes());
        // string pool chunk: 28 bytes
        let mut sp: Vec<u8> = Vec::new();
        sp.extend_from_slice(&axml::chunk_type::RES_STRING_POOL.to_le_bytes());
        sp.extend_from_slice(&28u16.to_le_bytes());
        sp.extend_from_slice(&28u32.to_le_bytes());
        sp.extend_from_slice(&[0u8; 20]);
        let total = (8u32 + sp.len() as u32).to_le_bytes();
        raw.extend_from_slice(&total);
        raw.extend_from_slice(&sp);

        let doc1 = axml::parse(&raw).expect("parse 1");
        let doc2 = axml::parse(&raw).expect("parse 2");
        assert_eq!(doc1, doc2, "parse is not deterministic");

        let out1 = axml::emit(&doc1);
        let out2 = axml::emit(&doc2);
        assert_eq!(out1, out2, "emit is not deterministic");
        assert_eq!(out1, raw, "emit is not byte-identical");
    }

    #[test]
    fn arsc_determinism_synthetic() {
        let mut inner: Vec<u8> = Vec::new();
        inner.extend_from_slice(&arsc::chunk_type::RES_STRING_POOL.to_le_bytes());
        inner.extend_from_slice(&28u16.to_le_bytes());
        inner.extend_from_slice(&28u32.to_le_bytes());
        inner.extend_from_slice(&[0u8; 20]);

        let mut raw: Vec<u8> = Vec::new();
        raw.extend_from_slice(&arsc::chunk_type::RES_TABLE.to_le_bytes());
        raw.extend_from_slice(&12u16.to_le_bytes());
        let total = (12u32 + inner.len() as u32).to_le_bytes();
        raw.extend_from_slice(&total);
        raw.extend_from_slice(&1u32.to_le_bytes()); // package_count
        raw.extend_from_slice(&inner);

        let doc1 = arsc::parse(&raw).expect("parse 1");
        let doc2 = arsc::parse(&raw).expect("parse 2");
        assert_eq!(doc1, doc2);

        let out1 = arsc::emit(&doc1);
        let out2 = arsc::emit(&doc2);
        assert_eq!(out1, out2);
        assert_eq!(out1, raw);
    }

    #[test]
    fn emit_manifest_glue_determinism() {
        let mut sp: Vec<u8> = Vec::new();
        sp.extend_from_slice(&axml::chunk_type::RES_STRING_POOL.to_le_bytes());
        sp.extend_from_slice(&28u16.to_le_bytes());
        sp.extend_from_slice(&28u32.to_le_bytes());
        sp.extend_from_slice(&[0u8; 20]);

        let mut axml_raw: Vec<u8> = Vec::new();
        axml_raw.extend_from_slice(&axml::chunk_type::RES_XML.to_le_bytes());
        axml_raw.extend_from_slice(&8u16.to_le_bytes());
        let total = (8u32 + sp.len() as u32).to_le_bytes();
        axml_raw.extend_from_slice(&total);
        axml_raw.extend_from_slice(&sp);

        let m = Manifest { axml_bytes: axml_raw.clone() };
        let ir1 = emit_manifest(&m).expect("emit 1");
        let ir2 = emit_manifest(&m).expect("emit 2");
        assert_eq!(ir1, ir2, "ManifestIr is not deterministic");
    }
}
