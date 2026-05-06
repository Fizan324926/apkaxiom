// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p115-semantic-check` — semantic decode validation on real APKs.
//!
//! For each APK in the corpus, emits the manifest and resource IRs and
//! checks that the `ManifestModule.package` field is non-empty. Prints a
//! table of package names, component counts, uses-permissions counts, SDK
//! bounds, and resource pool sizes.
//!
//! Usage: p115-semantic-check [--corpus DIR]

#![forbid(unsafe_code)]

use std::path::PathBuf;

use axiom_l1_rs::{Apk, Unverified};
use axiom_l1_rs::ir::emit::{emit_manifest, emit_resources};
use axiom_l1_rs::apk_data::{Manifest, Resources};

fn main() {
    let mut args = std::env::args().skip(1).peekable();
    let mut corpus: Option<PathBuf> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--corpus" => corpus = args.next().map(PathBuf::from),
            other => { eprintln!("unknown arg: {other}"); std::process::exit(2); }
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

    println!("{:<45} {:>5} {:>5} {:>4} {:>4} {:>7} {:>7}",
        "package", "comps", "perms", "min", "tgt", "gpool", "types");
    println!("{}", "-".repeat(85));

    let mut pkg_empty = 0usize;
    let mut total = 0usize;

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
            0usize, // type names not separately tracked in v0.1 ResourceTable
        )).unwrap_or_default();

        if pkg.is_empty() { pkg_empty += 1; }

        println!("{:<45} {:>5} {:>5} {:>4} {:>4} {:>7} {:>7}",
            if pkg.is_empty() { format!("[?] {name}") } else { pkg },
            comps, perms, min_sdk, tgt_sdk, gpool, types);
    }

    println!("{}", "-".repeat(85));
    println!("total={total}  package_empty={pkg_empty}");

    // Gate: every APK with a manifest must have a non-empty package name.
    if pkg_empty > 0 {
        eprintln!("GATE FAIL — {pkg_empty} APK(s) decoded with empty package name");
        std::process::exit(1);
    }
    println!("GATE PASS — all packages non-empty");
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
