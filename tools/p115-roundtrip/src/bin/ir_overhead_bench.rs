// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `ir-overhead-bench` — P1.15 IR emission overhead gate.
//!
//! Measures the throughput cost of calling `emit_manifest` +
//! `reencode_manifest` + `emit_resources` + `reencode_resources`
//! on top of the base `Apk::from_reader` parse.
//!
//! HARD gate: overhead ≤ 15 % (P1.15 §10 row 5).
//!
//! Two arms share the same in-memory APK fixture:
//!
//!  - **arm A — base:** `Apk::from_reader` only.
//!  - **arm B — IR:** arm A + `emit_manifest` + `reencode_manifest`
//!    + `emit_resources` + `reencode_resources`.

#![forbid(unsafe_code)]

use std::time::Instant;

use axiom_l1_rs::{Apk, Unverified};
use axiom_l1_rs::ir::{axml, arsc, emit as ir_emit};
use axiom_zip_ref::{cdr, eocd, lfh};

// ── Fixture ──────────────────────────────────────────────────────────────────

fn minimal_axml() -> Vec<u8> {
    let mut sp = Vec::new();
    sp.extend_from_slice(&axml::chunk_type::RES_STRING_POOL.to_le_bytes());
    sp.extend_from_slice(&28u16.to_le_bytes());
    sp.extend_from_slice(&28u32.to_le_bytes());
    sp.extend_from_slice(&[0u8; 20]);

    let mut out = Vec::new();
    out.extend_from_slice(&axml::chunk_type::RES_XML.to_le_bytes());
    out.extend_from_slice(&8u16.to_le_bytes());
    let total = (8u32 + sp.len() as u32).to_le_bytes();
    out.extend_from_slice(&total);
    out.extend_from_slice(&sp);
    out
}

fn minimal_arsc() -> Vec<u8> {
    let mut sp = Vec::new();
    sp.extend_from_slice(&arsc::chunk_type::RES_STRING_POOL.to_le_bytes());
    sp.extend_from_slice(&28u16.to_le_bytes());
    sp.extend_from_slice(&28u32.to_le_bytes());
    sp.extend_from_slice(&[0u8; 20]);

    let mut out = Vec::new();
    out.extend_from_slice(&arsc::chunk_type::RES_TABLE.to_le_bytes());
    out.extend_from_slice(&12u16.to_le_bytes());
    let total = (12u32 + sp.len() as u32).to_le_bytes();
    out.extend_from_slice(&total);
    out.extend_from_slice(&1u32.to_le_bytes()); // package_count
    out.extend_from_slice(&sp);
    out
}

fn build_fixture() -> Vec<u8> {
    let axml_body = minimal_axml();
    let arsc_body = minimal_arsc();
    let entries: &[(&[u8], &[u8])] = &[
        (b"META-INF/CERT.RSA", &[0xab; 32]),
        (b"AndroidManifest.xml", &axml_body),
        (b"classes.dex", &[0x5a; 512]),
        (b"resources.arsc", &arsc_body),
    ];
    build_stored_zip(entries)
}

fn build_stored_zip(entries: &[(&[u8], &[u8])]) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut lfh_offsets = Vec::with_capacity(entries.len());
    for (name, body) in entries {
        let nl = u16::try_from(name.len()).expect("name fits u16");
        let bl = u32::try_from(body.len()).expect("body fits u32");
        lfh_offsets.push(u32::try_from(bytes.len()).expect("offset fits u32"));
        bytes.extend_from_slice(&lfh::SIGNATURE.to_le_bytes());
        bytes.extend_from_slice(&0x0014u16.to_le_bytes()); // version
        bytes.extend_from_slice(&0x0000u16.to_le_bytes()); // gp flags
        bytes.extend_from_slice(&0x0000u16.to_le_bytes()); // compression = stored
        bytes.extend_from_slice(&[0u8; 4]); // mod time/date
        bytes.extend_from_slice(&0u32.to_le_bytes()); // crc (ignored for bench)
        bytes.extend_from_slice(&bl.to_le_bytes()); // compressed size
        bytes.extend_from_slice(&bl.to_le_bytes()); // uncompressed size
        bytes.extend_from_slice(&nl.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes()); // extra len
        bytes.extend_from_slice(name);
        bytes.extend_from_slice(body);
    }
    let cd_offset = u32::try_from(bytes.len()).expect("cd offset fits u32");
    let cdr_start = bytes.len();
    for (i, (name, body)) in entries.iter().enumerate() {
        let nl = u16::try_from(name.len()).expect("name fits u16");
        let bl = u32::try_from(body.len()).expect("body fits u32");
        bytes.extend_from_slice(&cdr::SIGNATURE.to_le_bytes());
        bytes.extend_from_slice(&0x0014u16.to_le_bytes()); // version made by
        bytes.extend_from_slice(&0x0014u16.to_le_bytes()); // version needed
        bytes.extend_from_slice(&0x0000u16.to_le_bytes()); // gp flags
        bytes.extend_from_slice(&0x0000u16.to_le_bytes()); // compression
        bytes.extend_from_slice(&[0u8; 4]); // mod time/date
        bytes.extend_from_slice(&0u32.to_le_bytes()); // crc
        bytes.extend_from_slice(&bl.to_le_bytes()); // compressed size
        bytes.extend_from_slice(&bl.to_le_bytes()); // uncompressed size
        bytes.extend_from_slice(&nl.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes()); // extra len
        bytes.extend_from_slice(&0u16.to_le_bytes()); // comment len
        bytes.extend_from_slice(&0u16.to_le_bytes()); // disk number start
        bytes.extend_from_slice(&0u16.to_le_bytes()); // internal attrs
        bytes.extend_from_slice(&0u32.to_le_bytes()); // external attrs
        bytes.extend_from_slice(&lfh_offsets[i].to_le_bytes());
        bytes.extend_from_slice(name);
    }
    let cd_size = u32::try_from(bytes.len() - cdr_start).expect("cd size fits u32");
    let entry_count = u16::try_from(entries.len()).expect("count fits u16");
    bytes.extend_from_slice(&eocd::SIGNATURE.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes()); // disk number
    bytes.extend_from_slice(&0u16.to_le_bytes()); // disk with cd start
    bytes.extend_from_slice(&entry_count.to_le_bytes());
    bytes.extend_from_slice(&entry_count.to_le_bytes());
    bytes.extend_from_slice(&cd_size.to_le_bytes());
    bytes.extend_from_slice(&cd_offset.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes()); // comment len
    bytes
}

// ── Bench ────────────────────────────────────────────────────────────────────

fn bench_arm_a(fixture: &[u8], iters: usize) -> f64 {
    let t0 = Instant::now();
    for _ in 0..iters {
        let apk: Apk<Unverified> =
            Apk::from_reader(std::io::Cursor::new(fixture)).expect("arm A parse");
        std::hint::black_box(apk);
    }
    t0.elapsed().as_nanos() as f64 / iters as f64
}

fn bench_arm_b(fixture: &[u8], iters: usize) -> f64 {
    let t0 = Instant::now();
    for _ in 0..iters {
        let apk: Apk<Unverified> =
            Apk::from_reader(std::io::Cursor::new(fixture)).expect("arm B parse");
        if let Some(raw) = apk.manifest_bytes() {
            let ir = ir_emit::emit_manifest(
                &axiom_l1_rs::apk_data::Manifest { axml_bytes: raw.to_vec() },
            )
            .expect("emit_manifest");
            std::hint::black_box(ir_emit::reencode_manifest(&ir));
        }
        if let Some(raw) = apk.resources_bytes() {
            let ir = ir_emit::emit_resources(
                &axiom_l1_rs::apk_data::Resources { arsc_bytes: raw.to_vec() },
            )
            .expect("emit_resources");
            std::hint::black_box(ir_emit::reencode_resources(&ir));
        }
    }
    t0.elapsed().as_nanos() as f64 / iters as f64
}

fn main() {
    let fixture = build_fixture();
    let fixture_len = fixture.len();

    const RUNS: usize = 5;
    const ITERS: usize = 100_000;
    const GATE_PCT: f64 = 15.0;

    println!(
        "ir-overhead-bench: {RUNS} runs × {ITERS} iters, fixture {fixture_len} bytes"
    );
    println!("  gate: IR overhead ≤ {GATE_PCT:.0} %");
    println!();

    // Warm up.
    let _ = bench_arm_a(&fixture, 1000);
    let _ = bench_arm_b(&fixture, 1000);

    let mut ns_a = 0.0f64;
    let mut ns_b = 0.0f64;
    for _ in 0..RUNS {
        ns_a += bench_arm_a(&fixture, ITERS);
        ns_b += bench_arm_b(&fixture, ITERS);
    }
    ns_a /= RUNS as f64;
    ns_b /= RUNS as f64;

    let mb_per_sec_a = fixture_len as f64 / (ns_a * 1e-9) / 1e6;
    let mb_per_sec_b = fixture_len as f64 / (ns_b * 1e-9) / 1e6;
    let overhead_pct = (ns_b - ns_a) / ns_a * 100.0;
    let pass = overhead_pct <= GATE_PCT;

    println!("arm A base-parse-only : {:>8.0} ns/iter  {:>8.1} MB/s", ns_a, mb_per_sec_a);
    println!("arm B base + IR emit  : {:>8.0} ns/iter  {:>8.1} MB/s", ns_b, mb_per_sec_b);
    println!(
        "Δ(IR vs base)         : {:>+7.2} %  {}",
        overhead_pct,
        if pass { "PASS" } else { "FAIL" }
    );

    if pass {
        println!("\nGATE PASS — IR overhead {overhead_pct:.2} % ≤ {GATE_PCT:.0} %");
        std::process::exit(0);
    } else {
        eprintln!("\nGATE FAIL — IR overhead {overhead_pct:.2} % > {GATE_PCT:.0} %");
        std::process::exit(1);
    }
}
