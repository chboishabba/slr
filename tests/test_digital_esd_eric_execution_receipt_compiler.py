"""Tests for the Digital-ESD real ERIC execution receipt compiler."""
from __future__ import annotations

import hashlib
import json
import tempfile
from pathlib import Path

import pytest

from interop_scripts.emit_digital_esd_eric_execution_agda import emit_agda


EXPECTED_OCCURRENCES = 46597
EXPECTED_UNIQUE = 43996
ARTIFACT_NAMES = (
    "parsed_metadata_corpus",
    "parser_manifest",
    "screening_ledger",
    "candidate_assessment",
    "pareto_queue",
)


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _make_receipt(tmp: Path, overrides: dict | None = None) -> tuple[dict, dict[str, Path]]:
    artifacts: dict[str, Path] = {}
    for name in ARTIFACT_NAMES:
        path = tmp / f"{name}.txt"
        path.write_text(f"artifact:{name}\n", encoding="utf-8")
        artifacts[name] = path

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
        "screening_ledger_all_unresolved": True,
        "scalar_screening_score_used": False,
        "creates_screening_decision": False,
        "creates_source_truth": False,
        "creates_source_audit_admission": False,
        "counts_match": True,
        "artifact_paths": {name: str(path) for name, path in artifacts.items()},
        "artifact_hashes": {name: sha256_file(path) for name, path in artifacts.items()},
    }
    if overrides:
        receipt.update(overrides)
    return receipt, artifacts


def _write_receipt(tmp: Path, receipt: dict) -> Path:
    path = tmp / "receipt.json"
    path.write_text(json.dumps(receipt), encoding="utf-8")
    return path


def test_valid_receipt_emits_agda():
    with tempfile.TemporaryDirectory() as td:
        tmp = Path(td)
        receipt, _ = _make_receipt(tmp)
        receipt_path = _write_receipt(tmp, receipt)
        output_path = tmp / "DigitalESDERICStudyExecutionObserved.agda"

        emit_agda(receipt_path, output_path)

        content = output_path.read_text()
        assert "observedRealERICExecution" in content
        assert str(EXPECTED_OCCURRENCES) in content
        assert str(EXPECTED_UNIQUE) in content


def test_count_mismatch_fails():
    with tempfile.TemporaryDirectory() as td:
        tmp = Path(td)
        receipt, _ = _make_receipt(
            tmp,
            {
                "observed_raw_occurrences": 99999,
                "counts_match": False,
            },
        )
        with pytest.raises(ValueError):
            emit_agda(_write_receipt(tmp, receipt), tmp / "out.agda")


def test_non_promotion_violation_fails():
    with tempfile.TemporaryDirectory() as td:
        tmp = Path(td)
        receipt, _ = _make_receipt(tmp, {"creates_screening_decision": True})
        with pytest.raises(ValueError):
            emit_agda(_write_receipt(tmp, receipt), tmp / "out.agda")


def test_full_text_stop_violation_fails():
    with tempfile.TemporaryDirectory() as td:
        tmp = Path(td)
        receipt, _ = _make_receipt(tmp, {"full_text_stop": False})
        with pytest.raises(ValueError):
            emit_agda(_write_receipt(tmp, receipt), tmp / "out.agda")


def test_real_eric_flag_violation_fails():
    with tempfile.TemporaryDirectory() as td:
        tmp = Path(td)
        receipt, _ = _make_receipt(tmp, {"real_eric": False})
        with pytest.raises(ValueError):
            emit_agda(_write_receipt(tmp, receipt), tmp / "out.agda")


def test_missing_artifact_fails_closed():
    with tempfile.TemporaryDirectory() as td:
        tmp = Path(td)
        receipt, artifacts = _make_receipt(tmp)
        artifacts["screening_ledger"].unlink()
        with pytest.raises(ValueError):
            emit_agda(_write_receipt(tmp, receipt), tmp / "out.agda")


def test_artifact_drift_after_receipt_fails_closed():
    with tempfile.TemporaryDirectory() as td:
        tmp = Path(td)
        receipt, artifacts = _make_receipt(tmp)
        receipt_path = _write_receipt(tmp, receipt)

        # Receipt is valid before drift.
        emit_agda(receipt_path, tmp / "before.agda")

        # Mutate an exact bound artifact after the receipt.
        artifacts["pareto_queue"].write_text("drifted\n", encoding="utf-8")

        with pytest.raises(ValueError):
            emit_agda(receipt_path, tmp / "after.agda")


def test_path_hash_key_mismatch_fails_closed():
    with tempfile.TemporaryDirectory() as td:
        tmp = Path(td)
        receipt, _ = _make_receipt(tmp)
        receipt["artifact_paths"].pop("parser_manifest")
        with pytest.raises(ValueError):
            emit_agda(_write_receipt(tmp, receipt), tmp / "out.agda")


def test_non_unresolved_ledger_receipt_fails():
    with tempfile.TemporaryDirectory() as td:
        tmp = Path(td)
        receipt, _ = _make_receipt(tmp, {"screening_ledger_all_unresolved": False})
        with pytest.raises(ValueError):
            emit_agda(_write_receipt(tmp, receipt), tmp / "out.agda")


def test_scalar_screening_score_receipt_fails():
    with tempfile.TemporaryDirectory() as td:
        tmp = Path(td)
        receipt, _ = _make_receipt(tmp, {"scalar_screening_score_used": True})
        with pytest.raises(ValueError):
            emit_agda(_write_receipt(tmp, receipt), tmp / "out.agda")
