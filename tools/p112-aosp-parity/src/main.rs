// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p112-aosp-parity` — P1.12 Gap-9 AOSP runtime parity gate on
//! Bench-10K.
//!
//! Pipes each Bench-10K archive through the AOSP runtime probe
//! (`target/zip-aosp-runtime-probe --archive-runtime`, which
//! links the real `external/libziparchive/zip_archive.cc`
//! end-to-end and calls `OpenArchiveFromMemory`) and compares
//! the accept/reject verdict against the verified path.
//!
//! AOSP runtime is more permissive than the verified path on a
//! few documented corners (it doesn't enforce APPNOTE.TXT
//! §4.4.4 zero-fields-in-DD-mode, doesn't validate every
//! cross-record consistency the verified path does). The gate
//! is therefore one-directional: **every archive accepted by
//! the verified path must also be accepted by AOSP**. The
//! reverse direction is informational (count of archives AOSP
//! accepts but verified rejects).
//!
//! Spec gate: ≥ 99 % verified-accept ⇒ AOSP-accept on Bench-10K.

#![forbid(unsafe_code)]
#![allow(
    clippy::cast_precision_loss,
    clippy::uninlined_format_args,
    clippy::too_many_lines
)]

use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

use axiom_l0_zip_verified::consistency::parse_archive;

const PROBE_DEFAULT: &str = "target/zip-aosp-runtime-probe";
const COUNT_DEFAULT: usize = 10_000;
const GATE_PCT_DEFAULT: f64 = 99.0;

fn parse_arg<T: std::str::FromStr>(name: &str, default: T) -> T {
    std::env::args()
        .skip_while(|a| a != name)
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn aosp_accept(probe: &str, bytes: &[u8]) -> std::io::Result<bool> {
    let mut child = Command::new(probe)
        .arg("--archive-runtime")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(bytes)?;
    }
    let out = child.wait_with_output()?;
    let s = String::from_utf8_lossy(&out.stdout);
    Ok(s.starts_with("ok "))
}

fn main() {
    let probe: String = parse_arg("--probe", PROBE_DEFAULT.to_string());
    let count: usize = parse_arg("--count", COUNT_DEFAULT);
    let gate: f64 = parse_arg("--gate-pct", GATE_PCT_DEFAULT);
    let corpus: String = parse_arg("--corpus", "corpus/zip/bench-10k".to_string());
    let dir = PathBuf::from(&corpus);

    if !std::path::Path::new(&probe).exists() {
        eprintln!("ERROR probe not found: {probe}; run `make p16-aosp-runtime-probe` first");
        std::process::exit(2);
    }

    println!(
        "p112-aosp-parity: {} archives, probe={}, gate ≥ {:.1} % verified-accept ⇒ AOSP-accept",
        count, probe, gate
    );

    let mut verified_accept_aosp_accept: u64 = 0;
    let mut verified_accept_aosp_reject: u64 = 0;
    let mut verified_reject_aosp_accept: u64 = 0;
    let mut verified_reject_aosp_reject: u64 = 0;
    let mut io_errors: u64 = 0;
    let mut first_diverge: Vec<String> = Vec::new();

    for i in 0..count {
        let p = dir.join(format!("{i:05}.bin"));
        let bytes = match std::fs::read(&p) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("ERROR read {}: {}", p.display(), e);
                std::process::exit(2);
            }
        };
        let v = parse_archive(&bytes).is_ok();
        let a = match aosp_accept(&probe, &bytes) {
            Ok(b) => b,
            Err(e) => {
                io_errors += 1;
                eprintln!("WARN probe i={i}: {e}");
                continue;
            }
        };
        match (v, a) {
            (true, true) => verified_accept_aosp_accept += 1,
            (true, false) => {
                verified_accept_aosp_reject += 1;
                if first_diverge.len() < 10 {
                    first_diverge.push(format!(
                        "  sample={i} verified=accept AOSP=reject ({} bytes)",
                        bytes.len()
                    ));
                }
            }
            (false, true) => verified_reject_aosp_accept += 1,
            (false, false) => verified_reject_aosp_reject += 1,
        }
        if i > 0 && i % 1000 == 0 {
            println!("  …{}/{}", i, count);
        }
    }

    let v_accept = verified_accept_aosp_accept + verified_accept_aosp_reject;
    let aligned_pct = if v_accept == 0 {
        100.0
    } else {
        (verified_accept_aosp_accept as f64 / v_accept as f64) * 100.0
    };

    println!();
    println!("=== summary ===");
    println!(
        "  verified-accept AOSP-accept : {}",
        verified_accept_aosp_accept
    );
    println!(
        "  verified-accept AOSP-reject : {}  (binding gate)",
        verified_accept_aosp_reject
    );
    println!(
        "  verified-reject AOSP-accept : {}  (verified is stricter; informational)",
        verified_reject_aosp_accept
    );
    println!(
        "  verified-reject AOSP-reject : {}",
        verified_reject_aosp_reject
    );
    println!("  io errors                  : {}", io_errors);
    println!(
        "  verified-accept ⇒ AOSP-accept: {:.2} %  (gate ≥ {:.1} %)",
        aligned_pct, gate
    );

    if !first_diverge.is_empty() {
        println!();
        println!(
            "first {} verified-accept AOSP-reject divergences:",
            first_diverge.len()
        );
        for s in &first_diverge {
            println!("{s}");
        }
    }

    let pass = aligned_pct >= gate;
    println!(
        "  verdict                     : {}",
        if pass { "PASS" } else { "FAIL" }
    );
    if !pass {
        eprintln!(
            "::error::p112-aosp-parity verified-accept ⇒ AOSP-accept {:.2} % below gate {:.1} %",
            aligned_pct, gate
        );
        std::process::exit(1);
    }
}
