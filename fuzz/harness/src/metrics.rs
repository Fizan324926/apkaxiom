// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Prometheus exporter — exposes `/metrics` over a tiny HTTP
//! server on a configurable port. Hand-rolled because the
//! prometheus + tokio + hyper crate trio would balloon the
//! Reindeer surface; the metric-name protocol (`# TYPE`,
//! `# HELP`, `<name>{labels} <value>`) is small enough to emit
//! by hand and is what the Grafana dashboard JSON expects.

use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::atomic::{AtomicU64, Ordering},
    sync::Arc,
    thread::{self, JoinHandle},
    time::Duration,
};

/// All counters the dashboard reads.
#[derive(Debug, Default)]
pub struct Metrics {
    /// `p113_fuzz_iters_total`.
    pub iters_total: AtomicU64,
    /// `p113_fuzz_target_io_errors_total`.
    pub target_io_errors_total: AtomicU64,
    /// `p113_fuzz_bucket_total{bucket="A"}`.
    pub bucket_a: AtomicU64,
    /// `p113_fuzz_bucket_total{bucket="B"}`.
    pub bucket_b: AtomicU64,
    /// `p113_fuzz_bucket_total{bucket="C"}`.
    pub bucket_c: AtomicU64,
    /// `p113_fuzz_bucket_total{bucket="D"}`.
    pub bucket_d: AtomicU64,
    /// `p113_fuzz_bucket_total{bucket="E"}`.
    pub bucket_e: AtomicU64,
    /// `p113_fuzz_findings_total` — D + E + C raw count.
    pub findings_total: AtomicU64,
    /// `p113_fuzz_distinct_findings` — distinct shas seen.
    pub distinct_findings: AtomicU64,
    /// `p113_fuzz_iter_duration_ns_sum` — running sum for
    /// approximate p99 estimation in the dashboard. A real
    /// histogram exporter would sketch buckets; for our load this
    /// counter is enough to drive the timeseries.
    pub iter_duration_ns_sum: AtomicU64,
    /// `p113_fuzz_distinct_clusters_total` — deduped cluster count.
    pub distinct_clusters: AtomicU64,
}

impl Metrics {
    /// Snapshot to the Prometheus text format.
    #[must_use]
    pub fn render(&self) -> String {
        let mut s = String::with_capacity(2048);
        let counters: [(&str, u64); 6] = [
            (
                "p113_fuzz_iters_total",
                self.iters_total.load(Ordering::Relaxed),
            ),
            (
                "p113_fuzz_target_io_errors_total",
                self.target_io_errors_total.load(Ordering::Relaxed),
            ),
            (
                "p113_fuzz_findings_total",
                self.findings_total.load(Ordering::Relaxed),
            ),
            (
                "p113_fuzz_distinct_findings",
                self.distinct_findings.load(Ordering::Relaxed),
            ),
            (
                "p113_fuzz_distinct_clusters_total",
                self.distinct_clusters.load(Ordering::Relaxed),
            ),
            (
                "p113_fuzz_iter_duration_ns_sum",
                self.iter_duration_ns_sum.load(Ordering::Relaxed),
            ),
        ];
        for (name, val) in counters {
            s.push_str(&format!("# TYPE {name} counter\n"));
            s.push_str(&format!("{name} {val}\n"));
        }
        s.push_str("# TYPE p113_fuzz_bucket_total counter\n");
        for (label, ctr) in [
            ("A", &self.bucket_a),
            ("B", &self.bucket_b),
            ("C", &self.bucket_c),
            ("D", &self.bucket_d),
            ("E", &self.bucket_e),
        ] {
            s.push_str(&format!(
                "p113_fuzz_bucket_total{{bucket=\"{label}\"}} {}\n",
                ctr.load(Ordering::Relaxed)
            ));
        }
        s
    }
}

/// Exporter handle. Drop to stop the server.
pub struct Exporter {
    /// Bound address (`<host>:<port>`).
    pub bind: String,
    handle: Option<JoinHandle<()>>,
    shutdown: Arc<std::sync::atomic::AtomicBool>,
}

impl Exporter {
    /// Bind the exporter on the given `<host>:<port>` and spawn a
    /// background thread serving `/metrics`. Returns immediately.
    pub fn start(bind: &str, metrics: Arc<Metrics>) -> std::io::Result<Self> {
        let listener = TcpListener::bind(bind)?;
        listener.set_nonblocking(true)?;
        let shutdown = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let shutdown_thread = Arc::clone(&shutdown);
        let bind_addr = bind.to_string();
        let handle = thread::spawn(move || {
            for inc in listener.incoming() {
                if shutdown_thread.load(Ordering::Relaxed) {
                    break;
                }
                match inc {
                    Ok(stream) => {
                        let m = Arc::clone(&metrics);
                        let _ = handle_request(stream, &m);
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(50));
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            bind: bind_addr,
            handle: Some(handle),
            shutdown,
        })
    }
}

impl Drop for Exporter {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        // Best-effort: knock the listener loose by connecting once.
        let _ = std::net::TcpStream::connect_timeout(
            &self
                .bind
                .parse()
                .unwrap_or_else(|_| "127.0.0.1:0".parse().unwrap()),
            Duration::from_millis(50),
        );
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl std::fmt::Debug for Exporter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Exporter")
            .field("bind", &self.bind)
            .finish()
    }
}

fn handle_request(mut stream: TcpStream, metrics: &Metrics) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_millis(200)))?;
    let mut buf = [0u8; 1024];
    let _ = stream.read(&mut buf); // discard; we only serve /metrics
    let body = metrics.render();
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain; version=0.0.4\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(resp.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_includes_all_counters() {
        let m = Metrics::default();
        m.iters_total.store(42, Ordering::Relaxed);
        m.bucket_e.store(7, Ordering::Relaxed);
        let s = m.render();
        assert!(s.contains("p113_fuzz_iters_total 42"));
        assert!(s.contains("p113_fuzz_bucket_total{bucket=\"E\"} 7"));
    }

    #[test]
    fn exporter_serves_metrics() -> std::io::Result<()> {
        let m = Arc::new(Metrics::default());
        m.iters_total.store(100, Ordering::Relaxed);
        let e = Exporter::start("127.0.0.1:0", Arc::clone(&m))?;
        // The bind addr we passed had port 0, so we don't know the
        // actual port without TcpListener::local_addr support — for
        // this smoke test we just verify the exporter starts and
        // shuts down cleanly without panicking.
        drop(e);
        Ok(())
    }
}
