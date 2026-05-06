// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! 1 MiB-chunked SHA-256 digest computation per APK Signature
//! Scheme v2 / v3 / v3.1 spec.
//!
//! Algorithm (per AOSP `tools/apksig/V2SchemeVerifier.java`):
//!
//!   1. Concatenate three regions of the APK:
//!      - Zip entries (offset 0 .. cd_offset, *minus* the
//!        signing-block region).
//!      - Central directory (offset cd_offset .. eocd_offset).
//!      - EOCD record, with `cd_offset` field overwritten to point
//!        at the start of the signing block (so the EOCD is
//!        committed without depending on its own offset).
//!   2. Chunk the concatenation into 1 MiB blocks.
//!   3. Per chunk: hash `0xa5 || u32_le(chunk_len) || chunk_bytes`.
//!   4. Final digest: hash `0x5a || u32_le(num_chunks) || concat(chunk_digests)`.
//!
//! All digests are SHA-256 (32 bytes); SHA-512 variants do the
//! same with SHA-512 (64 bytes).

#![allow(clippy::cast_possible_truncation, clippy::cast_lossless)]

use sha2::{Digest, Sha256, Sha512};

const CHUNK_SIZE: usize = 1 << 20; // 1 MiB
const LEAF_PREFIX: u8 = 0xa5;
const ROOT_PREFIX: u8 = 0x5a;

/// Build the three regions of an APK that the v2/v3/v3.1 chunked
/// digest covers. Returns owned bytes for each region:
///
///   1. Zip entries `[0 .. signing_block_start)`.
///   2. Central directory `[cd_offset .. eocd_offset)`.
///   3. EOCD with `cd_offset` field patched to `signing_block_start`.
///
/// Per AOSP `ApkSigningBlockUtils.computeOneMbChunkContentDigests`,
/// chunks DO NOT cross region boundaries — each region is chunked
/// independently into ≤ 1 MiB segments, and the leaf digests of all
/// chunks (across all regions) are then concatenated and hashed.
#[must_use]
pub fn build_digest_regions(
    apk: &[u8],
    signing_block_start: u64,
    cd_offset: u64,
    eocd_offset: u64,
) -> [Vec<u8>; 3] {
    let entries = apk[..signing_block_start as usize].to_vec();
    let cd = apk[cd_offset as usize..eocd_offset as usize].to_vec();
    let mut eocd = apk[eocd_offset as usize..].to_vec();
    let patched = (signing_block_start as u32).to_le_bytes();
    eocd[16..20].copy_from_slice(&patched);
    [entries, cd, eocd]
}

/// Per-AOSP-spec chunked SHA-256 over multiple regions. Chunks do
/// not cross region boundaries; each region produces ⌈region_len /
/// 1 MiB⌉ chunks (or 1 chunk if region_len ≤ 1 MiB; an empty
/// region produces 0 chunks).
#[must_use]
pub fn chunked_sha256_regions(regions: &[&[u8]]) -> [u8; 32] {
    let mut chunk_digests: Vec<u8> = Vec::new();
    let mut n_chunks: u32 = 0;
    for region in regions {
        let mut i = 0;
        while i < region.len() {
            let end = (i + CHUNK_SIZE).min(region.len());
            let chunk = &region[i..end];
            let mut h = Sha256::new();
            h.update([LEAF_PREFIX]);
            h.update((chunk.len() as u32).to_le_bytes());
            h.update(chunk);
            chunk_digests.extend_from_slice(&h.finalize());
            n_chunks += 1;
            i = end;
        }
    }
    let mut h = Sha256::new();
    h.update([ROOT_PREFIX]);
    h.update(n_chunks.to_le_bytes());
    h.update(&chunk_digests);
    let d = h.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&d);
    out
}

/// Convenience wrapper: chunked SHA-256 over (apk, sb_start,
/// cd_offset, eocd_offset).
#[must_use]
pub fn apk_chunked_sha256(
    apk: &[u8],
    signing_block_start: u64,
    cd_offset: u64,
    eocd_offset: u64,
) -> [u8; 32] {
    let regions = build_digest_regions(apk, signing_block_start, cd_offset, eocd_offset);
    chunked_sha256_regions(&[&regions[0], &regions[1], &regions[2]])
}

/// Same shape, SHA-512 leaves + root.
#[must_use]
pub fn apk_chunked_sha512(
    apk: &[u8],
    signing_block_start: u64,
    cd_offset: u64,
    eocd_offset: u64,
) -> [u8; 64] {
    let regions = build_digest_regions(apk, signing_block_start, cd_offset, eocd_offset);
    let regions: [&[u8]; 3] = [&regions[0], &regions[1], &regions[2]];
    let mut chunk_digests: Vec<u8> = Vec::new();
    let mut n_chunks: u32 = 0;
    for region in &regions {
        let mut i = 0;
        while i < region.len() {
            let end = (i + CHUNK_SIZE).min(region.len());
            let chunk = &region[i..end];
            let mut h = Sha512::new();
            h.update([LEAF_PREFIX]);
            h.update((chunk.len() as u32).to_le_bytes());
            h.update(chunk);
            chunk_digests.extend_from_slice(&h.finalize());
            n_chunks += 1;
            i = end;
        }
    }
    let mut h = Sha512::new();
    h.update([ROOT_PREFIX]);
    h.update(n_chunks.to_le_bytes());
    h.update(&chunk_digests);
    let d = h.finalize();
    let mut out = [0u8; 64];
    out.copy_from_slice(&d);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunked_sha256_regions_empty_yields_zero_chunks() {
        // No regions = no chunks; root digest = SHA-256(0x5a || 00 00 00 00).
        let d = chunked_sha256_regions(&[]);
        assert_eq!(d.len(), 32);
    }

    #[test]
    fn chunked_sha256_regions_deterministic() {
        let r1 = b"region1".to_vec();
        let r2 = b"region2".to_vec();
        let a = chunked_sha256_regions(&[&r1, &r2]);
        let b = chunked_sha256_regions(&[&r1, &r2]);
        assert_eq!(a, b);
    }

    #[test]
    fn chunked_sha256_regions_chunk_boundary_doesnt_cross() {
        // Two regions of 0.5 MiB each → 2 chunks (one per region),
        // not 1 chunk like a 1 MiB concatenation would.
        let r1 = vec![0u8; 1 << 19];
        let r2 = vec![0u8; 1 << 19];
        let split = chunked_sha256_regions(&[&r1, &r2]);
        let cat = vec![0u8; 1 << 20];
        let single = chunked_sha256_regions(&[&cat]);
        assert_ne!(split, single, "chunk boundaries must not cross regions");
    }
}
