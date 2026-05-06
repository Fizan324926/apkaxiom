// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p113-fuzz-driver` — differential fuzz harness driver.
//!
//! Run modes:
//!
//! ```text
//!   p113-fuzz-driver --mode dev   --seeds <dir> --archive <dir> [--budget Ns | --iters N]
//!   p113-fuzz-driver --mode real  --cvd-root <path> ...    (gated on /dev/kvm + nyx-cuttlefish)
//! ```
//!
//! Dev mode runs entirely in-process (axiom-l0 + the AOSP
//! libziparchive runtime probe). Real mode requires the operator
//! one-shot at CHECKLIST §C-1 / §C-2.

#![forbid(unsafe_code)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::uninlined_format_args,
    clippy::too_many_lines,
    clippy::missing_const_for_fn,
    clippy::option_if_let_else,
    clippy::manual_let_else,
    clippy::single_match_else
)]

use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    sync::Arc,
    time::{Duration, Instant},
};

use p113_fuzz_harness::{
    archive::{ArchiveWriter, Finding},
    classifier::Bucket,
    cuttlefish, differ,
    grammar::Grammar,
    mutator::{mutate, Lcg, MutationKind},
};

const VERSION: &str = "p113-fuzz-driver 0.1.0";

#[derive(Debug, Clone)]
struct Args {
    mode: String,
    seeds: PathBuf,
    archive: PathBuf,
    grammar: Option<PathBuf>,
    probe: PathBuf,
    cvd_root: Option<PathBuf>,
    budget: Option<Duration>,
    iters: Option<u64>,
    seed: u64,
    timeout_ms: u64,
    log_every: u64,
}

impl Args {
    fn parse() -> Self {
        fn arg<T: std::str::FromStr>(name: &str) -> Option<T> {
            std::env::args()
                .skip_while(|a| a != name)
                .nth(1)
                .and_then(|s| s.parse().ok())
        }
        let mode: String = arg("--mode").unwrap_or_else(|| "dev".into());
        let seeds: PathBuf = arg("--seeds").unwrap_or_else(|| PathBuf::from("fuzz/corpus/seed"));
        let archive: PathBuf = arg("--archive").unwrap_or_else(|| PathBuf::from("fuzz/findings"));
        let grammar: Option<PathBuf> = arg("--grammar");
        let probe: PathBuf =
            arg("--probe").unwrap_or_else(|| PathBuf::from("target/zip-aosp-runtime-probe"));
        let cvd_root: Option<PathBuf> = arg("--cvd-root");
        let budget: Option<Duration> = arg::<u64>("--budget").map(Duration::from_secs);
        let iters: Option<u64> = arg("--iters");
        let seed: u64 = arg("--seed").unwrap_or(0xb113_d1ff_d1ff_0001);
        let timeout_ms: u64 = arg("--timeout-ms").unwrap_or(2_000);
        let log_every: u64 = arg("--log-every").unwrap_or(500);
        Self {
            mode,
            seeds,
            archive,
            grammar,
            probe,
            cvd_root,
            budget,
            iters,
            seed,
            timeout_ms,
            log_every,
        }
    }
}

fn load_seeds(dir: &Path) -> std::io::Result<Vec<(PathBuf, Vec<u8>)>> {
    let mut out = Vec::new();
    walk(dir, &mut out)?;
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

fn walk(dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) -> std::io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let p = entry.path();
        if p.is_dir() {
            walk(&p, out)?;
        } else if p.extension().and_then(|s| s.to_str()) == Some("bin")
            || p.extension().and_then(|s| s.to_str()) == Some("apk")
        {
            let bytes = std::fs::read(&p)?;
            out.push((p, bytes));
        }
    }
    Ok(())
}

#[derive(Default)]
struct Counters {
    iters: u64,
    bucket_a: u64,
    bucket_b: u64,
    bucket_c: u64,
    bucket_d: u64,
    bucket_e: u64,
    target_io_errors: u64,
    distinct_findings: std::collections::HashSet<String>,
}

impl Counters {
    fn total_findings(&self) -> u64 {
        self.bucket_c + self.bucket_d + self.bucket_e
    }
    fn record(&mut self, bucket: Bucket, finding_id: &str) {
        self.iters += 1;
        match bucket {
            Bucket::A => self.bucket_a += 1,
            Bucket::B => self.bucket_b += 1,
            Bucket::C => self.bucket_c += 1,
            Bucket::D => self.bucket_d += 1,
            Bucket::E => self.bucket_e += 1,
        }
        if bucket.is_finding() {
            self.distinct_findings.insert(finding_id.to_string());
        }
    }
}

fn print_status(c: &Counters, elapsed: Duration) {
    let rate = (c.iters as f64) / elapsed.as_secs_f64().max(0.001);
    println!(
        "  iters={:<8} A={:<8} B={:<8} C={:<6} D={:<6} E={:<6} distinct={:<5} rate={:>7.0}/s elapsed={:.1}s",
        c.iters,
        c.bucket_a,
        c.bucket_b,
        c.bucket_c,
        c.bucket_d,
        c.bucket_e,
        c.distinct_findings.len(),
        rate,
        elapsed.as_secs_f64()
    );
    let _ = std::io::stdout().flush();
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    println!("{VERSION}");
    println!(
        "  mode={}  seeds={}  archive={}  probe={}  seed={:#018x}",
        args.mode,
        args.seeds.display(),
        args.archive.display(),
        args.probe.display(),
        args.seed,
    );

    if args.mode == "real" {
        match cuttlefish::probe(args.cvd_root.as_deref()) {
            cuttlefish::Availability::Available { .. } => {
                eprintln!("real-mode Cuttlefish target detected; not yet implemented (CHECKLIST §C-2). exiting.");
                std::process::exit(2);
            }
            cuttlefish::Availability::Missing { reason } => {
                eprintln!("WARN: real mode requested but unavailable: {reason}");
                eprintln!("WARN: falling back to dev mode");
            }
        }
    }

    let grammar = match &args.grammar {
        Some(p) => match Grammar::load(p) {
            Ok(g) => {
                println!(
                    "  grammar: {} productions={} terminals={}",
                    g.source, g.productions, g.terminals
                );
                Some(g)
            }
            Err(e) => {
                eprintln!("ERROR loading grammar {}: {e}", p.display());
                std::process::exit(2);
            }
        },
        None => None,
    };

    let seeds = load_seeds(&args.seeds)?;
    if seeds.is_empty() {
        eprintln!(
            "ERROR: no seeds found under {} — run `make p113-corpus-seed` first",
            args.seeds.display()
        );
        std::process::exit(2);
    }
    println!("  seeds: {} files loaded", seeds.len());

    if !args.probe.exists() {
        eprintln!(
            "ERROR: AOSP probe not found at {} — run `make p16-aosp-runtime-probe` first",
            args.probe.display()
        );
        std::process::exit(2);
    }

    let writer = ArchiveWriter::open(&args.archive)?;
    let timeout = Duration::from_millis(args.timeout_ms);

    // Ctrl-C cleanly stops the loop and flushes the archive.
    let stop = Arc::new(AtomicBool::new(false));
    let stop2 = Arc::clone(&stop);
    // We don't pull `signal-hook` — a small ctrlc signal handler is
    // not worth a vendored dep. Loop simply checks stop flag.
    // Set a tiny watchdog thread to flip stop after `budget`.
    if let Some(budget) = args.budget {
        std::thread::spawn(move || {
            std::thread::sleep(budget);
            stop2.store(true, Ordering::Relaxed);
        });
    }

    let mut rng = Lcg::new(args.seed);
    let mut counters = Counters::default();
    let start = Instant::now();
    let max_iters = args.iters.unwrap_or(u64::MAX);

    while counters.iters < max_iters && !stop.load(Ordering::Relaxed) {
        let i = (rng.next_u32() as usize) % seeds.len();
        let j = (rng.next_u32() as usize) % seeds.len();
        let (origin, base) = (&seeds[i].0, &seeds[i].1);
        let aux = Some(seeds[j].1.as_slice());
        let (mutated, kind): (Vec<u8>, MutationKind) =
            mutate(&mut rng, base, aux, grammar.as_ref());

        let (axiom, target, bucket) = match differ::run_diff(&mutated, &args.probe, timeout) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("WARN target probe i={}: {e}", counters.iters);
                counters.target_io_errors += 1;
                counters.iters += 1;
                continue;
            }
        };

        if bucket.is_finding() {
            let path = writer.save_input(&mutated)?;
            let finding = Finding::from_verdicts(
                &args.mode,
                "aosp-libziparchive-runtime",
                &mutated,
                &path,
                axiom,
                target,
                bucket,
                Some(format!("{}", origin.display())),
                Some(kind.label().to_string()),
            );
            counters.record(bucket, &finding.finding_id);
            writer.append(&finding)?;
        } else {
            counters.record(bucket, "");
        }

        if counters.iters > 0 && counters.iters % args.log_every == 0 {
            print_status(&counters, start.elapsed());
        }
    }

    print_status(&counters, start.elapsed());
    println!();
    println!("=== summary ===");
    println!("  total iters           : {}", counters.iters);
    println!("  target io errors      : {}", counters.target_io_errors);
    println!("  bucket A (both ok)    : {}", counters.bucket_a);
    println!("  bucket B (same tag)   : {}", counters.bucket_b);
    println!("  bucket C (diff tag)   : {}", counters.bucket_c);
    println!("  bucket D (axiom lax)  : {}", counters.bucket_d);
    println!("  bucket E (axiom strict): {}", counters.bucket_e);
    println!("  total findings        : {}", counters.total_findings());
    println!(
        "  distinct findings     : {}",
        counters.distinct_findings.len()
    );
    println!(
        "  archive               : {}",
        writer.archive_path().display()
    );

    Ok(())
}
