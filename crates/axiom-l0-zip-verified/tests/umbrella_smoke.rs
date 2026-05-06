// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
//
// P1.12 — integration smoke tests for the verified ZIP umbrella.
// Each test exercises a different re-exported entry point so the
// coverage gate (Gap-10) can attest the umbrella is real, not
// dead code.

use axiom_l0_zip_verified::{
    cdr, consistency, eocd, parse_lfh, LfhParseError, LFH_FIXED_SIZE, LFH_SIGNATURE,
};

/// Minimal well-formed LFH (30 bytes, no name, no extra). Same
/// shape `axiom-zip-ref::lfh::tests` uses.
fn minimal_lfh() -> Vec<u8> {
    let mut v = Vec::with_capacity(LFH_FIXED_SIZE);
    v.extend_from_slice(&LFH_SIGNATURE.to_le_bytes()); // 4 (sig)
    v.extend_from_slice(&[0x14, 0x00]); // 2 (versionNeeded)
    v.extend_from_slice(&[0u8; 20]); // 20 (flags+method+time+date+crc+csize+usize)
    v.extend_from_slice(&0u16.to_le_bytes()); // 2 (nameLen)
    v.extend_from_slice(&0u16.to_le_bytes()); // 2 (extraLen)
    debug_assert_eq!(v.len(), LFH_FIXED_SIZE);
    v
}

/// Minimal well-formed CDR record (46 bytes, zero-length name/
/// extra/comment, `lfh_offset` = 0).
fn minimal_cdr() -> Vec<u8> {
    let mut v = Vec::with_capacity(cdr::FIXED_SIZE);
    v.extend_from_slice(&cdr::SIGNATURE.to_le_bytes());
    v.extend_from_slice(&[0x14, 0x00, 0x14, 0x00]); // versionMadeBy + versionNeeded
    v.extend_from_slice(&[0u8; 8]);
    v.extend_from_slice(&[0u8; 12]);
    v.extend_from_slice(&0u16.to_le_bytes()); // nameLen
    v.extend_from_slice(&0u16.to_le_bytes()); // extraLen
    v.extend_from_slice(&0u16.to_le_bytes()); // commentLen
    v.extend_from_slice(&[0u8; 8]);
    v.extend_from_slice(&0u32.to_le_bytes()); // lfhOffset
    v
}

/// Minimal well-formed EOCD (22 bytes, zero-length comment).
fn minimal_eocd() -> Vec<u8> {
    let mut v = Vec::with_capacity(eocd::FIXED_SIZE);
    v.extend_from_slice(&eocd::SIGNATURE.to_le_bytes());
    v.extend_from_slice(&[0u8; 4]); // diskNumber + cdStartDisk
    v.extend_from_slice(&1u16.to_le_bytes()); // entriesOnThisDisk
    v.extend_from_slice(&1u16.to_le_bytes()); // totalEntries
    v.extend_from_slice(&46u32.to_le_bytes()); // cdSize
    v.extend_from_slice(&30u32.to_le_bytes()); // cdOffset
    v.extend_from_slice(&0u16.to_le_bytes()); // commentLen
    v
}

fn minimal_archive() -> Vec<u8> {
    let mut v = Vec::with_capacity(98);
    v.extend(minimal_lfh());
    v.extend(minimal_cdr());
    v.extend(minimal_eocd());
    v
}

#[test]
fn umbrella_lfh_parse_ok() {
    let bytes = minimal_lfh();
    let (lfh, n) = parse_lfh(&bytes).expect("minimal LFH parses");
    assert_eq!(n, LFH_FIXED_SIZE);
    assert_eq!(lfh.file_name.len(), 0);
    assert_eq!(lfh.extra_field.len(), 0);
}

#[test]
fn umbrella_lfh_parse_bad_signature() {
    let mut bytes = minimal_lfh();
    bytes[0] ^= 0xff;
    assert_eq!(parse_lfh(&bytes), Err(LfhParseError::BadSignature));
}

#[test]
fn umbrella_eocd_parse_ok() {
    let bytes = minimal_eocd();
    let res = eocd::parse_eocd(&bytes).expect("minimal EOCD parses");
    assert_eq!(res.0.total_entries, 1);
}

#[test]
fn umbrella_eocd_find() {
    let archive = minimal_archive();
    let off = eocd::find_eocd(&archive).expect("EOCD locatable");
    assert_eq!(off, 76);
}

#[test]
fn umbrella_cdr_parse_ok() {
    let bytes = minimal_cdr();
    let (c, n) = cdr::parse_cdr(&bytes).expect("minimal CDR parses");
    assert_eq!(n, cdr::FIXED_SIZE);
    assert_eq!(c.lfh_offset, 0);
}

#[test]
fn umbrella_cdr_sequence_parses_one() {
    let bytes = minimal_cdr();
    let cdrs = cdr::parse_cdr_sequence(&bytes).expect("single CDR sequence parses");
    assert_eq!(cdrs.len(), 1);
}

#[test]
fn umbrella_consistency_minimal_archive_ok() {
    let bytes = minimal_archive();
    let a = consistency::parse_archive(&bytes).expect("minimal archive parses");
    assert_eq!(a.cdrs.len(), 1);
    assert_eq!(a.lfhs.len(), 1);
    assert_eq!(a.eocd.total_entries, 1);
}

#[test]
fn umbrella_consistency_no_eocd_rejected() {
    // 50 bytes of zeros — no EOCD signature anywhere.
    let bytes = vec![0u8; 50];
    assert_eq!(
        consistency::parse_archive(&bytes),
        Err(consistency::ArchiveError::NoEocd)
    );
}

#[test]
fn umbrella_consistency_lfh_offset_oob_rejected() {
    // Minimal archive with CDR.lfh_offset patched past EOF.
    let mut bytes = minimal_archive();
    let cdr_lfh_off_field = 30 + 42; // CDR starts at 30, lfh_offset at offset 42
    bytes[cdr_lfh_off_field..cdr_lfh_off_field + 4].copy_from_slice(&0xffff_ffffu32.to_le_bytes());
    assert_eq!(
        consistency::parse_archive(&bytes),
        Err(consistency::ArchiveError::LfhOffsetOob)
    );
}

#[test]
fn umbrella_consistency_dd_mode_lfh_zero_accepted() {
    // DD-mode LFH with zero crc/sizes (the canonical APPNOTE.TXT shape).
    let mut bytes = minimal_archive();
    // Set DD bit on LFH (offset 6) and CDR (offset 30+8=38).
    bytes[6] = 0x08;
    bytes[7] = 0x00;
    bytes[38] = 0x08;
    bytes[39] = 0x00;
    // LFH crc/sizes already zero, CDR crc/sizes already zero — agree by both branches.
    assert!(consistency::parse_archive(&bytes).is_ok());
}

#[test]
fn umbrella_consistency_dd_mode_lfh_matches_cdr_accepted() {
    // Relaxed DD branch (Gap-2 closure): LFH carries canonical
    // values matching CDR. apksigner emits this shape.
    let mut bytes = minimal_archive();
    bytes[6] = 0x08;
    bytes[7] = 0x00;
    bytes[38] = 0x08;
    bytes[39] = 0x00;
    // Set both LFH and CDR crc/sizes to the same non-zero values.
    let crc: u32 = 0xdead_beef;
    let cs: u32 = 42;
    let us: u32 = 100;
    // LFH @ offset 14 (crc), 18 (cs), 22 (us)
    bytes[14..18].copy_from_slice(&crc.to_le_bytes());
    bytes[18..22].copy_from_slice(&cs.to_le_bytes());
    bytes[22..26].copy_from_slice(&us.to_le_bytes());
    // CDR starts at 30; crc @ +16, cs @ +20, us @ +24
    bytes[30 + 16..30 + 20].copy_from_slice(&crc.to_le_bytes());
    bytes[30 + 20..30 + 24].copy_from_slice(&cs.to_le_bytes());
    bytes[30 + 24..30 + 28].copy_from_slice(&us.to_le_bytes());
    assert!(consistency::parse_archive(&bytes).is_ok());
}

#[test]
fn umbrella_consistency_dd_mode_lfh_partial_rejected() {
    // Relaxed DD branch must still reject "only-some-fields-zero,
    // others-don't-match-CDR" — this is what catches the
    // BadPack-class field-tampering attempt.
    let mut bytes = minimal_archive();
    bytes[6] = 0x08;
    bytes[7] = 0x00;
    bytes[38] = 0x08;
    bytes[39] = 0x00;
    // LFH: crc=0, cs=42, us=0   (partial — neither all-zero nor all-match)
    bytes[14..18].copy_from_slice(&0u32.to_le_bytes());
    bytes[18..22].copy_from_slice(&42u32.to_le_bytes());
    bytes[22..26].copy_from_slice(&0u32.to_le_bytes());
    // CDR: crc=0, cs=99, us=0
    bytes[30 + 16..30 + 20].copy_from_slice(&0u32.to_le_bytes());
    bytes[30 + 20..30 + 24].copy_from_slice(&99u32.to_le_bytes());
    bytes[30 + 24..30 + 28].copy_from_slice(&0u32.to_le_bytes());
    assert_eq!(
        consistency::parse_archive(&bytes),
        Err(consistency::ArchiveError::FieldMismatch)
    );
}

#[test]
fn umbrella_constants_re_exported() {
    assert_eq!(LFH_SIGNATURE, 0x0403_4b50);
    assert_eq!(LFH_FIXED_SIZE, 30);
    assert_eq!(eocd::SIGNATURE, 0x0605_4b50);
    assert_eq!(eocd::FIXED_SIZE, 22);
    assert_eq!(cdr::SIGNATURE, 0x0201_4b50);
    assert_eq!(cdr::FIXED_SIZE, 46);
    assert_eq!(consistency::K_MAX_EOCD_SEARCH, 65557);
    assert_eq!(consistency::GPB_DATA_DESCRIPTOR_MASK, 0x0008);
}

#[test]
fn umbrella_tv_receipts_published() {
    use axiom_l0_zip_verified::tv;
    // Sanity-check the receipt re-export is wired up.
    let n = std::hint::black_box(tv::LFH_AGREEMENT_COUNT);
    let sha = std::hint::black_box(tv::LFH_LEAN_OUTPUT_SHA256);
    assert_eq!(sha.len(), 64);
    assert!(n > 0, "TV agreement count should be positive");
}
