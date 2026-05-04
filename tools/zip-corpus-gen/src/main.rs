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

use axiom_zip_ref::{archive, cdr, eocd, lfh};

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

#[allow(clippy::too_many_lines, clippy::cognitive_complexity)] // single deterministic generator end-to-end
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

    // ----- P1.6 additions -----------------------------------------------
    // The P1.6 corpus extends the P1.5 base with CDR + whole-archive
    // samples plus a hand-crafted BadPack-class adversarial set and a
    // mutation-engine bucket.

    // 5) 200 valid CDR samples — varying name + extra + comment lengths.
    let cdr_valid_dir = out_root.join("cdr-valid");
    let mut cdr_valid_manifest = Vec::with_capacity(200);
    for i in 0..200 {
        let name_len = (rng.next_in_range(0, 32)) as u16;
        let extra_len = (rng.next_in_range(0, 16)) as u16;
        let comment_len = (rng.next_in_range(0, 24)) as u16;
        let bytes = build_cdr(&mut rng, name_len, extra_len, comment_len, 0);
        assert!(cdr::parse_cdr(&bytes).is_ok());
        write_sample(&cdr_valid_dir, i, &bytes)?;
        cdr_valid_manifest.push((format!("{i:04}.bin"), "ok"));
    }
    write_manifest(
        &cdr_valid_dir,
        &cdr_valid_manifest
            .iter()
            .map(|(s, v)| (s.clone(), *v))
            .collect::<Vec<_>>(),
    )?;

    // 6) 200 adversarial CDR samples — 40 per ParseError variant.
    let cdr_adv_dir = out_root.join("cdr-adversarial");
    let mut cdr_adv_manifest = Vec::with_capacity(200);
    // 6a) ShortHeader: 0..40
    for i in 0..40 {
        let cut = (rng.next_in_range(0, 46)) as usize;
        let bytes = vec![0u8; cut];
        assert_eq!(cdr::parse_cdr(&bytes), Err(cdr::ParseError::ShortHeader));
        write_sample(&cdr_adv_dir, i, &bytes)?;
        cdr_adv_manifest.push((format!("{i:04}.bin"), "ShortHeader"));
    }
    // 6b) BadSignature: 40..80
    for i in 40..80 {
        let mut bytes = build_cdr(&mut rng, 0, 0, 0, 0);
        bytes[0] = bytes[0].wrapping_add(1);
        assert_eq!(cdr::parse_cdr(&bytes), Err(cdr::ParseError::BadSignature));
        write_sample(&cdr_adv_dir, i, &bytes)?;
        cdr_adv_manifest.push((format!("{i:04}.bin"), "BadSignature"));
    }
    // 6c) ShortName: 80..120
    for i in 80..120 {
        let mut bytes = build_cdr(&mut rng, 0, 0, 0, 0);
        let big = rng.next_in_range(50, 1000) as u16;
        bytes[28..30].copy_from_slice(&big.to_le_bytes());
        assert_eq!(cdr::parse_cdr(&bytes), Err(cdr::ParseError::ShortName));
        write_sample(&cdr_adv_dir, i, &bytes)?;
        cdr_adv_manifest.push((format!("{i:04}.bin"), "ShortName"));
    }
    // 6d) ShortExtra: 120..160
    for i in 120..160 {
        let mut bytes = build_cdr(&mut rng, 0, 0, 0, 0);
        let big = rng.next_in_range(50, 1000) as u16;
        bytes[30..32].copy_from_slice(&big.to_le_bytes());
        assert_eq!(cdr::parse_cdr(&bytes), Err(cdr::ParseError::ShortExtra));
        write_sample(&cdr_adv_dir, i, &bytes)?;
        cdr_adv_manifest.push((format!("{i:04}.bin"), "ShortExtra"));
    }
    // 6e) ShortComment: 160..200
    for i in 160..200 {
        let mut bytes = build_cdr(&mut rng, 0, 0, 0, 0);
        let big = rng.next_in_range(50, 1000) as u16;
        bytes[32..34].copy_from_slice(&big.to_le_bytes());
        assert_eq!(cdr::parse_cdr(&bytes), Err(cdr::ParseError::ShortComment));
        write_sample(&cdr_adv_dir, i, &bytes)?;
        cdr_adv_manifest.push((format!("{i:04}.bin"), "ShortComment"));
    }
    write_manifest(
        &cdr_adv_dir,
        &cdr_adv_manifest
            .iter()
            .map(|(s, v)| (s.clone(), *v))
            .collect::<Vec<_>>(),
    )?;

    // 7) 300 valid full archives — 1..=4 entries each.
    let archive_valid_dir = out_root.join("archive-valid");
    let mut archive_valid_manifest = Vec::with_capacity(300);
    for i in 0..300 {
        let n_entries = rng.next_in_range(1, 5) as usize;
        let bytes = build_archive(&mut rng, n_entries);
        assert!(
            archive::parse_archive(&bytes).is_ok(),
            "sample {i} failed to round-trip; n_entries={n_entries}"
        );
        write_sample(&archive_valid_dir, i, &bytes)?;
        archive_valid_manifest.push((format!("{i:04}.bin"), "ok"));
    }
    write_manifest(
        &archive_valid_dir,
        &archive_valid_manifest
            .iter()
            .map(|(s, v)| (s.clone(), *v))
            .collect::<Vec<_>>(),
    )?;

    // 8) 50 BadPack-class CVE-style adversarial archives.
    //
    //    Five families × 10 samples each:
    //      - lfh-oob          (CDR.lfh_offset > bs.size)
    //      - lfh-magic        (CDR.lfh_offset points at non-LFH bytes)
    //      - cdr-count        (EOCD.totalEntries != actual count)
    //      - cd-out-of-range  (EOCD.cdOffset + cdSize > bs.size)
    //      - filename-mismatch (CDR filename != LFH filename)
    let badpack_dir = out_root.join("badpack-cves");
    let mut badpack_manifest = Vec::with_capacity(50);
    for i in 0..10 {
        // lfh-oob: minimal archive (LFH@0, CDR@30, EOCD@76); patch
        // CDR's lfh_offset (bytes 72..76) past EOF.
        let mut bytes = build_minimal_archive(&mut rng);
        bytes[72..76].copy_from_slice(&0xffff_ffffu32.to_le_bytes());
        assert_eq!(
            archive::parse_archive(&bytes),
            Err(archive::ArchiveError::LfhOffsetOob)
        );
        write_sample(&badpack_dir, i, &bytes)?;
        badpack_manifest.push((format!("{i:04}.bin"), "LfhOffsetOob"));
    }
    for i in 10..20 {
        // lfh-magic: minimal archive; patch CDR's lfh_offset to 1 so
        // the parser sees non-LFH bytes at the offset.
        let mut bytes = build_minimal_archive(&mut rng);
        bytes[72..76].copy_from_slice(&1u32.to_le_bytes());
        assert_eq!(
            archive::parse_archive(&bytes),
            Err(archive::ArchiveError::LfhInvalid)
        );
        write_sample(&badpack_dir, i, &bytes)?;
        badpack_manifest.push((format!("{i:04}.bin"), "LfhInvalid"));
    }
    for i in 20..30 {
        // cdr-count: minimal archive; patch EOCD totalEntries to 2.
        let mut bytes = build_minimal_archive(&mut rng);
        let eocd_pos = bytes.len() - 22;
        bytes[eocd_pos + 8..eocd_pos + 10].copy_from_slice(&2u16.to_le_bytes());
        bytes[eocd_pos + 10..eocd_pos + 12].copy_from_slice(&2u16.to_le_bytes());
        assert_eq!(
            archive::parse_archive(&bytes),
            Err(archive::ArchiveError::CdrCountMismatch)
        );
        write_sample(&badpack_dir, i, &bytes)?;
        badpack_manifest.push((format!("{i:04}.bin"), "CdrCountMismatch"));
    }
    for i in 30..40 {
        // cd-out-of-range: minimal archive; patch EOCD's cdOffset.
        let mut bytes = build_minimal_archive(&mut rng);
        let eocd_pos = bytes.len() - 22;
        bytes[eocd_pos + 16..eocd_pos + 20].copy_from_slice(&0xffff_ffffu32.to_le_bytes());
        assert_eq!(
            archive::parse_archive(&bytes),
            Err(archive::ArchiveError::CdOutOfRange)
        );
        write_sample(&badpack_dir, i, &bytes)?;
        badpack_manifest.push((format!("{i:04}.bin"), "CdOutOfRange"));
    }
    for i in 40..50 {
        // filename-mismatch — build the canonical asymmetric archive
        let bytes = build_filename_mismatch_archive(&mut rng);
        assert_eq!(
            archive::parse_archive(&bytes),
            Err(archive::ArchiveError::FilenameMismatch)
        );
        write_sample(&badpack_dir, i, &bytes)?;
        badpack_manifest.push((format!("{i:04}.bin"), "FilenameMismatch"));
    }
    for i in 50..60 {
        // field-mismatch — minimal archive, patch CDR.crc32 (or one of
        // the other three structural fields) so it disagrees with the
        // LFH. Cycles through which field is patched so the bucket
        // exercises all four cross-checked fields.
        let mut bytes = build_minimal_archive(&mut rng);
        // Field positions inside the CDR (which starts at archive
        // offset 30):
        //   crc32              @ CDR offset 16  → archive offset 46
        //   compressedSize     @ CDR offset 20  → archive offset 50
        //   uncompressedSize   @ CDR offset 24  → archive offset 54
        //   compressionMethod  @ CDR offset 10  → archive offset 40
        let (offset, expected_label) = match i % 4 {
            0 => (46usize, "FieldMismatch[crc32]"),
            1 => (50, "FieldMismatch[compressedSize]"),
            2 => (54, "FieldMismatch[uncompressedSize]"),
            _ => (40, "FieldMismatch[compressionMethod]"),
        };
        let bump_size = if expected_label.starts_with("FieldMismatch[compressionMethod]") {
            // u16 field — bump 2 bytes
            bytes[offset..offset + 2].copy_from_slice(&(rng.next_u16() | 0x0001).to_le_bytes());
            2
        } else {
            // u32 field — bump 4 bytes
            bytes[offset..offset + 4]
                .copy_from_slice(&(rng.next_u32() | 0x0000_0001).to_le_bytes());
            4
        };
        let _ = bump_size; // suppress unused
        assert_eq!(
            archive::parse_archive(&bytes),
            Err(archive::ArchiveError::FieldMismatch),
            "sample {i} ({expected_label}): expected FieldMismatch"
        );
        write_sample(&badpack_dir, i, &bytes)?;
        badpack_manifest.push((format!("{i:04}.bin"), "FieldMismatch"));
    }
    write_manifest(
        &badpack_dir,
        &badpack_manifest
            .iter()
            .map(|(s, v)| (s.clone(), *v))
            .collect::<Vec<_>>(),
    )?;

    // 9) 250 adversarial-mutated samples — radamsa-style mutations of
    //    valid full archives. Verdict is "agreement-only": we don't
    //    predict the verdict, just assert all three parsers agree at
    //    differential time. The manifest records "agreement" as the
    //    expectation.
    let mut_dir = out_root.join("adversarial-mutated");
    let mut mut_manifest = Vec::with_capacity(250);
    for i in 0..250 {
        let n = rng.next_in_range(1, 4) as usize;
        let base = build_archive(&mut rng, n);
        let bytes = mutate(&mut rng, base);
        write_sample(&mut_dir, i, &bytes)?;
        mut_manifest.push((format!("{i:04}.bin"), "agreement"));
    }
    write_manifest(
        &mut_dir,
        &mut_manifest
            .iter()
            .map(|(s, v)| (s.clone(), *v))
            .collect::<Vec<_>>(),
    )?;

    eprintln!("Wrote corpus under {}", out_root.display());
    eprintln!("  lfh-valid:           1000");
    eprintln!("  lfh-adversarial:      500");
    eprintln!("  eocd-valid:           100");
    eprintln!("  eocd-adversarial:     200");
    eprintln!("  cdr-valid:            200  [P1.6]");
    eprintln!("  cdr-adversarial:      200  [P1.6]");
    eprintln!("  archive-valid:        300  [P1.6]");
    eprintln!("  badpack-cves:          60  [P1.6]");
    eprintln!("  adversarial-mutated:  250  [P1.6]");
    eprintln!("  TOTAL:               2810");
    Ok(())
}

/// Build a single CDR record with the given variable-length region
/// sizes and `lfh_offset` field. Mirrors `Apkaxiom.Zip.CentralDirectory`.
fn build_cdr(
    rng: &mut Lcg,
    name_len: u16,
    extra_len: u16,
    comment_len: u16,
    lfh_offset: u32,
) -> Vec<u8> {
    let total = cdr::FIXED_SIZE + name_len as usize + extra_len as usize + comment_len as usize;
    let mut bytes = Vec::with_capacity(total);
    bytes.extend_from_slice(&cdr::SIGNATURE.to_le_bytes());
    bytes.extend_from_slice(&rng.next_u16().to_le_bytes()); // versionMadeBy
    bytes.extend_from_slice(&rng.next_u16().to_le_bytes()); // versionNeeded
    bytes.extend_from_slice(&rng.next_u16().to_le_bytes()); // generalFlags
    bytes.extend_from_slice(&rng.next_u16().to_le_bytes()); // compressionMethod
    bytes.extend_from_slice(&rng.next_u16().to_le_bytes()); // lastModTime
    bytes.extend_from_slice(&rng.next_u16().to_le_bytes()); // lastModDate
    bytes.extend_from_slice(&rng.next_u32().to_le_bytes()); // crc32
    bytes.extend_from_slice(&rng.next_u32().to_le_bytes()); // compressedSize
    bytes.extend_from_slice(&rng.next_u32().to_le_bytes()); // uncompressedSize
    bytes.extend_from_slice(&name_len.to_le_bytes()); // nameLen
    bytes.extend_from_slice(&extra_len.to_le_bytes()); // extraLen
    bytes.extend_from_slice(&comment_len.to_le_bytes()); // commentLen
    bytes.extend_from_slice(&[0u8; 2]); // diskNumberStart
    bytes.extend_from_slice(&rng.next_u16().to_le_bytes()); // internalAttrs
    bytes.extend_from_slice(&rng.next_u32().to_le_bytes()); // externalAttrs
    bytes.extend_from_slice(&lfh_offset.to_le_bytes()); // lfhOffset
    let mut name = vec![0u8; name_len as usize];
    rng.fill(&mut name);
    bytes.extend_from_slice(&name);
    let mut extra = vec![0u8; extra_len as usize];
    rng.fill(&mut extra);
    bytes.extend_from_slice(&extra);
    let mut comment = vec![0u8; comment_len as usize];
    rng.fill(&mut comment);
    bytes.extend_from_slice(&comment);
    bytes
}

/// Per-entry attribute set. Sampled once per archive entry so the
/// LFH and CDR see the *same* values for the structural fields the
/// archive driver cross-checks (`crc32`, `compressed_size`,
/// `uncompressed_size`, `compression_method`).
struct EntryAttrs {
    filename: Vec<u8>,
    crc32: u32,
    compressed_size: u32,
    uncompressed_size: u32,
    compression_method: u16,
    last_mod_time: u16,
    last_mod_date: u16,
    general_flags: u16,
}

/// Build a well-formed full ZIP archive with `n` entries. Returns the
/// concatenated byte sequence in the canonical layout:
/// `lfh_0 || lfh_1 || … || cdr_0 || cdr_1 || … || eocd`.
///
/// Each entry's filename and structural fields (`crc32`,
/// `compressed_size`, `uncompressed_size`, `compression_method`) are
/// *shared* between its LFH and CDR so the field-set consistency
/// check passes.
fn build_archive(rng: &mut Lcg, n: usize) -> Vec<u8> {
    debug_assert!(n >= 1, "archive must have ≥1 entry");
    // Per-entry attribute draws. Done in one pass so each entry has a
    // single source of truth for the fields LFH and CDR share.
    let mut entries: Vec<EntryAttrs> = Vec::with_capacity(n);
    for _ in 0..n {
        let nl = rng.next_in_range(0, 16) as usize;
        let mut filename = vec![0u8; nl];
        rng.fill(&mut filename);
        entries.push(EntryAttrs {
            filename,
            crc32: rng.next_u32(),
            compressed_size: rng.next_u32(),
            uncompressed_size: rng.next_u32(),
            compression_method: rng.next_u16(),
            last_mod_time: rng.next_u16(),
            last_mod_date: rng.next_u16(),
            general_flags: rng.next_u16(),
        });
    }
    // Build LFHs in order, recording each one's start offset.
    let mut bytes: Vec<u8> = Vec::new();
    let mut lfh_offsets: Vec<u32> = Vec::with_capacity(n);
    for entry in &entries {
        #[allow(clippy::cast_possible_truncation)]
        lfh_offsets.push(bytes.len() as u32);
        let nl = entry.filename.len() as u16;
        bytes.extend_from_slice(&lfh::SIGNATURE.to_le_bytes());
        bytes.extend_from_slice(&rng.next_u16().to_le_bytes()); // versionNeeded
        bytes.extend_from_slice(&entry.general_flags.to_le_bytes());
        bytes.extend_from_slice(&entry.compression_method.to_le_bytes());
        bytes.extend_from_slice(&entry.last_mod_time.to_le_bytes());
        bytes.extend_from_slice(&entry.last_mod_date.to_le_bytes());
        bytes.extend_from_slice(&entry.crc32.to_le_bytes());
        bytes.extend_from_slice(&entry.compressed_size.to_le_bytes());
        bytes.extend_from_slice(&entry.uncompressed_size.to_le_bytes());
        bytes.extend_from_slice(&nl.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes()); // extraLen
        bytes.extend_from_slice(&entry.filename);
    }
    // CD region starts here.
    #[allow(clippy::cast_possible_truncation)]
    let cd_start = bytes.len() as u32;
    let mut cd_bytes_size = 0u32;
    for (entry, lfh_off) in entries.iter().zip(lfh_offsets.iter()) {
        let nl = entry.filename.len() as u16;
        let cdr_bytes = {
            let mut v: Vec<u8> = Vec::new();
            v.extend_from_slice(&cdr::SIGNATURE.to_le_bytes());
            v.extend_from_slice(&rng.next_u16().to_le_bytes()); // versionMadeBy
            v.extend_from_slice(&rng.next_u16().to_le_bytes()); // versionNeeded
                                                                // Shared with LFH: general_flags, compression_method,
                                                                // lastMod, crc32, compressed/uncompressed sizes.
            v.extend_from_slice(&entry.general_flags.to_le_bytes());
            v.extend_from_slice(&entry.compression_method.to_le_bytes());
            v.extend_from_slice(&entry.last_mod_time.to_le_bytes());
            v.extend_from_slice(&entry.last_mod_date.to_le_bytes());
            v.extend_from_slice(&entry.crc32.to_le_bytes());
            v.extend_from_slice(&entry.compressed_size.to_le_bytes());
            v.extend_from_slice(&entry.uncompressed_size.to_le_bytes());
            v.extend_from_slice(&nl.to_le_bytes()); // nameLen
            v.extend_from_slice(&0u16.to_le_bytes()); // extraLen
            v.extend_from_slice(&0u16.to_le_bytes()); // commentLen
            v.extend_from_slice(&[0u8; 2]); // diskNumberStart
            v.extend_from_slice(&rng.next_u16().to_le_bytes()); // internalAttrs
            v.extend_from_slice(&rng.next_u32().to_le_bytes()); // externalAttrs
            v.extend_from_slice(&lfh_off.to_le_bytes()); // lfhOffset
            v.extend_from_slice(&entry.filename); // shared filename bytes
            v
        };
        #[allow(clippy::cast_possible_truncation)]
        {
            cd_bytes_size += cdr_bytes.len() as u32;
        }
        bytes.extend_from_slice(&cdr_bytes);
    }
    // EOCD trailer.
    bytes.extend_from_slice(&eocd::SIGNATURE.to_le_bytes());
    bytes.extend_from_slice(&[0u8; 4]); // diskNumber + cdStartDisk = 0
    let entries_u16 = u16::try_from(n).unwrap_or(u16::MAX);
    bytes.extend_from_slice(&entries_u16.to_le_bytes()); // entriesOnThisDisk
    bytes.extend_from_slice(&entries_u16.to_le_bytes()); // totalEntries
    bytes.extend_from_slice(&cd_bytes_size.to_le_bytes()); // cdSize
    bytes.extend_from_slice(&cd_start.to_le_bytes()); // cdOffset
    bytes.extend_from_slice(&0u16.to_le_bytes()); // commentLen
    bytes
}

/// Build the canonical minimal well-formed archive: 1 entry with
/// zero-length filename / extra / comment. Layout:
///
///   - LFH at offset 0 (30 bytes)
///   - CDR at offset 30 (46 bytes)
///   - EOCD at offset 76 (22 bytes)
///
/// Total: 98 bytes. This is the substrate the BadPack-class samples
/// patch — guaranteeing the patch offsets are stable across rng
/// states.
fn build_minimal_archive(rng: &mut Lcg) -> Vec<u8> {
    let mut v: Vec<u8> = Vec::with_capacity(98);
    // LFH at offset 0 (30 bytes total: 4 sig + 2 ver + 20 body + 2 nl + 2 el)
    v.extend_from_slice(&lfh::SIGNATURE.to_le_bytes());
    v.extend_from_slice(&rng.next_u16().to_le_bytes()); // versionNeeded
    v.extend_from_slice(&[0u8; 20]); // body up to nameLen
    v.extend_from_slice(&0u16.to_le_bytes()); // nameLen = 0
    v.extend_from_slice(&0u16.to_le_bytes()); // extraLen = 0
    assert_eq!(v.len(), 30);
    // CDR at offset 30 (46 bytes)
    v.extend_from_slice(&cdr::SIGNATURE.to_le_bytes());
    v.extend_from_slice(&[0x14, 0x00, 0x14, 0x00]);
    v.extend_from_slice(&[0u8; 8]);
    v.extend_from_slice(&[0u8; 4]); // crc32
    v.extend_from_slice(&[0u8; 4]); // compressedSize
    v.extend_from_slice(&[0u8; 4]); // uncompressedSize
    v.extend_from_slice(&0u16.to_le_bytes()); // nameLen = 0
    v.extend_from_slice(&0u16.to_le_bytes()); // extraLen
    v.extend_from_slice(&0u16.to_le_bytes()); // commentLen
    v.extend_from_slice(&[0u8; 2]); // diskNumberStart
    v.extend_from_slice(&[0u8; 2]); // internalAttrs
    v.extend_from_slice(&[0u8; 4]); // externalAttrs
    v.extend_from_slice(&0u32.to_le_bytes()); // lfhOffset = 0
    debug_assert_eq!(v.len(), 76);
    // EOCD at offset 76 (22 bytes)
    v.extend_from_slice(&eocd::SIGNATURE.to_le_bytes());
    v.extend_from_slice(&[0u8; 4]); // diskNumber + cdStartDisk
    v.extend_from_slice(&1u16.to_le_bytes()); // entriesOnThisDisk
    v.extend_from_slice(&1u16.to_le_bytes()); // totalEntries
    v.extend_from_slice(&46u32.to_le_bytes()); // cdSize
    v.extend_from_slice(&30u32.to_le_bytes()); // cdOffset
    v.extend_from_slice(&0u16.to_le_bytes()); // commentLen
    debug_assert_eq!(v.len(), 98);
    v
}

/// Build a deliberately-asymmetric archive where the CDR's filename
/// (1 byte: `b'A'`) disagrees with the LFH's filename (empty).
fn build_filename_mismatch_archive(rng: &mut Lcg) -> Vec<u8> {
    let mut v: Vec<u8> = Vec::new();
    // LFH at offset 0, nameLen = 0, no filename appended.
    v.extend_from_slice(&lfh::SIGNATURE.to_le_bytes());
    v.extend_from_slice(&rng.next_u16().to_le_bytes()); // versionNeeded
    v.extend_from_slice(&[0u8; 20]); // body up to nameLen (20 bytes)
    v.extend_from_slice(&0u16.to_le_bytes()); // nameLen = 0
    v.extend_from_slice(&0u16.to_le_bytes()); // extraLen = 0
    assert_eq!(v.len(), 30);
    // CDR at offset 30, nameLen = 1, filename "A".
    v.extend_from_slice(&cdr::SIGNATURE.to_le_bytes());
    v.extend_from_slice(&[0x14, 0x00, 0x14, 0x00]);
    v.extend_from_slice(&[0u8; 8]);
    v.extend_from_slice(&[0u8; 4]); // crc32
    v.extend_from_slice(&[0u8; 4]); // compressedSize
    v.extend_from_slice(&[0u8; 4]); // uncompressedSize
    v.extend_from_slice(&1u16.to_le_bytes()); // nameLen = 1
    v.extend_from_slice(&0u16.to_le_bytes()); // extraLen
    v.extend_from_slice(&0u16.to_le_bytes()); // commentLen
    v.extend_from_slice(&[0u8; 2]); // diskNumberStart
    v.extend_from_slice(&[0u8; 2]); // internalAttrs
    v.extend_from_slice(&[0u8; 4]); // externalAttrs
    v.extend_from_slice(&0u32.to_le_bytes()); // lfhOffset = 0
    v.push(b'A'); // filename "A"
    debug_assert_eq!(v.len(), 30 + 47);
    // EOCD at offset 77, cdSize = 47, cdOffset = 30.
    v.extend_from_slice(&eocd::SIGNATURE.to_le_bytes());
    v.extend_from_slice(&[0u8; 4]);
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&47u32.to_le_bytes());
    v.extend_from_slice(&30u32.to_le_bytes());
    v.extend_from_slice(&0u16.to_le_bytes());
    v
}

/// In-house radamsa-style mutation engine. Picks one of four mutation
/// kinds at random and applies it to the input bytes:
///
///   - bit-flip a single byte at a random offset
///   - delete a random byte
///   - insert a random byte
///   - bump a random length-field-shaped u16
///
/// The output is *deterministic* given the rng state.
fn mutate(rng: &mut Lcg, mut bytes: Vec<u8>) -> Vec<u8> {
    if bytes.is_empty() {
        return bytes;
    }
    let kind = rng.next_in_range(0, 4);
    let off = (rng.next_u32() as usize) % bytes.len();
    match kind {
        0 => {
            // bit-flip one byte
            let mask = 1u8 << (rng.next_u32() % 8);
            bytes[off] ^= mask;
        }
        1 => {
            // delete one byte (skip if length 1)
            if bytes.len() > 1 {
                bytes.remove(off);
            }
        }
        2 => {
            // insert one random byte at off
            #[allow(clippy::cast_possible_truncation)]
            let b = (rng.next_u32() & 0xff) as u8;
            bytes.insert(off, b);
        }
        _ => {
            // bump a u16 at a random aligned offset
            if bytes.len() >= 2 {
                let pos = off.min(bytes.len() - 2);
                let cur = u16::from_le_bytes([bytes[pos], bytes[pos + 1]]);
                let bumped = cur.wrapping_add(rng.next_u16());
                bytes[pos..pos + 2].copy_from_slice(&bumped.to_le_bytes());
            }
        }
    }
    bytes
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
