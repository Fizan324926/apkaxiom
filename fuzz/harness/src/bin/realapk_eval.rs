// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p114-realapk-eval` — run each input file under `--seeds`
//! through the verified parser + every cross-version probe
//! exactly once (no mutation, no iteration). Produces an ndjson
//! archive identical in shape to the fuzz driver's output, so
//! `p114-classify` and downstream tools work unchanged.
//!
//! Usage:
//!
//! ```text
//!   p114-realapk-eval \
//!     --seeds fuzz/corpus/real-apks \
//!     --archive fuzz/findings-realapks \
//!     --probe target/zip-aosp-runtime-probe \
//!     --probes "A14:synthetic,A11:synthetic,A8:synthetic"
//! ```
//!
//! This is the right tool for **measuring false-positive rates
//! on legitimate inputs**. The fuzz driver always mutates, so
//! its output is dominated by adversarial inputs; this tool
//! measures what the classifier says about real, well-formed
//! APKs untouched.

#![forbid(unsafe_code)]
#![allow(clippy::uninlined_format_args)]

use std::path::PathBuf;
use std::time::Duration;

use p113_fuzz_harness::{
    archive::{ArchiveWriter, Finding},
    classifier::{classify, Bucket},
    differ,
    probe::PersistentProbe,
    version_probes::{parse_probes_csv, VersionedProbe},
};

const VERSION: &str = "p114-realapk-eval 0.1.0";

fn arg<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::args()
        .skip_while(|a| a != name)
        .nth(1)
        .and_then(|s| s.parse().ok())
}

fn collect_apks(dir: &std::path::Path) -> std::io::Result<Vec<(PathBuf, Vec<u8>)>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let p = entry.path();
        if p.is_dir() {
            continue;
        }
        let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
        if ext == "apk" || ext == "bin" || ext == "zip" {
            let bytes = std::fs::read(&p)?;
            out.push((p, bytes));
        }
    }
    Ok(out)
}

fn shard_input_path(sha: &str) -> String {
    let aa = &sha[0..2];
    let bb = &sha[2..4];
    format!("inputs/{aa}/{bb}/{sha}.bin")
}

fn save_sharded(writer: &ArchiveWriter, input: &[u8]) -> std::io::Result<String> {
    use axiom_blake3_hacl::{hex_encode, Blake3, Hasher};
    let mut h = Blake3::default();
    h.update(input);
    let sha = hex_encode(&h.finalize_borrow());
    let aa = &sha[0..2];
    let bb = &sha[2..4];
    let dir = writer.inputs_dir().join(aa).join(bb);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{sha}.bin"));
    if !path.exists() {
        std::fs::write(&path, input)?;
    }
    Ok(sha)
}

fn main() -> std::io::Result<()> {
    let seeds: PathBuf = arg("--seeds").unwrap_or_else(|| PathBuf::from("fuzz/corpus/real-apks"));
    let archive: PathBuf =
        arg("--archive").unwrap_or_else(|| PathBuf::from("fuzz/findings-realapks"));
    let probe: PathBuf =
        arg("--probe").unwrap_or_else(|| PathBuf::from("target/zip-aosp-runtime-probe"));
    let probes_csv: String =
        arg("--probes").unwrap_or_else(|| "A14:synthetic,A11:synthetic,A8:synthetic".into());
    let timeout_ms: u64 = arg("--probe-timeout-ms").unwrap_or(10_000);

    println!("{VERSION}");
    println!(
        "  seeds={}  archive={}  probe={}  probes={}",
        seeds.display(),
        archive.display(),
        probe.display(),
        probes_csv
    );

    let apks = collect_apks(&seeds)?;
    println!("  loaded {} APK files", apks.len());
    if apks.is_empty() {
        eprintln!("ERROR no APKs at {}", seeds.display());
        std::process::exit(2);
    }

    let writer = ArchiveWriter::open(&archive)?;
    let probe_timeout = Duration::from_millis(timeout_ms);

    // Primary A14 probe (used for the "real" A14 verdict and as
    // the base for synthetic A11/A8 layers).
    let primary = PersistentProbe::spawn("aosp-libziparchive-runtime", &probe)?
        .with_timeout(probe_timeout);
    println!("  primary-probe: {} (real)", primary.label());

    // Cross-version probes.
    let mut xv_probes: Vec<VersionedProbe> = Vec::new();
    for (version, path) in parse_probes_csv(&probes_csv) {
        let p_str = path.to_str().unwrap_or("");
        if p_str == "synthetic" {
            let base = PersistentProbe::spawn(
                &format!("aosp-libziparchive-base-{}", version.label().to_lowercase()),
                &probe,
            )?
            .with_timeout(probe_timeout);
            let vp = VersionedProbe::synthetic_layer(version, base);
            println!("  xv-probe     : {} (synthetic)", vp.label);
            xv_probes.push(vp);
        } else {
            match VersionedProbe::real(version, &path, probe_timeout) {
                Ok(vp) => {
                    println!("  xv-probe     : {} (real)", vp.label);
                    xv_probes.push(vp);
                }
                Err(e) => eprintln!(
                    "WARN failed to spawn real probe for {} at {}: {e}",
                    version.label(),
                    path.display()
                ),
            }
        }
    }

    let mut total = 0u64;
    let mut buckets = [0u64; 5];
    for (path, bytes) in &apks {
        let axiom = differ::run_axiom(bytes);
        let primary_v = match primary.run_one(bytes) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("WARN primary probe on {}: {e}", path.display());
                continue;
            }
        };
        let primary_bucket = classify(&axiom, &primary_v);
        match primary_bucket {
            Bucket::A => buckets[0] += 1,
            Bucket::B => buckets[1] += 1,
            Bucket::C => buckets[2] += 1,
            Bucket::D => buckets[3] += 1,
            Bucket::E => buckets[4] += 1,
        }
        let sha = save_sharded(&writer, bytes)?;
        let input_path = shard_input_path(&sha);
        let f_primary = Finding::from_verdicts_versioned(
            "real-apk",
            "aosp-libziparchive-runtime",
            "A14",
            false,
            bytes,
            &input_path,
            axiom.clone(),
            primary_v,
            primary_bucket,
            Some(format!("{}", path.display())),
            Some("none".to_string()),
        );
        writer.append(&f_primary)?;
        total += 1;
        for vp in &xv_probes {
            let v = match vp.run_one(bytes) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let bucket = classify(&axiom, &v);
            let f = Finding::from_verdicts_versioned(
                "real-apk",
                &vp.label,
                vp.version.label(),
                vp.synthetic,
                bytes,
                &input_path,
                axiom.clone(),
                v,
                bucket,
                Some(format!("{}", path.display())),
                Some("none".to_string()),
            );
            writer.append(&f)?;
            total += 1;
        }
    }

    println!();
    println!("=== summary ===");
    println!("  APKs processed     : {}", apks.len());
    println!("  records written    : {}", total);
    println!("  primary buckets:");
    println!("    A (both accept)  : {}", buckets[0]);
    println!("    B (same reject)  : {}", buckets[1]);
    println!("    C (taxonomy)     : {}", buckets[2]);
    println!("    D (axiom lax)    : {}", buckets[3]);
    println!("    E (axiom strict) : {}", buckets[4]);
    println!("  archive            : {}", writer.archive_path().display());
    Ok(())
}
