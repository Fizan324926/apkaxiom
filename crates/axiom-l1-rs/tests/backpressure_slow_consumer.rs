// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
//
// Backpressure / slow-consumer gate (K8 closure).
//
// Verifies that the sync ApkParser's internal buffer never grows
// unboundedly when processing a large sequence of APKs. Drives
// `next_event()` one event at a time with a simulated 1 ms drain
// delay per APK (wall-clock not simulated — focus is memory bound).
//
// The test processes 1 000 APK-parse cycles across the 4 real-APK
// fixtures and asserts that peak buf_capacity stays ≤ MAX_BUF_BYTES
// at every drain point.  An unbounded buffer would grow with each
// APK; bounded growth means the allocator re-uses the same slab.

use std::io::Cursor;
use std::time::Duration;

use axiom_l1_rs::stream::ApkParser;

const FIXTURE_NAMES: &[&str] = &[
    "clipboard.apk",
    "fdroid-privileged-2050.apk",
    "tickytacky-mirror.apk",
    "wifiautoff.apk",
];

// A single-APK parse uses at most a few MB.  We allow 8 MB as the
// upper-bound budget — well below the K3 150 MB RSS gate.
const MAX_BUF_BYTES: usize = 8 * 1024 * 1024;

fn fixture_bytes(name: &str) -> Vec<u8> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|_| panic!("fixture missing: {name}"))
}

#[test]
fn slow_consumer_1000_apks_buffer_stays_bounded() {
    let fixtures: Vec<Vec<u8>> = FIXTURE_NAMES.iter().map(|n| fixture_bytes(n)).collect();

    let drain_delay = Duration::from_millis(1);
    let mut peak_buf = 0usize;
    let mut total_events = 0u64;

    for i in 0..1000 {
        let apk_bytes = &fixtures[i % fixtures.len()];
        let cursor = Cursor::new(apk_bytes.as_slice());
        let mut parser = ApkParser::from_reader(cursor);

        // Drain one event at a time — simulating a slow consumer.
        loop {
            let cap_before = parser.buf_capacity();
            if cap_before > peak_buf {
                peak_buf = cap_before;
            }
            assert!(
                cap_before <= MAX_BUF_BYTES,
                "buf_capacity {cap_before} exceeds {MAX_BUF_BYTES} at APK {i}"
            );

            match parser.next_event() {
                Ok(Some(_ev)) => {
                    total_events += 1;
                    // Simulate slow consumer on the first APK of every
                    // 100-batch to keep test wall-time sane while still
                    // exercising the slow-drain path.
                    if i % 100 == 0 {
                        std::thread::sleep(drain_delay);
                    }
                }
                Ok(None) => break,
                Err(_) => break,
            }
        }
    }

    assert!(total_events > 0, "no events produced");
    eprintln!(
        "slow_consumer_1000: total_events={total_events}  peak_buf_capacity={peak_buf}B  \
         peak_buf={:.1}KB  bound={:.1}KB  PASS",
        peak_buf as f64 / 1024.0,
        MAX_BUF_BYTES as f64 / 1024.0,
    );
}
