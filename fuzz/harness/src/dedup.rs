// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Finding deduplication & root-cause clustering.
//!
//! A naive fuzz loop generates many findings that are
//! "same root cause, different bit-flip". The classifier sees
//! each one as distinct (different SHA, different byte offset)
//! but they all manifest the same `(axiom-l0 verdict, target
//! verdict, seed-origin file)` triple — which is the canonical
//! root-cause key.
//!
//! This module groups findings by that triple and reports only
//! one canonical reproducer per cluster, picking the
//! shortest-input member as the "minimal reproducer". A
//! follow-up `tmin` step (byte-level shrinking) is left to
//! Phase 2.

use std::collections::HashMap;

use crate::archive::Finding;

/// Cluster key — the triple that defines a root cause.
/// `(seed_origin, axiom_verdict, target_verdict)` is stable
/// across single-bit-flip mutations of the same seed.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ClusterKey {
    /// Path of the seed the mutation derived from.
    pub seed_origin: String,
    /// axiom-l0's verdict label.
    pub axiom: String,
    /// Target's verdict label.
    pub target: String,
}

impl ClusterKey {
    /// Build a key from a Finding.
    #[must_use]
    pub fn from_finding(f: &Finding) -> Self {
        Self {
            seed_origin: f.seed_origin.clone().unwrap_or_default(),
            axiom: f.axiom_l0.label(),
            target: f.target.label(),
        }
    }
}

/// Group findings by `ClusterKey` and pick the shortest-input
/// member as the canonical reproducer.
#[must_use]
pub fn dedupe(findings: &[Finding]) -> Vec<Finding> {
    let mut clusters: HashMap<ClusterKey, Finding> = HashMap::new();
    for f in findings {
        let k = ClusterKey::from_finding(f);
        match clusters.get(&k) {
            Some(existing) if existing.input_len <= f.input_len => {}
            _ => {
                clusters.insert(k, f.clone());
            }
        }
    }
    let mut out: Vec<Finding> = clusters.into_values().collect();
    out.sort_by(|a, b| a.finding_id.cmp(&b.finding_id));
    out
}

/// Cluster summary: count of distinct root-cause clusters split
/// by classifier bucket. The "honest finding count" is
/// `dedupe(...).len()`, restricted to D + E buckets (E is the
/// security-critical class; D is the leniency class).
#[derive(Debug, Default, Clone, Copy)]
pub struct DedupeSummary {
    /// Distinct D-bucket clusters (axiom-l0 lax).
    pub d_clusters: usize,
    /// Distinct E-bucket clusters (axiom-l0 strict — potential CVE).
    pub e_clusters: usize,
    /// Distinct C-bucket clusters (taxonomy delta — informational).
    pub c_clusters: usize,
    /// Total raw finding records (all of C + D + E).
    pub raw_findings: usize,
    /// Total deduped clusters across C + D + E.
    pub total_clusters: usize,
}

impl DedupeSummary {
    /// Honest finding count — D + E clusters only (excluding C
    /// taxonomy noise).
    #[must_use]
    pub fn honest_count(&self) -> usize {
        self.d_clusters + self.e_clusters
    }
}

/// Compute a dedupe summary over an archive's full set of
/// findings.
#[must_use]
pub fn summarise(findings: &[Finding]) -> DedupeSummary {
    use crate::classifier::Bucket;
    let mut s = DedupeSummary {
        raw_findings: findings.len(),
        ..Default::default()
    };
    let deduped = dedupe(findings);
    s.total_clusters = deduped.len();
    for f in &deduped {
        match f.bucket {
            Bucket::D => s.d_clusters += 1,
            Bucket::E => s.e_clusters += 1,
            Bucket::C => s.c_clusters += 1,
            _ => {}
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archive::Finding;
    use crate::classifier::{Bucket, Verdict};

    fn finding_with(seed: &str, len: u64, sha: &str, bucket: Bucket) -> Finding {
        Finding {
            finding_id: sha.into(),
            timestamp_ns: 0,
            mode: "dev".into(),
            target_label: "aosp".into(),
            input_sha256: sha.into(),
            input_path: format!("inputs/{sha}.bin"),
            input_len: len,
            axiom_l0: Verdict::Accept,
            target: Verdict::Reject("9".into()),
            bucket,
            seed_origin: Some(seed.into()),
            mutation_kind: Some("flip".into()),
            target_version: "A14".into(),
            synthetic: false,
        }
    }

    #[test]
    fn dedupe_picks_shortest_per_cluster() {
        let findings = vec![
            finding_with("seed-a", 100, "aaa", Bucket::E),
            finding_with("seed-a", 50, "bbb", Bucket::E),
            finding_with("seed-a", 200, "ccc", Bucket::E),
        ];
        let out = dedupe(&findings);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].finding_id, "bbb");
    }

    #[test]
    fn dedupe_separates_by_seed_origin() {
        let findings = vec![
            finding_with("seed-a", 100, "aaa", Bucket::E),
            finding_with("seed-b", 100, "bbb", Bucket::E),
        ];
        let out = dedupe(&findings);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn summarise_separates_buckets() {
        let findings = vec![
            finding_with("a", 10, "1", Bucket::D),
            finding_with("a", 20, "2", Bucket::D), // dups cluster (a, accept, reject:9)
            finding_with("b", 10, "3", Bucket::E),
            finding_with("c", 10, "4", Bucket::C),
        ];
        let s = summarise(&findings);
        assert_eq!(s.raw_findings, 4);
        assert_eq!(s.total_clusters, 3);
        assert_eq!(s.d_clusters, 1);
        assert_eq!(s.e_clusters, 1);
        assert_eq!(s.c_clusters, 1);
        assert_eq!(s.honest_count(), 2);
    }
}
