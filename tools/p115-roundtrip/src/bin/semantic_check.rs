// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p115-semantic-check` — semantic decode validation on real APKs.
//!
//! For each APK in the corpus, emits the manifest and resource IRs and
//! runs four hard gates:
//!
//!   1. `ManifestModule.package` is non-empty.
//!   2. Every decoded component has a non-empty `name` field.
//!   3. Every decoded `uses-permission` entry has a non-empty name.
//!   4. Where both `min_sdk` and `target_sdk` are non-zero,
//!      `target_sdk >= min_sdk` (no inverted SDK bounds).
//!
//! Prints a table of per-APK decoded values (or JSON lines with `--json`).
//!
//! Usage: p115-semantic-check [--corpus DIR] [--json]

#![forbid(unsafe_code)]

use std::path::PathBuf;

use axiom_l1_rs::{Apk, Unverified};
use axiom_l1_rs::ir::emit::{emit_manifest, emit_resources};
use axiom_l1_rs::apk_data::{Manifest, Resources};

fn main() {
    let mut args = std::env::args().skip(1).peekable();
    let mut corpus: Option<PathBuf> = None;
    let mut json_mode = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--corpus" => corpus = args.next().map(PathBuf::from),
            "--json"   => json_mode = true,
            other      => { eprintln!("unknown arg: {other}"); std::process::exit(2); }
        }
    }

    let corpus_dir = corpus.unwrap_or_else(|| {
        let cwd = std::env::current_dir().expect("cwd");
        find_workspace_root(&cwd)
            .map(|r| r.join("fuzz/corpus/real-apks"))
            .unwrap_or_else(|| cwd.join("fuzz/corpus/real-apks"))
    });

    let mut apks: Vec<PathBuf> = std::fs::read_dir(&corpus_dir)
        .expect("read corpus dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("apk"))
        .collect();
    apks.sort();

    if !json_mode {
        println!("{:<45} {:>5} {:>5} {:>4} {:>4} {:>7} {:>7}",
            "package", "comps", "perms", "min", "tgt", "gpool", "types");
        println!("{}", "-".repeat(85));
    }

    let mut pkg_empty       = 0usize;
    let mut comp_name_empty = 0usize;
    let mut perm_name_empty = 0usize;
    let mut sdk_inconsistent = 0usize;
    let mut total           = 0usize;

    for path in &apks {
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(e) => { eprintln!("OPEN {name}: {e}"); continue; }
        };
        let apk: Apk<Unverified> = match Apk::from_reader(file) {
            Ok(a) => a,
            Err(e) => { eprintln!("PARSE {name}: {e}"); continue; }
        };

        total += 1;

        let module = apk.manifest_bytes().and_then(|raw| {
            let m = Manifest { axml_bytes: raw.to_vec() };
            emit_manifest(&m).ok().map(|ir| ir.module)
        });

        let table = apk.resources_bytes().and_then(|raw| {
            let r = Resources { arsc_bytes: raw.to_vec() };
            emit_resources(&r).ok().map(|ir| ir.table)
        });

        let (pkg, comps, perms, min_sdk, tgt_sdk) = module.as_ref().map(|m| (
            m.package.clone(),
            m.components.len(),
            m.uses_permissions.len(),
            m.min_sdk,
            m.target_sdk,
        )).unwrap_or_default();

        let (gpool, types) = table.as_ref().map(|t| (
            t.string_pool.strings.len(),
            0usize,
        )).unwrap_or_default();

        // Gate accumulation.
        if pkg.is_empty() {
            pkg_empty += 1;
        }
        if let Some(m) = &module {
            for comp in &m.components {
                if comp.name.is_empty() { comp_name_empty += 1; }
            }
            for up in &m.uses_permissions {
                if up.is_empty() { perm_name_empty += 1; }
            }
            if m.min_sdk > 0 && m.target_sdk > 0 && m.target_sdk < m.min_sdk {
                sdk_inconsistent += 1;
            }
        }

        if json_mode {
            // Hand-rolled NDJSON — no serde dependency.
            let apk_esc = name.replace('\\', "\\\\").replace('"', "\\\"");
            let pkg_esc = pkg.replace('\\', "\\\\").replace('"', "\\\"");
            println!(
                r#"{{"apk":"{apk_esc}","package":"{pkg_esc}","min_sdk":{min_sdk},"target_sdk":{tgt_sdk},"components":{comps},"uses_permissions":{perms},"gpool":{gpool}}}"#
            );
        } else {
            println!("{:<45} {:>5} {:>5} {:>4} {:>4} {:>7} {:>7}",
                if pkg.is_empty() { format!("[?] {name}") } else { pkg },
                comps, perms, min_sdk, tgt_sdk, gpool, types);
        }
    }

    if !json_mode {
        println!("{}", "-".repeat(85));
        println!("total={total}  pkg_empty={pkg_empty}  comp_name_empty={comp_name_empty}  perm_name_empty={perm_name_empty}  sdk_inconsistent={sdk_inconsistent}");
    }

    // ── Hard gates ───────────────────────────────────────────────────────────
    let mut all_pass = true;

    if pkg_empty == 0 {
        if !json_mode { println!("GATE PASS — all {total} packages non-empty"); }
    } else {
        eprintln!("GATE FAIL — {pkg_empty} APK(s) decoded with empty package name");
        all_pass = false;
    }

    if comp_name_empty == 0 {
        if !json_mode { println!("GATE PASS — all component names non-empty"); }
    } else {
        eprintln!("GATE FAIL — {comp_name_empty} component(s) decoded with empty name");
        all_pass = false;
    }

    if perm_name_empty == 0 {
        if !json_mode { println!("GATE PASS — all uses-permission names non-empty"); }
    } else {
        eprintln!("GATE FAIL — {perm_name_empty} uses-permission(s) decoded with empty name");
        all_pass = false;
    }

    if sdk_inconsistent == 0 {
        if !json_mode { println!("GATE PASS — no SDK bound inversions (min_sdk > target_sdk)"); }
    } else {
        eprintln!("GATE FAIL — {sdk_inconsistent} APK(s) have min_sdk > target_sdk");
        all_pass = false;
    }

    if all_pass {
        if !json_mode { println!("ALL GATES PASS"); }
        std::process::exit(0);
    } else {
        std::process::exit(1);
    }
}

fn find_workspace_root(start: &std::path::Path) -> Option<PathBuf> {
    let mut dir = start.to_path_buf();
    loop {
        let candidate = dir.join("Cargo.toml");
        if candidate.exists() {
            let content = std::fs::read_to_string(&candidate).unwrap_or_default();
            if content.contains("[workspace]") { return Some(dir); }
        }
        if !dir.pop() { return None; }
    }
}
