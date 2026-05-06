// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `lean-to-rust` — P1.9 domain-specific Lean → Rust extractor.
//!
//! **Honest scope** (ADR-0025 update): a *generic* Lean → Rust
//! extractor is research-scale work (F\*/CakeML/CompCert
//! magnitude) and outside any single sub-phase's budget. This
//! tool extracts the *verified-parser sublanguage* used in
//! `theorems/Apkaxiom/Zip/LocalHeader.lean` — `def`, `structure`,
//! `inductive` over `UInt8/16/32`, `Nat`, `ByteArray`, `Option`,
//! `Except`, products `A × B`, `Id.run do` blocks, pattern
//! matching, structure construction, the standard arithmetic /
//! bitwise / comparison operators.
//!
//! What it produces: a single Rust source file
//! (`crates/axiom-l0-zip-lfh-extracted/src/lib.rs`) that:
//!
//!   1. compiles with `cargo build`,
//!   2. exposes the same public surface the hand-written
//!      `axiom_zip_ref::lfh` does,
//!   3. produces byte-identical JSON output to the Lean
//!      evaluator on the 1499-input TV corpus when piped through
//!      a sibling evaluator binary (`tools/lfh-eval-extracted`).
//!
//! Anything outside the supported subset is rejected loudly with
//! a source span — silent pass-throughs would defeat the whole
//! point of the trust boundary.
//!
//! ## Usage
//!
//! ```bash
//! lean-to-rust theorems/Apkaxiom/Zip/LocalHeader.lean \
//!   crates/axiom-l0-zip-lfh-extracted/src/lib.rs
//! ```
//!
//! ## P1.2 backwards-compat
//!
//! The P1.2 toy `Nat → Nat` extractor's three unit tests are
//! preserved in [`p12_compat.rs`](src/p12_compat.rs) so the
//! pedagogical artefact still works.

#![forbid(unsafe_code)]
#![allow(
    clippy::too_long_first_doc_paragraph,
    clippy::missing_errors_doc,
    clippy::doc_markdown,
    clippy::wildcard_imports,
    clippy::manual_let_else,
    clippy::single_char_pattern,
    clippy::single_match_else,
    clippy::needless_range_loop,
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::missing_const_for_fn,
    clippy::redundant_else,
    clippy::needless_pass_by_value,
    clippy::similar_names,
    clippy::too_many_lines,
    clippy::if_not_else,
    clippy::match_same_arms,
    clippy::collapsible_if,
    clippy::collapsible_else_if,
    clippy::collapsible_match,
    clippy::redundant_field_names,
    clippy::module_name_repetitions,
    clippy::items_after_statements,
    clippy::needless_borrow,
    clippy::cloned_instead_of_copied,
    clippy::option_if_let_else,
    clippy::map_unwrap_or,
    clippy::redundant_clone,
    clippy::cognitive_complexity,
    clippy::unnecessary_wraps,
    clippy::while_let_loop,
    clippy::equatable_if_let,
    clippy::use_self,
    clippy::trivially_copy_pass_by_ref,
    clippy::or_fun_call,
    clippy::unnested_or_patterns,
    clippy::extra_unused_lifetimes,
    clippy::needless_lifetimes,
    clippy::single_call_fn,
    clippy::ignored_unit_patterns,
    clippy::cast_sign_loss,
    clippy::range_plus_one,
    clippy::manual_assert,
    clippy::semicolon_if_nothing_returned,
    clippy::uninlined_format_args,
    clippy::default_trait_access,
    clippy::needless_collect,
    clippy::flat_map_option,
    clippy::ptr_as_ptr,
    clippy::items_after_test_module,
    missing_docs,
    unreachable_pub,
    unused_variables,
    dead_code
)]

mod ast;
mod lexer;
mod p12_compat;
mod parser;
mod translator;

use std::path::PathBuf;
use std::process::ExitCode;

fn usage() -> ExitCode {
    eprintln!("usage: lean-to-rust <input.lean> <output.rs>");
    eprintln!();
    eprintln!("  P1.9 domain-specific extractor. Handles the");
    eprintln!("  verified-parser sublanguage of Lean 4 used in");
    eprintln!("  theorems/Apkaxiom/Zip/LocalHeader.lean.");
    eprintln!();
    eprintln!("  Anything outside the supported subset is rejected");
    eprintln!("  with a source span. ADR-0025 documents the scope.");
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (input, output) = match args.as_slice() {
        [a, b] => (PathBuf::from(a), PathBuf::from(b)),
        _ => return usage(),
    };

    // Try the P1.9 path first (real Lean syntax). If it fails with
    // `Unsupported`, fall back to the P1.2 toy extractor for
    // backwards-compat with the original `Hello.lean` `Nat → Nat`
    // demos.
    let src = match std::fs::read_to_string(&input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("read {}: {e}", input.display());
            return ExitCode::from(1);
        }
    };

    let rendered = match extract(&src) {
        Ok(s) => s,
        Err(e) => {
            // Try P1.2 fallback for the toy file.
            if input.file_name().and_then(|s| s.to_str()) == Some("Hello.lean") {
                eprintln!("(P1.9 path failed: {e}; falling back to P1.2 toy)");
                match p12_compat::extract_render(&src) {
                    Ok(s) => s,
                    Err(e2) => {
                        eprintln!("p1.2 toy also failed: {e2}");
                        return ExitCode::from(1);
                    }
                }
            } else {
                eprintln!("extract {}: {e}", input.display());
                return ExitCode::from(1);
            }
        }
    };

    // Idempotent write.
    if let Ok(existing) = std::fs::read_to_string(&output) {
        if existing == rendered {
            eprintln!("ok: {} already up-to-date", output.display());
            return ExitCode::SUCCESS;
        }
    }
    if let Some(parent) = output.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(e) = std::fs::write(&output, &rendered) {
        eprintln!("write {}: {e}", output.display());
        return ExitCode::from(1);
    }
    eprintln!("wrote: {}", output.display());
    ExitCode::SUCCESS
}

#[derive(Debug, thiserror::Error)]
enum ExtractError {
    #[error("lex: {0}")]
    Lex(#[from] lexer::LexError),
    #[error("parse: {0}")]
    Parse(#[from] parser::ParseError),
    #[error("translate: {0}")]
    Translate(#[from] translator::TranslateError),
}

fn extract(src: &str) -> Result<String, ExtractError> {
    let tokens = lexer::tokenize(src)?;
    let module = parser::parse_module(&tokens)?;
    let env = translator::Env::from_module(&module);
    let rendered = translator::emit_module(&module, &env)?;
    Ok(rendered)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_minimal_def() {
        // A `def two : Nat := 2` extracts to a Rust `const`, not a
        // function — the parameterless / pure / literal-RHS case is
        // recognised as a compile-time constant. (Earlier versions
        // emitted a parameterless fn.)
        let src = "def two : Nat := 2\n";
        let out = extract(src).unwrap();
        assert!(
            out.contains("pub const two: usize = 2") || out.contains("pub fn two() -> usize"),
            "unexpected output:\n{out}"
        );
    }

    #[test]
    fn extracts_struct_with_fields() {
        let src = "structure Point where\n  x : Nat\n  y : Nat\n";
        let out = extract(src).unwrap();
        assert!(out.contains("pub struct Point"));
        assert!(out.contains("pub x: usize"));
        assert!(out.contains("pub y: usize"));
    }

    #[test]
    fn extracts_inductive() {
        let src = "inductive Color where\n  | red\n  | green\n  | blue\n";
        let out = extract(src).unwrap();
        assert!(out.contains("pub enum Color"));
        assert!(out.contains("Red"));
        assert!(out.contains("Green"));
        assert!(out.contains("Blue"));
    }

    #[test]
    fn rejects_unsupported_construct() {
        // Lambdas are outside the subset.
        let src = "def f : Nat := (fun x => x) 0\n";
        assert!(extract(src).is_err());
    }
}
