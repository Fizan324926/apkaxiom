// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `unsafe-census` — syn-based AST scanner replacing the regex
//! approximation used in P1.3's first audit pass.
//!
//! Counts genuine `unsafe` constructs in a Rust source tree:
//!   - `unsafe { ... }` blocks (`ExprUnsafe`)
//!   - `unsafe fn` definitions (function items / methods / impl methods)
//!   - `unsafe impl` (trait impls marked unsafe)
//!   - `unsafe trait` (trait declarations marked unsafe)
//!
//! Output is a single JSON object on stdout with both totals and a
//! per-file breakdown. Hand-rolled JSON emission (no serde dep — see
//! third-party/rust/Cargo.toml for why).

#![forbid(unsafe_code)]
#![warn(missing_docs, unreachable_pub)]

use std::{fmt::Write, path::Path, process::ExitCode};

use syn::{visit::Visit, ImplItemFn, ItemFn, ItemImpl, ItemTrait, TraitItemFn};
use walkdir::WalkDir;

#[derive(Default, Clone, Copy)]
struct Counts {
    blocks: usize,
    fns: usize,
    impls: usize,
    traits: usize,
}

impl Counts {
    const fn total(self) -> usize {
        self.blocks + self.fns + self.impls + self.traits
    }
}

#[derive(Default)]
struct Visitor {
    c: Counts,
}

impl<'ast> Visit<'ast> for Visitor {
    fn visit_expr_unsafe(&mut self, i: &'ast syn::ExprUnsafe) {
        self.c.blocks += 1;
        syn::visit::visit_expr_unsafe(self, i);
    }
    fn visit_item_fn(&mut self, i: &'ast ItemFn) {
        if i.sig.unsafety.is_some() {
            self.c.fns += 1;
        }
        syn::visit::visit_item_fn(self, i);
    }
    fn visit_impl_item_fn(&mut self, i: &'ast ImplItemFn) {
        if i.sig.unsafety.is_some() {
            self.c.fns += 1;
        }
        syn::visit::visit_impl_item_fn(self, i);
    }
    fn visit_trait_item_fn(&mut self, i: &'ast TraitItemFn) {
        if i.sig.unsafety.is_some() {
            self.c.fns += 1;
        }
        syn::visit::visit_trait_item_fn(self, i);
    }
    fn visit_item_impl(&mut self, i: &'ast ItemImpl) {
        if i.unsafety.is_some() {
            self.c.impls += 1;
        }
        syn::visit::visit_item_impl(self, i);
    }
    fn visit_item_trait(&mut self, i: &'ast ItemTrait) {
        if i.unsafety.is_some() {
            self.c.traits += 1;
        }
        syn::visit::visit_item_trait(self, i);
    }
}

fn scan_file(path: &Path) -> Result<Counts, String> {
    let src = std::fs::read_to_string(path).map_err(|e| format!("read: {e}"))?;
    let file = syn::parse_file(&src).map_err(|e| format!("parse: {e}"))?;
    let mut v = Visitor::default();
    v.visit_file(&file);
    Ok(v.c)
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(&mut out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[allow(clippy::too_many_lines)] // hand-rolled JSON emission is verbose by design
fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let Some(root) = args.first().cloned() else {
        eprintln!("usage: unsafe-census <root-dir>");
        return ExitCode::from(2);
    };

    let exclude = [
        "/target/",
        "/.git/",
        "/.lake/",
        "/buck-out/",
        "/result/",
        "/.direnv/",
    ];

    let mut total = Counts::default();
    let mut files_scanned = 0usize;
    let mut parse_failures: Vec<(String, String)> = Vec::new();
    let mut by_file: Vec<(String, Counts)> = Vec::new();

    for entry in WalkDir::new(&root).follow_links(false) {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("walk: {e}");
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let p = entry.path();
        if p.extension().and_then(|x| x.to_str()) != Some("rs") {
            continue;
        }
        let p_str = p.to_string_lossy();
        if exclude.iter().any(|e| p_str.contains(e)) {
            continue;
        }
        files_scanned += 1;
        let rel = p.strip_prefix(&root).map_or_else(
            |_| p_str.clone().into_owned(),
            |r| r.to_string_lossy().into_owned(),
        );
        match scan_file(p) {
            Ok(c) => {
                total.blocks += c.blocks;
                total.fns += c.fns;
                total.impls += c.impls;
                total.traits += c.traits;
                if c.total() > 0 {
                    by_file.push((rel, c));
                }
            }
            Err(e) => parse_failures.push((rel, e)),
        }
    }

    by_file.sort_by(|a, b| a.0.cmp(&b.0));

    let mut out = String::new();
    let _ = writeln!(&mut out, "{{");
    let _ = writeln!(&mut out, "  \"schema\": \"apkaxiom.unsafe-census/v1\",");
    let _ = writeln!(&mut out, "  \"root\": {},", json_escape(&root));
    let _ = writeln!(&mut out, "  \"files_scanned\": {files_scanned},");
    let _ = writeln!(&mut out, "  \"totals\": {{");
    let _ = writeln!(&mut out, "    \"unsafe_blocks\": {},", total.blocks);
    let _ = writeln!(&mut out, "    \"unsafe_fns\": {},", total.fns);
    let _ = writeln!(&mut out, "    \"unsafe_impls\": {},", total.impls);
    let _ = writeln!(&mut out, "    \"unsafe_traits\": {},", total.traits);
    let _ = writeln!(&mut out, "    \"grand_total\": {}", total.total());
    let _ = writeln!(&mut out, "  }},");
    let _ = writeln!(&mut out, "  \"parse_failures\": [");
    for (i, (f, e)) in parse_failures.iter().enumerate() {
        let sep = if i + 1 == parse_failures.len() {
            ""
        } else {
            ","
        };
        let _ = writeln!(
            &mut out,
            "    {{\"file\": {}, \"error\": {}}}{}",
            json_escape(f),
            json_escape(e),
            sep
        );
    }
    let _ = writeln!(&mut out, "  ],");
    let _ = writeln!(&mut out, "  \"by_file\": [");
    for (i, (f, c)) in by_file.iter().enumerate() {
        let sep = if i + 1 == by_file.len() { "" } else { "," };
        let _ = writeln!(
            &mut out,
            "    {{\"file\": {}, \"unsafe_blocks\": {}, \"unsafe_fns\": {}, \"unsafe_impls\": {}, \"unsafe_traits\": {}}}{}",
            json_escape(f),
            c.blocks,
            c.fns,
            c.impls,
            c.traits,
            sep
        );
    }
    let _ = writeln!(&mut out, "  ]");
    let _ = writeln!(&mut out, "}}");

    print!("{out}");
    ExitCode::SUCCESS
}
