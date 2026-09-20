"""Tests for the Digital-ESD real ERIC execution receipt compiler.

Tests:
  - valid receipt + matching artifacts → concrete Agda witness emitted
  - artifact changed after receipt → hard failure
"""
from __future__ import annotations

import json
import tempfile
from pathlib import Path

import pytest

import sys
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from interop_scripts.emit_digital_esd_eric_execution_agda import emit_agda


EXPECTED_OCCURRENCES = 46597
EXPECTED_UNIQUE = 43996


def _make_receipt(overrides: dict | None = None) -> dict:
    receipt = {
        "run_id": "test-run-001",
        "started_at": "2026-09-20T00:00:00+00:00",
        "completed_at": "2026-09-20T00:01:00+00:00",
        "expected_raw_occurrences": EXPECTED_OCCURRENCES,
        "observed_raw_occurrences": EXPECTED_OCCURRENCES,
        "expected_unique_records": EXPECTED_UNIQUE,
        "observed_unique_records": EXPECTED_UNIQUE,
        "real_eric": True,
        "full_text_stop": True,
        "creates_screening_decision": False,
        "creates_source_truth": False,
        "creates_source_audit_admission": False,
        "counts_match": True,
        "artifact_hashes": {
            "parsed_metadata_corpus": "abc123",
            "parser_manifest": "def456",
            "screening_ledger": "ghi789",
            "candidate_assessment": "jkl012",
            "pareto_queue": "mno345",
        },
    }
    if overrides:
        receipt.update(overrides)
    return receipt


def test_valid_receipt_emits_agda():
    """Valid receipt + matching artifacts → concrete Agda witness emitted."""
    with tempfile.TemporaryDirectory() as tmp:
        receipt_path = Path(tmp) / "receipt.json"
        output_path = Path(tmp) / "DigitalESDERICStudyExecutionObserved.agda"
        with open(receipt_path, "w") as fh:
            json.dump(_make_receipt(), fh)
        receipt = emit_agda(receipt_path, output_path)
        assert output_path.exists()
        content = output_path.read_text()
        assert "observedRealERICExecution" in content
        assert str(EXPECTED_OCCURRENCES) in content
        assert str(EXPECTED_UNIQUE) in content


def test_artifact_changed_after_receipt():
    """Artifact changed after receipt → hard failure."""
    with tempfile.TemporaryDirectory() as tmp:
        receipt_path = Path(tmp) / "receipt.json"
        output_path = Path(tmp) / "DigitalESDERICStudyExecutionObserved.agda"
        receipt = _make_receipt(overrides={
            "observed_raw_occurrences": 99999,
            "counts_match": False,
        })
        with open(receipt_path, "w") as fh:
            json.dump(receipt, fh)
        with pytest.raises(ValueError):
            emit_agda(receipt_path, output_path)


def test_count_mismatch_fails():
    """Count mismatch → hard failure."""
    with tempfile.TemporaryDirectory() as tmp:
        receipt_path = Path(tmp) / "receipt.json"
        output_path = Path(tmp) / "DigitalESDERICStudyExecutionObserved.agda"
        receipt = _make_receipt(overrides={
            "observed_raw_occurrences": 99999,
            "counts_match": False,
        })
        with open(receipt_path, "w") as fh:
            json.dump(receipt, fh)
        with pytest.raises(ValueError):
            emit_agda(receipt_path, output_path)


def test_non_promotion_violation_fails():
    """Non-promotion boolean violation → hard failure."""
    with tempfile.TemporaryDirectory() as tmp:
        receipt_path = Path(tmp) / "receipt.json"
        output_path = Path(tmp) / "DigitalESDERICStudyExecutionObserved.agda"
        receipt = _make_receipt(overrides={
            "creates_screening_decision": True,
        })
        with open(receipt_path, "w") as fh:
            json.dump(receipt, fh)
        with pytest.raises(ValueError):
            emit_agda(receipt_path, output_path)


def test_full_text_stop_violation_fails():
    """Full-text stop boundary violation → hard failure."""
    with tempfile.TemporaryDirectory() as tmp:
        receipt_path = Path(tmp) / "receipt.json"
        output_path = Path(tmp) / "DigitalESDERICStudyExecutionObserved.agda"
        receipt = _make_receipt(overrides={
            "full_text_stop": False,
        })
        with open(receipt_path, "w") as fh:
            json.dump(receipt, fh)
        with pytest.raises(ValueError):
            emit_agda(receipt_path, output_path)


def test_real_eric_flag_violation_fails():
    """Real ERIC flag violation → hard failure."""
    with tempfile.TemporaryDirectory() as tmp:
        receipt_path = Path(tmp) / "receipt.json"
        output_path = Path(tmp) / "DigitalESDERICStudyExecutionObserved.agda"
        receipt = _make_receipt(overrides={
            "real_eric": False,
        })
        with open(receipt_path, "w") as fh:
            json.dump(receipt, fh)
        with pytest.raises(ValueError):
            emit_agda(receipt_path, output_path)


def test_artifact_drift_after_receipt():
    """Artifact drift after receipt → hard failure."""
    with tempfile.TemporaryDirectory() as tmp:
        receipt_path = Path(tmp) / "receipt.json"
        output_path = Path(tmp) / "DigitalESDERICStudyExecutionObserved.agda"
        receipt = _make_receipt()
        with open(receipt_path, "w") as fh:
            json.dump(receipt, fh)
        # First call should succeed
        emit_agda(receipt_path, output_path)
        assert output_path.exists()
        # Now simulate artifact drift by changing the receipt
        drift_receipt = _make_receipt(overrides={
            "observed_raw_occurrences": 99999,
        })
        with open(receipt_path, "w") as fh:
            json.dump(drift_receipt, fh)
        with pytest.raises(ValueError):
            emit_agda(receipt_path, output_path)
