// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Additional differential targets — system tools whose
//! independent ZIP implementations widen the differential
//! surface beyond the libziparchive probe.
//!
//! Each arm is a thin wrapper that materialises the input to a
//! tempfile, runs the system tool, and maps its exit status to
//! a [`Verdict`].
//!
//! Arms shipped:
//!
//!   - **`unzip` (Info-ZIP)** — `/usr/bin/unzip -l <tmp>`. Exit 0 ⇒
//!     `Accept`; nonzero ⇒ `Reject(unzip:<exit>)`.
//!   - **`jar` (OpenJDK)** — `/usr/bin/jar tf <tmp>`. Same shape.
//!   - **`python zipfile`** — `python3 -c "import zipfile,
//!     sys; zipfile.ZipFile(sys.argv[1])" <tmp>`. Same shape.
//!
//! Each arm is opt-in via the harness `--arms` flag (comma-
//! separated list of arm labels). The driver writes one finding
//! per arm-disagreement so a single mutation that splits 4 ways
//! generates 4 archive records, all with stable `target_label`
//! fields for Grafana-side splitting.

use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

use crate::classifier::Verdict;

/// One additional differential arm.
pub trait Arm: Send + Sync {
    /// Stable label for the archive `target_label` field.
    fn label(&self) -> &str;
    /// Run the input. Returns a Verdict on success, std::io::Error
    /// on probe-side failure (caller increments the IO-error
    /// counter and continues).
    fn run(&self, input: &[u8]) -> std::io::Result<Verdict>;
}

fn write_temp(input: &[u8]) -> std::io::Result<PathBuf> {
    let mut p = std::env::temp_dir();
    let id = std::process::id();
    let ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    p.push(format!("p113-arm-{id}-{ns}.zip"));
    let mut f = std::fs::File::create(&p)?;
    f.write_all(input)?;
    f.flush()?;
    Ok(p)
}

fn run_tool(label: &str, prog: &str, args: &[&str], input: &[u8]) -> std::io::Result<Verdict> {
    let p = write_temp(input)?;
    let mut full_args: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
    full_args.push(p.to_string_lossy().into_owned());
    let out = Command::new(prog)
        .args(&full_args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output();
    let _ = std::fs::remove_file(&p);
    let status = out?.status;
    if status.success() {
        Ok(Verdict::Accept)
    } else {
        let code = status.code().unwrap_or(-1);
        Ok(Verdict::Reject(format!("{label}:{code}")))
    }
}

/// `unzip -l` arm.
#[derive(Debug)]
pub struct UnzipArm {
    binary: String,
}

impl UnzipArm {
    /// Default Info-ZIP `unzip` at `/usr/bin/unzip`.
    #[must_use]
    pub fn default() -> Self {
        Self {
            binary: "/usr/bin/unzip".into(),
        }
    }
}

impl Arm for UnzipArm {
    fn label(&self) -> &str {
        "unzip"
    }
    fn run(&self, input: &[u8]) -> std::io::Result<Verdict> {
        run_tool("unzip", &self.binary, &["-l"], input)
    }
}

/// `jar tf` arm (OpenJDK).
#[derive(Debug)]
pub struct JarArm {
    binary: String,
}

impl JarArm {
    /// Default OpenJDK `jar` at `/usr/bin/jar`.
    #[must_use]
    pub fn default() -> Self {
        Self {
            binary: "/usr/bin/jar".into(),
        }
    }
}

impl Arm for JarArm {
    fn label(&self) -> &str {
        "jdk-jar"
    }
    fn run(&self, input: &[u8]) -> std::io::Result<Verdict> {
        run_tool("jdk-jar", &self.binary, &["tf"], input)
    }
}

/// Python `zipfile` arm.
#[derive(Debug)]
pub struct PyZipfileArm;

impl Arm for PyZipfileArm {
    fn label(&self) -> &str {
        "py-zipfile"
    }
    fn run(&self, input: &[u8]) -> std::io::Result<Verdict> {
        run_tool(
            "py-zipfile",
            "python3",
            &[
                "-c",
                "import zipfile,sys; \
                 z=zipfile.ZipFile(sys.argv[1]); \
                 [z.getinfo(n) for n in z.namelist()]",
            ],
            input,
        )
    }
}

/// Build a list of arms from a comma-separated label list.
#[must_use]
pub fn arms_from_csv(labels: &str) -> Vec<Box<dyn Arm>> {
    let mut out: Vec<Box<dyn Arm>> = Vec::new();
    for raw in labels.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        match raw {
            "unzip" => out.push(Box::new(UnzipArm::default())),
            "jdk-jar" => out.push(Box::new(JarArm::default())),
            "py-zipfile" => out.push(Box::new(PyZipfileArm)),
            other => eprintln!("WARN: unknown arm `{other}` — ignoring"),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arms_from_csv_parses() {
        let a = arms_from_csv("unzip, jdk-jar,py-zipfile");
        assert_eq!(a.len(), 3);
        assert_eq!(a[0].label(), "unzip");
    }

    #[test]
    fn arms_from_csv_skips_unknown() {
        let a = arms_from_csv("unzip, what, jdk-jar");
        assert_eq!(a.len(), 2);
    }
}
