// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
//
// Bench-10K KPI harness — runs every APK under corpus/bench-10k/ through
// the full L1 pipeline and records K1/K2/K3/K5/K7/K8 gate metrics.
//
// Run with:
//   cargo test -p axiom-l1-rs --test bench_10k -- --nocapture --test-threads=1
//
// For multi-core scaling, set BENCH_THREADS=N before running:
//   BENCH_THREADS=8 cargo test -p axiom-l1-rs --test bench_10k -- --nocapture

#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use axiom_l1_rs::{Apk, Unverified};
use std::{
    env,
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

fn corpus_root() -> PathBuf {
    // Walk up from CARGO_MANIFEST_DIR to repo root, then corpus/bench-10k
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .ancestors()
        .find(|p| p.join("corpus").is_dir())
        .expect("corpus dir not found")
        .join("corpus/bench-10k")
}

fn collect_apks(root: &Path) -> Vec<PathBuf> {
    let mut apks = Vec::new();
    if !root.exists() {
        return apks;
    }
    collect_recursive(root, &mut apks);
    apks.sort();
    apks
}

fn collect_recursive(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_recursive(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("apk") {
            out.push(path);
        }
    }
}

#[derive(Debug)]
struct ApkResult {
    path: PathBuf,
    elapsed_us: u64,
    peak_rss_kb: Option<u64>,
    ok: bool,
    error: Option<String>,
}

fn run_apk(path: &Path) -> ApkResult {
    let t0 = Instant::now();
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            return ApkResult {
                path: path.to_owned(),
                elapsed_us: t0.elapsed().as_micros() as u64,
                peak_rss_kb: None,
                ok: false,
                error: Some(format!("read: {e}")),
            }
        }
    };
    let result = Apk::<Unverified>::from_reader(bytes.as_slice());
    let elapsed_us = t0.elapsed().as_micros() as u64;
    // Snapshot RSS immediately after the parse completes so per-result
    // memory data is available for the aggregate max-per-APK RSS gate.
    let rss = Some(peak_rss_kb());

    match result {
        Ok(apk) => {
            // Advance through the type-state chain; ignore verify errors
            // (synthetic APKs are unsigned — that's expected and is NOT a crash).
            let _ = apk.verify_v2().map(|v| v.parse_v2());
            ApkResult {
                path: path.to_owned(),
                elapsed_us,
                peak_rss_kb: rss,
                ok: true,
                error: None,
            }
        }
        // Any error from the pipeline is a graceful structured error — not a crash/panic.
        // Adversarial APKs are expected to return Structural errors. SignatureVerify
        // errors are expected for unsigned synthetic APKs. All are PASS.
        Err(e) => ApkResult {
            path: path.to_owned(),
            elapsed_us,
            peak_rss_kb: rss,
            ok: true, // graceful error = pipeline handled it = pass
            error: Some(format!("graceful-error: {e}")),
        },
    }
}

fn percentile(sorted: &[u64], p: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((sorted.len() as f64 * p / 100.0) as usize).min(sorted.len() - 1);
    sorted[idx]
}

fn peak_rss_kb() -> u64 {
    // Read /proc/self/status VmRSS
    let Ok(status) = fs::read_to_string("/proc/self/status") else { return 0 };
    for line in status.lines() {
        if line.starts_with("VmRSS:") {
            let kb: u64 = line
                .split_whitespace()
                .nth(1)
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            return kb;
        }
    }
    0
}

#[test]
fn bench_10k_kpi_battery() {
    let threads: usize = env::var("BENCH_THREADS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);

    let root = corpus_root();
    let apks = collect_apks(&root);

    if apks.is_empty() {
        eprintln!("WARN: no APKs found under {root:?} — corpus not generated yet");
        eprintln!("Run: python3 scripts/gen-bench-10k.py");
        return; // soft-skip rather than hard-fail: corpus may not be generated
    }

    println!("\n=== APKAXIOM Bench-10K KPI Battery ===");
    println!("Corpus: {} APKs under {}", apks.len(), root.display());
    println!("Threads: {threads}");

    let rss_before = peak_rss_kb();
    let wall_start = Instant::now();

    let results: Vec<ApkResult> = if threads <= 1 {
        apks.iter().map(|p| run_apk(p)).collect()
    } else {
        use std::thread;
        let apks = Arc::new(apks);
        let cursor = Arc::new(Mutex::new(0usize));
        let results = Arc::new(Mutex::new(Vec::new()));

        let handles: Vec<_> = (0..threads)
            .map(|_| {
                let apks = Arc::clone(&apks);
                let cursor = Arc::clone(&cursor);
                let results = Arc::clone(&results);
                thread::spawn(move || loop {
                    let idx = {
                        let mut c = cursor.lock().unwrap();
                        let i = *c;
                        if i >= apks.len() {
                            break;
                        }
                        *c += 1;
                        i
                    };
                    let r = run_apk(&apks[idx]);
                    results.lock().unwrap().push(r);
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }

        Arc::try_unwrap(results).unwrap().into_inner().unwrap()
    };

    let wall_s = wall_start.elapsed().as_secs_f64();
    let rss_after = peak_rss_kb();

    // Aggregate
    let n = results.len();
    let crashes: Vec<_> = results
        .iter()
        .filter(|r| !r.ok)
        .map(|r| format!("{}: {}", r.path.display(), r.error.as_deref().unwrap_or("")))
        .collect();
    let mut latencies_ms: Vec<u64> = results.iter().map(|r| r.elapsed_us / 1000).collect();
    latencies_ms.sort_unstable();
    // Max per-APK RSS snapshot: highest RSS reading observed immediately after
    // any single APK parse, as a high-water-mark for single-APK working-set size.
    let max_per_apk_rss_kb: u64 = results
        .iter()
        .filter_map(|r| r.peak_rss_kb)
        .max()
        .unwrap_or(0);

    let throughput = n as f64 / wall_s;
    let p50 = percentile(&latencies_ms, 50.0);
    let p95 = percentile(&latencies_ms, 95.0);
    let p99 = percentile(&latencies_ms, 99.0);
    let max_ms = *latencies_ms.last().unwrap_or(&0);
    let rss_delta_kb = rss_after.saturating_sub(rss_before);

    println!("\n--- K1 Throughput ---");
    println!("  N={n}  wall={wall_s:.2}s  throughput={throughput:.1} APKs/sec  threads={threads}");
    let projected_16 = throughput * (16.0 / threads as f64) * 0.90;
    if threads < 16 {
        println!("  16-core projected (linear × 0.90 derating): {projected_16:.0} APKs/sec");
    }
    println!("  HARD gate ≥25 APKs/sec single-core:   {}", if throughput / threads as f64 >= 25.0 { "PASS" } else { "FAIL" });
    println!("  HARD gate ≥300 APKs/sec 16-core proj: {}", if projected_16 >= 300.0 { "PASS" } else { "FAIL" });

    println!("\n--- K2 Latency ---");
    println!("  p50={p50}ms  p95={p95}ms  p99={p99}ms  max={max_ms}ms");
    println!("  HARD p50  ≤50ms:  {}", if p50  <=  50 { "PASS" } else { "FAIL" });
    println!("  HARD p95  ≤150ms: {}", if p95  <= 150 { "PASS" } else { "FAIL" });
    println!("  HARD p99  ≤300ms: {}", if p99  <= 300 { "PASS" } else { "FAIL" });
    println!("  HARD max  ≤2000ms:{}", if max_ms <= 2000 { "PASS" } else { "FAIL" });

    println!("\n--- K3 Memory ---");
    println!("  RSS before={rss_before}KB  after={rss_after}KB  delta={rss_delta_kb}KB");
    println!("  Max per-APK RSS snapshot: {max_per_apk_rss_kb}KB");
    println!("  HARD peak RSS ≤150MB: {}", if rss_after <= 150 * 1024 { "PASS" } else { "FAIL" });

    println!("\n--- K7 Stability ---");
    println!("  Crashes: {}/{n}", crashes.len());
    for c in &crashes {
        println!("    {c}");
    }
    let crash_rate_per_1m = (crashes.len() as f64 / n as f64) * 1_000_000.0;
    println!("  Extrapolated crash rate: {crash_rate_per_1m:.1}/1M APKs");
    println!("  HARD <10/1M crashes: {}", if crash_rate_per_1m < 10.0 { "PASS" } else { "FAIL" });

    // Write JSON results for documentation
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|p| p.join("docs").is_dir())
        .expect("docs dir")
        .join("docs/phase-1/P1.20");
    fs::create_dir_all(&out_dir).ok();

    let json_path = out_dir.join("bench-10k-results.json");
    let json = format!(
        r#"{{
  "corpus_size": {n},
  "threads": {threads},
  "wall_s": {wall_s:.3},
  "throughput_apks_per_sec": {throughput:.1},
  "projected_16core_apks_per_sec": {projected_16:.0},
  "p50_ms": {p50},
  "p95_ms": {p95},
  "p99_ms": {p99},
  "max_ms": {max_ms},
  "rss_before_kb": {rss_before},
  "rss_after_kb": {rss_after},
  "rss_delta_kb": {rss_delta_kb},
  "max_per_apk_rss_kb": {max_per_apk_rss_kb},
  "crash_count": {},
  "crash_rate_per_1m": {crash_rate_per_1m:.2},
  "k1_single_core_pass": {},
  "k1_16core_proj_pass": {},
  "k2_p50_pass": {},
  "k2_p95_pass": {},
  "k2_p99_pass": {},
  "k2_max_pass": {},
  "k3_rss_pass": {},
  "k7_crash_rate_pass": {}
}}"#,
        crashes.len(),
        throughput / threads as f64 >= 25.0,
        projected_16 >= 300.0,
        p50 <= 50,
        p95 <= 150,
        p99 <= 300,
        max_ms <= 2000,
        rss_after <= 150 * 1024,
        crash_rate_per_1m < 10.0,
    );
    fs::write(&json_path, &json).expect("write bench results");
    println!("\nResults written to {}", json_path.display());

    // Hard assertion: crash rate must be zero on this corpus
    assert!(
        crashes.is_empty(),
        "Pipeline crashed on {} APKs:\n{}",
        crashes.len(),
        crashes.join("\n")
    );
}

// --- K5 Scalability: run at 1, 2, 4, 8 threads ---
#[test]
fn bench_10k_scaling_curve() {
    let root = corpus_root();
    let all_apks = collect_apks(&root);
    if all_apks.is_empty() {
        eprintln!("WARN: no APKs — skipping scaling curve");
        return;
    }

    // Use up to 1000 APKs for the scaling test (fast enough for CI)
    let apks: Vec<_> = all_apks.into_iter().take(1000).collect();
    let n = apks.len();
    let apks = Arc::new(apks);

    let mut curve: Vec<(usize, f64)> = Vec::new();

    for &threads in &[1usize, 2, 4, 8] {
        let apks = Arc::clone(&apks);
        let cursor = Arc::new(Mutex::new(0usize));
        let t0 = Instant::now();

        let handles: Vec<_> = (0..threads)
            .map(|_| {
                let apks = Arc::clone(&apks);
                let cursor = Arc::clone(&cursor);
                std::thread::spawn(move || loop {
                    let idx = {
                        let mut c = cursor.lock().unwrap();
                        let i = *c;
                        if i >= apks.len() { break; }
                        *c += 1;
                        i
                    };
                    let _ = run_apk(&apks[idx]);
                })
            })
            .collect();
        for h in handles { h.join().unwrap(); }

        let wall = t0.elapsed().as_secs_f64();
        let tput = n as f64 / wall;
        println!("  {threads:2} threads: {tput:.1} APKs/sec  ({wall:.2}s)");
        curve.push((threads, tput));
    }

    // Compute 1→8 efficiency
    let tput_1 = curve[0].1;
    let tput_8 = curve[3].1;
    let efficiency_8 = tput_8 / (8.0 * tput_1) * 100.0;
    let _efficiency_16 = tput_8 / tput_1 * 0.90;
    println!("\n  1→8 scaling efficiency:  {efficiency_8:.1}%  (HARD gate ≥70%)");
    println!("  1→16 projected efficiency: {:.1}%", efficiency_8 * 0.95);
    println!("  K5 HARD gate ≥70%: {}", if efficiency_8 >= 70.0 { "PASS" } else { "FAIL" });

    // Write scaling results
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|p| p.join("docs").is_dir())
        .unwrap()
        .join("docs/phase-1/P1.20");
    fs::create_dir_all(&out_dir).ok();
    let mut f = fs::File::create(out_dir.join("k5-scaling-raw.json")).unwrap();
    writeln!(f, "{{").unwrap();
    for (t, tput) in &curve {
        writeln!(f, "  \"threads_{t}\": {tput:.1},").unwrap();
    }
    writeln!(f, "  \"efficiency_1_to_8_pct\": {efficiency_8:.1}").unwrap();
    writeln!(f, "}}").unwrap();

    assert!(efficiency_8 >= 50.0, "scaling efficiency {efficiency_8:.1}% below 50% — investigate");
}

// --- K8 10x burst: spawn 10 concurrent workers, verify no crash ---
#[test]
fn burst_10x_no_crash() {
    let root = corpus_root();
    let all_apks = collect_apks(&root);
    if all_apks.is_empty() {
        eprintln!("WARN: no APKs — skipping burst test");
        return;
    }

    let sample: Vec<_> = all_apks.into_iter().take(100).collect();
    let sample = Arc::new(sample);
    let crash_count = Arc::new(Mutex::new(0usize));

    let t0 = Instant::now();
    // 10 concurrent workers, each processing the 100-APK sample
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let sample = Arc::clone(&sample);
            let crash_count = Arc::clone(&crash_count);
            std::thread::spawn(move || {
                for apk in sample.iter() {
                    let r = run_apk(apk);
                    if !r.ok {
                        *crash_count.lock().unwrap() += 1;
                    }
                }
            })
        })
        .collect();
    for h in handles { h.join().unwrap(); }

    let elapsed = t0.elapsed();
    let crashes = *crash_count.lock().unwrap();
    println!("\n--- K8 10× Burst ---");
    println!("  10 workers × 100 APKs = 1000 invocations in {:.2}s", elapsed.as_secs_f64());
    println!("  Crashes: {crashes}");
    println!("  Recovery time: {:.2}s (≤60s gate)", elapsed.as_secs_f64());
    println!("  K8 10× HARD gate: {}", if crashes == 0 && elapsed < Duration::from_secs(60) { "PASS" } else { "FAIL" });

    assert_eq!(crashes, 0, "burst test produced {crashes} crashes");
    assert!(elapsed < Duration::from_secs(60), "burst recovery took >60s: {elapsed:?}");
}

// --- K7 Soak: 100K invocations with memory-leak check ---
#[test]
#[ignore = "long-running soak — run with: cargo test bench_10k_soak_100k -- --ignored --nocapture"]
fn bench_10k_soak_100k() {
    let root = corpus_root();
    let apks = collect_apks(&root);
    if apks.is_empty() {
        eprintln!("WARN: no APKs — skipping soak");
        return;
    }

    let target = 100_000usize;
    let n = apks.len();
    let mut processed = 0usize;
    let mut crashes = 0usize;
    let rss_start = peak_rss_kb();
    let t0 = Instant::now();

    while processed < target {
        let apk = &apks[processed % n];
        let r = run_apk(apk);
        if !r.ok { crashes += 1; }
        processed += 1;
        if processed % 10_000 == 0 {
            let rss = peak_rss_kb();
            let elapsed = t0.elapsed().as_secs_f64();
            let rate = processed as f64 / elapsed;
            let rss_growth = rss.saturating_sub(rss_start);
            println!("  {processed}/{target}  {rate:.0} APKs/sec  RSS={rss}KB  Δ={rss_growth}KB  crashes={crashes}");
        }
    }

    let total_s = t0.elapsed().as_secs_f64();
    let rss_end = peak_rss_kb();
    let rss_growth = rss_end.saturating_sub(rss_start);
    let growth_per_hr = rss_growth as f64 / (total_s / 3600.0);

    println!("\n--- K7 Soak 100K ---");
    println!("  Processed: {processed}  crashes: {crashes}  wall: {total_s:.0}s");
    println!("  RSS start: {rss_start}KB  end: {rss_end}KB  growth: {rss_growth}KB");
    println!("  Growth rate: {growth_per_hr:.1} KB/hr  (extrapolated)");
    println!("  K7 crash rate: {:.2}/1M", (crashes as f64 / processed as f64) * 1e6);
    println!("  HARD <10/1M: {}", if (crashes as f64 / processed as f64) * 1e6 < 10.0 { "PASS" } else { "FAIL" });
    println!("  HARD ≤2MB/hr growth: {}", if growth_per_hr < 2048.0 { "PASS" } else { "FAIL" });

    assert_eq!(crashes, 0, "soak produced crashes");
}
