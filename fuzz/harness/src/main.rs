// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p113-fuzz-driver` — differential fuzz harness driver.
//!
//! Run modes:
//!
//! ```text
//!   p113-fuzz-driver --mode dev   --seeds <dir> --archive <dir> [--budget Ns | --iters N]
//!                    [--probe target/zip-aosp-runtime-probe]      ← persistent server
//!                    [--asan-probe target/zip-aosp-runtime-probe-asan]  ← Gap-7
//!                    [--arms unzip,jdk-jar,py-zipfile]              ← Gap-9
//!                    [--metrics 127.0.0.1:9913]                     ← Gap-3
//!   p113-fuzz-driver --mode real  --cvd-root <path> ...    (gated on /dev/kvm + nyx-cuttlefish)
//! ```

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
    clippy::single_match_else,
    clippy::needless_pass_by_value,
    clippy::cognitive_complexity
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
    classifier::{classify, Bucket, Verdict},
    coverage::CoverageMap,
    cuttlefish, differ,
    grammar::Grammar,
    metrics::{Exporter, Metrics},
    mutator::{mutate, Lcg},
    probe::PersistentProbe,
    third_arms::{arms_from_csv, Arm},
    version_probes::{parse_probes_csv, VersionedProbe},
};

const VERSION: &str = "p113-fuzz-driver 0.2.0";

#[derive(Debug, Clone)]
struct Args {
    mode: String,
    seeds: PathBuf,
    archive: PathBuf,
    grammar: Option<PathBuf>,
    probe: PathBuf,
    asan_probe: Option<PathBuf>,
    arms_csv: String,
    arms_sample_rate: u32,
    metrics_bind: Option<String>,
    cvd_root: Option<PathBuf>,
    budget: Option<Duration>,
    iters: Option<u64>,
    seed: u64,
    log_every: u64,
    min_findings_gate: Option<u64>,
    min_e_findings_gate: Option<u64>,
    max_io_errors_gate: Option<u64>,
    /// Per-input timeout in milliseconds. Watchdog kills the
    /// probe with `kill -9` on overrun; the next call re-spawns
    /// transparently. Default 5000 ms.
    probe_timeout_ms: u64,
    /// Cross-version probes CSV. Format:
    ///   `A14:path,A11:path,A8:path` — real per-version probes.
    ///   `A14:synthetic,A11:synthetic,A8:synthetic` — synthetic
    ///   layer (P1.14 §A — runs on top of `--probe`'s A14 base).
    /// Empty = single-version mode (legacy P1.13 behaviour).
    probes_csv: String,
}

impl Args {
    fn parse() -> Self {
        fn arg<T: std::str::FromStr>(name: &str) -> Option<T> {
            std::env::args()
                .skip_while(|a| a != name)
                .nth(1)
                .and_then(|s| s.parse().ok())
        }
        Self {
            mode: arg("--mode").unwrap_or_else(|| "dev".into()),
            seeds: arg("--seeds").unwrap_or_else(|| PathBuf::from("fuzz/corpus/seed")),
            archive: arg("--archive").unwrap_or_else(|| PathBuf::from("fuzz/findings")),
            grammar: arg("--grammar"),
            probe: arg("--probe").unwrap_or_else(|| PathBuf::from("target/zip-aosp-runtime-probe")),
            asan_probe: arg("--asan-probe"),
            arms_csv: arg("--arms").unwrap_or_default(),
            arms_sample_rate: arg("--arms-sample-rate").unwrap_or(1),
            metrics_bind: arg("--metrics"),
            cvd_root: arg("--cvd-root"),
            budget: arg::<u64>("--budget").map(Duration::from_secs),
            iters: arg("--iters"),
            seed: arg("--seed").unwrap_or(0xb113_d1ff_d1ff_0001),
            log_every: arg("--log-every").unwrap_or(500),
            min_findings_gate: arg("--min-findings-gate"),
            min_e_findings_gate: arg("--min-e-gate"),
            max_io_errors_gate: arg("--max-io-errors"),
            probe_timeout_ms: arg("--probe-timeout-ms").unwrap_or(5_000),
            probes_csv: arg("--probes").unwrap_or_default(),
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
        } else {
            let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
            if ext == "bin" || ext == "apk" {
                let bytes = std::fs::read(&p)?;
                out.push((p, bytes));
            }
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
    asan_findings: u64,
    distinct_finding_ids: std::collections::HashSet<String>,
}

impl Counters {
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
            self.distinct_finding_ids.insert(finding_id.to_string());
        }
    }
    /// Honest finding count — D + E only. C is taxonomy delta.
    fn honest_findings(&self) -> u64 {
        self.bucket_d + self.bucket_e
    }
}

fn print_status(c: &Counters, elapsed: Duration) {
    let rate = (c.iters as f64) / elapsed.as_secs_f64().max(0.001);
    println!(
        "  iters={:<8} A={:<8} B={:<8} C={:<6} D={:<6} E={:<6} ASan={:<5} distinct={:<5} D+E={:<5} rate={:>7.0}/s elapsed={:.1}s",
        c.iters,
        c.bucket_a,
        c.bucket_b,
        c.bucket_c,
        c.bucket_d,
        c.bucket_e,
        c.asan_findings,
        c.distinct_finding_ids.len(),
        c.honest_findings(),
        rate,
        elapsed.as_secs_f64()
    );
    let _ = std::io::stdout().flush();
}

fn install_signal_handler(stop: Arc<AtomicBool>) {
    // Real SIGINT/SIGTERM handler (Gap-17 closure). `signal-hook`
    // is `#![forbid(unsafe_code)]`-clean at its public surface —
    // the unsafe is sealed inside `signal-hook-registry`. We
    // register both signals against a single Iterator and spawn
    // a dedicated thread that drains it; on first delivery the
    // stop flag flips, the main loop observes it on its next
    // iteration, and the driver shuts down cleanly (flushing
    // ndjson, killing probes, returning a successful exit code).
    //
    // A second signal of the same kind exits the process hard —
    // operators expect Ctrl-C twice to mean "I really mean it".
    use signal_hook::consts::{SIGINT, SIGTERM};
    use signal_hook::iterator::Signals;
    let mut signals = match Signals::new([SIGINT, SIGTERM]) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("WARN signal-hook registration failed: {e} (continuing without handler)");
            return;
        }
    };
    let stop_thread = Arc::clone(&stop);
    std::thread::Builder::new()
        .name("p113-signal-handler".into())
        .spawn(move || {
            let mut count: u32 = 0;
            for sig in signals.forever() {
                count += 1;
                let label = match sig {
                    SIGINT => "SIGINT",
                    SIGTERM => "SIGTERM",
                    _ => "signal",
                };
                if count == 1 {
                    eprintln!(
                        "{label} received — initiating clean shutdown (send again to force-exit)"
                    );
                    stop_thread.store(true, Ordering::Relaxed);
                } else {
                    eprintln!("{label} again — force-exiting");
                    std::process::exit(130);
                }
            }
        })
        .expect("signal handler thread spawn");
}

fn shard_input_path(sha: &str) -> String {
    // Hash-shard inputs/<aa>/<bb>/<sha>.bin so 100K+ findings
    // don't pile up in one directory (Gap-18).
    let aa = &sha[0..2];
    let bb = &sha[2..4];
    format!("inputs/{aa}/{bb}/{sha}.bin")
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    println!("{VERSION}");
    println!(
        "  mode={}  seeds={}  archive={}  probe={}  seed={:#018x}  arms={}  metrics={}",
        args.mode,
        args.seeds.display(),
        args.archive.display(),
        args.probe.display(),
        args.seed,
        if args.arms_csv.is_empty() {
            "(none)"
        } else {
            &args.arms_csv
        },
        args.metrics_bind.as_deref().unwrap_or("(off)"),
    );

    // Cuttlefish probe (real-mode fallback).
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

    // Grammar (loadability gate).
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

    // Seeds.
    let seeds = load_seeds(&args.seeds)?;
    if seeds.is_empty() {
        eprintln!(
            "ERROR: no seeds found under {} — run `make p113-corpus-seed` first",
            args.seeds.display()
        );
        std::process::exit(2);
    }
    println!("  seeds: {} files loaded", seeds.len());

    // Probes.
    if !args.probe.exists() {
        eprintln!(
            "ERROR: AOSP probe not found at {} — run `make p16-aosp-runtime-probe` first",
            args.probe.display()
        );
        std::process::exit(2);
    }
    let probe_timeout = Duration::from_millis(args.probe_timeout_ms);
    let primary_probe = PersistentProbe::spawn("aosp-libziparchive-runtime", &args.probe)?
        .with_timeout(probe_timeout);
    println!(
        "  primary-probe: {} (persistent, timeout={}ms)",
        primary_probe.label(),
        args.probe_timeout_ms
    );
    let asan_probe = match &args.asan_probe {
        Some(p) if p.exists() => {
            let pp = PersistentProbe::spawn("aosp-libziparchive-asan", p)?
                .with_timeout(probe_timeout);
            println!("  asan-probe   : {} (persistent, timeout={}ms)", pp.label(), args.probe_timeout_ms);
            Some(pp)
        }
        Some(p) => {
            eprintln!("WARN: asan probe {} missing — disabling", p.display());
            None
        }
        None => None,
    };
    let third_arms: Vec<Box<dyn Arm>> = arms_from_csv(&args.arms_csv);
    if !third_arms.is_empty() {
        println!("  third-arms   : {}", args.arms_csv);
    }

    // Cross-version probes (P1.14 §A) — built on top of the
    // primary A14 probe. Each entry is either a path to a real
    // per-version probe binary or the literal `synthetic` token.
    // Synthetic entries reuse the primary A14 probe and apply the
    // documented per-version filter list from `version_probes.rs`.
    let cross_version_probes: Vec<VersionedProbe> = if args.probes_csv.is_empty() {
        Vec::new()
    } else {
        let mut out: Vec<VersionedProbe> = Vec::new();
        for (version, path) in parse_probes_csv(&args.probes_csv) {
            let p_str = path.to_str().unwrap_or("");
            if p_str == "synthetic" {
                // Synthetic probes share the primary A14 probe.
                // We can't move primary_probe in (still in use),
                // so spawn a new dedicated A14 base for each
                // synthetic version. This is intentional: it
                // avoids serialising every cross-version request
                // through the same probe child.
                match PersistentProbe::spawn(
                    &format!("aosp-libziparchive-base-{}", version.label().to_lowercase()),
                    &args.probe,
                ) {
                    Ok(base) => {
                        let vp = VersionedProbe::synthetic_layer(
                            version,
                            base.with_timeout(probe_timeout),
                        );
                        println!("  xv-probe     : {} (synthetic)", vp.label);
                        out.push(vp);
                    }
                    Err(e) => eprintln!(
                        "WARN failed to spawn synthetic base for {}: {e}",
                        version.label()
                    ),
                }
            } else {
                if !path.exists() {
                    eprintln!("WARN xv probe {} missing — skipping", path.display());
                    continue;
                }
                match VersionedProbe::real(version, &path, probe_timeout) {
                    Ok(vp) => {
                        println!("  xv-probe     : {} (real)", vp.label);
                        out.push(vp);
                    }
                    Err(e) => eprintln!(
                        "WARN failed to spawn real probe for {} at {}: {e}",
                        version.label(),
                        path.display()
                    ),
                }
            }
        }
        out
    };

    // Archive.
    let writer = Arc::new(ArchiveWriter::open(&args.archive)?);

    // Metrics.
    let metrics = Arc::new(Metrics::default());
    let _exporter = match &args.metrics_bind {
        Some(b) => match Exporter::start(b, Arc::clone(&metrics)) {
            Ok(e) => {
                println!("  metrics      : http://{} /metrics", e.bind);
                Some(e)
            }
            Err(e) => {
                eprintln!("WARN metrics exporter on {b} failed: {e}");
                None
            }
        },
        None => None,
    };

    // Stop flag (budget watchdog).
    let stop = Arc::new(AtomicBool::new(false));
    install_signal_handler(Arc::clone(&stop));
    if let Some(budget) = args.budget {
        let s = Arc::clone(&stop);
        std::thread::spawn(move || {
            std::thread::sleep(budget);
            s.store(true, Ordering::Relaxed);
        });
    }

    let mut rng = Lcg::new(args.seed);
    let mut counters = Counters::default();
    let coverage = CoverageMap::new();
    let mut new_edge_count: u64 = 0;
    // Coverage-guided feedback (Gap-14): inputs that hit a new
    // edge get added to a queue. Subsequent mutations sample
    // 50/50 from the original seeds vs the new-edge queue,
    // dramatically improving exploration over blind random.
    // Kept bounded (1024 entries) so memory doesn't grow
    // unbounded over a multi-day soak; eviction is FIFO.
    let mut edge_queue: std::collections::VecDeque<Vec<u8>> =
        std::collections::VecDeque::with_capacity(1024);
    let start = Instant::now();
    let max_iters = args.iters.unwrap_or(u64::MAX);

    while counters.iters < max_iters && !stop.load(Ordering::Relaxed) {
        // 50% of the time, prefer a parent from the new-edge
        // queue if non-empty; otherwise fall back to the seed
        // pool. This is the AFL-style "queue cycling" the dev-
        // mode loop runs without sancov.
        let from_queue = !edge_queue.is_empty() && (rng.next_u32() & 1 == 0);
        let (origin, base): (PathBuf, Vec<u8>) = if from_queue {
            let idx = (rng.next_u32() as usize) % edge_queue.len();
            (PathBuf::from("queue"), edge_queue[idx].clone())
        } else {
            let i = (rng.next_u32() as usize) % seeds.len();
            (seeds[i].0.clone(), seeds[i].1.clone())
        };
        let j = (rng.next_u32() as usize) % seeds.len();
        let aux = Some(seeds[j].1.as_slice());
        let (mutated, kind) = mutate(&mut rng, &base, aux, grammar.as_ref());

        let iter_t0 = Instant::now();
        let axiom = differ::run_axiom(&mutated);
        let target = match primary_probe.run_one(&mutated) {
            Ok(v) => v,
            Err(e) => {
                let is_timeout = e.kind() == std::io::ErrorKind::TimedOut;
                if is_timeout {
                    metrics.probe_timeouts.fetch_add(1, Ordering::Relaxed);
                } else {
                    eprintln!("WARN primary probe i={}: {e}", counters.iters);
                    counters.target_io_errors += 1;
                    metrics
                        .target_io_errors_total
                        .fetch_add(1, Ordering::Relaxed);
                }
                counters.iters += 1;
                continue;
            }
        };
        let bucket = classify(&axiom, &target);
        let new_edge = coverage.observe(&axiom, &target);
        if new_edge {
            new_edge_count += 1;
            // Add this input to the new-edge queue so subsequent
            // mutations can derive from it. FIFO eviction at 1024.
            if edge_queue.len() >= 1024 {
                edge_queue.pop_front();
            }
            edge_queue.push_back(mutated.clone());
        }
        let dt = iter_t0.elapsed().as_nanos() as u64;
        metrics
            .iter_duration_ns_sum
            .fetch_add(dt, Ordering::Relaxed);
        metrics.iters_total.fetch_add(1, Ordering::Relaxed);
        match bucket {
            Bucket::A => metrics.bucket_a.fetch_add(1, Ordering::Relaxed),
            Bucket::B => metrics.bucket_b.fetch_add(1, Ordering::Relaxed),
            Bucket::C => metrics.bucket_c.fetch_add(1, Ordering::Relaxed),
            Bucket::D => metrics.bucket_d.fetch_add(1, Ordering::Relaxed),
            Bucket::E => metrics.bucket_e.fetch_add(1, Ordering::Relaxed),
        };

        // Hash-sharded input dir for the primary finding (if any).
        let mut maybe_path: Option<String> = None;
        let mut input_sha = String::new();
        if bucket.is_finding() {
            input_sha = save_sharded(&writer, &mutated)?;
            maybe_path = Some(shard_input_path(&input_sha));
            metrics.findings_total.fetch_add(1, Ordering::Relaxed);
            let finding = Finding::from_verdicts(
                &args.mode,
                "aosp-libziparchive-runtime",
                &mutated,
                maybe_path.as_deref().unwrap_or("inputs/<missing>"),
                axiom.clone(),
                target.clone(),
                bucket,
                Some(format!("{}", origin.display())),
                Some(kind.label().to_string()),
            );
            counters.record(bucket, &finding.finding_id);
            writer.append(&finding)?;
        } else {
            counters.record(bucket, "");
        }
        metrics.distinct_findings.store(
            counters.distinct_finding_ids.len() as u64,
            Ordering::Relaxed,
        );

        // Cross-version probe arm — every per-version probe sees
        // the same input. Each probe emits its own Finding record,
        // tagged with `target_version` + `synthetic`. The
        // classifier (p114-classify) groups by input_sha256 and
        // sorts cross-version disagreements into the
        // `cross-version-evasion` label.
        if !cross_version_probes.is_empty() {
            let xv_path = match &maybe_path {
                Some(p) => p.clone(),
                None => {
                    let sha = save_sharded(&writer, &mutated)?;
                    shard_input_path(&sha)
                }
            };
            for vp in &cross_version_probes {
                let v_verdict = match vp.run_one(&mutated) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let v_bucket = classify(&axiom, &v_verdict);
                if !v_bucket.is_finding() {
                    // Every probe writes A/B records too so the
                    // classifier sees the full version verdict
                    // matrix; without an A record for one version,
                    // a cross-version disagreement could be
                    // miscategorised as model-bug.
                }
                let f = Finding::from_verdicts_versioned(
                    &args.mode,
                    &vp.label,
                    vp.version.label(),
                    vp.synthetic,
                    &mutated,
                    &xv_path,
                    axiom.clone(),
                    v_verdict,
                    v_bucket,
                    Some(format!("{}", origin.display())),
                    Some(kind.label().to_string()),
                );
                writer.append(&f)?;
            }
        }

        // ASan arm — log SEPARATE finding if it diverges from the
        // primary probe's verdict (an ASan crash surfaces as a
        // pipe-broken / non-zero exit, which `PersistentProbe` reports
        // as `Reject(malformed:...)`). Any non-`Accept`/non-matching
        // verdict from the ASan arm is a real C++ UB finding.
        if let Some(asan) = &asan_probe {
            if let Ok(asan_v) = asan.run_one(&mutated) {
                if !asan_v.is_accept() && !verdicts_compatible(&target, &asan_v) {
                    let path = if let Some(p) = &maybe_path {
                        p.clone()
                    } else {
                        let sha = save_sharded(&writer, &mutated)?;
                        shard_input_path(&sha)
                    };
                    let _ = input_sha;
                    let asan_bucket = classify(&axiom, &asan_v);
                    let f = Finding::from_verdicts(
                        &args.mode,
                        "aosp-libziparchive-asan",
                        &mutated,
                        &path,
                        axiom.clone(),
                        asan_v,
                        asan_bucket,
                        Some(format!("{}", origin.display())),
                        Some(kind.label().to_string()),
                    );
                    writer.append(&f)?;
                    counters.asan_findings += 1;
                }
            }
        }

        // Third arms — same protocol, one finding-record per arm
        // that disagrees with axiom-l0. Rate-limited via the
        // `--arms-sample-rate N` flag (only every Nth iteration
        // hits the third arms, since they spawn a child per input
        // and would otherwise dominate the loop).
        let run_third =
            args.arms_sample_rate > 0 && counters.iters % args.arms_sample_rate as u64 == 0;
        if !run_third {
            if counters.iters > 0 && counters.iters % args.log_every == 0 {
                print_status(&counters, start.elapsed());
            }
            continue;
        }
        for arm in &third_arms {
            let v = match arm.run(&mutated) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let arm_bucket = classify(&axiom, &v);
            if arm_bucket.is_finding() {
                let path = if let Some(p) = &maybe_path {
                    p.clone()
                } else {
                    let sha = save_sharded(&writer, &mutated)?;
                    shard_input_path(&sha)
                };
                let f = Finding::from_verdicts(
                    &args.mode,
                    arm.label(),
                    &mutated,
                    &path,
                    axiom.clone(),
                    v,
                    arm_bucket,
                    Some(format!("{}", origin.display())),
                    Some(kind.label().to_string()),
                );
                writer.append(&f)?;
            }
        }

        if counters.iters > 0 && counters.iters % args.log_every == 0 {
            print_status(&counters, start.elapsed());
        }
    }

    print_status(&counters, start.elapsed());
    println!();
    println!("=== summary ===");
    println!("  total iters             : {}", counters.iters);
    println!("  target io errors        : {}", counters.target_io_errors);
    println!(
        "  primary-probe timeouts  : {}",
        primary_probe.timed_out()
    );
    if let Some(p) = asan_probe.as_ref() {
        println!("  asan-probe   timeouts   : {}", p.timed_out());
    }
    println!("  bucket A (both ok)      : {}", counters.bucket_a);
    println!("  bucket B (same tag)     : {}", counters.bucket_b);
    println!("  bucket C (taxonomy)     : {}", counters.bucket_c);
    println!("  bucket D (axiom lax)    : {}", counters.bucket_d);
    println!("  bucket E (axiom strict) : {}", counters.bucket_e);
    println!("  ASan-arm findings       : {}", counters.asan_findings);
    println!(
        "  honest findings (D+E)   : {} (gate >= {})",
        counters.honest_findings(),
        args.min_findings_gate.unwrap_or(0)
    );
    println!(
        "  distinct finding shas   : {}",
        counters.distinct_finding_ids.len()
    );
    println!(
        "  archive                 : {}",
        writer.archive_path().display()
    );
    println!("  primary-probe served    : {}", primary_probe.served());
    println!("  distinct edges          : {}", coverage.distinct_edges());
    println!("  new edges this run      : {}", new_edge_count);

    let mut gate_failed = false;
    if let Some(min) = args.min_findings_gate {
        if counters.honest_findings() < min {
            eprintln!(
                "::error::p113-fuzz-driver: honest findings (D+E) {} below gate {}",
                counters.honest_findings(),
                min
            );
            gate_failed = true;
        }
    }
    if let Some(min) = args.min_e_findings_gate {
        if counters.bucket_e < min {
            eprintln!(
                "::error::p113-fuzz-driver: bucket-E findings {} below gate {} (potential CVE-class regression: target stopped accepting what verified rejects)",
                counters.bucket_e,
                min
            );
            gate_failed = true;
        }
    }
    if let Some(max) = args.max_io_errors_gate {
        if counters.target_io_errors > max {
            eprintln!(
                "::error::p113-fuzz-driver: target IO errors {} above gate {} (probe is unstable)",
                counters.target_io_errors, max
            );
            gate_failed = true;
        }
    }
    if gate_failed {
        std::process::exit(1);
    }

    Ok(())
}

/// Save mutated input bytes hash-sharded under
/// `<archive_root>/inputs/<aa>/<bb>/<sha>.bin`. Returns the sha.
fn save_sharded(writer: &Arc<ArchiveWriter>, input: &[u8]) -> std::io::Result<String> {
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

fn verdicts_compatible(a: &Verdict, b: &Verdict) -> bool {
    match (a, b) {
        (Verdict::Accept, Verdict::Accept) => true,
        (Verdict::Reject(x), Verdict::Reject(y)) => x == y,
        _ => false,
    }
}
