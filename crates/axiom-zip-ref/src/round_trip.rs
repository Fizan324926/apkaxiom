// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Round-trip tests for `parse_lfh` / `parse_eocd`.
//!
//! Each test builds a known-shape `Lfh` / `Eocd`, encodes it via the
//! in-test encoder helpers below, parses the bytes back, and asserts
//! field-level equality.

use crate::{eocd, lfh};

/// Encode an `Lfh` into the canonical APPNOTE.TXT byte sequence.
/// In-test only — production encoding lives in P1.15.
fn encode_lfh(h: &lfh::Lfh) -> Vec<u8> {
    let mut out = Vec::with_capacity(lfh::FIXED_SIZE + h.file_name.len() + h.extra_field.len());
    out.extend_from_slice(&lfh::SIGNATURE.to_le_bytes());
    out.extend_from_slice(&h.version_needed.to_le_bytes());
    out.extend_from_slice(&h.general_flags.to_le_bytes());
    out.extend_from_slice(&h.compression_method.to_le_bytes());
    out.extend_from_slice(&h.last_mod_time.to_le_bytes());
    out.extend_from_slice(&h.last_mod_date.to_le_bytes());
    out.extend_from_slice(&h.crc32.to_le_bytes());
    out.extend_from_slice(&h.compressed_size.to_le_bytes());
    out.extend_from_slice(&h.uncompressed_size.to_le_bytes());
    let name_len: u16 = h.file_name.len().try_into().expect("name fits u16");
    let extra_len: u16 = h.extra_field.len().try_into().expect("extra fits u16");
    out.extend_from_slice(&name_len.to_le_bytes());
    out.extend_from_slice(&extra_len.to_le_bytes());
    out.extend_from_slice(&h.file_name);
    out.extend_from_slice(&h.extra_field);
    out
}

fn encode_eocd(e: &eocd::Eocd) -> Vec<u8> {
    let mut out = Vec::with_capacity(eocd::FIXED_SIZE + e.comment.len());
    out.extend_from_slice(&eocd::SIGNATURE.to_le_bytes());
    out.extend_from_slice(&e.disk_number.to_le_bytes());
    out.extend_from_slice(&e.cd_start_disk.to_le_bytes());
    out.extend_from_slice(&e.entries_on_this_disk.to_le_bytes());
    out.extend_from_slice(&e.total_entries.to_le_bytes());
    out.extend_from_slice(&e.cd_size.to_le_bytes());
    out.extend_from_slice(&e.cd_offset.to_le_bytes());
    let comment_len: u16 = e.comment.len().try_into().expect("comment fits u16");
    out.extend_from_slice(&comment_len.to_le_bytes());
    out.extend_from_slice(&e.comment);
    out
}

#[test]
fn lfh_round_trip_minimal() {
    let h = lfh::Lfh {
        version_needed: 20,
        general_flags: 0,
        compression_method: 0,
        last_mod_time: 0,
        last_mod_date: 0,
        crc32: 0,
        compressed_size: 0,
        uncompressed_size: 0,
        file_name: vec![],
        extra_field: vec![],
    };
    let bytes = encode_lfh(&h);
    let (parsed, n) = lfh::parse_lfh(&bytes).unwrap();
    assert_eq!(parsed, h);
    assert_eq!(n, lfh::FIXED_SIZE);
}

#[test]
fn lfh_round_trip_with_name() {
    let h = lfh::Lfh {
        version_needed: 20,
        general_flags: 0x0800, // UTF-8 filename flag
        compression_method: 8, // deflate
        last_mod_time: 0x1234,
        last_mod_date: 0x5678,
        crc32: 0xdead_beef,
        compressed_size: 100,
        uncompressed_size: 200,
        file_name: b"AndroidManifest.xml".to_vec(),
        extra_field: vec![],
    };
    let bytes = encode_lfh(&h);
    let (parsed, n) = lfh::parse_lfh(&bytes).unwrap();
    assert_eq!(parsed, h);
    assert_eq!(n, lfh::FIXED_SIZE + 19);
}

#[test]
fn lfh_round_trip_with_extra() {
    let h = lfh::Lfh {
        version_needed: 45, // ZIP64 flag
        general_flags: 0,
        compression_method: 0,
        last_mod_time: 0,
        last_mod_date: 0,
        crc32: 0xffff_ffff,
        compressed_size: 0xffff_ffff,
        uncompressed_size: 0xffff_ffff,
        file_name: b"classes.dex".to_vec(),
        // 4-byte extra-field header (tag + len) followed by 16
        // payload bytes — matches a typical ZIP64 extended-info
        // extra block layout.
        extra_field: {
            let mut v = vec![0x01, 0x00, 0x10, 0x00];
            v.extend_from_slice(&[0u8; 16]);
            v
        },
    };
    let bytes = encode_lfh(&h);
    let (parsed, _) = lfh::parse_lfh(&bytes).unwrap();
    assert_eq!(parsed, h);
}

#[test]
fn lfh_round_trip_max_field_lengths() {
    // Just under u16::MAX for both name and extra. We don't actually
    // build a 65535-byte name (it'd be slow); 1 KB is enough.
    let h = lfh::Lfh {
        version_needed: 20,
        general_flags: 0,
        compression_method: 0,
        last_mod_time: 0,
        last_mod_date: 0,
        crc32: 0,
        compressed_size: 0,
        uncompressed_size: 0,
        file_name: vec![b'A'; 1024],
        extra_field: vec![b'B'; 1024],
    };
    let bytes = encode_lfh(&h);
    let (parsed, n) = lfh::parse_lfh(&bytes).unwrap();
    assert_eq!(parsed, h);
    assert_eq!(n, lfh::FIXED_SIZE + 1024 + 1024);
}

#[test]
fn lfh_round_trip_multi_byte_filename() {
    let h = lfh::Lfh {
        version_needed: 20,
        general_flags: 0x0800, // UTF-8
        compression_method: 0,
        last_mod_time: 0,
        last_mod_date: 0,
        crc32: 0,
        compressed_size: 0,
        uncompressed_size: 0,
        // Mix of ASCII + multibyte UTF-8.
        file_name: "résumé.pdf".as_bytes().to_vec(),
        extra_field: vec![],
    };
    let bytes = encode_lfh(&h);
    let (parsed, _) = lfh::parse_lfh(&bytes).unwrap();
    assert_eq!(parsed, h);
}

#[test]
fn eocd_round_trip_minimal() {
    let e = eocd::Eocd {
        disk_number: 0,
        cd_start_disk: 0,
        entries_on_this_disk: 0,
        total_entries: 0,
        cd_size: 0,
        cd_offset: 0,
        comment: vec![],
    };
    let bytes = encode_eocd(&e);
    let (parsed, n) = eocd::parse_eocd(&bytes).unwrap();
    assert_eq!(parsed, e);
    assert_eq!(n, eocd::FIXED_SIZE);
}

#[test]
fn eocd_round_trip_realistic_apk() {
    let e = eocd::Eocd {
        disk_number: 0,
        cd_start_disk: 0,
        entries_on_this_disk: 47,
        total_entries: 47,
        cd_size: 0x1234,
        cd_offset: 0x5678,
        comment: b"APKAXIOM-built".to_vec(),
    };
    let bytes = encode_eocd(&e);
    let (parsed, n) = eocd::parse_eocd(&bytes).unwrap();
    assert_eq!(parsed, e);
    assert_eq!(n, eocd::FIXED_SIZE + 14);
}

#[test]
fn eocd_round_trip_max_entries() {
    let e = eocd::Eocd {
        disk_number: 0,
        cd_start_disk: 0,
        entries_on_this_disk: 0xffff,
        total_entries: 0xffff,
        cd_size: 0xffff_ffff,
        cd_offset: 0xffff_ffff,
        comment: vec![],
    };
    let bytes = encode_eocd(&e);
    let (parsed, _) = eocd::parse_eocd(&bytes).unwrap();
    assert_eq!(parsed, e);
}

#[test]
fn eocd_round_trip_long_comment() {
    let e = eocd::Eocd {
        disk_number: 0,
        cd_start_disk: 0,
        entries_on_this_disk: 0,
        total_entries: 0,
        cd_size: 0,
        cd_offset: 0,
        comment: vec![b'.'; 4096],
    };
    let bytes = encode_eocd(&e);
    let (parsed, n) = eocd::parse_eocd(&bytes).unwrap();
    assert_eq!(parsed, e);
    assert_eq!(n, eocd::FIXED_SIZE + 4096);
}

#[test]
fn find_eocd_locates_at_end() {
    let e = eocd::Eocd {
        disk_number: 0,
        cd_start_disk: 0,
        entries_on_this_disk: 1,
        total_entries: 1,
        cd_size: 100,
        cd_offset: 0,
        comment: vec![],
    };
    // Pad with 200 bytes of garbage (no false-positive signatures
    // from the LCG seed).
    let mut bytes = vec![0xaa; 200];
    bytes.extend_from_slice(&encode_eocd(&e));
    let off = eocd::find_eocd(&bytes).expect("found EOCD");
    assert_eq!(off, 200);
    let (parsed, _) = eocd::parse_eocd(&bytes[off..]).unwrap();
    assert_eq!(parsed, e);
}

#[test]
fn lfh_50_round_trips_random_shapes() {
    // Use the same LCG as the fuzz harness; build 50 *valid* LFHs
    // from random fields and round-trip each.
    use super::fuzz_helpers::Lcg;
    let mut rng = Lcg::new(0xc0de_d0d0_1234_abcd);
    for _ in 0..50 {
        let name_len = (rng.next_u32() % 64) as usize;
        let extra_len = (rng.next_u32() % 32) as usize;
        let mut name = vec![0u8; name_len];
        rng.fill(&mut name);
        let mut extra = vec![0u8; extra_len];
        rng.fill(&mut extra);
        let h = lfh::Lfh {
            version_needed: rng.next_u32() as u16,
            general_flags: rng.next_u32() as u16,
            compression_method: rng.next_u32() as u16,
            last_mod_time: rng.next_u32() as u16,
            last_mod_date: rng.next_u32() as u16,
            crc32: rng.next_u32(),
            compressed_size: rng.next_u32(),
            uncompressed_size: rng.next_u32(),
            file_name: name,
            extra_field: extra,
        };
        let bytes = encode_lfh(&h);
        let (parsed, _) = lfh::parse_lfh(&bytes).unwrap();
        assert_eq!(parsed, h);
    }
}

#[test]
fn eocd_50_round_trips_random_shapes() {
    use super::fuzz_helpers::Lcg;
    let mut rng = Lcg::new(0xfeed_face_5678_9012);
    for _ in 0..50 {
        let comment_len = (rng.next_u32() % 128) as usize;
        let mut comment = vec![0u8; comment_len];
        rng.fill(&mut comment);
        let e = eocd::Eocd {
            disk_number: 0,
            cd_start_disk: 0,
            entries_on_this_disk: rng.next_u32() as u16,
            total_entries: rng.next_u32() as u16,
            cd_size: rng.next_u32(),
            cd_offset: rng.next_u32(),
            comment,
        };
        let bytes = encode_eocd(&e);
        let (parsed, _) = eocd::parse_eocd(&bytes).unwrap();
        assert_eq!(parsed, e);
    }
}
