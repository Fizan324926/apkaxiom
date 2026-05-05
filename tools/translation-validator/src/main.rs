// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `translation-validator` — P1.9 trust-boundary harness.
//!
//! Runs the Lean LFH evaluator (`lake exe lfh-eval`) and the Rust
//! LFH evaluator (`lfh-eval-rust`) over a corpus of binary LFH
//! inputs and asserts the two produce **byte-identical JSON
//! output** line-for-line. Any divergence fails closed.
//!
//! ## Pipeline
//!
//! 1. Walk a corpus directory of `*.bin` LFH inputs (any file is
//!    treated as an opaque byte blob; the harness doesn't care
//!    whether the input is well-formed).
//! 2. Emit one `<lower-hex>\n` line per input to a temp file.
//! 3. Pipe the file through `lake exe lfh-eval` (Lean evaluator)
//!    and through `lfh-eval-rust` (Rust evaluator).
//! 4. Diff the two outputs line-by-line. Print the first N
//!    divergences. Exit non-zero on any divergence.
//!
//! ## Determinism guarantees
//!
//! - Inputs are sorted by file name before encoding, so the
//!   per-line index is stable across runs.
//! - Both evaluators emit JSON with **fixed field order** and
//!   lowercase fixed-width hex; the cross-language receipt
//!   `<corpus-sha256>-<output-sha256>` is reproducible bit-for-bit.
//!
//! ## Run protocol
//!
//! ```bash
//! tv \
//!   --corpus corpus/zip/lfh-valid \
//!   --corpus corpus/zip/lfh-adversarial \
//!   --lean-bin "lake exe lfh-eval" \
//!   --rust-bin target/release/lfh-eval-rust
//! ```
//!
//! `make tv` wraps this with the standard P1.9 corpus and gates.

#![forbid(unsafe_code)]
#![warn(missing_docs, unreachable_pub)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::needless_range_loop,
    clippy::many_single_char_names,
    clippy::too_long_first_doc_paragraph,
    clippy::too_many_lines,
    clippy::redundant_closure_for_method_calls,
    clippy::missing_const_for_fn,
    clippy::items_after_statements,
    clippy::similar_names,
    clippy::unreadable_literal,
    clippy::manual_let_else,
    clippy::single_match_else,
    clippy::single_match
)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::Instant;

use thiserror::Error;

/// Validator error surface.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ValidatorError {
    /// I/O failure (corpus read, temp-file write, child process spawn).
    #[error("io: {0}")]
    Io(#[from] io::Error),
    /// One of the evaluator child processes returned non-zero.
    #[error("evaluator `{name}` exited with status {status:?}; stderr:\n{stderr}")]
    EvaluatorFailed {
        /// "lean" or "rust".
        name: String,
        /// Exit status (`None` if killed by signal).
        status: Option<i32>,
        /// Captured stderr.
        stderr: String,
    },
    /// Line counts disagree between Lean and Rust.
    #[error("line-count mismatch: lean={lean_lines} rust={rust_lines}")]
    LineCountMismatch {
        /// Number of output lines from the Lean evaluator.
        lean_lines: usize,
        /// Number of output lines from the Rust evaluator.
        rust_lines: usize,
    },
    /// Output bytes diverge on at least one input.
    #[error("{count} divergences across {total} inputs (first divergence at index {first})")]
    Diverges {
        /// Total inputs evaluated.
        total: usize,
        /// Number of divergent inputs.
        count: usize,
        /// Index of the first divergent input.
        first: usize,
    },
}

/// CLI args parsed from argv.
#[derive(Debug)]
struct Args {
    corpora: Vec<PathBuf>,
    lean_cmd: Vec<String>,
    rust_bin: PathBuf,
    /// Optional third evaluator — `tools/lfh-eval-extracted` (the
    /// auto-extracted Rust). When set, the validator diffs all
    /// three arms and writes a three-way receipt.
    extracted_bin: Option<PathBuf>,
    receipt_out: Option<PathBuf>,
    /// Maximum number of divergent inputs to print before aborting
    /// the diff. The harness still counts every divergence in the
    /// final summary.
    max_show: usize,
    /// Skip the actual diff and just report the corpus + receipt
    /// shape. Used when re-running outside the dev shell to verify
    /// the fixture looks reasonable without invoking Lake.
    dry_run: bool,
}

fn parse_args() -> Args {
    let mut a = std::env::args().skip(1);
    let mut corpora = Vec::new();
    let mut lean_cmd: Vec<String> = vec!["lake".into(), "exe".into(), "lfh-eval".into()];
    let mut rust_bin = PathBuf::from("target/release/lfh-eval-rust");
    let mut extracted_bin: Option<PathBuf> = None;
    let mut receipt_out: Option<PathBuf> = None;
    let mut max_show: usize = 5;
    let mut dry_run = false;
    while let Some(arg) = a.next() {
        match arg.as_str() {
            "--corpus" => corpora.push(PathBuf::from(a.next().unwrap_or_default())),
            "--lean-cmd" => {
                let s = a.next().unwrap_or_default();
                lean_cmd = shlex_split_simple(&s);
            }
            "--rust-bin" => rust_bin = PathBuf::from(a.next().unwrap_or_default()),
            "--extracted-bin" => {
                extracted_bin = Some(PathBuf::from(a.next().unwrap_or_default()));
            }
            "--receipt" => receipt_out = Some(PathBuf::from(a.next().unwrap_or_default())),
            "--max-show" => max_show = a.next().and_then(|s| s.parse().ok()).unwrap_or(5),
            "--dry-run" => dry_run = true,
            _ => {
                eprintln!("unknown arg: {arg}");
                std::process::exit(2);
            }
        }
    }
    if corpora.is_empty() {
        corpora.push(PathBuf::from("corpus/zip/lfh-valid"));
    }
    Args {
        corpora,
        lean_cmd,
        rust_bin,
        extracted_bin,
        receipt_out,
        max_show,
        dry_run,
    }
}

/// Minimal whitespace splitter — no quote handling, deliberately
/// simple (no regex / shlex dep). The `--lean-cmd` users are
/// us / CI, not arbitrary shell pipelines.
fn shlex_split_simple(s: &str) -> Vec<String> {
    s.split_whitespace().map(|w| w.to_string()).collect()
}

fn nibble_hex(n: u8) -> u8 {
    if n < 10 {
        b'0' + n
    } else {
        b'a' + (n - 10)
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(nibble_hex(b >> 4) as char);
        out.push(nibble_hex(b & 0x0f) as char);
    }
    out
}

/// Sort all `*.bin` files in every corpus directory by
/// `(corpus-name, file-name)` to give a stable, reproducible
/// per-line index.
fn collect_corpus(corpora: &[PathBuf]) -> io::Result<Vec<(String, Vec<u8>)>> {
    let mut all: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for dir in corpora {
        let dir_name = dir
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("bin") {
                continue;
            }
            let leaf = path.file_name().unwrap().to_string_lossy().into_owned();
            let key = format!("{dir_name}/{leaf}");
            let bytes = fs::read(&path)?;
            all.insert(key, bytes);
        }
    }
    Ok(all.into_iter().collect())
}

/// Concatenate all `bytes` arrays into one big SHA-256 — the
/// "corpus content hash" identifies which inputs the receipt was
/// computed against.
fn corpus_sha256(items: &[(String, Vec<u8>)]) -> [u8; 32] {
    let mut buf = Vec::new();
    for (k, v) in items {
        buf.extend_from_slice(k.as_bytes());
        buf.push(0);
        buf.extend_from_slice(v);
        buf.push(0);
    }
    sha256(&buf)
}

fn write_hex_inputs(items: &[(String, Vec<u8>)], path: &Path) -> io::Result<()> {
    let mut f = fs::File::create(path)?;
    for (_, bytes) in items {
        let hex = hex_encode(bytes);
        f.write_all(hex.as_bytes())?;
        f.write_all(b"\n")?;
    }
    Ok(())
}

fn run_evaluator(
    name: &str,
    cmd: &mut Command,
    input_path: &Path,
) -> Result<String, ValidatorError> {
    let input = fs::File::open(input_path)?;
    let out = cmd
        .stdin(Stdio::from(input))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;
    if !out.status.success() {
        return Err(ValidatorError::EvaluatorFailed {
            name: name.into(),
            status: out.status.code(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        });
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Per-divergence index → corpus filename map (for human-friendly
/// failure reporting).
fn build_index_to_filename(items: &[(String, Vec<u8>)]) -> Vec<&str> {
    items
        .iter()
        .filter(|(_, v)| !v.is_empty())
        .map(|(k, _)| k.as_str())
        .collect()
}

fn diff_outputs(
    lean: &str,
    rust: &str,
    index_to_file: &[&str],
    max_show: usize,
) -> Result<usize, ValidatorError> {
    let lean_lines: Vec<&str> = lean.lines().collect();
    let rust_lines: Vec<&str> = rust.lines().collect();
    if lean_lines.len() != rust_lines.len() {
        return Err(ValidatorError::LineCountMismatch {
            lean_lines: lean_lines.len(),
            rust_lines: rust_lines.len(),
        });
    }
    let total = lean_lines.len();
    let mut divergences: Vec<usize> = Vec::new();
    for (i, (l, r)) in lean_lines.iter().zip(rust_lines.iter()).enumerate() {
        if l != r {
            divergences.push(i);
            if divergences.len() <= max_show {
                let fname = index_to_file.get(i).copied().unwrap_or("?");
                eprintln!("DIVERGE @ index {i} (corpus file: {fname})");
                eprintln!("  lean: {l}");
                eprintln!("  rust: {r}");
            }
        }
    }
    if divergences.is_empty() {
        Ok(total)
    } else {
        Err(ValidatorError::Diverges {
            total,
            count: divergences.len(),
            first: divergences[0],
        })
    }
}

fn run() -> Result<(), ValidatorError> {
    let args = parse_args();
    println!("translation-validator (P1.9):");
    for c in &args.corpora {
        println!("  corpus: {}", c.display());
    }
    println!("  lean:    {}", args.lean_cmd.join(" "));
    println!("  rust:    {}", args.rust_bin.display());
    if let Some(p) = &args.extracted_bin {
        println!("  extr:    {}", p.display());
    }

    let items = collect_corpus(&args.corpora)?;
    let total = items.len();
    let corpus_hash = corpus_sha256(&items);
    println!("  inputs: {total}");
    println!("  corpus-sha256: {}", hex_encode(&corpus_hash));

    let index_to_file = build_index_to_filename(&items);

    if args.dry_run {
        println!("DRY-RUN: skipping evaluator invocations.");
        return Ok(());
    }

    let tmpdir = std::env::temp_dir().join(format!("apkaxiom-tv-{}", std::process::id()));
    fs::create_dir_all(&tmpdir)?;
    let inputs_path = tmpdir.join("inputs.hex");
    write_hex_inputs(&items, &inputs_path)?;

    // Run Lean.
    let t0 = Instant::now();
    let mut lean_cmd = Command::new(&args.lean_cmd[0]);
    lean_cmd.args(&args.lean_cmd[1..]);
    let lean_out = run_evaluator("lean", &mut lean_cmd, &inputs_path)?;
    let lean_secs = t0.elapsed().as_secs_f64();
    println!("  lean elapsed: {lean_secs:.2}s");

    // Run hand-Rust.
    let t0 = Instant::now();
    let mut rust_cmd = Command::new(&args.rust_bin);
    let rust_out = run_evaluator("rust", &mut rust_cmd, &inputs_path)?;
    let rust_secs = t0.elapsed().as_secs_f64();
    println!("  rust elapsed: {rust_secs:.2}s");

    // Optionally run extracted-Rust.
    let extracted_out = if let Some(bin) = &args.extracted_bin {
        let t0 = Instant::now();
        let mut cmd = Command::new(bin);
        let out = run_evaluator("extracted", &mut cmd, &inputs_path)?;
        let secs = t0.elapsed().as_secs_f64();
        println!("  extr elapsed: {secs:.2}s");
        Some(out)
    } else {
        None
    };

    let non_empty = items.iter().filter(|(_, v)| !v.is_empty()).count();
    let skipped = total - non_empty;
    let skipped_note = if skipped > 0 {
        format!(
            " ({skipped} empty input{} skipped)",
            if skipped == 1 { "" } else { "s" }
        )
    } else {
        String::new()
    };

    // Pair-1: Lean ↔ hand-Rust.
    let agreed_lr = diff_outputs(&lean_out, &rust_out, &index_to_file, args.max_show)?;
    println!("AGREE Lean ↔ hand-Rust: {agreed_lr}/{non_empty} byte-identical{skipped_note}.");
    if agreed_lr != non_empty {
        return Err(ValidatorError::LineCountMismatch {
            lean_lines: agreed_lr,
            rust_lines: non_empty,
        });
    }

    // Pair-2 + 3: when extracted_out present, Lean ↔ extracted, hand ↔ extracted.
    if let Some(extr) = &extracted_out {
        let agreed_le = diff_outputs(&lean_out, extr, &index_to_file, args.max_show)?;
        println!("AGREE Lean ↔ extracted-Rust: {agreed_le}/{non_empty} byte-identical.");
        let agreed_re = diff_outputs(&rust_out, extr, &index_to_file, args.max_show)?;
        println!("AGREE hand-Rust ↔ extracted-Rust: {agreed_re}/{non_empty} byte-identical.");
        if agreed_le != non_empty || agreed_re != non_empty {
            return Err(ValidatorError::LineCountMismatch {
                lean_lines: agreed_le.min(agreed_re),
                rust_lines: non_empty,
            });
        }
        println!("THREE-WAY AGREEMENT: Lean ↔ hand-Rust ↔ extracted-Rust.");
    }

    if let Some(out) = &args.receipt_out {
        let lean_hash = sha256(lean_out.as_bytes());
        let rust_hash = sha256(rust_out.as_bytes());
        let extracted_hash = extracted_out.as_ref().map(|s| sha256(s.as_bytes()));

        // Reproducibility metadata: pin the toolchains we ran
        // against. These are committed into the receipt so a
        // future re-run on a different toolchain produces a
        // visibly different receipt (rather than failing
        // silently or vacuously).
        let lean_toolchain = read_toolchain("lean", &["--version"]);
        let rust_toolchain = read_toolchain("rustc", &["--version"]);
        let lake_manifest_sha =
            file_sha256_hex("lake-manifest.json").unwrap_or_else(|_| "(unavailable)".into());
        let cargo_lock_sha =
            file_sha256_hex("Cargo.lock").unwrap_or_else(|_| "(unavailable)".into());

        let mut receipt = String::new();
        let _ = writeln!(&mut receipt, "tv-receipt v2");
        let _ = writeln!(&mut receipt, "corpus-sha256: {}", hex_encode(&corpus_hash));
        let _ = writeln!(
            &mut receipt,
            "corpus-size: {total} (non-empty: {non_empty}, skipped: {skipped})"
        );
        let _ = writeln!(
            &mut receipt,
            "lean-output-sha256: {}",
            hex_encode(&lean_hash)
        );
        let _ = writeln!(
            &mut receipt,
            "rust-output-sha256: {}",
            hex_encode(&rust_hash)
        );
        if let Some(h) = &extracted_hash {
            let _ = writeln!(&mut receipt, "extracted-output-sha256: {}", hex_encode(h));
            let _ = writeln!(
                &mut receipt,
                "agree: {agreed_lr}/{non_empty} (lean ↔ rust ↔ extracted)"
            );
        } else {
            let _ = writeln!(&mut receipt, "agree: {agreed_lr}/{non_empty} (lean ↔ rust)");
        }
        let _ = writeln!(&mut receipt, "lean-toolchain: {lean_toolchain}");
        let _ = writeln!(&mut receipt, "rust-toolchain: {rust_toolchain}");
        let _ = writeln!(&mut receipt, "lake-manifest-sha256: {lake_manifest_sha}");
        let _ = writeln!(&mut receipt, "cargo-lock-sha256: {cargo_lock_sha}");

        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(out, &receipt)?;
        println!("  receipt: {} ({} bytes)", out.display(), receipt.len());
    }

    let _ = fs::remove_dir_all(&tmpdir);
    Ok(())
}

/// Run a toolchain `--version`-style command, return the first
/// stdout line (or an empty placeholder on failure). Used to pin
/// toolchain versions in the receipt.
fn read_toolchain(prog: &str, args: &[&str]) -> String {
    let out = Command::new(prog).args(args).output();
    match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string(),
        _ => "(unavailable)".into(),
    }
}

/// SHA-256 of the file at `path`, hex-encoded. Returns an
/// `io::Error` if the file isn't readable; the caller decides
/// whether that's fatal.
fn file_sha256_hex(path: &str) -> io::Result<String> {
    let bytes = fs::read(path)?;
    Ok(hex_encode(&sha256(&bytes)))
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("FAIL: {e}");
            ExitCode::from(1)
        }
    }
}

// ---------------------------------------------------------------------
// Inline SHA-256 (FIPS-180-4) — no extra crate dep.
// ---------------------------------------------------------------------

fn sha256(input: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a_2f98,
        0x7137_4491,
        0xb5c0_fbcf,
        0xe9b5_dba5,
        0x3956_c25b,
        0x59f1_11f1,
        0x923f_82a4,
        0xab1c_5ed5,
        0xd807_aa98,
        0x1283_5b01,
        0x2431_85be,
        0x550c_7dc3,
        0x72be_5d74,
        0x80de_b1fe,
        0x9bdc_06a7,
        0xc19b_f174,
        0xe49b_69c1,
        0xefbe_4786,
        0x0fc1_9dc6,
        0x240c_a1cc,
        0x2de9_2c6f,
        0x4a74_84aa,
        0x5cb0_a9dc,
        0x76f9_88da,
        0x983e_5152,
        0xa831_c66d,
        0xb003_27c8,
        0xbf59_7fc7,
        0xc6e0_0bf3,
        0xd5a7_9147,
        0x06ca_6351,
        0x1429_2967,
        0x27b7_0a85,
        0x2e1b_2138,
        0x4d2c_6dfc,
        0x5338_0d13,
        0x650a_7354,
        0x766a_0abb,
        0x81c2_c92e,
        0x9272_2c85,
        0xa2bf_e8a1,
        0xa81a_664b,
        0xc24b_8b70,
        0xc76c_51a3,
        0xd192_e819,
        0xd699_0624,
        0xf40e_3585,
        0x106a_a070,
        0x19a4_c116,
        0x1e37_6c08,
        0x2748_774c,
        0x34b0_bcb5,
        0x391c_0cb3,
        0x4ed8_aa4a,
        0x5b9c_ca4f,
        0x682e_6ff3,
        0x748f_82ee,
        0x78a5_636f,
        0x84c8_7814,
        0x8cc7_0208,
        0x90be_fffa,
        0xa450_6ceb,
        0xbef9_a3f7,
        0xc671_78f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09_e667,
        0xbb67_ae85,
        0x3c6e_f372,
        0xa54f_f53a,
        0x510e_527f,
        0x9b05_688c,
        0x1f83_d9ab,
        0x5be0_cd19,
    ];
    let bit_len = (input.len() as u64) * 8;
    let mut msg = input.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes(chunk[i * 4..i * 4 + 4].try_into().unwrap());
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] =
            [h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]];
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    let mut out = [0u8; 32];
    for i in 0..8 {
        out[i * 4..i * 4 + 4].copy_from_slice(&h[i].to_be_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_known_answers() {
        // FIPS 180-4 test vectors.
        let empty: [u8; 32] = [
            0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f,
            0xb9, 0x24, 0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b,
            0x78, 0x52, 0xb8, 0x55,
        ];
        assert_eq!(sha256(b""), empty);
        let abc: [u8; 32] = [
            0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
            0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
            0xf2, 0x00, 0x15, 0xad,
        ];
        assert_eq!(sha256(b"abc"), abc);
        // 56-byte boundary case (right at `% 64 == 56`, the
        // padding edge) — `abcdbcdecdefdefgefghfghighijhijkijkljklm
        // klmnlmnomnopnopq` (the FIPS 56-byte test vector).
        let two_blocks_input = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
        let two_blocks: [u8; 32] = [
            0x24, 0x8d, 0x6a, 0x61, 0xd2, 0x06, 0x38, 0xb8, 0xe5, 0xc0, 0x26, 0x93, 0x0c, 0x3e,
            0x60, 0x39, 0xa3, 0x3c, 0xe4, 0x59, 0x64, 0xff, 0x21, 0x67, 0xf6, 0xec, 0xed, 0xd4,
            0x19, 0xdb, 0x06, 0xc1,
        ];
        assert_eq!(sha256(two_blocks_input), two_blocks);
    }

    /// Cross-check the inline FIPS-180-4 implementation against
    /// the system `sha256sum` on a randomised payload (P1.9 §IV
    /// gap 11). If our implementation drifts (typo, off-by-one,
    /// table corruption), this test catches it. Skipped if
    /// `sha256sum` isn't available — but the dev shell's
    /// `coreutils` always provides it.
    #[test]
    fn sha256_cross_check_against_system_sha256sum() {
        use std::process::{Command, Stdio};
        // Quasi-random payload (LCG-seeded — deterministic).
        let mut payload = Vec::with_capacity(8192);
        let mut s: u64 = 0xdead_beef_dead_beef;
        for _ in 0..8192 {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            payload.push((s >> 32) as u8);
        }
        let ours = sha256(&payload);
        let mut child = match Command::new("sha256sum")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(_) => {
                eprintln!("sha256sum not available — skipping cross-check");
                return;
            }
        };
        use std::io::Write;
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(&payload)
            .expect("write to sha256sum stdin");
        drop(child.stdin.take());
        let out = child.wait_with_output().expect("sha256sum exit");
        let line = String::from_utf8_lossy(&out.stdout);
        let theirs_hex = line.split_whitespace().next().unwrap_or("");
        let ours_hex = hex_encode(&ours);
        assert_eq!(
            ours_hex, theirs_hex,
            "inline SHA-256 disagrees with system sha256sum"
        );
    }

    #[test]
    fn hex_round_trip() {
        let s = hex_encode(&[0xde, 0xad, 0xbe, 0xef]);
        assert_eq!(s, "deadbeef");
    }

    #[test]
    fn diff_finds_first_divergence() {
        let lean = "{\"i\":0}\n{\"i\":1}\n";
        let rust = "{\"i\":0}\n{\"i\":1,\"x\":1}\n";
        let res = diff_outputs(lean, rust, &[], 1);
        assert!(matches!(
            res,
            Err(ValidatorError::Diverges {
                total: 2,
                count: 1,
                first: 1,
            })
        ));
    }

    #[test]
    fn diff_line_count_mismatch() {
        let lean = "a\nb\n";
        let rust = "a\n";
        let res = diff_outputs(lean, rust, &[], 1);
        assert!(matches!(
            res,
            Err(ValidatorError::LineCountMismatch {
                lean_lines: 2,
                rust_lines: 1,
            })
        ));
    }
}
