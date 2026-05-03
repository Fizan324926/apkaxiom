// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `translation-validator` — P1.2 skeleton.
//!
//! Runs the extracted Rust function (`axiom_extract_hello::double`)
//! against the *Lean ground truth* obtained by invoking `lean --run`
//! on a small driver Lean script, on a fixed input set. Asserts equality
//! per input.
//!
//! P1.2 scope is *operational equivalence* on a finite test corpus, not
//! semantic equivalence. The real refinement-relation proof lands in
//! P1.9 once AXIOM-IR is real and we can use `decide`/`omega` to
//! discharge the obligations inside Lean itself.

#![forbid(unsafe_code)]
#![warn(missing_docs, unreachable_pub)]

use std::{
    fs,
    path::PathBuf,
    process::{Command, ExitCode, Stdio},
};
use thiserror::Error;

/// Inputs evaluated against both Lean and Rust. The set is small but
/// covers: zero, small, mid-range, near-`u32::MAX/2` (so `2 * n` does
/// not overflow `u64`).
const INPUTS: &[u64] = &[0, 1, 7, 100, 1_000_000_000, 2_147_483_647];

/// Validator error surface.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ValidatorError {
    /// Lean process spawn / IO failed.
    #[error("lean invocation failed: {0}")]
    Lean(String),
    /// Lean produced non-numeric or unexpected output.
    #[error("lean output for input {0}: parse error ({1:?})")]
    Parse(u64, String),
    /// Lean and Rust disagreed on a specific input.
    #[error("divergence on input {input}: lean={lean} rust={rust}")]
    Divergence {
        /// The input that triggered the divergence.
        input: u64,
        /// Value Lean reported for that input.
        lean: u64,
        /// Value the extracted Rust function returned for that input.
        rust: u64,
    },
}

fn write_driver(tmpdir: &std::path::Path, input: u64) -> Result<PathBuf, ValidatorError> {
    // Lean's `--run` evaluates a `def main : IO Unit`; that's how we
    // get a printable u64 to stdout. The driver is regenerated per
    // input so no parsing is needed beyond `parse::<u64>`.
    let path = tmpdir.join(format!("driver_{input}.lean"));
    let body = format!(
        "import Apkaxiom.Hello\n\
         def main : IO Unit := do\n\
           IO.println (Apkaxiom.Hello.double {input})\n"
    );
    fs::write(&path, body).map_err(|e| ValidatorError::Lean(format!("write driver: {e}")))?;
    Ok(path)
}

fn run_lean(tmpdir: &std::path::Path, input: u64) -> Result<u64, ValidatorError> {
    let driver = write_driver(tmpdir, input)?;
    // Use `lake env lean --run` so the Lean process sees our project's
    // search path (Apkaxiom is importable). Plain `lean --run` outside
    // of the lake env cannot find our modules.
    let out = Command::new("lake")
        .args(["env", "lean", "--run"])
        .arg(&driver)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| ValidatorError::Lean(format!("spawn lean: {e}")))?;
    if !out.status.success() {
        return Err(ValidatorError::Lean(format!(
            "non-zero exit ({:?}): {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr)
        )));
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let line = stdout.lines().next_back().unwrap_or("").trim().to_string();
    line.parse::<u64>()
        .map_err(|_| ValidatorError::Parse(input, stdout.into_owned()))
}

fn validate() -> Result<usize, ValidatorError> {
    // Drivers + Lean must run with the lakefile in CWD. Lake env relies
    // on .lake/ being in the cwd; we therefore `cd` to the repo root
    // (resolved via the `LAKE_PROJECT` env var if set, else by walking
    // up from cwd looking for `lakefile.toml`).
    let project = match std::env::var("LAKE_PROJECT") {
        Ok(p) => PathBuf::from(p),
        Err(_) => find_lake_project()?,
    };
    std::env::set_current_dir(&project)
        .map_err(|e| ValidatorError::Lean(format!("cd {}: {e}", project.display())))?;

    let tmpdir = std::env::temp_dir().join(format!("apkaxiom-validator-{}", std::process::id()));
    fs::create_dir_all(&tmpdir).map_err(|e| ValidatorError::Lean(format!("mkdir tmp: {e}")))?;

    let mut checked = 0;
    for &input in INPUTS {
        let lean = run_lean(&tmpdir, input)?;
        let rust = axiom_extract_hello::double(input);
        if lean != rust {
            return Err(ValidatorError::Divergence { input, lean, rust });
        }
        println!("  input={input:>11}  lean={lean:>11}  rust={rust:>11}  OK");
        checked += 1;
    }
    let _ = fs::remove_dir_all(&tmpdir);
    Ok(checked)
}

fn find_lake_project() -> Result<PathBuf, ValidatorError> {
    let mut p = std::env::current_dir().map_err(|e| ValidatorError::Lean(format!("cwd: {e}")))?;
    loop {
        if p.join("lakefile.toml").exists() {
            return Ok(p);
        }
        if !p.pop() {
            return Err(ValidatorError::Lean(
                "no lakefile.toml found walking up from cwd".into(),
            ));
        }
    }
}

fn main() -> ExitCode {
    println!("translation-validator (P1.2 skeleton)");
    println!("  function: Apkaxiom.Hello.double");
    println!("  Rust target: axiom_extract_hello::double");
    println!("  inputs: {INPUTS:?}");
    println!();
    match validate() {
        Ok(n) => {
            println!("\nPASS: {n} inputs agree.");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("\nFAIL: {e}");
            ExitCode::from(1)
        }
    }
}
