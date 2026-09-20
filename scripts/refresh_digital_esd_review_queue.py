#!/usr/bin/env python3
"""Refresh Digital-ESD calibration diagnostics and Pareto queue after review.

Consumes:
- current authoritative screening ledger;
- candidate-only assessments;
- candidate family hypotheses.

Produces:
- calibration_estimate.json based only on explicitly reviewed rows;
- calibration_selection.jsonl for the remaining unresolved rows;
- screening_pareto_queue.jsonl for the remaining unresolved rows.

This script never modifies the authoritative ledger.
"""

from __future__ import annotations

import argparse
import csv
import json
import sys
from collections import Counter
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[1]
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))

from scripts.run_digital_esd_real_eric import build_adaptive_work_queue, write_jsonl


def read_tsv(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as fh:
        return [dict(row) for row in csv.DictReader(fh, delimiter="	")]


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    with path.open("r", encoding="utf-8") as fh:
        for n, line in enumerate(fh, 1):
            if not line.strip():
                continue
            row = json.loads(line)
            if not isinstance(row, dict):
                raise ValueError(f"{path}:{n}: expected JSON object")
            rows.append(row)
    return rows


def explicitly_reviewed(row: dict[str, str]) -> bool:
    """True only when an authority-changing review receipt has been applied.

    decision == unresolved is not enough to distinguish pending work from a
    reviewed ambiguity. The applied overlay replaces the unassigned reviewer
    and records a superseded decision reference.
    """
    reviewer = str(row.get("reviewer_or_model_reference") or "").strip()
    supersedes = str(row.get("supersedes_decision_reference") or "").strip()
    return reviewer not in {"", "unassigned"} and bool(supersedes)


class DSU:
    def __init__(self) -> None:
        self.parent: dict[str, str] = {}

    def find(self, x: str) -> str:
        self.parent.setdefault(x, x)
        if self.parent[x] != x:
            self.parent[x] = self.find(self.parent[x])
        return self.parent[x]

    def union(self, a: str, b: str) -> None:
        ra, rb = self.find(a), self.find(b)
        if ra != rb:
            self.parent[rb] = ra


def fibre_sizes(hypotheses: list[dict[str, Any]]) -> dict[str, int]:
    dsu = DSU()
    refs: set[str] = set()
    for row in hypotheses:
        a = str(row["left_source_identity_reference"])
        b = str(row["right_source_identity_reference"])
        refs.update((a, b))
        dsu.union(a, b)
    groups: dict[str, list[str]] = {}
    for ref in refs:
        groups.setdefault(dsu.find(ref), []).append(ref)
    out: dict[str, int] = {}
    for members in groups.values():
        for ref in members:
            out[ref] = len(members)
    return out


def calibration_estimate(
    ledger_rows: list[dict[str, str]],
    assessments: list[dict[str, Any]],
) -> dict[str, Any]:
    assessment_by_ref = {
        str(row["source_identity_reference"]): row for row in assessments
    }
    pairs: list[tuple[str, str]] = []
    for row in ledger_rows:
        actual = str(row.get("decision") or "")
        if actual == "unresolved" and not explicitly_reviewed(row):
            continue
        ref = str(row["source_identity_reference"])
        assessment = assessment_by_ref.get(ref)
        if assessment is None:
            continue
        candidate = str(assessment.get("candidate_decision") or "unresolved")
        pairs.append((actual, candidate))

    confusion = Counter(pairs)
    positives = sum(1 for actual, _ in pairs if actual in {"include", "probable"})
    false_negatives = sum(
        1
        for actual, candidate in pairs
        if actual in {"include", "probable"} and candidate == "exclude"
    )
    disagreements = sum(1 for actual, candidate in pairs if actual != candidate)

    return {
        "schema": "digital-esd-screening-calibration-estimate-v0_2",
        "reviewed_pair_count": len(pairs),
        "reviewed_positive_count": positives,
        "candidate_false_negative_proxy_n": false_negatives,
        "candidate_false_negative_proxy": (
            false_negatives / positives if positives else None
        ),
        "candidate_review_disagreement_n": disagreements,
        "candidate_review_disagreement_rate": (
            disagreements / len(pairs) if pairs else None
        ),
        "confusion_counts": {
            f"{actual}->{candidate}": count
            for (actual, candidate), count in sorted(confusion.items())
        },
        "estimate_scope_reference": "explicitly reviewed screening subset only",
        "estimate_creates_source_truth": False,
        "estimate_creates_population_truth": False,
        "estimate_creates_automatic_decision": False,
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--ledger", type=Path, required=True)
    ap.add_argument("--assessments", type=Path, required=True)
    ap.add_argument("--hypotheses", type=Path, required=True)
    ap.add_argument("--output-dir", type=Path, required=True)
    ap.add_argument("--calibration-per-stratum", type=int, default=25)
    args = ap.parse_args()

    ledger = read_tsv(args.ledger)
    assessments = read_jsonl(args.assessments)
    hypotheses = read_jsonl(args.hypotheses)

    ledger_refs = {str(row["source_identity_reference"]) for row in ledger}
    assessment_refs = {str(row["source_identity_reference"]) for row in assessments}
    if ledger_refs != assessment_refs:
        raise ValueError("ledger and candidate assessment identity sets differ")

    sizes = fibre_sizes(hypotheses)
    title_abstract_pending = [
        row for row in ledger
        if not (
            str(row.get("decision") or "") == "unresolved"
            and explicitly_reviewed(row)
        )
    ]
    queue, calibration, _ = build_adaptive_work_queue(
        title_abstract_pending,
        assessments,
        sizes,
        calibration_per_stratum=args.calibration_per_stratum,
        salt="digital-esd-reviewed-refresh-v1",
    )
    estimate = calibration_estimate(ledger, assessments)

    args.output_dir.mkdir(parents=True, exist_ok=True)
    write_jsonl(args.output_dir / "screening_pareto_queue.jsonl", queue)
    write_jsonl(args.output_dir / "calibration_selection.jsonl", calibration)
    (args.output_dir / "calibration_estimate.json").write_text(
        json.dumps(estimate, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )

    counts = Counter(str(row.get("decision") or "") for row in ledger)
    manifest = {
        "schema": "sensiblaw.digital-esd-adaptive-refresh.v0_1",
        "screening_record_count": len(ledger),
        "decision_counts": dict(sorted(counts.items())),
        "unresolved_queue_count": len(queue),
        "pareto_front_count": sum(1 for row in queue if row.get("pareto_front") is True),
        "calibration_selection_count": len(calibration),
        "reviewed_pair_count": estimate["reviewed_pair_count"],
        "reviewed_unresolved_count": sum(
            1
            for row in ledger
            if str(row.get("decision") or "") == "unresolved"
            and explicitly_reviewed(row)
        ),
        "candidate_false_negative_proxy": estimate["candidate_false_negative_proxy"],
        "candidate_review_disagreement_rate": estimate["candidate_review_disagreement_rate"],
        "ledger_modified": False,
        "automatic_screening_decisions_created": False,
    }
    (args.output_dir / "adaptive_refresh_manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8",
    )
    print(json.dumps(manifest, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
