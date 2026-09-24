"""Integration regressions for live Digital-ESD retrieval -> ESD-4 handoff."""
from __future__ import annotations

import csv
import hashlib
import json
from pathlib import Path

from interop_scripts.digital_esd.scholarly_fulltext import prepare as prepare_scholarly
from interop_scripts.digital_esd_fulltext_cache import FullTextCacheWrapper
from scripts.prepare_digital_esd_fulltext_index import DigitalESDFulltextIndexer


def _write_tsv(path: Path, rows: list[dict[str, str]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fields = list(rows[0])
    with path.open("w", newline="", encoding="utf-8") as fh:
        writer = csv.DictWriter(fh, fieldnames=fields, delimiter="\t")
        writer.writeheader()
        writer.writerows(rows)


def _write_jsonl(path: Path, rows: list[dict]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as fh:
        for row in rows:
            fh.write(json.dumps(row, sort_keys=True) + "\n")


def _ledger_row(ref: str, decision: str) -> dict[str, str]:
    return {
        "source_identity_reference": ref,
        "decision": decision,
        "decision_reference": f"decision:{ref}",
        "metadata_revision_reference": f"metadata:{ref}",
    }


def test_fulltext_index_emits_only_authoritative_include_probable_worklist(
    tmp_path: Path,
):
    ledger = tmp_path / "screening.tsv"
    _write_tsv(
        ledger,
        [
            _ledger_row("ERIC:EJ000001", "include"),
            _ledger_row("ERIC:EJ000002", "probable"),
            _ledger_row("ERIC:EJ000003", "exclude"),
            _ledger_row("ERIC:EJ000004", "unresolved"),
        ],
    )
    out = tmp_path / "fulltext"
    result = DigitalESDFulltextIndexer(
        ledger_path=ledger,
        output_dir=out,
    ).run()

    rows = [
        json.loads(line)
        for line in Path(result["worklist_reference"]).read_text().splitlines()
    ]
    assert [r["source_identity_reference"] for r in rows] == [
        "ERIC:EJ000001",
        "ERIC:EJ000002",
    ]
    assert result["eligible_for_fulltext"] == 2
    assert result["worklist_count"] == 2
    assert all(r["creates_source_audit_admission"] is False for r in rows)


def test_cache_register_preserves_retrieval_digest_and_revision_for_scholarly_prepare(
    tmp_path: Path,
):
    fulltext = tmp_path / "fulltext"
    cache_dir = fulltext / "cache"
    cache_dir.mkdir(parents=True)
    worklist = fulltext / "fulltext-worklist.jsonl"
    _write_jsonl(
        worklist,
        [{
            "source_identity_reference": "ERIC:EJ000001",
            "decision": "include",
            "decision_reference": "decision:1",
        }],
    )

    artifact = cache_dir / "ERIC:EJ000001.pdf"
    artifact.write_bytes(b"%PDF-1.4\nfixture scholarly paper\n")
    digest = hashlib.sha256(artifact.read_bytes()).hexdigest()
    _write_jsonl(
        fulltext / "retrieved-manifest.jsonl",
        [{
            "source_identity_reference": "ERIC:EJ000001",
            "artifact_path": str(artifact),
            "sha256": digest,
            "source_revision_reference": f"fulltext-sha256:{digest}",
            "retrieval_reference": "https://files.eric.ed.gov/fulltext/EJ000001.pdf",
        }],
    )

    wrapper = FullTextCacheWrapper(
        cache_dir=cache_dir,
        worklist=worklist,
        priority_queue=tmp_path / "missing-priority.jsonl",
        max_items=20,
        max_cache_gib=1,
        reserve_gib=0,
        planning_size_mb=1,
    )
    plan = wrapper.plan()
    assert plan["eligible_for_fetch"] is True

    registered = wrapper.register()
    assert registered["registered_count"] == 1
    row = registered["registered"][0]
    assert row["content_sha256"] == digest
    assert row["source_revision_reference"] == f"fulltext-sha256:{digest}"
    assert row["retrieval_receipt_observed"] is True

    handoff = wrapper.handoff()
    assert handoff["retained_for_slr"] == 1
    handoff_path = fulltext / "fulltext-handoff-manifest.jsonl"
    requests_path = fulltext / "scholarly-requests.jsonl"
    prepared = prepare_scholarly(handoff_path, requests_path, verify_files=True)
    assert prepared["prepared_count"] == 1
    request = json.loads(requests_path.read_text().splitlines()[0])
    assert request["content_sha256"] == digest
    assert request["source_revision_reference"] == f"fulltext-sha256:{digest}"
    assert request["candidate_only"] is True


def test_cache_registration_fails_closed_on_retrieval_digest_mismatch(tmp_path: Path):
    fulltext = tmp_path / "fulltext"
    cache_dir = fulltext / "cache"
    cache_dir.mkdir(parents=True)
    worklist = fulltext / "fulltext-worklist.jsonl"
    _write_jsonl(
        worklist,
        [{
            "source_identity_reference": "ERIC:EJ000002",
            "decision": "include",
            "decision_reference": "decision:2",
        }],
    )
    artifact = cache_dir / "ERIC:EJ000002.pdf"
    artifact.write_bytes(b"%PDF-actual")
    _write_jsonl(
        fulltext / "retrieved-manifest.jsonl",
        [{
            "source_identity_reference": "ERIC:EJ000002",
            "artifact_path": str(artifact),
            "sha256": "0" * 64,
            "source_revision_reference": "fulltext-sha256:" + "0" * 64,
            "retrieval_reference": "fixture",
        }],
    )

    wrapper = FullTextCacheWrapper(
        cache_dir=cache_dir,
        worklist=worklist,
        priority_queue=tmp_path / "missing.jsonl",
        max_items=20,
        max_cache_gib=1,
        reserve_gib=0,
        planning_size_mb=1,
    )
    result = wrapper.register()
    assert result["registered_count"] == 0
    assert result["rejected_count"] == 1
    assert result["rejected"][0]["reason"] == "retrieval-receipt-digest-mismatch"
