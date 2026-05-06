// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
//
// P1.10 §B item 7 (HARD) — chunk-size invariance.
//
// `parse_with_commit_chain` MUST produce a bit-identical Merkle
// root regardless of how the underlying `Read` returns data.
// We exercise this by wrapping each fixture in a `ChunkedReader`
// that yields exactly `chunk_size` bytes per `read()` call —
// forcing the streaming parser through every relevant chunking
// regime: byte-by-byte (size = 1), small (17, 64), medium
// (4096), and large (65536, 1 MiB).
//
// The body accumulator in `commit_chain.rs` collects body chunks
// under a single BLAKE3 hash, so the body leaf is one-per-entry
// regardless of how many `ZipEntryData` events the parser fires.
// Without that accumulator this test would catch a real
// non-determinism bug.

#![allow(clippy::needless_lifetimes)]

use std::io::Read;

use axiom_l1_rs::commit_chain::parse_with_commit_chain;

const FIXTURES: &[&str] = &[
    "fdroid-privileged-2050.apk",
    "clipboard.apk",
    "tickytacky-mirror.apk",
    "wifiautoff.apk",
];

fn fixture_path(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// `Read` adapter that returns at most `chunk` bytes per `read()`,
/// regardless of the caller's buffer size. Forces the streaming
/// parser to consume the input at the requested granularity.
struct ChunkedReader<'a> {
    bytes: &'a [u8],
    pos: usize,
    chunk: usize,
}

impl<'a> ChunkedReader<'a> {
    fn new(bytes: &'a [u8], chunk: usize) -> Self {
        assert!(chunk > 0);
        Self {
            bytes,
            pos: 0,
            chunk,
        }
    }
}

impl<'a> Read for ChunkedReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let remaining = self.bytes.len() - self.pos;
        if remaining == 0 {
            return Ok(0);
        }
        let take = remaining.min(buf.len()).min(self.chunk);
        buf[..take].copy_from_slice(&self.bytes[self.pos..self.pos + take]);
        self.pos += take;
        Ok(take)
    }
}

#[test]
fn merkle_root_is_invariant_under_chunked_read_on_all_fixtures() {
    let chunk_sizes = [1usize, 7, 17, 64, 65, 256, 1024, 4096, 4097, 65536];
    for &name in FIXTURES {
        let bytes = std::fs::read(fixture_path(name)).expect("fixture read");
        // Reference: default chunk size (whatever ApkParser picks
        // for a `&[u8]`-backed Read).
        let (_, reference) = parse_with_commit_chain(bytes.as_slice()).expect("parse ref");
        for &cs in &chunk_sizes {
            let r = ChunkedReader::new(&bytes, cs);
            let (_, chain) =
                parse_with_commit_chain(r).unwrap_or_else(|e| panic!("{name} cs={cs} parse: {e}"));
            assert_eq!(
                chain.root, reference.root,
                "{name}: Merkle root drifted at chunk_size={cs}"
            );
            assert_eq!(
                chain.leaves.len(),
                reference.leaves.len(),
                "{name}: leaf count drifted at chunk_size={cs}"
            );
            for (i, (a, b)) in chain.leaves.iter().zip(reference.leaves.iter()).enumerate() {
                assert_eq!(
                    a.hash, b.hash,
                    "{name}: leaf #{i} (tag={}) hash drifted at chunk_size={cs}",
                    a.tag
                );
                assert_eq!(
                    a.length, b.length,
                    "{name}: leaf #{i} length drifted at chunk_size={cs}",
                );
                assert_eq!(
                    a.tag, b.tag,
                    "{name}: leaf #{i} tag drifted at chunk_size={cs}",
                );
            }
        }
        eprintln!(
            "{name}: leaves={}, root={} — invariant across {} chunk sizes",
            reference.leaves.len(),
            hex_encode(&reference.root),
            chunk_sizes.len()
        );
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(nibble(b >> 4));
        out.push(nibble(b & 0x0f));
    }
    out
}

const fn nibble(n: u8) -> char {
    if n < 10 {
        (b'0' + n) as char
    } else {
        (b'a' + n - 10) as char
    }
}
