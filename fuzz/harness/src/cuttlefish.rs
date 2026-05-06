// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Cuttlefish A14 lifecycle — KVM-gated.
//!
//! In `dev` mode this module is a stub that reports
//! `Availability::Missing(reason)`. The driver falls back to the
//! AOSP libziparchive runtime probe with a loud warning.
//!
//! In `real` mode (Cargo feature `nyx-cuttlefish`, KVM present)
//! it would launch a Cuttlefish CVD via `launch_cvd`, snapshot
//! the boot, and hand the snapshot to Nyx for fuzz iteration.
//! That codepath is **not** implemented here — it requires the
//! KVM host (operator one-shot CHECKLIST §C-1) and pulls in
//! libnyx + cuttlefish-base which are not on this host. The
//! real-mode driver is a separate binary built only when the
//! `nyx-cuttlefish` feature is enabled and the host has
//! `/dev/kvm`.

use std::path::{Path, PathBuf};

/// Whether a Cuttlefish target is available on this host.
#[derive(Debug, Clone)]
pub enum Availability {
    /// `/dev/kvm` is present, the `nyx-cuttlefish` feature was
    /// compiled in, and a Cuttlefish CVD image was found at the
    /// configured path.
    Available {
        /// Resolved Cuttlefish CVD root directory.
        cvd_root: PathBuf,
    },
    /// Not available; the driver should fall back to dev mode.
    Missing {
        /// Human-readable reason; surfaced to stderr.
        reason: String,
    },
}

/// Probe the host for a Cuttlefish target. Cheap; called once on
/// driver startup.
#[must_use]
pub fn probe(cvd_root: Option<&Path>) -> Availability {
    if !cfg!(feature = "nyx-cuttlefish") {
        return Availability::Missing {
            reason: "the `nyx-cuttlefish` Cargo feature is not enabled in this build".into(),
        };
    }
    if !Path::new("/dev/kvm").exists() {
        return Availability::Missing {
            reason: "/dev/kvm is not present on this host".into(),
        };
    }
    let Some(root) = cvd_root else {
        return Availability::Missing {
            reason: "no --cvd-root path was supplied".into(),
        };
    };
    if !root.join("system.img").exists() {
        return Availability::Missing {
            reason: format!(
                "{}/system.img not found — run the operator one-shot CHECKLIST §C-2",
                root.display()
            ),
        };
    }
    Availability::Available {
        cvd_root: root.to_path_buf(),
    }
}

/// Launch a Cuttlefish CVD and hand the resulting Nyx snapshot to
/// the caller. Not implemented in this sub-phase; documented
/// here so the operator one-shot has a concrete file/line target
/// to build out.
///
/// # Errors
///
/// Always returns `Err("not implemented")` in this build. The
/// operator one-shot at CHECKLIST §C-2 brings up the KVM host;
/// this function is the natural insertion point for the launch
/// + snapshot logic.
pub fn launch(_cvd_root: &Path) -> std::io::Result<NyxSnapshot> {
    Err(std::io::Error::other(
        "cuttlefish::launch is gated on the `nyx-cuttlefish` feature + a KVM host; \
         see CHECKLIST §C-2 for the operator one-shot",
    ))
}

/// Opaque handle to a Nyx snapshot of a booted Cuttlefish CVD.
/// In dev builds this struct is unconstructible from outside the
/// module (no `pub fn launch` succeeds), so the rest of the
/// codebase compiles without taking on libnyx as a dep.
#[derive(Debug)]
#[allow(dead_code)]
pub struct NyxSnapshot {
    /// Reserved for the libnyx handle.
    handle: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_in_dev_build_returns_missing() {
        // This binary is built without `nyx-cuttlefish`, so probe
        // always returns Missing — regardless of /dev/kvm.
        let a = probe(None);
        assert!(matches!(a, Availability::Missing { .. }));
    }
}
