"""Regressions for explicit Digital-ESD review decisions and adaptive refresh."""
from __future__ import annotations

import csv
import json
from pathlib import Path

import pytest

from scripts.apply_digital_esd_screening_decisions import apply_decisions, read_tsv, write_tsv
from scripts.prepare_digital_esd_screening_resolution_fulltext import (
    explicitly_reviewed as resolution_explicitly_reviewed,
)
from scripts.refresh_digital_esd_review_queue import calibration_estimate, explicitly_reviewed


def _ledger_row(ref: str) -> dict[str, str]:
    return {
        "source_identity_reference": ref,
        "metadata_sha256": "a" * 64,
        "decision_reference": f"screening-decision:{ref}",
        "decision": "unresolved",
        "reason_code": "awaitingScreeningReview",
        "reviewer_or_model_reference": "unassigned",
        "title": f"Title {ref}",
        "abstract": "Abstract",
    }


def _assessment(ref: str, candidate: str) -> dict:
    return {
        "source_identity_reference": ref,
        "candidate_decision": candidate,
    }


def test_review_overlay_preserves_denominator_and_unmentioned_rows():
    ledger = [_ledger_row("ERIC:EJ1"), _ledger_row("ERIC:EJ2"), _ledger_row("ERIC:EJ3")]
    decisions = [
        {
            "source_identity_reference": "ERIC:EJ1",
            "reviewed": True,
            "decision": "include",
            "reason_code": "potentiallyRelevant",
            "reviewer_or_process_reference": "reviewer:human-1",
            "decision_timestamp": "2026-09-20T00:00:00Z",
        },
        {
            "source_identity_reference": "ERIC:EJ2",
            "reviewed": True,
            "decision": "exclude",
            "reason_code": "sustainabilityQuestionMismatch",
            "reviewer_or_process_reference": "reviewer:human-1",
            "decision_timestamp": "2026-09-20T00:01:00Z",
        },
    ]

    output, counts = apply_decisions(ledger, decisions)

    assert len(output) == len(ledger)
    by_ref = {row["source_identity_reference"]: row for row in output}
    assert by_ref["ERIC:EJ1"]["decision"] == "include"
    assert by_ref["ERIC:EJ2"]["decision"] == "exclude"
    assert by_ref["ERIC:EJ3"]["decision"] == "unresolved"
    assert counts == {
        "include": 1,
        "probable": 0,
        "exclude": 1,
        "unresolved": 1,
    }


def test_review_overlay_rejects_unknown_source_identity():
    ledger = [_ledger_row("ERIC:EJ1")]
    decisions = [
        {
            "source_identity_reference": "ERIC:UNKNOWN",
            "reviewed": True,
            "decision": "include",
            "reason_code": "potentiallyRelevant",
            "reviewer_or_process_reference": "reviewer:human-1",
        }
    ]
    with pytest.raises(ValueError, match="outside exact denominator"):
        apply_decisions(ledger, decisions)


def test_review_overlay_rejects_unreviewed_candidate_promotion():
    ledger = [_ledger_row("ERIC:EJ1")]
    decisions = [
        {
            "source_identity_reference": "ERIC:EJ1",
            "reviewed": False,
            "decision": "exclude",
            "reason_code": "educationContextMismatch",
            "reviewer_or_process_reference": "model:candidate-only",
        }
    ]
    with pytest.raises(ValueError, match="reviewed=true"):
        apply_decisions(ledger, decisions)


def test_calibration_estimate_uses_reviewed_subset_only():
    ledger = [_ledger_row("ERIC:EJ1"), _ledger_row("ERIC:EJ2"), _ledger_row("ERIC:EJ3")]
    ledger[0]["decision"] = "include"
    ledger[1]["decision"] = "exclude"
    assessments = [
        _assessment("ERIC:EJ1", "exclude"),
        _assessment("ERIC:EJ2", "exclude"),
        _assessment("ERIC:EJ3", "probable"),
    ]

    estimate = calibration_estimate(ledger, assessments)

    assert estimate["reviewed_pair_count"] == 2
    assert estimate["reviewed_positive_count"] == 1
    assert estimate["candidate_false_negative_proxy_n"] == 1
    assert estimate["candidate_false_negative_proxy"] == 1.0
    assert estimate["candidate_review_disagreement_n"] == 1
    assert estimate["estimate_creates_population_truth"] is False
    assert estimate["estimate_creates_automatic_decision"] is False


def test_explicitly_reviewed_unresolved_is_counted_as_reviewed_pair():
    row = _ledger_row("ERIC:EJ1")
    row["decision"] = "unresolved"
    row["reviewer_or_model_reference"] = "reviewer:human-1"
    row["supersedes_decision_reference"] = "screening-decision:prior"
    assert explicitly_reviewed(row) is True

    estimate = calibration_estimate([row], [_assessment("ERIC:EJ1", "exclude")])
    assert estimate["reviewed_pair_count"] == 1
    assert estimate["confusion_counts"]["unresolved->exclude"] == 1


def test_unreviewed_unresolved_is_not_counted_as_reviewed_pair():
    row = _ledger_row("ERIC:EJ1")
    assert explicitly_reviewed(row) is False

    estimate = calibration_estimate([row], [_assessment("ERIC:EJ1", "exclude")])
    assert estimate["reviewed_pair_count"] == 0


def test_reviewed_ledger_round_trip_preserves_review_receipt_columns(tmp_path: Path):
    ledger = [
        _ledger_row("ERIC:EJ1"),
        _ledger_row("ERIC:EJ2"),
        _ledger_row("ERIC:EJ3"),
    ]
    decisions = [
        {
            "source_identity_reference": "ERIC:EJ1",
            "reviewed": True,
            "decision": "include",
            "reason_code": "potentiallyRelevant",
            "reviewer_or_process_reference": "reviewer:human-1",
            "decision_timestamp": "2026-09-20T00:00:00Z",
        },
        {
            "source_identity_reference": "ERIC:EJ2",
            "reviewed": True,
            "decision": "unresolved",
            "reason_code": "insufficientTitleAbstractEvidence",
            "reviewer_or_process_reference": "reviewer:human-1",
            "decision_timestamp": "2026-09-20T00:01:00Z",
        },
    ]
    output, _ = apply_decisions(ledger, decisions)

    ledger_path = tmp_path / "reviewed.tsv"
    write_tsv(ledger_path, output)
    reread = read_tsv(ledger_path)

    assert len(reread) == len(ledger)
    by_ref = {row["source_identity_reference"]: row for row in reread}
    for ref, expected_decision in (("ERIC:EJ1", "include"), ("ERIC:EJ2", "unresolved")):
        row = by_ref[ref]
        assert row["decision"] == expected_decision
        assert row["reviewer_or_model_reference"] == "reviewer:human-1"
        assert row["supersedes_decision_reference"].startswith("screening-decision:")
        assert row["decision_timestamp"] == decisions[
            0 if ref == "ERIC:EJ1" else 1
        ]["decision_timestamp"]
    assert by_ref["ERIC:EJ3"]["reviewer_or_model_reference"] == "unassigned"
    assert "" == by_ref["ERIC:EJ3"].get("supersedes_decision_reference", "")

    assert explicitly_reviewed(by_ref["ERIC:EJ2"]) is True
    assert explicitly_reviewed(by_ref["ERIC:EJ3"]) is False


def test_screening_resolution_lane_selects_reviewed_unresolved_after_round_trip(
    tmp_path: Path,
):
    ledger = [
        _ledger_row("ERIC:EJ1"),
        _ledger_row("ERIC:EJ2"),
        _ledger_row("ERIC:EJ3"),
    ]
    decisions = [
        {
            "source_identity_reference": "ERIC:EJ2",
            "reviewed": True,
            "decision": "unresolved",
            "reason_code": "insufficientTitleAbstractEvidence",
            "reviewer_or_process_reference": "reviewer:human-1",
        }
    ]
    output, _ = apply_decisions(ledger, decisions)
    ledger_path = tmp_path / "reviewed.tsv"
    write_tsv(ledger_path, output)

    order = list(output[0].keys())
    row_to_map = {
        row["source_identity_reference"]: row
        for row in csv.DictReader(ledger_path.open(newline="", encoding="utf-8"), delimiter="\t")
    }
    rederived = [dict(row_to_map[ref]) for ref in (r["source_identity_reference"] for r in output)]

    resolution_hits = []
    for row in rederived:
        if row["decision"] != "unresolved":
            continue
        if not resolution_explicitly_reviewed(row):
            continue
        if row["reason_code"] not in {
            "insufficientTitleAbstractEvidence",
            "inaccessibleAbstract",
            "requiresFullText",
        }:
            continue
        resolution_hits.append(row["source_identity_reference"])

    assert resolution_hits == ["ERIC:EJ2"]
    assert resolution_explicitly_reviewed(
        next(r for r in rederived if r["source_identity_reference"] == "ERIC:EJ2")
    ) is True
