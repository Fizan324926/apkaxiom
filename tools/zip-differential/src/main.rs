// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `zip-differential` — P1.5 Lean ↔ Rust differential harness.
//!
//! For every sample under `corpus/zip/`, runs:
//!
//!   - the Rust reference parser (`axiom_zip_ref::parse_lfh` /
//!     `parse_eocd`), and
//!
//!   - the Lean reference parser (via `lake env lean --run` on a
//!     small driver script that prints the parse verdict to stdout).
//!
//! The harness asserts (verdict, error_tag) agreement on every
//! sample. The reference parser tag enums in
//! `axiom-zip-ref::lfh::ParseError::tag` and Lean's
//! `Apkaxiom.Zip.LocalHeader.ParseError.tag` agree by construction
//! (numerical 1..=4); the harness diffs the integers.
//!
//! Output: `tools/zip-differential/differential-summary.json` with
//! per-sample agreement and aggregate `pass / fail` counts. Exit 1
//! on any disagreement.

#![forbid(unsafe_code)]
#![warn(missing_docs, unreachable_pub)]
// String-builder + std::process orchestration is verbose by design.
#![allow(
    clippy::too_many_lines,
    clippy::items_after_statements,
    clippy::doc_markdown,
    clippy::default_trait_access,
    clippy::unnecessary_wraps,
    clippy::single_match_else
)]

use std::{
    fmt::Write as _,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, ExitCode, Stdio},
};

use axiom_zip_ref::{eocd, lfh};

/// Verdict shape we serialise for cross-language comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Verdict {
    Ok { consumed: usize },
    Err { tag: u8 },
}

impl Verdict {
    fn from_lfh(r: Result<lfh::ParseOk, lfh::ParseError>) -> Self {
        match r {
            Ok((_, n)) => Self::Ok { consumed: n },
            Err(e) => Self::Err { tag: e.tag() },
        }
    }
    fn from_eocd(r: Result<eocd::ParseOk, eocd::ParseError>) -> Self {
        match r {
            Ok((_, n)) => Self::Ok { consumed: n },
            Err(e) => Self::Err { tag: e.tag() },
        }
    }
    fn render(&self) -> String {
        match self {
            Self::Ok { consumed } => format!("ok {consumed}"),
            Self::Err { tag } => format!("err {tag}"),
        }
    }
}

fn lake_lean_run(driver_path: &Path) -> Result<String, String> {
    let out = Command::new("lake")
        .args(["env", "lean", "--run"])
        .arg(driver_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("spawn lean: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "lean exit {:?}: {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Build a single Lean driver that processes the entire batch of
/// samples in one process. Stdout: one verdict line per sample,
/// in input order.
fn build_batch_driver(samples: &[(SampleKind, Vec<u8>)]) -> String {
    let mut s = String::new();
    s.push_str("import Apkaxiom.Zip.LocalHeader\n");
    s.push_str("import Apkaxiom.Zip.Eocd\n");
    s.push_str("def main : IO Unit := do\n");
    for (kind, bs) in samples {
        let arr = render_byte_array_literal(bs);
        match kind {
            SampleKind::Lfh => {
                s.push_str("  let bs : ByteArray := ");
                s.push_str(&arr);
                s.push('\n');
                s.push_str("  match Apkaxiom.Zip.LocalHeader.parseLfh bs with\n");
                s.push_str("  | .ok (_, n) => IO.println s!\"ok {n}\"\n");
                s.push_str(
                    "  | .error e   => IO.println s!\"err {Apkaxiom.Zip.LocalHeader.ParseError.tag e}\"\n",
                );
            }
            SampleKind::Eocd => {
                s.push_str("  let bs : ByteArray := ");
                s.push_str(&arr);
                s.push('\n');
                s.push_str("  match Apkaxiom.Zip.Eocd.parseEocd bs with\n");
                s.push_str("  | .ok (_, n) => IO.println s!\"ok {n}\"\n");
                s.push_str(
                    "  | .error e   => IO.println s!\"err {Apkaxiom.Zip.Eocd.ParseError.tag e}\"\n",
                );
            }
        }
    }
    s
}

/// Run the C++ AOSP-probe binary against a single sample's bytes
/// (piped over stdin) and parse its `ok N` / `err T` stdout. Any
/// failure path returns a sentinel `Err { tag: 255 }` so the
/// equality check flunks loudly.
fn run_aosp_probe(probe: &str, kind: SampleKind, bs: &[u8]) -> Verdict {
    let mode = match kind {
        SampleKind::Lfh => "--lfh",
        SampleKind::Eocd => "--eocd",
    };
    let Ok(mut child) = Command::new(probe)
        .arg(mode)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    else {
        return Verdict::Err { tag: 255 };
    };
    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write as _;
        let _ = stdin.write_all(bs);
    }
    let Ok(out) = child.wait_with_output() else {
        return Verdict::Err { tag: 255 };
    };
    if !out.status.success() {
        return Verdict::Err { tag: 255 };
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    parse_lean_verdict(stdout.trim()).unwrap_or(Verdict::Err { tag: 255 })
}

fn parse_lean_verdict(line: &str) -> Result<Verdict, String> {
    let mut parts = line.split_whitespace();
    match parts.next() {
        Some("ok") => {
            let n = parts
                .next()
                .ok_or_else(|| format!("ok line missing consumed: {line:?}"))?
                .parse::<usize>()
                .map_err(|e| format!("parse consumed: {e}"))?;
            Ok(Verdict::Ok { consumed: n })
        }
        Some("err") => {
            let t = parts
                .next()
                .ok_or_else(|| format!("err line missing tag: {line:?}"))?
                .parse::<u8>()
                .map_err(|e| format!("parse tag: {e}"))?;
            Ok(Verdict::Err { tag: t })
        }
        _ => Err(format!("unrecognised verdict line: {line:?}")),
    }
}

fn render_byte_array_literal(bs: &[u8]) -> String {
    let mut out = String::new();
    let _ = write!(&mut out, "ByteArray.mk #[");
    for (i, b) in bs.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        let _ = write!(&mut out, "0x{b:02x}");
    }
    out.push(']');
    out
}

#[derive(Clone, Copy)]
enum SampleKind {
    Lfh,
    Eocd,
}

fn run(repo_root: &Path, sample_limit: Option<usize>) -> Result<bool, std::io::Error> {
    std::env::set_current_dir(repo_root)?;
    let corpus = repo_root.join("corpus/zip");
    let tmp_driver =
        std::env::temp_dir().join(format!("apkaxiom-zip-diff-{}.lean", std::process::id()));

    let mut all_pass = true;
    let mut total = 0usize;
    let mut agree = 0usize;
    let mut by_dir: Vec<(String, usize, usize)> = Vec::new();

    // Collect every (kind, bytes, file_path) the harness must
    // process, build a single batch driver, run lean ONCE, then
    // diff verdicts in order.
    type Entry = (SampleKind, Vec<u8>, PathBuf, String);
    let mut all_entries: Vec<Entry> = Vec::new();
    for (sub, kind) in [
        ("lfh-valid", SampleKind::Lfh),
        ("lfh-adversarial", SampleKind::Lfh),
        ("eocd-valid", SampleKind::Eocd),
        ("eocd-adversarial", SampleKind::Eocd),
    ] {
        let dir = corpus.join(sub);
        let mut entries: Vec<PathBuf> = fs::read_dir(&dir)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("bin"))
            .collect();
        entries.sort();
        if let Some(n) = sample_limit {
            entries.truncate(n);
        }
        for p in entries {
            let bs = fs::read(&p)?;
            all_entries.push((kind, bs, p, sub.to_string()));
        }
    }

    eprintln!(
        "Running Lean differential on {} samples …",
        all_entries.len()
    );
    // Single Lean invocations OOM at the workspace scale (~1800 byte
    // arrays inlined into one source file); 50 samples / batch keeps
    // each driver under ~150 KB and well inside Lean's elaborator
    // budget.
    const BATCH_SIZE: usize = 50;
    let mut lean_lines: Vec<String> = Vec::with_capacity(all_entries.len());
    for chunk in all_entries.chunks(BATCH_SIZE) {
        let batch: Vec<(SampleKind, Vec<u8>)> =
            chunk.iter().map(|(k, b, _, _)| (*k, b.clone())).collect();
        let driver = build_batch_driver(&batch);
        fs::write(&tmp_driver, driver)?;
        let stdout = match lake_lean_run(&tmp_driver) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("FAIL: lean batch invocation failed: {e}");
                return Ok(false);
            }
        };
        for line in stdout.lines() {
            lean_lines.push(line.to_string());
        }
    }
    if lean_lines.len() != all_entries.len() {
        eprintln!(
            "FAIL: lean produced {} verdict lines for {} samples",
            lean_lines.len(),
            all_entries.len()
        );
        return Ok(false);
    }

    // Optionally run the C++ AOSP-probe third prong. Path comes
    // from $ZIP_AOSP_PROBE; falls back to `target/zip-aosp-probe`
    // (where `make p15-aosp-probe` writes it). When the binary is
    // missing we still report the Lean ↔ Rust differential — but
    // print a notice that the third prong was skipped. This keeps
    // CI green on environments without g++ while leaving the
    // strong gate when it is available.
    let probe_path = std::env::var("ZIP_AOSP_PROBE").unwrap_or_else(|_| {
        repo_root
            .join("target/zip-aosp-probe")
            .to_string_lossy()
            .into_owned()
    });
    let probe_available = std::path::Path::new(&probe_path).is_file();
    if probe_available {
        eprintln!("Three-way differential — AOSP probe at {probe_path}");
    } else {
        eprintln!("Two-way differential — AOSP probe not at {probe_path} (skipped third prong)");
    }

    let mut per_dir_counts: std::collections::BTreeMap<String, (usize, usize)> = Default::default();
    for ((kind, bs, p, sub), lean_line) in all_entries.iter().zip(lean_lines.iter()) {
        let rust = match kind {
            SampleKind::Lfh => Verdict::from_lfh(lfh::parse_lfh(bs)),
            SampleKind::Eocd => Verdict::from_eocd(eocd::parse_eocd(bs)),
        };
        let lean = parse_lean_verdict(lean_line).unwrap_or(Verdict::Err { tag: 255 });
        let aosp = if probe_available {
            run_aosp_probe(&probe_path, *kind, bs)
        } else {
            // Mirror Rust's verdict so the agreement check still
            // works without the probe; the harness print-out
            // already noted the third prong was skipped.
            rust.clone()
        };
        let agree_one = rust == lean && rust == aosp;
        total += 1;
        let entry = per_dir_counts.entry(sub.clone()).or_insert((0, 0));
        entry.1 += 1;
        if agree_one {
            agree += 1;
            entry.0 += 1;
        } else {
            all_pass = false;
            println!(
                "DISAGREE {} rust={} lean={} aosp={}",
                p.display(),
                rust.render(),
                lean.render(),
                aosp.render()
            );
        }
    }
    for (sub, (a, t)) in &per_dir_counts {
        println!("  {sub}: {a}/{t} agree");
        by_dir.push((sub.clone(), *a, *t));
    }

    let _ = fs::remove_file(&tmp_driver);

    // Write JSON summary.
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str(&format!("  \"total\": {total},\n"));
    s.push_str(&format!("  \"agree\": {agree},\n"));
    s.push_str(&format!("  \"all_pass\": {all_pass},\n"));
    s.push_str("  \"per_dir\": [\n");
    for (i, (d, a, t)) in by_dir.iter().enumerate() {
        let comma = if i + 1 == by_dir.len() { "" } else { "," };
        s.push_str(&format!(
            "    {{\"dir\": \"{d}\", \"agree\": {a}, \"total\": {t}}}{comma}\n"
        ));
    }
    s.push_str("  ]\n}\n");
    let out = repo_root
        .join("tools/zip-differential")
        .join("differential-summary.json");
    let mut f = fs::File::create(out)?;
    f.write_all(s.as_bytes())?;
    println!("\nTOTAL: {agree}/{total} agree (all_pass={all_pass})");
    Ok(all_pass)
}

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().collect();
    let repo_root = match args.get(1) {
        Some(p) => PathBuf::from(p),
        None => {
            eprintln!("usage: zip-differential <repo-root> [--limit N]");
            return ExitCode::from(2);
        }
    };
    let mut sample_limit: Option<usize> = None;
    let mut i = 2;
    while let Some(arg) = args.get(i) {
        if arg == "--limit" {
            sample_limit = args.get(i + 1).and_then(|s| s.parse().ok());
            i += 2;
        } else {
            i += 1;
        }
    }
    match run(&repo_root, sample_limit) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            eprintln!("FAIL: {e}");
            ExitCode::from(1)
        }
    }
}
