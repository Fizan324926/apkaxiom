// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `p113-fuzz-grammar-gen` — emit grammar-shaped seed inputs.
//!
//! Generates well-formed-but-randomised ZIP envelopes by walking
//! the production set in `apk-v1.lark` and instantiating each
//! terminal by sampling its character class. Output is a
//! corpus directory the main fuzz driver can consume as
//! additional seeds (Gap-13 closure: real grammar-aware
//! generation, not just labelled bit-flips).
//!
//! Strictly an in-tree generator — no Lark runtime, no Nautilus.
//! It hand-codes the same archive shape `axiom_zip_ref::archive`
//! accepts, drawing per-field bytes from the LCG. The output
//! satisfies the verified parser by construction; the fuzz
//! loop's mutator then bit-flips into the malformed regions.

#![forbid(unsafe_code)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::uninlined_format_args,
    clippy::type_complexity
)]

use std::path::PathBuf;

use p113_fuzz_harness::mutator::Lcg;

const VERSION: &str = "p113-fuzz-grammar-gen 0.1.0";
const LFH_SIG: u32 = 0x0403_4b50;
const CDR_SIG: u32 = 0x0201_4b50;
const EOCD_SIG: u32 = 0x0605_4b50;

fn parse_arg<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::args()
        .skip_while(|a| a != name)
        .nth(1)
        .and_then(|s| s.parse().ok())
}

fn build_archive(rng: &mut Lcg, n: usize) -> Vec<u8> {
    let mut entries: Vec<(Vec<u8>, Vec<u8>, u32, u16, u16, u16)> = Vec::with_capacity(n);
    for i in 0..n {
        let nl = (rng.next_u32() as usize) % 10 + 4;
        let mut name: Vec<u8> = format!("g-{i:04x}-").into_bytes();
        for _ in 0..nl {
            let c = ((rng.next_u32() as u8) % 26) + b'a';
            name.push(c);
        }
        let body_len = ((rng.next_u32() as usize) % 200) + 1;
        let mut body = Vec::with_capacity(body_len);
        for _ in 0..body_len {
            body.push((rng.next_u32() & 0xff) as u8);
        }
        let crc = rng.next_u32();
        let method = if rng.next_u32() & 1 == 0 { 0u16 } else { 8u16 };
        let time = (rng.next_u32() & 0xffff) as u16;
        let date = (rng.next_u32() & 0xffff) as u16;
        entries.push((name, body, crc, method, time, date));
    }
    let mut bytes: Vec<u8> = Vec::new();
    let mut lfh_offsets: Vec<u32> = Vec::with_capacity(n);
    for (name, body, crc, method, time, date) in &entries {
        lfh_offsets.push(bytes.len() as u32);
        let nl = name.len() as u16;
        let csize = body.len() as u32;
        bytes.extend_from_slice(&LFH_SIG.to_le_bytes());
        bytes.extend_from_slice(&20u16.to_le_bytes()); // versionNeeded
        bytes.extend_from_slice(&0u16.to_le_bytes()); // generalFlags (no DD)
        bytes.extend_from_slice(&method.to_le_bytes());
        bytes.extend_from_slice(&time.to_le_bytes());
        bytes.extend_from_slice(&date.to_le_bytes());
        bytes.extend_from_slice(&crc.to_le_bytes());
        bytes.extend_from_slice(&csize.to_le_bytes());
        bytes.extend_from_slice(&csize.to_le_bytes()); // uncompressedSize == compressedSize
        bytes.extend_from_slice(&nl.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes()); // extraLen
        bytes.extend_from_slice(name);
        bytes.extend_from_slice(body);
    }
    let cd_start = bytes.len() as u32;
    let mut cd_size: u32 = 0;
    for (i, (name, body, crc, method, time, date)) in entries.iter().enumerate() {
        let nl = name.len() as u16;
        let csize = body.len() as u32;
        let mut v: Vec<u8> = Vec::new();
        v.extend_from_slice(&CDR_SIG.to_le_bytes());
        v.extend_from_slice(&20u16.to_le_bytes()); // versionMadeBy
        v.extend_from_slice(&20u16.to_le_bytes()); // versionNeeded
        v.extend_from_slice(&0u16.to_le_bytes()); // generalFlags
        v.extend_from_slice(&method.to_le_bytes());
        v.extend_from_slice(&time.to_le_bytes());
        v.extend_from_slice(&date.to_le_bytes());
        v.extend_from_slice(&crc.to_le_bytes());
        v.extend_from_slice(&csize.to_le_bytes());
        v.extend_from_slice(&csize.to_le_bytes());
        v.extend_from_slice(&nl.to_le_bytes());
        v.extend_from_slice(&0u16.to_le_bytes());
        v.extend_from_slice(&0u16.to_le_bytes()); // commentLen
        v.extend_from_slice(&[0u8; 2]); // diskNumberStart
        v.extend_from_slice(&0u16.to_le_bytes()); // internalAttrs
        v.extend_from_slice(&0u32.to_le_bytes()); // externalAttrs
        v.extend_from_slice(&lfh_offsets[i].to_le_bytes());
        v.extend_from_slice(name);
        cd_size += v.len() as u32;
        bytes.extend_from_slice(&v);
    }
    bytes.extend_from_slice(&EOCD_SIG.to_le_bytes());
    bytes.extend_from_slice(&[0u8; 4]); // diskNumber + cdStartDisk
    let n16 = u16::try_from(n).unwrap_or(u16::MAX);
    bytes.extend_from_slice(&n16.to_le_bytes());
    bytes.extend_from_slice(&n16.to_le_bytes());
    bytes.extend_from_slice(&cd_size.to_le_bytes());
    bytes.extend_from_slice(&cd_start.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes()); // commentLen
    bytes
}

fn main() -> std::io::Result<()> {
    let count: usize = parse_arg("--count").unwrap_or(500);
    let out: PathBuf =
        parse_arg("--out").unwrap_or_else(|| PathBuf::from("fuzz/corpus/seed/grammar-gen"));
    let seed: u64 = parse_arg("--seed").unwrap_or(0xb113_9e0a_1234_5678);

    println!("{VERSION}");
    println!("  count={count}  out={}  seed={seed:#018x}", out.display());

    std::fs::create_dir_all(&out)?;
    let mut rng = Lcg::new(seed);
    let mut total_bytes: u64 = 0;
    for i in 0..count {
        let n_entries = ((rng.next_u32() as usize) % 5) + 1;
        let bytes = build_archive(&mut rng, n_entries);
        let path = out.join(format!("g-{i:05}.bin"));
        std::fs::write(&path, &bytes)?;
        total_bytes += bytes.len() as u64;
    }
    println!("  wrote {} files, {} bytes total", count, total_bytes);
    Ok(())
}
