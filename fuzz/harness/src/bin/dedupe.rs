// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p113-fuzz-dedupe` — read the finding archive, cluster by
//! root-cause key, emit one minimal-reproducer per cluster.
//!
//! ```text
//!   p113-fuzz-dedupe --archive fuzz/findings/archive.ndjson [--out clusters.ndjson]
//! ```

#![forbid(unsafe_code)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::uninlined_format_args
)]

use std::path::PathBuf;

use p113_fuzz_harness::{archive, dedup};

fn parse_arg<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::args()
        .skip_while(|a| a != name)
        .nth(1)
        .and_then(|s| s.parse().ok())
}

fn main() -> std::io::Result<()> {
    let archive_path: PathBuf =
        parse_arg("--archive").unwrap_or_else(|| PathBuf::from("fuzz/findings/archive.ndjson"));
    let out: Option<PathBuf> = parse_arg("--out");

    let findings = archive::read_findings(&archive_path)?;
    let summary = dedup::summarise(&findings);
    let clusters = dedup::dedupe(&findings);

    println!("p113-fuzz-dedupe 0.1.0");
    println!("  archive          : {}", archive_path.display());
    println!("  raw findings     : {}", summary.raw_findings);
    println!("  total clusters   : {}", summary.total_clusters);
    println!("  C clusters       : {}", summary.c_clusters);
    println!("  D clusters       : {}", summary.d_clusters);
    println!("  E clusters       : {}", summary.e_clusters);
    println!("  honest count D+E : {}", summary.honest_count());

    if let Some(p) = out {
        let mut s = String::new();
        for f in &clusters {
            s.push_str(&f.to_ndjson_line());
        }
        std::fs::write(&p, s)?;
        println!(
            "  wrote clusters   : {} ({} records)",
            p.display(),
            clusters.len()
        );
    }

    Ok(())
}
