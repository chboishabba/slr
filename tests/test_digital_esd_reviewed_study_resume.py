from __future__ import annotations

import hashlib
import json
from pathlib import Path

from interop_scripts.digital_esd_fulltext_cache import FullTextCacheWrapper, write_jsonl
from scripts.digital_esd_fetcher import FullTextFetcher
from scripts.run_digital_esd_reviewed_study_ingestion import merge_parsed_receipts


def _write_jsonl(path: Path, rows: list[dict]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as fh:
        for row in rows:
            fh.write(json.dumps(row, sort_keys=True) + "\n")


def test_planner_skips_cumulatively_parsed_but_keeps_retrieved_unparsed(tmp_path: Path) -> None:
    fulltext = tmp_path / "fulltext"
    cache = fulltext / "cache"
    worklist = fulltext / "fulltext-worklist.jsonl"
    queue = tmp_path / "screening-pareto-queue.jsonl"

    refs = ["ERIC:EJ1", "ERIC:EJ2", "ERIC:EJ3"]
    _write_jsonl(
        worklist,
        [
            {
                "source_identity_reference": ref,
                "decision": "probable",
                "decision_reference": f"decision:{ref}",
            }
            for ref in refs
        ],
    )
    _write_jsonl(
        queue,
        [{"source_identity_reference": ref} for ref in refs],
    )
    _write_jsonl(
        fulltext / "parsed-manifest.jsonl",
        [
            {
                "source_identity_reference": "ERIC:EJ1",
                "verified": True,
                "source_revision_reference": "fulltext-sha256:one",
            }
        ],
    )
    _write_jsonl(
        fulltext / "retrieved-manifest.jsonl",
        [
            {
                "source_identity_reference": "ERIC:EJ2",
                "sha256": "0" * 64,
            }
        ],
    )

    plan = FullTextCacheWrapper(
        cache_dir=cache,
        worklist=worklist,
        priority_queue=queue,
        max_items=20,
        max_cache_gib=1,
        reserve_gib=0,
        planning_size_mb=1,
    ).plan()

    selected = [row["source_identity_reference"] for row in plan["batch"]]
    assert "ERIC:EJ1" not in selected
    assert selected == ["ERIC:EJ2", "ERIC:EJ3"]
    assert plan["already_parsed_count"] == 1
    assert plan["remaining_unparsed_count"] == 2
    assert plan["already_retrieved_unparsed_count"] == 1


def test_fetcher_reuses_matching_retained_artifact_with_network_disabled(tmp_path: Path) -> None:
    fulltext = tmp_path / "fulltext"
    cache = fulltext / "cache"
    cache.mkdir(parents=True)

    ref = "ERIC:EJ123"
    artifact = cache / f"{ref}.pdf"
    payload = b"%PDF-1.4\nfixture\n"
    artifact.write_bytes(payload)
    digest = hashlib.sha256(payload).hexdigest()

    _write_jsonl(
        fulltext / "retrieved-manifest.jsonl",
        [
            {
                "source_identity_reference": ref,
                "artifact_path": str(artifact),
                "sha256": digest,
                "retrieval_reference": "https://files.eric.ed.gov/fulltext/EJ123.pdf",
                "retrieval_timestamp": "2026-09-20T00:00:00+00:00",
                "source_revision_reference": f"fulltext-sha256:{digest}",
            }
        ],
    )

    fetcher = FullTextFetcher(
        cache_dir=cache,
        output_dir=fulltext,
        network_enabled=False,
    )
    result = fetcher.fetch(
        {
            "selected_refs": [ref],
            "selected_count": 1,
        }
    )

    assert result["downloaded_count"] == 0
    assert result["cache_hit_count"] == 1
    assert result["failed_count"] == 0
    assert result["results"][0]["status"] == "cache-hit"
    assert result["results"][0]["sha256"] == digest


def test_cumulative_parsed_receipts_survive_later_batches(tmp_path: Path) -> None:
    artifact_root = tmp_path / "artifacts"

    first = {
        "verified": [
            {
                "source_identity_reference": "ERIC:EJ1",
                "source_revision_reference": "fulltext-sha256:one",
                "content_sha256": "1" * 64,
                "document_node_count": 10,
                "study_facet_count": 2,
                "candidate_only": True,
                "creates_semantic_authority": False,
                "applicability_promoted": False,
                "claim_truth_promoted": False,
            }
        ]
    }
    second = {
        "verified": [
            {
                "source_identity_reference": "ERIC:EJ2",
                "source_revision_reference": "fulltext-sha256:two",
                "content_sha256": "2" * 64,
                "document_node_count": 20,
                "study_facet_count": 3,
                "candidate_only": True,
                "creates_semantic_authority": False,
                "applicability_promoted": False,
                "claim_truth_promoted": False,
            }
        ]
    }

    r1 = merge_parsed_receipts(artifact_root, first)
    r2 = merge_parsed_receipts(artifact_root, second)

    assert r1["cumulative_parsed_verified"] == 1
    assert r2["cumulative_parsed_verified"] == 2

    rows = [
        json.loads(line)
        for line in (artifact_root / "fulltext" / "parsed-manifest.jsonl")
        .read_text(encoding="utf-8")
        .splitlines()
        if line.strip()
    ]
    assert [row["source_identity_reference"] for row in rows] == [
        "ERIC:EJ1",
        "ERIC:EJ2",
    ]
    assert all(row["creates_source_truth"] is False for row in rows)
    assert all(row["creates_source_audit_admission"] is False for row in rows)
