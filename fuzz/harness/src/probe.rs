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

use std::{
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::Mutex,
    time::Duration,
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
        Ok(Self {
            child: Mutex::new(child),
            label: label.to_string(),
            binary: binary.to_path_buf(),
            timeout: Duration::from_secs(5),
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
        let mut c = Command::new(binary)
            .arg("--archive-runtime-server")
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

    /// Run one input through the probe. On a child crash, restarts
    /// the child once and retries; further failures are returned
    /// as `Err`.
    pub fn run_one(&self, input: &[u8]) -> std::io::Result<Verdict> {
        // Try once; if the pipe is dead, restart and try once more.
        match self.run_one_inner(input) {
            Ok(v) => Ok(v),
            Err(e) if is_pipe_broken(&e) => {
                self.restart()?;
                self.run_one_inner(input)
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
            return Err(std::io::Error::other("probe closed stdout"));
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
        *g = fresh;
        Ok(())
    }

    /// Cleanly stop the probe — drops stdin (EOF triggers clean
    /// shutdown server-side) and waits for the child to exit.
    /// Idempotent.
    pub fn shutdown(&self) {
        let mut g = self.child.lock().expect("probe mutex poisoned");
        // Dropping stdin closes the pipe and signals EOF.
        // Replace with a fresh dummy stdin so the field is valid
        // for any further (no-op) calls.
        let _ = g.child.kill();
        let _ = g.child.wait();
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
}
