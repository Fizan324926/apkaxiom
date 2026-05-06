// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Persistent target probe — long-lived child process speaking
//! a length-prefixed protocol over stdin/stdout. Amortises the
//! ~20 ms process-startup cost of the per-call mode across many
//! iterations.
//!
//! Protocol:
//!
//! ```text
//!   driver  →  probe :  <u32 LE length>  <length bytes>
//!   probe   →  driver:  "ok <len>\n"  |  "err <reject> <code>\n"
//! ```
//!
//! Spawned with `--archive-runtime-server`. EOF on stdin closes
//! the child cleanly.
//!
//! ## Per-call timeout (Gap-10 closure)
//!
//! A single shared watchdog thread enforces per-input timeouts.
//! Each `run_one` call registers `(deadline, pid_atomic, counter)`
//! into a shared registry before its stdin write, then deregisters
//! after the stdout read returns. The watchdog wakes every 5 ms,
//! scans the registry, and, for any entry whose deadline has
//! passed, issues `kill -9 <pid>`, bumps the probe's `timed_out`
//! counter, and removes the entry (so the next call re-registers
//! fresh, avoiding any re-kill of the post-restart pid). The
//! probe's existing pipe-broken auto-restart path then transparently
//! re-spawns the child on the next call; a delta on `timed_out`
//! is what tells the caller a kill happened.
//!
//! Why `kill(2)` via `/bin/kill -9` and not `libc::kill`?
//! The crate is `#![forbid(unsafe_code)]`. `Child::kill` requires
//! `&mut self`, so the watchdog can't grab the probe mutex
//! without deadlocking against the in-flight `run_one` (which
//! holds the mutex while blocked on stdout). Storing the pid as
//! an `AtomicU32` and shelling out to `/bin/kill` is the safe
//! escape hatch — it only fires on overrun (rare) and the cost
//! is dominated by the kill itself.

use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::atomic::{AtomicU32, AtomicU64, Ordering},
    sync::{Arc, Mutex, OnceLock},
    thread,
    time::{Duration, Instant},
};

use crate::classifier::Verdict;

/// Persistent probe handle. Wraps the child process and its
/// stdin/stdout pipes under a single mutex so concurrent callers
/// serialise on the protocol stream.
pub struct PersistentProbe {
    child: Mutex<ChildState>,
    label: String,
    binary: std::path::PathBuf,
    /// Per-input timeout. If a single input takes longer than
    /// this, the probe is killed + restarted (Gap-10).
    timeout: Duration,
    /// Atomic mirror of the child's pid. Updated on every
    /// (re)spawn; read by the watchdog to issue `kill -9`
    /// without holding the probe mutex.
    pid: Arc<AtomicU32>,
    /// Stable identity in the watchdog registry.
    handle_id: u64,
    /// Count of inputs killed by the watchdog. Bumped from
    /// inside the watchdog thread; read by the run path to
    /// detect the kill.
    timed_out: Arc<AtomicU64>,
}

struct ChildState {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    /// Total inputs served by this child. Used for the auto-restart
    /// heuristic — if a child crashes (signalled exit), we restart
    /// it and continue.
    served: u64,
}

impl PersistentProbe {
    /// Spawn a new probe. `binary` should point at the
    /// `zip-aosp-runtime-probe` (or compatible) executable; it
    /// will be invoked with `--archive-runtime-server`. Default
    /// per-input timeout is 5 s; override with [`Self::with_timeout`].
    pub fn spawn(label: &str, binary: &Path) -> std::io::Result<Self> {
        let child = Self::spawn_child(binary)?;
        let pid = Arc::new(AtomicU32::new(child.child.id()));
        Ok(Self {
            child: Mutex::new(child),
            label: label.to_string(),
            binary: binary.to_path_buf(),
            timeout: Duration::from_secs(5),
            pid,
            handle_id: next_handle_id(),
            timed_out: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Override the per-input timeout. After this duration the
    /// probe is killed + restarted (Gap-10).
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    fn spawn_child(binary: &Path) -> std::io::Result<ChildState> {
        Self::spawn_child_argv(binary, &["--archive-runtime-server"])
    }

    fn spawn_child_argv(binary: &Path, argv: &[&str]) -> std::io::Result<ChildState> {
        let mut c = Command::new(binary)
            .args(argv)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let stdin = c
            .stdin
            .take()
            .ok_or_else(|| std::io::Error::other("probe stdin pipe missing"))?;
        let stdout = BufReader::new(
            c.stdout
                .take()
                .ok_or_else(|| std::io::Error::other("probe stdout pipe missing"))?,
        );
        Ok(ChildState {
            child: c,
            stdin,
            stdout,
            served: 0,
        })
    }

    /// Stable label for the archive `target_label` field.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Number of inputs killed by the watchdog since spawn.
    #[must_use]
    pub fn timed_out(&self) -> u64 {
        self.timed_out.load(Ordering::Relaxed)
    }

    /// Run one input through the probe. On a child crash, restarts
    /// the child once and retries; further failures are returned
    /// as `Err`. On per-call overrun, the watchdog kills the
    /// child; this function returns `Err(TimedOut)` and the next
    /// call re-spawns.
    pub fn run_one(&self, input: &[u8]) -> std::io::Result<Verdict> {
        let watchdog = ensure_watchdog();
        let timed_out_before = self.timed_out.load(Ordering::Relaxed);
        let deadline = Instant::now() + self.timeout;
        watchdog.register(
            self.handle_id,
            deadline,
            Arc::clone(&self.pid),
            Arc::clone(&self.timed_out),
        );
        let raw = self.run_one_inner(input);
        watchdog.deregister(self.handle_id);

        match raw {
            Ok(v) => Ok(v),
            Err(e) if is_pipe_broken(&e) => {
                let timed_out_after = self.timed_out.load(Ordering::Relaxed);
                self.restart()?;
                if timed_out_after > timed_out_before {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        format!("probe timed out after {:?}", self.timeout),
                    ));
                }
                // Re-register for the retry.
                let retry_deadline = Instant::now() + self.timeout;
                watchdog.register(
                    self.handle_id,
                    retry_deadline,
                    Arc::clone(&self.pid),
                    Arc::clone(&self.timed_out),
                );
                let retry = self.run_one_inner(input);
                watchdog.deregister(self.handle_id);
                retry
            }
            Err(e) => Err(e),
        }
    }

    fn run_one_inner(&self, input: &[u8]) -> std::io::Result<Verdict> {
        let mut g = self.child.lock().expect("probe mutex poisoned");
        let len: u32 =
            u32::try_from(input.len()).map_err(|_| std::io::Error::other("input >= 4 GB"))?;
        g.stdin.write_all(&len.to_le_bytes())?;
        if !input.is_empty() {
            g.stdin.write_all(input)?;
        }
        g.stdin.flush()?;
        let mut line = String::new();
        let n = g.stdout.read_line(&mut line)?;
        if n == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "probe closed stdout",
            ));
        }
        g.served = g.served.wrapping_add(1);
        Ok(parse_reply(line.trim_end()))
    }

    fn restart(&self) -> std::io::Result<()> {
        let mut g = self.child.lock().expect("probe mutex poisoned");
        // Best-effort kill of the dead child.
        let _ = g.child.kill();
        let _ = g.child.wait();
        let fresh = Self::spawn_child(&self.binary)?;
        self.pid.store(fresh.child.id(), Ordering::Relaxed);
        *g = fresh;
        Ok(())
    }

    /// Cleanly stop the probe — drops stdin (EOF triggers clean
    /// shutdown server-side) and waits for the child to exit.
    /// Idempotent.
    pub fn shutdown(&self) {
        let mut g = self.child.lock().expect("probe mutex poisoned");
        let _ = g.child.kill();
        let _ = g.child.wait();
        ensure_watchdog().deregister(self.handle_id);
    }

    /// Number of inputs served since spawn (or last restart).
    pub fn served(&self) -> u64 {
        let g = self.child.lock().expect("probe mutex poisoned");
        g.served
    }
}

impl std::fmt::Debug for PersistentProbe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PersistentProbe")
            .field("label", &self.label)
            .field("served", &self.served())
            .field("timed_out", &self.timed_out())
            .finish()
    }
}

fn parse_reply(line: &str) -> Verdict {
    if let Some(rest) = line.strip_prefix("ok ") {
        let _ = rest;
        Verdict::Accept
    } else if let Some(rest) = line.strip_prefix("err ") {
        let code = rest.split_whitespace().nth(1).unwrap_or("?");
        Verdict::Reject(format!("aosp:{code}"))
    } else {
        Verdict::Reject(format!("malformed:{line}"))
    }
}

fn is_pipe_broken(e: &std::io::Error) -> bool {
    matches!(
        e.kind(),
        std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::UnexpectedEof
    )
}

/// Convenience helper: spawn one probe, run one input, drop. Used
/// only by the legacy per-call differ for fall-back paths and tests.
/// Production fuzz loops should use `PersistentProbe::run_one`.
pub fn run_one_oneshot(binary: &Path, input: &[u8], timeout: Duration) -> std::io::Result<Verdict> {
    let mut child = Command::new(binary)
        .arg("--archive-runtime")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input)?;
    }
    let start = std::time::Instant::now();
    loop {
        match child.try_wait()? {
            Some(_) => break,
            None => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    return Err(std::io::Error::other("probe timeout"));
                }
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }
    let mut s = String::new();
    let _ = child.stdout.take().unwrap().read_to_string(&mut s);
    Ok(parse_reply(s.lines().next().unwrap_or("")))
}

// ---------------------------------------------------------------------
// Watchdog — single shared thread; lazily started on first probe.
// ---------------------------------------------------------------------

fn next_handle_id() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

struct WatchdogEntry {
    deadline: Instant,
    pid: Arc<AtomicU32>,
    timed_out: Arc<AtomicU64>,
}

struct Watchdog {
    inner: Mutex<HashMap<u64, WatchdogEntry>>,
}

static WATCHDOG: OnceLock<Watchdog> = OnceLock::new();

fn ensure_watchdog() -> &'static Watchdog {
    WATCHDOG.get_or_init(|| {
        thread::Builder::new()
            .name("p113-probe-watchdog".into())
            .spawn(watchdog_loop)
            .expect("watchdog thread spawn");
        Watchdog {
            inner: Mutex::new(HashMap::new()),
        }
    })
}

impl Watchdog {
    fn register(
        &self,
        id: u64,
        deadline: Instant,
        pid: Arc<AtomicU32>,
        timed_out: Arc<AtomicU64>,
    ) {
        let mut g = self.inner.lock().expect("watchdog mutex");
        g.insert(
            id,
            WatchdogEntry {
                deadline,
                pid,
                timed_out,
            },
        );
    }

    fn deregister(&self, id: u64) {
        let mut g = self.inner.lock().expect("watchdog mutex");
        g.remove(&id);
    }
}

fn watchdog_loop() {
    // 5 ms cadence — fine grained enough that timeouts above
    // ~50 ms are accurate to ~10 % and the wakeup cost is
    // negligible (one mutex acquire on an empty map).
    let cadence = Duration::from_millis(5);
    loop {
        thread::sleep(cadence);
        let w = match WATCHDOG.get() {
            Some(w) => w,
            None => continue,
        };
        // Snapshot expired entries under the lock, drop the lock,
        // then issue kills outside it (kill is slow vs holding
        // a contended mutex).
        let to_kill: Vec<(u64, u32, Arc<AtomicU64>)> = {
            let mut g = match w.inner.lock() {
                Ok(g) => g,
                Err(_) => return,
            };
            let now = Instant::now();
            let expired: Vec<u64> = g
                .iter()
                .filter(|(_, e)| e.deadline <= now)
                .map(|(&id, _)| id)
                .collect();
            expired
                .into_iter()
                .filter_map(|id| {
                    g.remove(&id)
                        .map(|e| (id, e.pid.load(Ordering::Relaxed), e.timed_out))
                })
                .collect()
        };
        for (_id, pid, timed_out) in to_kill {
            if pid != 0 {
                // Best-effort SIGKILL via /bin/kill — bypasses
                // the unsafe_code forbid that blocks libc::kill.
                let _ = Command::new("kill")
                    .arg("-9")
                    .arg(pid.to_string())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
            }
            timed_out.fetch_add(1, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_reply_ok() {
        assert_eq!(parse_reply("ok 98"), Verdict::Accept);
    }

    #[test]
    fn parse_reply_err() {
        assert_eq!(parse_reply("err 100 -3"), Verdict::Reject("aosp:-3".into()));
    }

    #[test]
    fn parse_reply_malformed() {
        let v = parse_reply("garbage");
        assert!(matches!(v, Verdict::Reject(s) if s.starts_with("malformed:")));
    }

    /// Test-only constructor that lets the caller pick argv. Real
    /// callers go through [`PersistentProbe::spawn`] which always
    /// passes `--archive-runtime-server`.
    fn spawn_with_argv_for_test(
        label: &str,
        binary: &Path,
        argv: &[&str],
    ) -> std::io::Result<PersistentProbe> {
        let child = PersistentProbe::spawn_child_argv(binary, argv)?;
        let pid = Arc::new(AtomicU32::new(child.child.id()));
        Ok(PersistentProbe {
            child: Mutex::new(child),
            label: label.to_string(),
            binary: binary.to_path_buf(),
            timeout: Duration::from_secs(5),
            pid,
            handle_id: next_handle_id(),
            timed_out: Arc::new(AtomicU64::new(0)),
        })
    }

    /// End-to-end watchdog test. Uses `/bin/sleep 30` as the "probe":
    /// it ignores stdin and never writes stdout, so any
    /// `run_one` would block indefinitely without the watchdog.
    /// With a 250 ms timeout the watchdog must kill it and the
    /// call must return promptly (well under 3 s).
    #[test]
    fn watchdog_kills_runaway_probe() {
        let bin = std::path::PathBuf::from("/bin/sleep");
        if !bin.exists() {
            return;
        }
        let probe = spawn_with_argv_for_test("sleep-test", &bin, &["30"])
            .expect("spawn sleep")
            .with_timeout(Duration::from_millis(250));
        let t0 = Instant::now();
        let r = probe.run_one(&[0u8; 4]);
        let dt = t0.elapsed();
        assert!(
            r.is_err(),
            "expected error from timed-out runaway probe, got {r:?}"
        );
        assert!(
            dt < Duration::from_secs(3),
            "watchdog did not return promptly: {dt:?}"
        );
        assert!(
            probe.timed_out() >= 1,
            "expected timed_out >= 1, got {}",
            probe.timed_out()
        );
    }

    /// Watchdog must NOT fire when the probe responds within the
    /// deadline. Uses `/bin/cat` echoing fixed-length frames; the
    /// probe protocol is wrong (cat just echoes raw bytes), so
    /// `parse_reply` will return `Reject(malformed:...)`. That's
    /// fine — what we're testing is that `run_one` returns a
    /// verdict (not a TimedOut error) and `timed_out()` stays 0.
    /// We use a single 4-byte length prefix (no body) so cat
    /// echoes exactly 4 bytes back, which contains a `\n`?
    /// Probably not — so we accept either Ok or non-Timeout Err.
    #[test]
    fn watchdog_does_not_fire_when_probe_responds() {
        // Build a tiny shell-script probe that echoes a fixed reply
        // immediately, satisfying read_line and exiting fast.
        let script = "/tmp/p113-watchdog-fast-probe.sh";
        std::fs::write(
            script,
            "#!/bin/sh\nwhile true; do head -c 4 > /dev/null && echo 'ok 0'; done\n",
        )
        .expect("write script");
        let _ = std::process::Command::new("chmod")
            .arg("+x")
            .arg(script)
            .status();
        let bin = std::path::PathBuf::from(script);
        let probe = spawn_with_argv_for_test("fast-probe", &bin, &[])
            .expect("spawn fast probe")
            .with_timeout(Duration::from_millis(500));
        let v = probe.run_one(&[]).expect("run_one fast");
        assert!(
            matches!(v, Verdict::Accept),
            "expected Accept, got {v:?}"
        );
        assert_eq!(probe.timed_out(), 0, "watchdog must not fire on fast path");
        let _ = std::fs::remove_file(script);
    }
}
