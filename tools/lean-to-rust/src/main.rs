// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `lean-to-rust` — P1.2 prototype extractor.
//!
//! Scope: parses the *bring-up subset* of Lean 4 — single-`def` modules
//! whose body is one of:
//!
//!   * a literal numeric expression
//!   * a `Nat → Nat` arithmetic expression involving `+`, `-`, `*`, `/`,
//!     `%` and integer literals
//!
//! It emits a Rust crate's `lib.rs` containing the extracted function
//! plus a small `#[cfg(test)]` module with reference test cases derived
//! from `#[apkaxiom_test(...)]` annotations in adjacent comments.
//!
//! This is *deliberately* not a real extractor — that work belongs in
//! P1.9 once the AXIOM-IR is real. The point of P1.2 is to prove the
//! pipeline (Lean → Rust → cargo test → reproducibility) round-trips on
//! a trivial example.

#![forbid(unsafe_code)]
#![warn(missing_docs, unreachable_pub)]

use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
    process::ExitCode,
};
use thiserror::Error;

/// Errors surfaced by the prototype extractor.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ExtractError {
    /// Wraps an underlying I/O failure.
    #[error("io: {0}")]
    Io(#[from] io::Error),
    /// The CLI was invoked with the wrong number of arguments.
    #[error("usage: lean-to-rust <input.lean> <output.rs>")]
    Usage,
    /// The Lean source did not contain a `def`-body the prototype recognises.
    #[error("no extractable `def` found in {0}")]
    NoDef(PathBuf),
    /// The body of the `def` is outside the bring-up subset.
    #[error("unsupported expression: {0:?}")]
    Unsupported(String),
}

/// One extractable function, after parsing.
#[derive(Debug)]
struct Extracted {
    name: String,
    arg: String, // single `Nat` argument, or empty for nullary
    body: String,
    /// Test cases declared by `--! test name(args) = expected` lines that
    /// directly precede the def.
    test_cases: Vec<TestCase>,
}

#[derive(Debug)]
struct TestCase {
    name: String,
    arg_value: String,
    expected: String,
}

fn extract(src: &str) -> Result<Extracted, ExtractError> {
    // Parse the bring-up subset by line scan; deliberately not a full Lean
    // parser. The grammar:
    //
    //   def <name> (<arg> : Nat) : Nat := <expr>
    //   def <name> : Nat := <expr>             -- nullary
    //
    // and optional preceding comment lines:
    //
    //   --! test <name>(<value>) = <expected>
    //
    let mut pending_tests: Vec<TestCase> = Vec::new();

    for line in src.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("--!") {
            if let Some(tc) = parse_test_directive(rest.trim()) {
                pending_tests.push(tc);
            }
            continue;
        }
        if !trimmed.starts_with("def ") {
            continue;
        }
        // def <name>[ (<arg> : Nat)]? : Nat := <expr>
        let after_def = trimmed.strip_prefix("def ").unwrap_or(trimmed);
        let (name, rest) = split_first_token(after_def);
        let rest = rest.trim_start();

        let (arg, after_arg) = if let Some(rest) = rest.strip_prefix('(') {
            let close = rest
                .find(')')
                .ok_or_else(|| ExtractError::Unsupported(line.to_string()))?;
            let inside = &rest[..close];
            let after = rest[close + 1..].trim_start();
            // inside looks like "n : Nat"
            let arg_name = inside.split(':').next().unwrap_or("").trim();
            (arg_name.to_string(), after)
        } else {
            (String::new(), rest)
        };

        let after_colon_nat = after_arg
            .trim_start_matches(':')
            .trim_start()
            .strip_prefix("Nat")
            .ok_or_else(|| ExtractError::Unsupported(line.to_string()))?;
        let body = after_colon_nat
            .trim_start()
            .strip_prefix(":=")
            .ok_or_else(|| ExtractError::Unsupported(line.to_string()))?
            .trim()
            .to_string();

        return Ok(Extracted {
            name,
            arg,
            body,
            test_cases: std::mem::take(&mut pending_tests),
        });
    }
    Err(ExtractError::NoDef(PathBuf::from("<stdin>")))
}

fn parse_test_directive(rest: &str) -> Option<TestCase> {
    // "test name(value) = expected"
    let rest = rest.strip_prefix("test ")?.trim();
    let paren = rest.find('(')?;
    let close = rest.find(')')?;
    let eq = rest.find('=')?;
    if !(paren < close && close < eq) {
        return None;
    }
    Some(TestCase {
        name: rest[..paren].trim().to_string(),
        arg_value: rest[paren + 1..close].trim().to_string(),
        expected: rest[eq + 1..].trim().to_string(),
    })
}

fn split_first_token(s: &str) -> (String, &str) {
    let mut end = 0usize;
    for (i, c) in s.char_indices() {
        if c.is_whitespace() || c == '(' || c == ':' {
            end = i;
            break;
        }
        end = i + c.len_utf8();
    }
    (s[..end].to_string(), &s[end..])
}

fn lean_expr_to_rust(expr: &str) -> Result<String, ExtractError> {
    // The bring-up subset accepts Lean Nat expressions over `+ - * / %`
    // and decimal literals, plus a single bound argument that becomes
    // the Rust function's parameter. The Rust target type is `u64`.
    //
    // Validation: walk the bytes; reject anything outside the subset.
    let allowed = |c: char| {
        c.is_ascii_alphanumeric()
            || matches!(
                c,
                '+' | '-' | '*' | '/' | '%' | '(' | ')' | ' ' | '_' | '\t'
            )
    };
    if !expr.chars().all(allowed) {
        return Err(ExtractError::Unsupported(expr.to_string()));
    }
    Ok(expr.to_string())
}

fn render(ext: &Extracted) -> Result<String, ExtractError> {
    let body_rust = lean_expr_to_rust(&ext.body)?;
    // Emit `pub const fn` — every Lean `def` over `Nat` arithmetic is
    // pure and `const`-evaluable in Rust. clippy will complain
    // otherwise (`missing_const_for_fn`).
    // Emit the function body on multiple lines so rustfmt --check leaves
    // it alone. Single-line form `fn f() -> u64 { 2 * n }` is technically
    // fine but cargo fmt rewrites it.
    let fn_sig = if ext.arg.is_empty() {
        format!(
            "pub const fn {}() -> u64 {{\n    {}\n}}",
            ext.name, body_rust
        )
    } else {
        let name = &ext.name;
        let arg = &ext.arg;
        format!("pub const fn {name}({arg}: u64) -> u64 {{\n    {body_rust}\n}}")
    };

    // The non-empty test-mod ends with `\n}\n` (file already terminates
    // cleanly). The empty case needs an explicit trailing newline so
    // rustfmt --check is happy.
    let tests_trailing_newline = ext.test_cases.is_empty();
    let tests = if ext.test_cases.is_empty() {
        String::new()
    } else {
        let cases = ext
            .test_cases
            .iter()
            .map(|tc| {
                let test_name = &tc.name;
                let fn_name = &ext.name;
                let input = &tc.arg_value;
                let expected = &tc.expected;
                format!(
                    "    #[test]\n\
                     \x20   fn {test_name}() {{\n\
                     \x20       assert_eq!({fn_name}({input}), {expected});\n\
                     \x20   }}"
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!("\n\n#[cfg(test)]\nmod tests {{\n    use super::*;\n{cases}\n}}\n")
    };

    let trailing = if tests_trailing_newline { "\n" } else { "" };
    Ok(format!(
        "// Auto-generated by tools/lean-to-rust from a Lean 4 source.\n\
         // Do not edit by hand; re-run the extractor instead.\n\
         //\n\
         // P1.2 prototype output. The production extractor lands in P1.9 and\n\
         // will replace this file shape.\n\
         #![forbid(unsafe_code)]\n\
         \n\
         /// Auto-extracted from a Lean `def`. Operational equivalence is\n\
         /// asserted by `tools/translation-validator` on a fixed input set.\n\
         #[must_use]\n\
         {fn_sig}{tests}{trailing}"
    ))
}

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let [input, output] = if let [a, b] = args.as_slice() {
        [a.clone(), b.clone()]
    } else {
        eprintln!("{}", ExtractError::Usage);
        return ExitCode::from(2);
    };

    let src = match fs::read_to_string(&input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("read {input}: {e}");
            return ExitCode::from(1);
        }
    };
    let ext = match extract(&src) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("extract {input}: {e}");
            return ExitCode::from(1);
        }
    };
    let rendered = match render(&ext) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("render {input}: {e}");
            return ExitCode::from(1);
        }
    };

    // Idempotent write: short-circuit if the target already matches.
    if let Ok(existing) = fs::read_to_string(&output) {
        if existing == rendered {
            eprintln!("ok: {output} already up-to-date");
            return ExitCode::SUCCESS;
        }
    }
    if let Some(parent) = std::path::Path::new(&output).parent() {
        let _ = fs::create_dir_all(parent);
    }
    let mut f = match fs::File::create(&output) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("create {output}: {e}");
            return ExitCode::from(1);
        }
    };
    if let Err(e) = f.write_all(rendered.as_bytes()) {
        eprintln!("write {output}: {e}");
        return ExitCode::from(1);
    }
    eprintln!("wrote: {output}");
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_double() {
        let src = "\
            --! test double_zero(0) = 0\n\
            --! test double_seven(7) = 14\n\
            def double (n : Nat) : Nat := 2 * n\n\
        ";
        let e = extract(src).expect("extract");
        assert_eq!(e.name, "double");
        assert_eq!(e.arg, "n");
        assert_eq!(e.body, "2 * n");
        assert_eq!(e.test_cases.len(), 2);
        let rendered = render(&e).expect("render");
        assert!(rendered.contains("pub const fn double(n: u64) -> u64"));
        assert!(rendered.contains("assert_eq!(double(0), 0);"));
        assert!(rendered.contains("assert_eq!(double(7), 14);"));
        // Test cases must be multiline so rustfmt --check leaves them alone.
        assert!(rendered.contains("    #[test]\n    fn double_zero() {\n"));
    }

    #[test]
    fn rejects_unsupported_expr() {
        // String-literal in body is outside the bring-up subset.
        let bad = "def g (n : Nat) : Nat := \"hello\"\n";
        let e = extract(bad).expect("extract");
        assert!(matches!(render(&e), Err(ExtractError::Unsupported(_))));
    }

    #[test]
    fn nullary_def() {
        let src = "def answer : Nat := 42\n";
        let e = extract(src).expect("extract");
        assert_eq!(e.name, "answer");
        assert_eq!(e.arg, "");
        assert_eq!(e.body, "42");
        let r = render(&e).expect("render");
        assert!(r.contains("pub const fn answer() -> u64 {\n    42\n}"));
    }
}
