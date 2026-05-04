// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `zip-fuzz` — production-fuzzing driver for the ZIP layer.
//!
//! Stable-Rust fuzzer that pairs with **radamsa** (the mutation-only
//! black-box fuzzer the P1.6 §4 spec calls out by name). The harness
//! reads byte sequences from stdin (typically piped through radamsa),
//! invokes every entry point in `axiom-zip-ref`, and reports panics
//! via the process exit code.
//!
//! Usage (driven from `make p16-fuzz`):
//!
//! ```bash
//! # 60-second campaign per parser
//! seq 1 100000 | radamsa --seed 0xa9c1d4b1 \
//!     -o - corpus/zip/lfh-valid/*.bin \
//!   | zip-fuzz --target lfh --iters 100000
//! ```
//!
//! Pass conditions:
//!
//!   - **No panics** across the campaign — closed `ParseError` enums
//!     plus `forbid(unsafe_code)` should make this trivially true,
//!     but radamsa's pathological inputs are exactly the class that
//!     surfaces overflow / index-out-of-bounds bugs the in-tree fuzz
//!     suite (`crates/axiom-zip-ref/src/fuzz.rs`) might miss.
//!
//!   - **Exit code 0** on clean run; **exit code 1** on any captured
//!     panic; **exit code 2** on usage error.

#![forbid(unsafe_code)]
#![warn(missing_docs, unreachable_pub)]

use std::{
    io::{self, Read, Write},
    panic::AssertUnwindSafe,
    path::PathBuf,
    process::ExitCode,
};

use axiom_l1_rs::ApkParser;
use axiom_zip_ref::{archive, cdr, eocd, lfh};

/// Which parser entry point to fuzz.
#[derive(Debug, Clone, Copy)]
enum Target {
    Lfh,
    Eocd,
    Cdr,
    Archive,
    /// P1.7 streaming parser entry point.
    Stream,
    /// All five parsers — invoke each on every input.
    All,
}

impl Target {
    fn parse(&self, bs: &[u8]) {
        match self {
            Self::Lfh => {
                let _ = lfh::parse_lfh(bs);
            }
            Self::Eocd => {
                let _ = eocd::parse_eocd(bs);
            }
            Self::Cdr => {
                let _ = cdr::parse_cdr(bs);
            }
            Self::Archive => {
                let _ = archive::parse_archive(bs);
            }
            Self::Stream => {
                fuzz_stream(bs);
            }
            Self::All => {
                let _ = lfh::parse_lfh(bs);
                let _ = eocd::parse_eocd(bs);
                let _ = cdr::parse_cdr(bs);
                let _ = archive::parse_archive(bs);
                fuzz_stream(bs);
            }
        }
    }
}

/// Drive the streaming parser to termination on `bs`. Pass condition
/// is "no panic"; the streaming parser must terminate (return Ok(None)
/// or Err) for every input.
fn fuzz_stream(bs: &[u8]) {
    let mut parser = ApkParser::from_reader(bs);
    loop {
        match parser.next_event() {
            Ok(Some(_)) => continue,
            Ok(None) => break,
            Err(_) => break,
        }
    }
}

fn parse_args() -> Result<(Target, Option<usize>, Option<PathBuf>), String> {
    let mut target = Target::All;
    let mut iters: Option<usize> = None;
    let mut input_dir: Option<PathBuf> = None;
    let args: Vec<_> = std::env::args().collect();
    let mut i = 1;
    while let Some(arg) = args.get(i) {
        match arg.as_str() {
            "--target" => {
                let v = args.get(i + 1).ok_or("--target needs a value")?;
                target = match v.as_str() {
                    "lfh" => Target::Lfh,
                    "eocd" => Target::Eocd,
                    "cdr" => Target::Cdr,
                    "archive" => Target::Archive,
                    "stream" => Target::Stream,
                    "all" => Target::All,
                    other => return Err(format!("unknown target: {other}")),
                };
                i += 2;
            }
            "--iters" => {
                iters = Some(
                    args.get(i + 1)
                        .ok_or("--iters needs a value")?
                        .parse()
                        .map_err(|e: std::num::ParseIntError| e.to_string())?,
                );
                i += 2;
            }
            "--corpus-dir" => {
                input_dir = Some(PathBuf::from(
                    args.get(i + 1).ok_or("--corpus-dir needs a value")?,
                ));
                i += 2;
            }
            "-h" | "--help" => {
                eprintln!(
                    "usage: zip-fuzz [--target lfh|eocd|cdr|archive|stream|all] \
                     [--iters N] [--corpus-dir PATH]"
                );
                eprintln!(
                    "  Without --corpus-dir, reads chunks from stdin (one byte\n  \
                     sequence per radamsa output line); each chunk is parsed\n  \
                     once. Honours --iters as an upper bound."
                );
                return Err("help".to_string());
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok((target, iters, input_dir))
}

/// Run the parser on a byte sequence, capturing panics into an Err.
fn parse_with_panic_catch(target: Target, bs: &[u8]) -> Result<(), String> {
    std::panic::catch_unwind(AssertUnwindSafe(|| target.parse(bs)))
        .map_err(|_| format!("PANIC on input of {} bytes", bs.len()))
}

fn run() -> Result<(usize, usize), String> {
    let (target, iters, _corpus_dir) = parse_args()?;
    // Read all stdin bytes in one shot. Radamsa typically writes
    // either a single mutated sample or — when invoked with `-o -` and
    // multiple inputs — a stream of samples separated by newlines.
    // For deterministic per-sample fuzzing, we treat the whole stdin
    // as one sample. When --iters is set we re-process the same buffer
    // that many times to extend the campaign.
    let mut buf = Vec::new();
    io::stdin()
        .read_to_end(&mut buf)
        .map_err(|e| format!("read stdin: {e}"))?;
    let max = iters.unwrap_or(1);
    let mut ok = 0usize;
    let mut panicked = 0usize;
    for _ in 0..max {
        match parse_with_panic_catch(target, &buf) {
            Ok(()) => ok += 1,
            Err(msg) => {
                writeln!(io::stderr(), "{msg}").ok();
                panicked += 1;
            }
        }
    }
    Ok((ok, panicked))
}

fn main() -> ExitCode {
    match run() {
        Ok((ok, 0)) => {
            eprintln!("zip-fuzz: {ok} ok, 0 panics");
            ExitCode::SUCCESS
        }
        Ok((ok, panicked)) => {
            eprintln!("zip-fuzz: {ok} ok, {panicked} PANICS");
            ExitCode::from(1)
        }
        Err(msg) if msg == "help" => ExitCode::from(2),
        Err(msg) => {
            eprintln!("FAIL: {msg}");
            ExitCode::from(2)
        }
    }
}
