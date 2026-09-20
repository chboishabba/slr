"""Regressions for the Digital-ESD L1->L3 reviewed-screening and census seam."""

from __future__ import annotations

import csv
import json
import subprocess
import sys
from pathlib import Path


REPO = Path(__file__).resolve().parents[1]


def run_script(script: str, *args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(REPO / "scripts" / script), *args],
        text=True,
        capture_output=True,
        check=check,
        cwd=REPO,
    )


def write_tsv(path: Path, rows: list[dict[str, str]]) -> None:
    fields = list(rows[0].keys())
    with path.open("w", newline="", encoding="utf-8") as fh:
        writer = csv.DictWriter(fh, fieldnames=fields, delimiter="\t")
        writer.writeheader()
        writer.writerows(rows)


def write_jsonl(path: Path, rows: list[dict[str, object]]) -> None:
    with path.open("w", encoding="utf-8") as fh:
        for row in rows:
            fh.write(json.dumps(row, sort_keys=True) + "\n")


def test_census_rejects_triage_status_as_screening_decision(tmp_path: Path) -> None:
    ledger = tmp_path / "ledger.tsv"
    write_tsv(
        ledger,
        [{
            "source_identity_reference": "ERIC:EJ1",
            "triage_status": "verified",
            "title": "Example",
        }],
    )
    output = tmp_path / "census.json"

    result = run_script(
        "census_digital_esd_study_processing.py",
        "--screening-ledger", str(ledger),
        "--output", str(output),
        "--expected-metadata-count", "1",
        check=False,
    )

    assert result.returncode != 0
    assert "invalid/missing decision" in result.stderr
    assert not output.exists()


def test_census_advances_only_with_explicit_stage_receipts(tmp_path: Path) -> None:
    ledger = tmp_path / "ledger.tsv"
    write_tsv(
        ledger,
        [
            {
                "source_identity_reference": "ERIC:EJ1",
                "decision": "include",
                "reviewer_or_model_reference": "reviewer:test",
            },
            {
                "source_identity_reference": "ERIC:EJ2",
                "decision": "exclude",
                "reviewer_or_model_reference": "reviewer:test",
            },
            {
                "source_identity_reference": "ERIC:EJ3",
                "decision": "unresolved",
                "reviewer_or_model_reference": "unassigned",
            },
        ],
    )

    fulltext = tmp_path / "fulltext.tsv"
    write_tsv(
        fulltext,
        [{
            "source_identity_reference": "ERIC:EJ1",
            "status": "verified",
        }],
    )

    handoff = tmp_path / "handoff.jsonl"
    parse = tmp_path / "parse.jsonl"
    review = tmp_path / "review.jsonl"
    audit = tmp_path / "audit.jsonl"
    write_jsonl(handoff, [{
        "source_identity_reference": "ERIC:EJ1",
        "handoff_status": "pending-slr-evidence",
    }])
    write_jsonl(parse, [{
        "source_identity_reference": "ERIC:EJ1",
        "parsed": True,
    }])
    write_jsonl(review, [{
        "source_identity_reference": "ERIC:EJ1",
        "reviewed": True,
        "review_ref": "review:EJ1",
    }])
    write_jsonl(audit, [{
        "source_identity_reference": "ERIC:EJ1",
        "source_audit_admission_complete": True,
    }])

    output = tmp_path / "census.json"
    run_script(
        "census_digital_esd_study_processing.py",
        "--screening-ledger", str(ledger),
        "--fulltext-index", str(fulltext),
        "--slr-handoff", str(handoff),
        "--slr-parse-receipts", str(parse),
        "--slr-review-receipts", str(review),
        "--source-audit-receipts", str(audit),
        "--output", str(output),
        "--expected-metadata-count", "3",
    )

    census = json.loads(output.read_text(encoding="utf-8"))
    assert census["metadata_records"] == 3
    assert census["genuinely_screened_records"] == 2
    assert census["include_probable_eligible_for_materialisation"] == 1
    assert census["verified_fulltext_artifacts"] == 1
    assert census["handed_to_slr"] == 1
    assert census["successfully_parsed_by_slr"] == 1
    assert census["reviewed_canonical_evidence"] == 1
    assert census["source_audit_admission_complete"] == 1
    assert census["denominator_integrity"] is True
    assert census["later_stage_inference_used"] is False


def test_review_packet_overlay_template_is_not_authoritative(tmp_path: Path) -> None:
    root = tmp_path / "artifacts"
    root.mkdir()

    ledger = root / "screening_ledger.tsv"
    write_tsv(
        ledger,
        [{
            "source_identity_reference": "ERIC:EJ1",
            "eric_accession": "EJ1",
            "metadata_revision_reference": "meta:EJ1",
            "metadata_sha256": "abc",
            "title_abstract_snapshot_reference": "snapshot:EJ1",
            "title_abstract_snapshot_sha256": "def",
            "title": "Digital learning for sustainability",
            "abstract": "A study of education for sustainable development.",
            "authors": "Example, A.",
            "subjects": "Education",
            "publication_date": "2025",
            "journal": "Example",
            "publication_type": "Journal Article",
            "language": "en",
            "query_memberships": "Q1",
            "decision": "unresolved",
            "decision_reference": "decision:EJ1",
        }],
    )

    write_jsonl(root / "candidate_assessments.jsonl", [{
        "source_identity_reference": "ERIC:EJ1",
        "assessment_reference": "assessment:EJ1",
        "candidate_decision": "probable",
        "candidate_reason_codes": ["potentiallyRelevant"],
        "confidence_reference": "high-candidate-relevance",
        "margin_reference": "4",
        "feature_evidence": {"digital_hits": ["digital learning"]},
    }])
    write_jsonl(root / "calibration_selection.jsonl", [{
        "source_identity_reference": "ERIC:EJ1",
        "selection_creates_decision": False,
    }])
    write_jsonl(root / "screening_pareto_queue.jsonl", [{
        "source_identity_reference": "ERIC:EJ1",
        "pareto_front": True,
        "selection_creates_screening_decision": False,
        "selection_creates_exclusion": False,
    }])

    run_script(
        "prepare_digital_esd_review_packets.py",
        "--artifact-dir", str(root),
        "--max-packets", "1",
    )

    overlay_path = root / "review" / "screening_decision_overlay_template.jsonl"
    overlay = json.loads(overlay_path.read_text(encoding="utf-8").strip())
    assert overlay["reviewed"] is False
    assert overlay["decision"] is None
    assert overlay["candidate_auto_promoted"] is False

    applied = tmp_path / "applied.tsv"
    result = run_script(
        "apply_digital_esd_screening_decisions.py",
        "--ledger", str(ledger),
        "--decisions", str(overlay_path),
        "--output", str(applied),
        check=False,
    )
    assert result.returncode != 0
    assert "reviewed=true" in result.stderr
    assert not applied.exists()
