// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! P1.14 auto-classifier — sorts every finding from the
//! cross-version differential into one of four labels:
//!
//!   - **AOSP CVE candidate** — the verified Lean parser rejects
//!     an input that the AOSP libziparchive runtime accepts
//!     (Bucket E). These are direct evidence of a verifier-vs-
//!     runtime gap and are the highest-priority category for
//!     coordinated disclosure.
//!   - **Cross-version evasion** — two different AOSP-version
//!     targets produce different verdicts on the same input.
//!     Any input where {A8, A11, A14} aren't unanimous lands
//!     here. Empirically the highest-yield category for
//!     install-pipeline attacks: an input that passes A11's
//!     verifier but A14 rejects can be staged on an A11 device
//!     to attack a later install on an A14 device.
//!   - **Model bug** — the verified Lean parser accepts an
//!     input that every AOSP-version target rejects (Bucket D
//!     unanimously). Indicates the verified spec is too lax;
//!     the spec needs tightening, not a libziparchive change.
//!   - **Spec ambiguity** — both axiom and target reject but with
//!     different rejection tags (Bucket C). The spec is
//!     ambiguous about *why* the input is invalid; useful for
//!     spec-quality work but not for CVE filing.
//!
//! ## Rules-based engine
//!
//! The rule list is intentionally small (15 rules across 4
//! categories). Each rule fires a (label, weight) pair; the
//! highest-weighted matching rule wins, with explicit
//! tie-breaking favouring the more-specific category. See
//! [`Classifier::classify`].

#![forbid(unsafe_code)]
#![warn(missing_docs, unreachable_pub)]
#![allow(clippy::doc_markdown, clippy::too_long_first_doc_paragraph)]

use p113_fuzz_harness::archive::Finding;
use p113_fuzz_harness::classifier::Bucket;

/// One of the four ground-truth labels the classifier emits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Label {
    /// Verified-rejects, runtime-accepts. Highest-priority for
    /// coordinated disclosure.
    AospCveCandidate,
    /// Different AOSP-version targets produce different verdicts
    /// on the same input. Direct evidence of install-pipeline
    /// evasion potential.
    CrossVersionEvasion,
    /// Verifier accepts something every target rejects. The spec
    /// is too lax; the *spec* needs tightening.
    ModelBug,
    /// Both reject, but with different tags. Spec-quality finding.
    SpecAmbiguity,
}

impl Label {
    /// Stable string label used for archive output + dashboards.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Label::AospCveCandidate => "aosp-cve-candidate",
            Label::CrossVersionEvasion => "cross-version-evasion",
            Label::ModelBug => "model-bug",
            Label::SpecAmbiguity => "spec-ambiguity",
        }
    }
}

/// One of the rules the classifier evaluates. The order matters
/// only for tie-breaking; the engine picks the highest-weighted
/// matching rule.
#[derive(Debug, Clone, Copy)]
pub struct Rule {
    /// Stable rule id, used for explainability (`why was this
    /// labelled X?`).
    pub id: &'static str,
    /// The label this rule emits when it fires.
    pub label: Label,
    /// Higher-weight rules win on a tie. Range 0..100.
    pub weight: u8,
    /// Predicate.
    pub fires: fn(&FindingGroup) -> bool,
}

/// A group of findings sharing the same `input_sha256`. The
/// cross-version classifier needs all version verdicts for a
/// given input to make the AOSP-CVE-vs-CVE-evasion call.
#[derive(Debug, Clone)]
pub struct FindingGroup {
    /// Stable input identity.
    pub input_sha256: String,
    /// All findings (one per version) for this input.
    pub findings: Vec<Finding>,
}

impl FindingGroup {
    /// True iff at least one target accepts AND at least one
    /// target rejects on the same input, **AND** the verifier
    /// accepts.
    ///
    /// The verifier-accepts gate is essential for the threat
    /// model: cross-version evasion is "an APK that the verified
    /// pre-install gate lets through, accepted by version *X*
    /// and rejected by version *Y*." If the verifier rejects,
    /// the input is blocked at install time regardless of which
    /// runtime version is targeted — whatever else the
    /// per-version targets say, that's a CVE on whichever version
    /// accepts (caught by `cve.bucket-e-real`), not a staging
    /// path.
    ///
    /// Two rejections with different tags don't count as a
    /// disagreement — that's a taxonomy delta, not evasion.
    #[must_use]
    pub fn has_cross_version_disagreement(&self) -> bool {
        use p113_fuzz_harness::classifier::Verdict;
        // Require verifier to accept; otherwise it's a CVE class.
        let axiom_accepts = matches!(self.axiom(), Some(Verdict::Accept));
        if !axiom_accepts {
            return false;
        }
        let mut accepts = false;
        let mut rejects = false;
        for f in &self.findings {
            match &f.target {
                Verdict::Accept => accepts = true,
                Verdict::Reject(_) => rejects = true,
            }
            if accepts && rejects {
                return true;
            }
        }
        false
    }

    /// True iff every finding's target rejects the input.
    #[must_use]
    pub fn all_targets_reject(&self) -> bool {
        self.findings.iter().all(|f| match &f.target {
            p113_fuzz_harness::classifier::Verdict::Accept => false,
            p113_fuzz_harness::classifier::Verdict::Reject(_) => true,
        })
    }

    /// True iff every finding's target accepts the input.
    #[must_use]
    pub fn all_targets_accept(&self) -> bool {
        self.findings
            .iter()
            .all(|f| matches!(f.target, p113_fuzz_harness::classifier::Verdict::Accept))
    }

    /// True iff every finding came from the synthetic version
    /// layer (no real per-version probe contributed).
    #[must_use]
    pub fn all_synthetic(&self) -> bool {
        !self.findings.is_empty() && self.findings.iter().all(|f| f.synthetic)
    }

    /// `axiom_l0` verdict (consistent across all findings in the
    /// group, since the axiom side is version-independent).
    #[must_use]
    pub fn axiom(&self) -> Option<&p113_fuzz_harness::classifier::Verdict> {
        self.findings.first().map(|f| &f.axiom_l0)
    }

    /// True iff every finding has the same `bucket`.
    #[must_use]
    pub fn all_bucket(&self, b: Bucket) -> bool {
        !self.findings.is_empty() && self.findings.iter().all(|f| f.bucket == b)
    }
}

/// The full rule list. See module docs for category descriptions.
pub const RULES: &[Rule] = &[
    // --- AOSP CVE candidate (highest priority) ---------------
    Rule {
        id: "cve.bucket-e-real",
        label: Label::AospCveCandidate,
        weight: 95,
        fires: |g| {
            g.findings.iter().any(|f| f.bucket == Bucket::E && !f.synthetic)
        },
    },
    Rule {
        id: "cve.bucket-e-synthetic",
        label: Label::AospCveCandidate,
        weight: 60, // weighted lower than real probes
        fires: |g| {
            g.findings.iter().any(|f| f.bucket == Bucket::E && f.synthetic)
                && !g.findings.iter().any(|f| f.bucket == Bucket::E && !f.synthetic)
        },
    },
    // --- Cross-version evasion -------------------------------
    // Cross-version evasion outranks plain CVE: an accept↔reject
    // split across Android versions is the highest-value finding
    // class per P1.14 README §2 ("cross-version disagreements
    // are gold"). When both fire on the same input, the
    // version-aware label is more actionable for disclosure
    // (it identifies *which* device cohort is exposed).
    Rule {
        id: "xv.disagreement-real",
        label: Label::CrossVersionEvasion,
        weight: 96,
        fires: |g| g.has_cross_version_disagreement() && !g.all_synthetic(),
    },
    Rule {
        id: "xv.disagreement-synthetic-only",
        label: Label::CrossVersionEvasion,
        weight: 50,
        fires: |g| g.has_cross_version_disagreement() && g.all_synthetic(),
    },
    // --- Model bug -------------------------------------------
    Rule {
        id: "model.all-d",
        label: Label::ModelBug,
        weight: 85,
        fires: |g| g.all_bucket(Bucket::D),
    },
    Rule {
        id: "model.axiom-accept-all-reject",
        label: Label::ModelBug,
        weight: 80,
        fires: |g| {
            matches!(
                g.axiom(),
                Some(p113_fuzz_harness::classifier::Verdict::Accept)
            ) && g.all_targets_reject()
        },
    },
    // --- Spec ambiguity --------------------------------------
    Rule {
        id: "spec.all-c",
        label: Label::SpecAmbiguity,
        weight: 30,
        fires: |g| g.all_bucket(Bucket::C),
    },
];

/// Stateless classifier — applies the rule list to a finding
/// group and returns `(label, rule_id, weight)`. Returns `None`
/// when no rule fires (empty group, or every finding is bucket-A
/// which is not a finding category).
#[derive(Debug, Default, Clone, Copy)]
pub struct Classifier;

impl Classifier {
    /// Apply [`RULES`] to a group; pick the highest-weight
    /// matching rule. Returns the label, the firing rule id, and
    /// the weight, so downstream consumers can audit *why* a
    /// label was emitted.
    #[must_use]
    pub fn classify(group: &FindingGroup) -> Option<(Label, &'static str, u8)> {
        RULES
            .iter()
            .filter(|r| (r.fires)(group))
            .max_by_key(|r| r.weight)
            .map(|r| (r.label, r.id, r.weight))
    }
}

/// Group an unsorted slice of findings by `input_sha256`.
#[must_use]
pub fn group_by_input(findings: &[Finding]) -> Vec<FindingGroup> {
    use std::collections::BTreeMap;
    let mut by_sha: BTreeMap<String, Vec<Finding>> = BTreeMap::new();
    for f in findings {
        by_sha.entry(f.input_sha256.clone()).or_default().push(f.clone());
    }
    by_sha
        .into_iter()
        .map(|(input_sha256, findings)| FindingGroup {
            input_sha256,
            findings,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use p113_fuzz_harness::classifier::Verdict;

    fn finding_axiom(
        axiom: Verdict,
        target_version: &str,
        bucket: Bucket,
        target: Verdict,
        synthetic: bool,
    ) -> Finding {
        Finding {
            finding_id: "abc".into(),
            timestamp_ns: 0,
            mode: "dev".into(),
            target_label: format!("aosp-{}", target_version.to_lowercase()),
            input_sha256: "abc".into(),
            input_path: "inputs/abc.bin".into(),
            input_len: 4,
            axiom_l0: axiom,
            target,
            bucket,
            seed_origin: Some("seed".into()),
            mutation_kind: Some("flip".into()),
            target_version: target_version.into(),
            synthetic,
        }
    }

    fn finding(target_version: &str, bucket: Bucket, target: Verdict, synthetic: bool) -> Finding {
        finding_axiom(
            Verdict::Reject("axiom:bad-eocd".into()),
            target_version,
            bucket,
            target,
            synthetic,
        )
    }

    #[test]
    fn cve_real_outranks_cve_synthetic() {
        let group = FindingGroup {
            input_sha256: "abc".into(),
            findings: vec![
                finding("A14", Bucket::E, Verdict::Accept, false),
                finding("A11", Bucket::E, Verdict::Accept, true),
            ],
        };
        let (label, rule_id, _) = Classifier::classify(&group).unwrap();
        assert_eq!(label, Label::AospCveCandidate);
        assert_eq!(rule_id, "cve.bucket-e-real");
    }

    #[test]
    fn cve_synthetic_only_lower_weight() {
        let group = FindingGroup {
            input_sha256: "abc".into(),
            findings: vec![finding("A11", Bucket::E, Verdict::Accept, true)],
        };
        let (label, _, weight) = Classifier::classify(&group).unwrap();
        assert_eq!(label, Label::AospCveCandidate);
        assert!(weight < 90, "synthetic-only CVE should score below real cross-version: {weight}");
    }

    #[test]
    fn cross_version_evasion_real() {
        // axiom accepts; A14 accepts; A11 rejects → cross-version
        // disagreement under the verifier-accepts gate.
        let group = FindingGroup {
            input_sha256: "xv1".into(),
            findings: vec![
                finding_axiom(Verdict::Accept, "A14", Bucket::A, Verdict::Accept, false),
                finding_axiom(
                    Verdict::Accept,
                    "A11",
                    Bucket::D,
                    Verdict::Reject("aosp:-3".into()),
                    false,
                ),
            ],
        };
        let (label, rule_id, _) = Classifier::classify(&group).unwrap();
        assert_eq!(label, Label::CrossVersionEvasion);
        assert_eq!(rule_id, "xv.disagreement-real");
    }

    #[test]
    fn axiom_rejects_split_does_not_fire_xv() {
        // axiom rejects; A14 accepts; A11 rejects. Even though
        // the targets disagree, the verifier rejection blocks
        // install at the gate — this is bucket-E CVE territory,
        // not cross-version evasion.
        let group = FindingGroup {
            input_sha256: "ce1".into(),
            findings: vec![
                finding_axiom(
                    Verdict::Reject("axiom:bad".into()),
                    "A14",
                    Bucket::E,
                    Verdict::Accept,
                    false,
                ),
                finding_axiom(
                    Verdict::Reject("axiom:bad".into()),
                    "A11",
                    Bucket::B,
                    Verdict::Reject("aosp:-3".into()),
                    false,
                ),
            ],
        };
        let (label, rule_id, _) = Classifier::classify(&group).unwrap();
        assert_eq!(label, Label::AospCveCandidate);
        assert_eq!(rule_id, "cve.bucket-e-real");
    }

    #[test]
    fn model_bug_unanimous_d() {
        let group = FindingGroup {
            input_sha256: "mb1".into(),
            findings: vec![
                finding("A14", Bucket::D, Verdict::Reject("aosp:-3".into()), false),
                finding("A11", Bucket::D, Verdict::Reject("aosp:-3".into()), false),
            ],
        };
        let (label, _, _) = Classifier::classify(&group).unwrap();
        assert_eq!(label, Label::ModelBug);
    }

    #[test]
    fn spec_ambiguity_all_c() {
        let group = FindingGroup {
            input_sha256: "sa1".into(),
            findings: vec![
                finding("A14", Bucket::C, Verdict::Reject("aosp:-3".into()), false),
                finding("A11", Bucket::C, Verdict::Reject("aosp:-3".into()), false),
            ],
        };
        let (label, _, _) = Classifier::classify(&group).unwrap();
        assert_eq!(label, Label::SpecAmbiguity);
    }

    #[test]
    fn no_rule_fires_on_bucket_a() {
        let group = FindingGroup {
            input_sha256: "a1".into(),
            findings: vec![finding("A14", Bucket::A, Verdict::Accept, false)],
        };
        assert!(Classifier::classify(&group).is_none());
    }

    #[test]
    fn group_by_input_collapses_duplicates() {
        let findings = vec![
            finding("A14", Bucket::E, Verdict::Accept, false),
            finding("A11", Bucket::E, Verdict::Accept, true),
            finding("A8", Bucket::E, Verdict::Accept, true),
        ];
        let groups = group_by_input(&findings);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].findings.len(), 3);
    }
}
