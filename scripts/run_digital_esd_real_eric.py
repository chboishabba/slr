#!/usr/bin/env python3
"""Run the real Digital-ESD ERIC metadata -> adaptive screening work queue.

This is the executable P0-A..P0-F path over retained ERIC exports.

It performs:
  retained ERIC JSON -> verified parser -> 46,597 occurrences
  -> 43,996 accession-deduplicated metadata records
  -> authoritative unresolved screening ledger
  -> candidate-only title/abstract assessments
  -> candidate publication/report-family hypotheses
  -> deterministic calibration worklist
  -> non-scalar Pareto review queue

It deliberately stops before full-text acquisition.  P0-G is owned by
prepare_digital_esd_fulltext_index.py and requires authoritative include|probable
screening decisions plus actual retrieved artifacts with matching SHA-256.

Nothing in this driver creates a screening decision, source truth, or
SourceAuditAdmission.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import itertools
import json
import re
import uuid
from collections import defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from interop_scripts.digital_esd_eric import (
    ERICParser,
    ERICRecord,
    QUERY_OCCURRENCE_EXPECTED,
    UNIQUE_RECORD_EXPECTED,
)
from interop_scripts.digital_esd_screening import (
    candidate_assessment_from_screening_row,
    screening_row_from_eric,
)


FULL_TEXT_STOP = True
AXES = (
    "information_gain_loss_cost",
    "corpus_contraction_loss_cost",
    "rare_cell_coverage_loss_cost",
    "duplicate_family_payoff_loss_cost",
    "reviewer_cost",
)
WORD_RE = re.compile(r"[a-z0-9]+")


def now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def sha256_json(value: Any) -> str:
    data = json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")
    return sha256_bytes(data)


def stable_key(value: str, salt: str) -> str:
    return hashlib.sha256((salt + "\0" + value).encode("utf-8")).hexdigest()


def normalized_title(record: ERICRecord) -> str:
    return " ".join(WORD_RE.findall(record.title.lower()))


def write_tsv(path: Path, rows: list[dict[str, Any]]) -> None:
    if not rows:
        path.write_text("", encoding="utf-8")
        return
    fields = list(rows[0].keys())
    with path.open("w", newline="", encoding="utf-8") as fh:
        writer = csv.DictWriter(fh, fieldnames=fields, delimiter="\t", extrasaction="ignore")
        writer.writeheader()
        writer.writerows(rows)


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    with path.open("w", encoding="utf-8") as fh:
        for row in rows:
            fh.write(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n")


def build_family_hypotheses(records: list[ERICRecord]) -> tuple[list[dict[str, Any]], dict[str, int]]:
    """Generate conservative candidate duplicate/report-family edges.

    Exact normalized title equality is advisory only.  It does not create a
    publication-duplicate or same-empirical-study decision.
    """
    by_title: dict[str, list[ERICRecord]] = defaultdict(list)
    for record in records:
        title = normalized_title(record)
        if title:
            by_title[title].append(record)

    hypotheses: list[dict[str, Any]] = []
    fibre_sizes: dict[str, int] = {}
    for title, members in by_title.items():
        if len(members) < 2:
            continue
        refs = [f"ERIC:{m.accession}" for m in members]
        for ref in refs:
            fibre_sizes[ref] = max(fibre_sizes.get(ref, 0), len(refs))
        for a, b in itertools.combinations(sorted(refs), 2):
            payload = {
                "left": a,
                "right": b,
                "basis": "exact-normalized-title",
                "normalized_title": title,
            }
            hypotheses.append(
                {
                    "schema": "digital-esd-study-family-hypothesis-v1",
                    "hypothesis_reference": "study-family-hypothesis:" + sha256_json(payload),
                    "left_source_identity_reference": a,
                    "right_source_identity_reference": b,
                    "proposed_relation": "publicationDuplicate",
                    "evidence": {
                        "basis": "exact-normalized-title",
                        "normalized_title": title,
                    },
                    "candidate_only": True,
                    "creates_duplicate_decision": False,
                    "creates_same_empirical_study": False,
                }
            )
    return hypotheses, fibre_sizes


def stratum(
    row: dict[str, Any],
    assessment: dict[str, Any],
    fibre_size: int,
) -> str:
    reasons = set(assessment["candidate_reason_codes"])
    candidate = assessment["candidate_decision"]
    confidence = assessment["confidence_reference"]
    if "inaccessibleAbstract" in reasons:
        return "missingAbstractOrMalformedMetadata"
    if fibre_size >= 2:
        return "highDuplicateAmbiguity"
    if candidate == "probable" and confidence.startswith("high"):
        return "obviousIncludeCandidate"
    if candidate == "exclude" and confidence.startswith("high"):
        return "obviousExcludeCandidate"
    return "highUncertaintyCandidate"


def priority_costs(
    row: dict[str, Any],
    assessment: dict[str, Any],
    stratum_name: str,
    fibre_size: int,
) -> dict[str, int]:
    info = {
        "highUncertaintyCandidate": 0,
        "missingAbstractOrMalformedMetadata": 1,
        "highDuplicateAmbiguity": 2,
        "obviousIncludeCandidate": 4,
        "obviousExcludeCandidate": 4,
    }[stratum_name]
    contraction = 0 if stratum_name == "obviousExcludeCandidate" else (
        1 if stratum_name == "highDuplicateAmbiguity" else 3
    )
    rare = 0 if len(assessment["feature_evidence"]["query_memberships"].split(";")) == 1 else 3
    duplicate = max(0, 8 - min(fibre_size, 8)) if fibre_size >= 2 else 8
    abstract_len = int(assessment["feature_evidence"]["abstract_length"])
    reviewer = 2 if abstract_len == 0 else (1 if abstract_len <= 1000 else 2 if abstract_len <= 2500 else 3)
    return {
        "information_gain_loss_cost": info,
        "corpus_contraction_loss_cost": contraction,
        "rare_cell_coverage_loss_cost": rare,
        "duplicate_family_payoff_loss_cost": duplicate,
        "reviewer_cost": reviewer,
    }


def dominates(a: dict[str, Any], b: dict[str, Any]) -> bool:
    return all(int(a[k]) <= int(b[k]) for k in AXES) and any(
        int(a[k]) < int(b[k]) for k in AXES
    )


def pareto_front(rows: list[dict[str, Any]]) -> set[str]:
    front: set[str] = set()
    for i, row in enumerate(rows):
        if not any(i != j and dominates(other, row) for j, other in enumerate(rows)):
            front.add(str(row["source_identity_reference"]))
    return front


def build_adaptive_work_queue(
    ledger_rows: list[dict[str, Any]],
    assessments: list[dict[str, Any]],
    fibre_sizes: dict[str, int],
    *,
    calibration_per_stratum: int,
    salt: str,
) -> tuple[list[dict[str, Any]], list[dict[str, Any]], dict[str, Any]]:
    assessment_by_ref = {
        str(row["source_identity_reference"]): row for row in assessments
    }
    queue: list[dict[str, Any]] = []
    strata: dict[str, list[str]] = defaultdict(list)

    for row in ledger_rows:
        ref = str(row["source_identity_reference"])
        if row["decision"] != "unresolved":
            continue
        assessment = assessment_by_ref[ref]
        fibre_size = fibre_sizes.get(ref, 0)
        s = stratum(row, assessment, fibre_size)
        strata[s].append(ref)
        candidate = {
            "schema": "digital-esd-screening-pareto-candidate-v1",
            "source_identity_reference": ref,
            "candidate_assessment_reference": assessment["assessment_reference"],
            "calibration_stratum_reference": s,
            "candidate_only": True,
            "pending_explicit_review": True,
            "selection_creates_screening_decision": False,
            "selection_creates_exclusion": False,
        }
        candidate.update(priority_costs(row, assessment, s, fibre_size))
        queue.append(candidate)

    front = pareto_front(queue)
    for row in queue:
        row["pareto_front"] = row["source_identity_reference"] in front
    queue.sort(
        key=lambda row: (
            not row["pareto_front"],
            tuple(int(row[k]) for k in AXES),
            stable_key(row["source_identity_reference"], salt),
        )
    )

    calibration: list[dict[str, Any]] = []
    for s, refs in sorted(strata.items()):
        ordered = sorted(refs, key=lambda ref: stable_key(ref, salt + ":" + s))
        for ref in ordered[:calibration_per_stratum]:
            payload = {"source": ref, "stratum": s, "salt": salt}
            calibration.append(
                {
                    "schema": "digital-esd-calibration-selection-v1",
                    "source_identity_reference": ref,
                    "stratum": s,
                    "selection_reference": "calibration-selection:" + sha256_json(payload),
                    "selected_for_explicit_review": True,
                    "selection_creates_decision": False,
                }
            )

    calibration_estimate = {
        "schema": "digital-esd-screening-calibration-estimate-v1",
        "reviewed_pair_count": 0,
        "candidate_false_negative_proxy": None,
        "candidate_review_disagreement_rate": None,
        "estimate_scope_reference": "no reviewed calibration decisions supplied yet",
        "estimate_creates_source_truth": False,
        "estimate_creates_population_truth": False,
        "estimate_creates_automatic_decision": False,
    }
    return queue, calibration, calibration_estimate


def main() -> int:
    parser = argparse.ArgumentParser(description="Run real ERIC -> adaptive Digital-ESD screening work queue")
    parser.add_argument("--export-root", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, default=Path("artifacts/digital-esd/real-eric"))
    parser.add_argument("--expected-hashes", type=Path)
    parser.add_argument("--expect-occurrences", type=int, default=QUERY_OCCURRENCE_EXPECTED)
    parser.add_argument("--expect-unique", type=int, default=UNIQUE_RECORD_EXPECTED)
    parser.add_argument("--calibration-per-stratum", type=int, default=25)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    args.output_root.mkdir(parents=True, exist_ok=True)
    started = now_iso()
    run = uuid.uuid4().hex[:16]

    expected_hashes = None
    if args.expected_hashes:
        expected_hashes = {int(k): v for k, v in json.loads(args.expected_hashes.read_text()).items()}

    eric = ERICParser(args.export_root)
    eric.load_all_pages(expected_hashes)
    parsed = eric.parse_all_pages()
    unique = eric.deduplicate(parsed)
    count_receipt = eric.validate_counts(unique)

    if eric.query_occurrences != args.expect_occurrences:
        raise RuntimeError(
            f"raw occurrence mismatch: observed={eric.query_occurrences} expected={args.expect_occurrences}"
        )
    if len(unique) != args.expect_unique:
        raise RuntimeError(
            f"unique record mismatch: observed={len(unique)} expected={args.expect_unique}"
        )

    metadata_path = args.output_root / "digital_esd_eric_metadata.tsv"
    eric.export_records(metadata_path, unique)

    ledger_rows = [screening_row_from_eric(record) for record in unique]
    ledger_path = args.output_root / "screening_ledger.tsv"
    write_tsv(ledger_path, ledger_rows)

    assessments = [candidate_assessment_from_screening_row(row) for row in ledger_rows]
    assessments_path = args.output_root / "candidate_assessments.jsonl"
    write_jsonl(assessments_path, assessments)

    hypotheses, fibre_sizes = build_family_hypotheses(unique)
    hypotheses_path = args.output_root / "study_family_hypotheses.jsonl"
    write_jsonl(hypotheses_path, hypotheses)

    queue, calibration, calibration_estimate = build_adaptive_work_queue(
        ledger_rows,
        assessments,
        fibre_sizes,
        calibration_per_stratum=args.calibration_per_stratum,
        salt="digital-esd-real-eric-v1",
    )
    queue_path = args.output_root / "screening_pareto_queue.jsonl"
    calibration_path = args.output_root / "calibration_selection.jsonl"
    estimate_path = args.output_root / "calibration_estimate.json"
    write_jsonl(queue_path, queue)
    write_jsonl(calibration_path, calibration)
    estimate_path.write_text(
        json.dumps(calibration_estimate, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )

    artifact_paths = {
        "parsed_metadata_corpus": metadata_path,
        "screening_ledger": ledger_path,
        "candidate_assessment": assessments_path,
        "study_family_hypotheses": hypotheses_path,
        "calibration_selection": calibration_path,
        "calibration_estimate": estimate_path,
        "pareto_queue": queue_path,
    }
    artifact_hashes = {name: sha256_file(path) for name, path in artifact_paths.items()}

    receipt = {
        "schema": "sensiblaw.digital-esd-real-eric-execution.v0_2",
        "run_id": run,
        "started_at": started,
        "completed_at": now_iso(),
        "expected_raw_occurrences": args.expect_occurrences,
        "observed_raw_occurrences": eric.query_occurrences,
        "expected_unique_records": args.expect_unique,
        "observed_unique_records": len(unique),
        "real_eric": True,
        "full_text_stop": FULL_TEXT_STOP,
        "screening_ledger_all_unresolved": all(row["decision"] == "unresolved" for row in ledger_rows),
        "candidate_assessment_count": len(assessments),
        "study_family_hypothesis_count": len(hypotheses),
        "calibration_selection_count": len(calibration),
        "pareto_queue_count": len(queue),
        "pareto_front_count": sum(1 for row in queue if row["pareto_front"]),
        "scalar_screening_score_used": False,
        "creates_screening_decision": False,
        "creates_source_truth": False,
        "creates_source_audit_admission": False,
        "artifact_paths": {name: str(path) for name, path in artifact_paths.items()},
        "artifact_hashes": artifact_hashes,
        "count_receipt": count_receipt,
        "counts_match": (
            eric.query_occurrences == args.expect_occurrences
            and len(unique) == args.expect_unique
        ),
    }
    receipt_path = args.output_root / "execution_receipt.json"
    receipt_path.write_text(
        json.dumps(receipt, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )

    if args.json:
        print(json.dumps(receipt, indent=2, sort_keys=True))
    else:
        print(
            "DIGITAL_ESD_REAL_ERIC "
            f"occurrences={eric.query_occurrences} "
            f"unique={len(unique)} "
            f"unresolved={len(ledger_rows)} "
            f"pareto_front={receipt['pareto_front_count']} "
            f"receipt={receipt_path}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
