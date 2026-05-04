// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `zip-corpus-gen` — P1.5 deterministic corpus generator.
//!
//! Emits ≥1,500 samples under `corpus/zip/`:
//!
//!   - `lfh-valid/`        — ≥1,000 well-formed LFH bytes
//!   - `lfh-adversarial/`  — ≥500 malformed LFH (BadPack-class +
//!                           oversize + truncated + bad magic)
//!   - `eocd-valid/`       — ≥100 valid EOCD bytes
//!   - `eocd-adversarial/` — ≥200 malformed EOCD
//!
//! Each output is `<index>.bin` (raw bytes — what the parser sees).
//! A sibling `manifest.json` records, per sample, the **expected**
//! parse verdict from the Rust reference parser. The differential
//! harness reads this manifest as ground truth.
//!
//! Determinism contract: **same seed ⇒ byte-identical corpus**. The
//! generator uses an in-tree LCG (no external `rand` dep) to keep
//! the Reindeer surface small. The CI gate `p15-corpus-drift`
//! re-runs this binary and asserts `git diff --exit-code` over
//! `corpus/zip/`.

#![forbid(unsafe_code)]
#![warn(missing_docs, unreachable_pub)]
// The corpus generator does deliberate `as u16` truncation (sample
// sizes always ≤ 1000 — provably-safe for our seed). Allow at crate
// scope; the differential harness on 1800 samples is the binding
// gate for correctness.
#![allow(clippy::cast_possible_truncation)]

use std::{
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use axiom_zip_ref::{eocd, lfh};

/// Linear-congruential PRNG. Numerical Recipes constants. Tiny but
/// adequate for corpus generation; we don't need cryptographic
/// quality here, only determinism.
struct Lcg {
    state: u64,
}

impl Lcg {
    const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.state >> 32) as u32
    }

    fn next_u16(&mut self) -> u16 {
        self.next_u32() as u16
    }

    fn next_in_range(&mut self, lo: u32, hi: u32) -> u32 {
        debug_assert!(lo < hi);
        lo + (self.next_u32() % (hi - lo))
    }

    fn fill(&mut self, out: &mut [u8]) {
        for byte in out {
            *byte = (self.next_u32() & 0xff) as u8;
        }
    }
}

fn write_sample(dir: &Path, idx: usize, bytes: &[u8]) -> Result<(), std::io::Error> {
    fs::create_dir_all(dir)?;
    let path = dir.join(format!("{idx:04}.bin"));
    fs::write(&path, bytes)
}

fn build_lfh(rng: &mut Lcg, name_len: u16, extra_len: u16) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(lfh::FIXED_SIZE + name_len as usize + extra_len as usize);
    bytes.extend_from_slice(&lfh::SIGNATURE.to_le_bytes());
    bytes.extend_from_slice(&rng.next_u16().to_le_bytes()); // versionNeeded
    bytes.extend_from_slice(&rng.next_u16().to_le_bytes()); // generalFlags
    bytes.extend_from_slice(&rng.next_u16().to_le_bytes()); // compressionMethod
    bytes.extend_from_slice(&rng.next_u16().to_le_bytes()); // lastModTime
    bytes.extend_from_slice(&rng.next_u16().to_le_bytes()); // lastModDate
    bytes.extend_from_slice(&rng.next_u32().to_le_bytes()); // crc32
    bytes.extend_from_slice(&rng.next_u32().to_le_bytes()); // compressedSize
    bytes.extend_from_slice(&rng.next_u32().to_le_bytes()); // uncompressedSize
    bytes.extend_from_slice(&name_len.to_le_bytes());
    bytes.extend_from_slice(&extra_len.to_le_bytes());
    let mut name = vec![0u8; name_len as usize];
    rng.fill(&mut name);
    bytes.extend_from_slice(&name);
    let mut extra = vec![0u8; extra_len as usize];
    rng.fill(&mut extra);
    bytes.extend_from_slice(&extra);
    bytes
}

fn build_eocd(rng: &mut Lcg, comment_len: u16, multi_volume: bool) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(eocd::FIXED_SIZE + comment_len as usize);
    bytes.extend_from_slice(&eocd::SIGNATURE.to_le_bytes());
    let disk: u16 = u16::from(multi_volume); // 0 or 1
    bytes.extend_from_slice(&disk.to_le_bytes()); // diskNumber
    bytes.extend_from_slice(&[0u8; 2]); // cdStartDisk = 0
    bytes.extend_from_slice(&rng.next_u16().to_le_bytes()); // entriesOnThisDisk
    bytes.extend_from_slice(&rng.next_u16().to_le_bytes()); // totalEntries
    bytes.extend_from_slice(&rng.next_u32().to_le_bytes()); // cdSize
    bytes.extend_from_slice(&rng.next_u32().to_le_bytes()); // cdOffset
    bytes.extend_from_slice(&comment_len.to_le_bytes());
    let mut comment = vec![0u8; comment_len as usize];
    rng.fill(&mut comment);
    bytes.extend_from_slice(&comment);
    bytes
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c => out.push(c),
        }
    }
    out
}

fn write_manifest(dir: &Path, entries: &[(String, &str)]) -> Result<(), std::io::Error> {
    let mut s = String::new();
    s.push_str("{\n  \"samples\": [\n");
    for (i, (name, verdict)) in entries.iter().enumerate() {
        let comma = if i + 1 == entries.len() { "" } else { "," };
        s.push_str(&format!(
            "    {{\"file\": \"{}\", \"expected\": \"{}\"}}{}\n",
            json_escape(name),
            json_escape(verdict),
            comma
        ));
    }
    s.push_str("  ]\n}\n");
    fs::write(dir.join("manifest.json"), s)
}

#[allow(clippy::too_many_lines)] // single deterministic generator end-to-end
fn run(out_root: &Path) -> Result<(), std::io::Error> {
    let mut rng = Lcg::new(0xa9c1_d4b1_f7e2_3d51); // schema id from capnp
    let lfh_valid_dir = out_root.join("lfh-valid");
    let lfh_adv_dir = out_root.join("lfh-adversarial");
    let eocd_valid_dir = out_root.join("eocd-valid");
    let eocd_adv_dir = out_root.join("eocd-adversarial");

    // 1) 1000 valid LFH samples — varying name + extra lengths.
    let mut lfh_valid_manifest = Vec::with_capacity(1000);
    for i in 0..1000 {
        let name_len = (rng.next_in_range(0, 32)) as u16;
        let extra_len = (rng.next_in_range(0, 16)) as u16;
        let bytes = build_lfh(&mut rng, name_len, extra_len);
        // Confirm verdict via reference parser (sanity).
        assert!(lfh::parse_lfh(&bytes).is_ok());
        write_sample(&lfh_valid_dir, i, &bytes)?;
        lfh_valid_manifest.push((format!("{i:04}.bin"), "ok"));
    }
    write_manifest(
        &lfh_valid_dir,
        &lfh_valid_manifest
            .iter()
            .map(|(s, v)| (s.clone(), *v))
            .collect::<Vec<_>>(),
    )?;

    // 2) 500 adversarial LFH samples covering each ParseError variant.
    //
    //    [0..125]   — too short (truncated fixed header)
    //    [125..250] — bad magic
    //    [250..375] — short name (declared name > available bytes)
    //    [375..500] — short extra
    let mut lfh_adv_manifest = Vec::with_capacity(500);
    for i in 0..125 {
        let cut = (rng.next_in_range(0, 30)) as usize;
        let bytes = vec![0u8; cut];
        assert_eq!(lfh::parse_lfh(&bytes), Err(lfh::ParseError::ShortHeader));
        write_sample(&lfh_adv_dir, i, &bytes)?;
        lfh_adv_manifest.push((format!("{i:04}.bin"), "ShortHeader"));
    }
    for i in 125..250 {
        let mut bytes = build_lfh(&mut rng, 0, 0);
        bytes[0] = bytes[0].wrapping_add(1);
        assert_eq!(lfh::parse_lfh(&bytes), Err(lfh::ParseError::BadSignature));
        write_sample(&lfh_adv_dir, i, &bytes)?;
        lfh_adv_manifest.push((format!("{i:04}.bin"), "BadSignature"));
    }
    for i in 250..375 {
        // Declare a name length larger than the bytes that follow.
        let mut bytes = build_lfh(&mut rng, 0, 0);
        let big = rng.next_in_range(50, 1000) as u16;
        bytes[26..28].copy_from_slice(&big.to_le_bytes());
        assert_eq!(lfh::parse_lfh(&bytes), Err(lfh::ParseError::ShortName));
        write_sample(&lfh_adv_dir, i, &bytes)?;
        lfh_adv_manifest.push((format!("{i:04}.bin"), "ShortName"));
    }
    for i in 375..500 {
        let mut bytes = build_lfh(&mut rng, 0, 0);
        let big = rng.next_in_range(50, 1000) as u16;
        bytes[28..30].copy_from_slice(&big.to_le_bytes());
        assert_eq!(lfh::parse_lfh(&bytes), Err(lfh::ParseError::ShortExtra));
        write_sample(&lfh_adv_dir, i, &bytes)?;
        lfh_adv_manifest.push((format!("{i:04}.bin"), "ShortExtra"));
    }
    write_manifest(
        &lfh_adv_dir,
        &lfh_adv_manifest
            .iter()
            .map(|(s, v)| (s.clone(), *v))
            .collect::<Vec<_>>(),
    )?;

    // 3) 100 valid EOCD samples, varying comment length.
    let mut eocd_valid_manifest = Vec::with_capacity(100);
    for i in 0..100 {
        let comment_len = rng.next_in_range(0, 64) as u16;
        let bytes = build_eocd(&mut rng, comment_len, false);
        assert!(eocd::parse_eocd(&bytes).is_ok());
        write_sample(&eocd_valid_dir, i, &bytes)?;
        eocd_valid_manifest.push((format!("{i:04}.bin"), "ok"));
    }
    write_manifest(
        &eocd_valid_dir,
        &eocd_valid_manifest
            .iter()
            .map(|(s, v)| (s.clone(), *v))
            .collect::<Vec<_>>(),
    )?;

    // 4) 200 adversarial EOCD samples — split four ways.
    let mut eocd_adv_manifest = Vec::with_capacity(200);
    for i in 0..50 {
        let cut = (rng.next_in_range(0, 22)) as usize;
        let bytes = vec![0u8; cut];
        assert_eq!(eocd::parse_eocd(&bytes), Err(eocd::ParseError::ShortFixed));
        write_sample(&eocd_adv_dir, i, &bytes)?;
        eocd_adv_manifest.push((format!("{i:04}.bin"), "ShortFixed"));
    }
    for i in 50..100 {
        let mut bytes = build_eocd(&mut rng, 0, false);
        bytes[0] = bytes[0].wrapping_add(1);
        assert_eq!(
            eocd::parse_eocd(&bytes),
            Err(eocd::ParseError::BadSignature)
        );
        write_sample(&eocd_adv_dir, i, &bytes)?;
        eocd_adv_manifest.push((format!("{i:04}.bin"), "BadSignature"));
    }
    for i in 100..150 {
        let bytes = build_eocd(&mut rng, 0, true); // multi-volume
        assert_eq!(
            eocd::parse_eocd(&bytes),
            Err(eocd::ParseError::InconsistentDisks)
        );
        write_sample(&eocd_adv_dir, i, &bytes)?;
        eocd_adv_manifest.push((format!("{i:04}.bin"), "InconsistentDisks"));
    }
    for i in 150..200 {
        let mut bytes = build_eocd(&mut rng, 0, false);
        let big = rng.next_in_range(50, 500) as u16;
        bytes[20..22].copy_from_slice(&big.to_le_bytes());
        assert_eq!(
            eocd::parse_eocd(&bytes),
            Err(eocd::ParseError::ShortComment)
        );
        write_sample(&eocd_adv_dir, i, &bytes)?;
        eocd_adv_manifest.push((format!("{i:04}.bin"), "ShortComment"));
    }
    write_manifest(
        &eocd_adv_dir,
        &eocd_adv_manifest
            .iter()
            .map(|(s, v)| (s.clone(), *v))
            .collect::<Vec<_>>(),
    )?;

    eprintln!("Wrote corpus under {}", out_root.display());
    eprintln!("  lfh-valid:        1000");
    eprintln!("  lfh-adversarial:   500");
    eprintln!("  eocd-valid:        100");
    eprintln!("  eocd-adversarial:  200");
    eprintln!("  TOTAL:           1800");
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().collect();
    let Some(out_root) = args.get(1).cloned() else {
        eprintln!("usage: zip-corpus-gen <output-dir>");
        return ExitCode::from(2);
    };
    let path = PathBuf::from(out_root);
    match run(&path) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("FAIL: {e}");
            ExitCode::from(1)
        }
    }
}
