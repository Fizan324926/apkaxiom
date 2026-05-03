// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! AXIOM-IR Cap'n Proto schema gate.
//!
//! Two-phase verification:
//!
//!   1. **Always:** SHA-256 of `schema/axiom_ir_v0_1.capnp` matches the
//!      committed `docs/phase-1/P1.4/ir-data/schema-capnp-hash.txt` pin.
//!      Drift here means someone edited the schema text (or the pin)
//!      without running `make p14-ir`.
//!
//!   2. **If `capnp` is on PATH:** invoke `capnp compile -onull` to
//!      verify the schema is syntactically valid. We use `-onull` (the
//!      no-op codegen output) because we don't actually need to
//!      generate Rust code — Phase-4 inter-process IR transmission is
//!      where capnp becomes load-bearing.
//!
//! Without capnp installed, only step 1 runs and the tool reports
//! "skipped capnp compile (capnp not on PATH)" with exit 0. With capnp
//! installed, both steps run; either failing exits non-zero.
//!
//! Wired into `make p14-ir` and the `p14-ir-drift` CI gate.
//!
//! Usage: `ir-schema-check <repo-root>`

#![forbid(unsafe_code)]
#![warn(missing_docs, unreachable_pub)]
#![allow(clippy::single_match_else, clippy::option_if_let_else)]

use std::{path::PathBuf, process::ExitCode};

const SCHEMA_PATH: &str = "schema/axiom_ir_v0_1.capnp";
const PIN_PATH: &str = "docs/phase-1/P1.4/ir-data/schema-capnp-hash.txt";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let [_, repo_root] = args.as_slice() else {
        eprintln!("usage: ir-schema-check <repo-root>");
        return ExitCode::from(2);
    };
    let root = PathBuf::from(repo_root);

    let schema = root.join(SCHEMA_PATH);
    let pin = root.join(PIN_PATH);

    let schema_bytes = match std::fs::read(&schema) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("FAIL: cannot read {}: {e}", schema.display());
            return ExitCode::from(1);
        }
    };
    let pinned = match std::fs::read_to_string(&pin) {
        Ok(s) => s.trim().to_string(),
        Err(e) => {
            eprintln!("FAIL: cannot read {}: {e}", pin.display());
            return ExitCode::from(1);
        }
    };

    // Step 1 — SHA-256 match.
    let actual = axiom_ir::hash::hex(&axiom_ir::hash::sha256(&schema_bytes));
    if actual != pinned {
        eprintln!("FAIL: schema-capnp-hash drift");
        eprintln!("  schema:   {}", schema.display());
        eprintln!("  pinned:   {pinned}");
        eprintln!("  computed: {actual}");
        eprintln!("Re-run `make p14-schema-hash` and review the diff.");
        return ExitCode::from(1);
    }
    println!("PASS: schema-capnp-hash matches pin");
    println!("  schema: {}", schema.display());
    println!("  hash:   {actual}");

    // Step 2 — invoke capnp if present.
    match find_capnp() {
        Some(capnp) => {
            println!("FOUND: {capnp} — running `capnp compile -onull`");
            let status = std::process::Command::new(&capnp)
                .args(["compile", "-onull"])
                .arg(&schema)
                .status();
            match status {
                Ok(s) if s.success() => {
                    println!("PASS: capnp compile -onull");
                    ExitCode::SUCCESS
                }
                Ok(s) => {
                    eprintln!("FAIL: capnp compile exited with {s}");
                    ExitCode::from(1)
                }
                Err(e) => {
                    eprintln!("FAIL: cannot invoke {capnp}: {e}");
                    ExitCode::from(1)
                }
            }
        }
        None => {
            println!(
                "SKIP: capnp not on PATH — schema-text drift gate is sufficient. \
                 Operator install: `nix-env -iA nixpkgs.capnproto` or `apt install capnproto`."
            );
            ExitCode::SUCCESS
        }
    }
}

/// Locate the `capnp` binary on PATH. Returns the absolute path string
/// if present, `None` otherwise.
fn find_capnp() -> Option<String> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join("capnp");
        if candidate.is_file() {
            return Some(candidate.to_string_lossy().into_owned());
        }
        let candidate_exe = dir.join("capnp.exe");
        if candidate_exe.is_file() {
            return Some(candidate_exe.to_string_lossy().into_owned());
        }
    }
    None
}
