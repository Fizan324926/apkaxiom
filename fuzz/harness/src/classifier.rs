// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Differential-classifier taxonomy.
//!
//! Every fuzz iteration produces a verdict pair `(axiom-l0,
//! target)`. The classifier maps that pair to one of five buckets:
//!
//!   * `A` — both accept (informational; expected on a healthy seed)
//!   * `B` — both reject with the same error tag
//!   * `C` — both reject with **different** error tags (taxonomy
//!     drift; classifier improvement target — not a finding)
//!   * `D` — axiom-l0 accepts, target rejects (verified path is
//!     **more permissive**; potential leniency bug in the L0 layer)
//!   * `E` — axiom-l0 rejects, target accepts (verified path is
//!     **stricter**; potential CVE-class — the target accepted an
//!     archive the verified path called malformed)
//!
//! Buckets `D` and `E` are the load-bearing finding buckets; both
//! get logged to the finding archive. `C` is logged at lower
//! severity for classifier improvement.

use core::fmt;

/// Classifier bucket. The ordering matters for display only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bucket {
    /// Both accept.
    A,
    /// Both reject with the same tag.
    B,
    /// Both reject with different tags.
    C,
    /// axiom-l0 accepts, target rejects (axiom-l0 is more permissive).
    D,
    /// axiom-l0 rejects, target accepts (axiom-l0 is stricter; potential CVE).
    E,
}

impl Bucket {
    /// Stable string label used in the finding archive + Grafana.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::A => "A_BOTH_ACCEPT",
            Self::B => "B_BOTH_REJECT_SAME_TAG",
            Self::C => "C_BOTH_REJECT_DIFFERENT_TAG",
            Self::D => "D_AXIOM_ACCEPT_TARGET_REJECT",
            Self::E => "E_AXIOM_REJECT_TARGET_ACCEPT",
        }
    }

    /// `true` if this bucket should be logged as a finding.
    /// `D` and `E` are always findings; `C` is logged at lower
    /// severity.
    #[must_use]
    pub const fn is_finding(self) -> bool {
        matches!(self, Self::C | Self::D | Self::E)
    }

    /// `true` for the "potential CVE" bucket (E).
    #[must_use]
    pub const fn is_high_severity(self) -> bool {
        matches!(self, Self::E)
    }
}

impl fmt::Display for Bucket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// One side's verdict on a single input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Parser accepted (returned `Ok`).
    Accept,
    /// Parser rejected with a tag (one of `ArchiveError`'s `tag()`
    /// values for axiom-l0; AOSP `ZipError` integer for the
    /// libziparchive probe; `Reject(String)` for opaque targets).
    Reject(String),
}

impl Verdict {
    /// Stable label for archive serialisation.
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::Accept => "accept".into(),
            Self::Reject(tag) => format!("reject:{tag}"),
        }
    }

    /// `true` for accept verdicts.
    #[must_use]
    pub const fn is_accept(&self) -> bool {
        matches!(self, Self::Accept)
    }
}

/// Classify a verdict pair into a bucket.
#[must_use]
pub fn classify(axiom: &Verdict, target: &Verdict) -> Bucket {
    match (axiom.is_accept(), target.is_accept()) {
        (true, true) => Bucket::A,
        (true, false) => Bucket::D,
        (false, true) => Bucket::E,
        (false, false) => {
            if axiom.label() == target.label() {
                Bucket::B
            } else {
                Bucket::C
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_a() {
        assert_eq!(classify(&Verdict::Accept, &Verdict::Accept), Bucket::A);
    }

    #[test]
    fn classify_b() {
        assert_eq!(
            classify(&Verdict::Reject("9".into()), &Verdict::Reject("9".into())),
            Bucket::B
        );
    }

    #[test]
    fn classify_c() {
        assert_eq!(
            classify(&Verdict::Reject("9".into()), &Verdict::Reject("3".into())),
            Bucket::C
        );
    }

    #[test]
    fn classify_d() {
        assert_eq!(
            classify(&Verdict::Accept, &Verdict::Reject("9".into())),
            Bucket::D
        );
    }

    #[test]
    fn classify_e_is_high_severity() {
        let b = classify(&Verdict::Reject("9".into()), &Verdict::Accept);
        assert_eq!(b, Bucket::E);
        assert!(b.is_high_severity());
        assert!(b.is_finding());
    }

    #[test]
    fn buckets_a_b_are_not_findings() {
        assert!(!Bucket::A.is_finding());
        assert!(!Bucket::B.is_finding());
    }
}
