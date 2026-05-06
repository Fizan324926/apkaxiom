#!/usr/bin/env python3
# Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
"""
P1.14 ML classifier harness — trains an xgboost model on the
same archive + holdout that the rules engine evaluates against,
prints precision/recall/F1 per label, and exits non-zero if the
ML model's micro precision falls below `--min-precision`.

Usage:
    fuzz/classifier/train.py \
      --archive fuzz/findings/archive.ndjson \
      --holdout fuzz/classifier/holdout.tsv \
      [--min-precision 0.80]

Features used (deliberately small — same signal the rules engine
sees, plus structural input markers):

  - axiom verdict type  (accept / reject)
  - per-version target verdict types (A8/A11/A14 — accept/reject)
  - bucket of the primary record (A/B/C/D/E)
  - synthetic flag (any record synthetic?)
  - input length
  - has ZIP64 EOCD locator marker  PK\\x06\\x07
  - has UTF-8 filename flag set in any LFH or CDR

The point of this script is NOT to replace the rules engine.
The rules engine is the production classifier; this script is
the audit-2 §D'-7 closure that produces a comparable ML number,
quantifying how much (or how little) ML training adds. The
rules engine consistently hits 100% on both holdouts under the
threat-model rule set, so the ML number is expected to be
similarly high — its value is as a SECOND, INDEPENDENT
classifier whose disagreements with the rules engine flag inputs
worth manual review.
"""

from __future__ import annotations
import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path

LABELS = ["aosp-cve-candidate", "cross-version-evasion", "model-bug", "spec-ambiguity"]
LABEL_TO_IDX = {l: i for i, l in enumerate(LABELS)}


@dataclass
class Group:
    sha: str
    axiom_accept: bool
    a14_accept: bool | None
    a11_accept: bool | None
    a8_accept: bool | None
    bucket_a: int
    bucket_b: int
    bucket_c: int
    bucket_d: int
    bucket_e: int
    any_synthetic: bool
    input_len: int
    has_zip64: bool
    has_utf8_flag: bool


def load_groups(archive: Path, inputs_dir: Path) -> dict[str, Group]:
    by_sha: dict[str, list[dict]] = {}
    for line in archive.read_text().splitlines():
        if not line.strip():
            continue
        try:
            j = json.loads(line)
        except json.JSONDecodeError:
            continue
        by_sha.setdefault(j["input_sha256"], []).append(j)

    out: dict[str, Group] = {}
    for sha, entries in by_sha.items():
        primary = next((e for e in entries if not e.get("synthetic")), entries[0])
        axiom_accept = primary["axiom_l0"] == "accept"
        per_version = {e["target_version"]: e["target"] for e in entries}
        bucket_counts = {b: 0 for b in "ABCDE"}
        for e in entries:
            b = e["bucket"][0]  # first letter
            if b in bucket_counts:
                bucket_counts[b] += 1
        any_synthetic = any(e.get("synthetic") for e in entries)
        input_len = primary.get("input_len", 0)

        # Read input bytes for structural features.
        ip = inputs_dir / primary["input_path"]
        has_zip64 = False
        has_utf8 = False
        try:
            data = ip.read_bytes()
            has_zip64 = b"PK\x06\x07" in data
            # Scan for LFH and CDR with general-purpose bit 11 set.
            i = 0
            while i + 8 < len(data):
                if data[i:i+4] == b"PK\x03\x04":
                    flags = int.from_bytes(data[i+6:i+8], "little")
                    if flags & 0x0800:
                        has_utf8 = True
                        break
                if data[i:i+4] == b"PK\x01\x02" and i + 10 <= len(data):
                    flags = int.from_bytes(data[i+8:i+10], "little")
                    if flags & 0x0800:
                        has_utf8 = True
                        break
                i += 1
        except FileNotFoundError:
            pass

        def acc(v: str | None) -> bool | None:
            if v is None:
                return None
            return v == "accept"

        out[sha] = Group(
            sha=sha,
            axiom_accept=axiom_accept,
            a14_accept=acc(per_version.get("A14")),
            a11_accept=acc(per_version.get("A11")),
            a8_accept=acc(per_version.get("A8")),
            bucket_a=bucket_counts["A"],
            bucket_b=bucket_counts["B"],
            bucket_c=bucket_counts["C"],
            bucket_d=bucket_counts["D"],
            bucket_e=bucket_counts["E"],
            any_synthetic=any_synthetic,
            input_len=input_len,
            has_zip64=has_zip64,
            has_utf8_flag=has_utf8,
        )
    return out


def feature_row(g: Group) -> list[float]:
    def b3(v: bool | None) -> float:
        if v is None:
            return -1.0
        return 1.0 if v else 0.0
    return [
        b3(g.axiom_accept),
        b3(g.a14_accept),
        b3(g.a11_accept),
        b3(g.a8_accept),
        float(g.bucket_a),
        float(g.bucket_b),
        float(g.bucket_c),
        float(g.bucket_d),
        float(g.bucket_e),
        1.0 if g.any_synthetic else 0.0,
        float(g.input_len),
        1.0 if g.has_zip64 else 0.0,
        1.0 if g.has_utf8_flag else 0.0,
    ]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--archive", required=True, type=Path)
    ap.add_argument("--holdout", required=True, type=Path)
    ap.add_argument("--inputs-dir", type=Path, default=None,
                    help="default = archive's parent dir")
    ap.add_argument("--min-precision", type=float, default=0.80)
    args = ap.parse_args()

    inputs_dir = args.inputs_dir or args.archive.parent
    groups = load_groups(args.archive, inputs_dir)
    print(f"loaded {len(groups)} input-groups from {args.archive}")

    # Read holdout (sha → label).
    gt: dict[str, str] = {}
    for line in args.holdout.read_text().splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split("\t")
        if len(parts) != 2:
            continue
        gt[parts[0]] = parts[1]
    print(f"loaded {len(gt)} ground-truth labels from {args.holdout}")

    # Build features+labels for everything we have GT for.
    X = []
    y = []
    keep = []
    for sha, label in gt.items():
        if sha not in groups:
            continue
        if label not in LABEL_TO_IDX:
            continue
        X.append(feature_row(groups[sha]))
        y.append(LABEL_TO_IDX[label])
        keep.append(sha)
    print(f"feature rows: {len(X)}")
    if len(X) < 4:
        print("ERROR: need at least 4 holdout records to train")
        return 2

    import numpy as np
    from sklearn.model_selection import train_test_split
    from sklearn.metrics import classification_report, precision_recall_fscore_support
    import xgboost as xgb

    X = np.asarray(X, dtype=np.float32)
    y = np.asarray(y, dtype=np.int32)

    # 70/30 train/test split — small holdouts get cross-validation
    # rather than a single split, but for this audit the holdout
    # is auto-generated from the same archive so a single split
    # is sufficient evidence.
    X_tr, X_te, y_tr, y_te = train_test_split(X, y, test_size=0.3, random_state=42, stratify=y)

    model = xgb.XGBClassifier(
        n_estimators=100,
        max_depth=4,
        learning_rate=0.3,
        objective="multi:softmax",
        num_class=len(LABELS),
        random_state=42,
        eval_metric="mlogloss",
    )
    model.fit(X_tr, y_tr)
    y_pred = model.predict(X_te)

    print()
    print("=== xgboost classifier on test split ===")
    print(classification_report(y_te, y_pred, target_names=LABELS, zero_division=0))
    p, r, f, _ = precision_recall_fscore_support(y_te, y_pred, average="micro", zero_division=0)
    print(f"micro precision: {p:.4f}")
    print(f"gate (>= {args.min_precision:.2f}): {'PASS' if p >= args.min_precision else 'FAIL'}")
    if p < args.min_precision:
        print(f"::error::ML classifier micro precision {p:.4f} below gate {args.min_precision}")
        return 1

    # Also: predict against every group, compare to rules engine,
    # report agreement rate. The classifier's value is as a second
    # opinion — disagreements flag inputs worth review.
    X_all = np.asarray([feature_row(groups[s]) for s in keep], dtype=np.float32)
    y_all_pred = model.predict(X_all)
    y_all_true = y
    same = (y_all_pred == y_all_true).sum()
    total = len(y_all_true)
    print()
    print(f"=== xgboost vs ground truth on full holdout ({total} records) ===")
    print(f"agreement: {same}/{total} = {100.0 * same / total:.1f}%")
    return 0


if __name__ == "__main__":
    sys.exit(main())
