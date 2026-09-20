"""Regressions for the real Digital-ESD screening semantics.

These tests pin the authority boundary around the real ERIC parser:
- every parsed ERIC study starts unresolved;
- advisory assessment depends on title/abstract evidence, not run randomness;
- P0-G verifies only an actually retrieved artifact with a matching SHA-256.
"""
from __future__ import annotations

import hashlib
import json
from pathlib import Path

import pytest

from interop_scripts.digital_esd_eric import ERICRecord
from interop_scripts.digital_esd_screening import (
    screening_row_from_eric,
    candidate_assessment_from_screening_row,
)
from scripts.prepare_digital_esd_fulltext_index import DigitalESDFulltextIndexer


def _record(accession: str, title: str, abstract: str) -> ERICRecord:
    return ERICRecord(
        accession=accession,
        title=title,
        abstract=abstract,
        authors=["Example, A."],
        subjects=["Education"],
        publication_date="2025",
        journal="Example Journal",
        publication_type="Journal Article",
        language="en",
        query_memberships=["Q1"],
        source_page=1,
    )


def test_real_eric_record_enters_authoritative_ledger_unresolved():
    row = screening_row_from_eric(
        _record(
            "EJ000001",
            "Digital learning and education for sustainable development",
            "A study of students using digital learning for sustainability education.",
        )
    )
    assert row["source_identity_reference"] == "ERIC:EJ000001"
    assert row["decision"] == "unresolved"
    assert row["reason_code"] == "awaitingScreeningReview"
    assert row["creates_source_truth"] is False
    assert row["creates_source_audit_admission"] is False


def test_candidate_assessment_is_text_evidence_based_and_non_authoritative():
    relevant = screening_row_from_eric(
        _record(
            "EJ000002",
            "Digital learning for sustainability education",
            "This mixed methods study evaluates online learning for education for sustainable development.",
        )
    )
    irrelevant = screening_row_from_eric(
        _record(
            "EJ000003",
            "Nineteenth century handwriting instruction",
            "Historical archival discussion of penmanship in primary schools.",
        )
    )

    a = candidate_assessment_from_screening_row(relevant)
    b = candidate_assessment_from_screening_row(irrelevant)

    assert a["candidate_decision"] in {"probable", "unresolved"}
    assert a["feature_evidence"]["digital_hits"]
    assert a["feature_evidence"]["sustainability_hits"]
    assert b["candidate_decision"] != a["candidate_decision"]
    assert a["candidate_only"] is True
    assert a["creates_screening_decision"] is False
    assert b["creates_screening_decision"] is False


def test_fulltext_gate_requires_real_file_and_matching_digest(tmp_path: Path):
    ledger = tmp_path / "ledger.tsv"
    ledger.write_text(
        "source_identity_reference\tdecision\ttitle\tabstract\tmetadata_sha256\n"
        "ERIC:EJ000004\tinclude\tExample\tAbstract\tmetadatahash\n",
        encoding="utf-8",
    )

    artifact = tmp_path / "paper.txt"
    artifact.write_text("real full text", encoding="utf-8")
    digest = hashlib.sha256(artifact.read_bytes()).hexdigest()

    retrieved = tmp_path / "retrieved.jsonl"
    retrieved.write_text(
        json.dumps(
            {
                "source_identity_reference": "ERIC:EJ000004",
                "artifact_path": str(artifact),
                "sha256": digest,
                "retrieval_reference": "retrieval:test",
            }
        )
        + "\n",
        encoding="utf-8",
    )

    indexer = DigitalESDFulltextIndexer(
        ledger_path=ledger,
        output_dir=tmp_path / "out",
        retrieved_manifest_path=retrieved,
    )
    result = indexer.run()
    assert result["eligible_for_fulltext"] == 1
    assert result["retrieved"] == 1
    assert result["verified"] == 1
    assert result["failed"] == 0
    assert result["pending"] == 0


def test_fulltext_gate_does_not_verify_missing_artifact(tmp_path: Path):
    ledger = tmp_path / "ledger.tsv"
    ledger.write_text(
        "source_identity_reference\tdecision\ttitle\tabstract\tmetadata_sha256\n"
        "ERIC:EJ000005\tinclude\tExample\tAbstract\tmetadatahash\n",
        encoding="utf-8",
    )
    retrieved = tmp_path / "retrieved.jsonl"
    retrieved.write_text("", encoding="utf-8")

    indexer = DigitalESDFulltextIndexer(
        ledger_path=ledger,
        output_dir=tmp_path / "out",
        retrieved_manifest_path=retrieved,
    )
    result = indexer.run()
    assert result["eligible_for_fulltext"] == 1
    assert result["retrieved"] == 0
    assert result["verified"] == 0
    assert result["pending"] == 1
    assert result["failed"] == 0
