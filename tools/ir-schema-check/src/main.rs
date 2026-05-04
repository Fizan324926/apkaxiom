// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! AXIOM-IR Cap'n Proto schema gate.
//!
//! Two-phase verification, both phases mandatory:
//!
//!   1. SHA-256 of `schema/axiom_ir_v0_1.capnp` matches the committed
//!      `docs/phase-1/P1.4/ir-data/schema-capnp-hash.txt` pin.
//!      Drift here means someone edited the schema text (or the pin)
//!      without running `make p14-ir`.
//!
//!   2. `capnp compile -onull` against the schema verifies syntax /
//!      semantics. `capnp` is now in the flake (see ADR-0014); if it
//!      is missing the tool fails — the v0.1 wire-format contract is
//!      "schema is byte-stable AND syntactically valid", not "byte-
//!      stable AND best-effort syntactically valid".
//!
//! A Rust-side round-trip via generated bindings stays a Phase-4
//! deliverable per ADR-0014: native capnp emit becomes load-bearing
//! when inter-process IR transmission is real, and adding `capnp` /
//! `capnpc` runtime crates as workspace deps now would inflate the
//! Reindeer surface for no Phase-1 win.
//!
//! Wired into `make p14-schema-check`, `make p14-ir`, and the
//! `p14-ir-drift` CI gate.
//!
//! Usage: `ir-schema-check <repo-root>`
//!        `ir-schema-check <repo-root> --allow-missing-capnp`
//!            (escape hatch — operator-only, intended for rare
//!             bring-up situations on hosts without capnp.)

#![forbid(unsafe_code)]
#![warn(missing_docs, unreachable_pub)]
#![allow(clippy::single_match_else, clippy::option_if_let_else)]

use std::{path::PathBuf, process::ExitCode};

const SCHEMA_PATH: &str = "schema/axiom_ir_v0_1.capnp";
const PIN_PATH: &str = "docs/phase-1/P1.4/ir-data/schema-capnp-hash.txt";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let (repo_root, allow_missing_capnp) = match args.as_slice() {
        [_, root] => (root.clone(), false),
        [_, root, flag] if flag == "--allow-missing-capnp" => (root.clone(), true),
        _ => {
            eprintln!(
                "usage: ir-schema-check <repo-root> [--allow-missing-capnp]\n\
                 (the --allow-missing-capnp escape hatch is operator-only; default is mandatory)"
            );
            return ExitCode::from(2);
        }
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
            // `capnp compile -ocapnp` re-emits the schema through the
            // built-in canonical printer. The re-emit fully type-checks
            // (parser + name resolution + slot layout); we discard the
            // output. The earlier `-onull` form failed because `null`
            // is not a built-in plugin.
            println!("FOUND: {capnp} — running `capnp compile -ocapnp`");
            let output = std::process::Command::new(&capnp)
                .args(["compile", "-ocapnp"])
                .arg(&schema)
                .output();
            match output {
                Ok(out) if out.status.success() => {
                    let bytes = out.stdout.len();
                    println!("PASS: capnp compile -ocapnp ({bytes} bytes re-emitted)");
                    ExitCode::SUCCESS
                }
                Ok(out) => {
                    eprintln!("FAIL: capnp compile exited with {}", out.status);
                    eprintln!("stderr: {}", String::from_utf8_lossy(&out.stderr));
                    ExitCode::from(1)
                }
                Err(e) => {
                    eprintln!("FAIL: cannot invoke {capnp}: {e}");
                    ExitCode::from(1)
                }
            }
        }
        None if allow_missing_capnp => {
            println!(
                "SKIP: capnp not on PATH — escape hatch invoked via --allow-missing-capnp. \
                 Schema-text drift gate is the only check that ran."
            );
            ExitCode::SUCCESS
        }
        None => {
            eprintln!(
                "FAIL: capnp not on PATH. capnproto is in the flake (see ADR-0014); \
                 enter the dev shell with `nix develop`. \
                 Escape hatch (operator-only): re-run with --allow-missing-capnp."
            );
            ExitCode::from(1)
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
