// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
//
// P1.8 §F-5 — sync ↔ async wrapper parity test.
//
// `apk.rs` (sync) and `apk_async.rs` (async) duplicate three
// internal helpers: `inflate_raw`, `classify_for_capture`,
// `persist_capture`. If one side gets a fix the other could
// drift. This test feeds the four committed real-APK fixtures
// through both pipelines and asserts the produced
// `(entries, signature_block, manifest, resources)` tuple is
// identical. Drift on either side fails the test.

#![allow(clippy::needless_pass_by_value)]

use std::io;
use std::sync::Arc;
use std::task::{Context, Poll, Wake};

use axiom_l1_rs::{Apk, ApkAsync, AsyncByteSource, Unverified};

const FIXTURE_NAMES: &[&str] = &[
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

struct VecSource {
    bytes: Vec<u8>,
    pos: usize,
    chunk_size: usize,
}

impl AsyncByteSource for VecSource {
    async fn read_chunk(&mut self, n: usize) -> io::Result<Vec<u8>> {
        if self.pos >= self.bytes.len() {
            return Ok(Vec::new());
        }
        let take = n.min(self.chunk_size).min(self.bytes.len() - self.pos);
        let chunk = self.bytes[self.pos..self.pos + take].to_vec();
        self.pos += take;
        Ok(chunk)
    }
}

fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    struct Noop;
    impl Wake for Noop {
        fn wake(self: Arc<Self>) {}
        fn wake_by_ref(self: &Arc<Self>) {}
    }
    let waker = Arc::new(Noop).into();
    let mut ctx = Context::from_waker(&waker);
    let mut fut = Box::pin(fut);
    match fut.as_mut().poll(&mut ctx) {
        Poll::Ready(out) => out,
        Poll::Pending => panic!("test source must not yield Pending"),
    }
}

#[test]
fn sync_and_async_produce_identical_output() {
    for name in FIXTURE_NAMES {
        let bytes =
            std::fs::read(fixture_path(name)).unwrap_or_else(|e| panic!("{name}: read err {e}"));

        let sync_apk = Apk::<Unverified>::from_reader(bytes.as_slice())
            .unwrap_or_else(|e| panic!("{name}: sync from_reader err {e}"));
        let async_apk = block_on(ApkAsync::<Unverified>::from_async_source(VecSource {
            bytes: bytes.clone(),
            pos: 0,
            chunk_size: 65536,
        }))
        .unwrap_or_else(|e| panic!("{name}: async from_async_source err {e}"));

        // Entry tables identical, in order.
        assert_eq!(
            sync_apk.entries().len(),
            async_apk.entries().len(),
            "{name}: entry-count mismatch"
        );
        for (s, a) in sync_apk.entries().iter().zip(async_apk.entries().iter()) {
            assert_eq!(s, a, "{name}: entry meta mismatch");
        }

        // Carry both through verify_v2 → parse_v2 and compare the
        // signature_block + manifest + resources.
        let sp = sync_apk.verify_v2().unwrap().parse_v2().unwrap();
        let ap = async_apk.verify_v2().unwrap().parse_v2().unwrap();
        assert_eq!(
            sp.manifest().axml_bytes,
            ap.manifest().axml_bytes,
            "{name}: manifest bytes diverge between sync and async"
        );
        assert_eq!(
            sp.resources().arsc_bytes,
            ap.resources().arsc_bytes,
            "{name}: resources bytes diverge"
        );
        assert_eq!(
            sp.signature_block().jar_v1_carrier.block_bytes,
            ap.signature_block().jar_v1_carrier.block_bytes,
            "{name}: v1 carrier bytes diverge"
        );
        assert_eq!(
            sp.signature_block().variant_tag,
            ap.signature_block().variant_tag,
            "{name}: variant_tag diverges"
        );
    }
}

/// Drives the async path through tiny chunks (256 bytes) to
/// exercise the streaming-async path differently than the sync
/// `Read` slice. Same fixture set; same expected output.
#[test]
fn async_with_tiny_chunks_matches_sync() {
    for name in FIXTURE_NAMES {
        let bytes = std::fs::read(fixture_path(name)).unwrap();
        let sync_apk = Apk::<Unverified>::from_reader(bytes.as_slice())
            .unwrap()
            .verify_v2()
            .unwrap()
            .parse_v2()
            .unwrap();
        let async_apk = block_on(ApkAsync::<Unverified>::from_async_source(VecSource {
            bytes: bytes.clone(),
            pos: 0,
            chunk_size: 256, // many tiny reads — stresses the chunk-boundary handling
        }))
        .unwrap()
        .verify_v2()
        .unwrap()
        .parse_v2()
        .unwrap();
        assert_eq!(
            sync_apk.manifest().axml_bytes,
            async_apk.manifest().axml_bytes
        );
        assert_eq!(
            sync_apk.resources().arsc_bytes,
            async_apk.resources().arsc_bytes
        );
        assert_eq!(
            sync_apk.signature_block().jar_v1_carrier.block_bytes,
            async_apk.signature_block().jar_v1_carrier.block_bytes
        );
    }
}
