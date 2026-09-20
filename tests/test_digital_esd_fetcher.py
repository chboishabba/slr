"""Regressions for the Digital-ESD retrieval fetcher layer.

These tests pin the retrieval boundary:
- every ERIC query family needs an explicit fetch until exported;
- the full-text fetch plan draws only include|probable worklist rows;
- a dry-run fetch writes nothing to disk;
- a fetch with no network backing fails closed (download-failed, no artifact);
- fetched_bytes and cached artifact never acquire source/audit authority.

Without any HTTP transport, these are transport/schema regressions, not
evidence that retrieval is scientifically accurate.
"""
from __future__ import annotations

import json
from pathlib import Path

import pytest

from scripts.digital_esd_fetcher import (
    DEFAULT_CACHE_DIR,
    DEFAULT_EXPORT_ROOT,
    ERICFetcher,
    FullTextFetcher,
)


@pytest.fixture()
def export_root(tmp_path: Path) -> Path:
    return tmp_path / "eric-export"


@pytest.fixture()
def cache_dir(tmp_path: Path) -> Path:
    return tmp_path / "cache"


def _write_jsonl(path: Path, rows: list[dict]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as fh:
        for row in rows:
            fh.write(json.dumps(row, sort_keys=True) + "\n")


def _retained(ref: str, decision: str, decision_reference: str = "DEC:000001") -> dict:
    return {
        "source_identity_reference": ref,
        "decision": decision,
        "decision_reference": decision_reference,
    }


def test_eric_fetcher_plan_lists_seven_query_families(export_root: Path):
    fetcher = ERICFetcher(export_root=export_root)
    plan = fetcher.plan()
    assert len(plan) == 7
    assert [p["query"] for p in plan] == [f"Q{n}" for n in range(1, 8)]
    assert all(p["needs_fetch"] is True for p in plan)


def test_eric_fetcher_plan_skips_exported_families(export_root: Path):
    (export_root / "Q3").mkdir(parents=True, exist_ok=True)
    (export_root / "Q3" / "page-000000.json").write_text("{}", encoding="utf-8")
    fetcher = ERICFetcher(export_root=export_root)
    by_query = {p["query"]: p for p in fetcher.plan()}
    assert by_query["Q3"]["needs_fetch"] is False
    assert by_query["Q1"]["needs_fetch"] is True


def test_eric_dry_run_writes_nothing(export_root: Path):
    fetcher = ERICFetcher(export_root=export_root)
    result = fetcher.fetch_all(dry_run=True)
    assert result["schema"] == "sensiblaw.digital-esd-eric-fetch.v0_1"
    assert result["dry_run"] is True
    assert len(result["results"]) == 7
    assert not any(export_root.rglob("*.json"))


def test_fulltext_plan_selects_only_include_probable(cache_dir: Path, tmp_path: Path):
    worklist = tmp_path / "fulltext-worklist.jsonl"
    _write_jsonl(worklist, [
        _retained("ERIC:EJ000001", "include"),
        _retained("ERIC:EJ000002", "probable"),
        _retained("ERIC:EJ000003", "exclude"),
        _retained("ERIC:EJ000004", "unresolved"),
    ])
    fetcher = FullTextFetcher(cache_dir=cache_dir)
    plan = fetcher.plan(worklist=worklist, max_items=20)
    assert plan["schema"] == "sensiblaw.digital-esd-fulltext-fetch-plan.v0_1"
    assert plan["selected_refs"] == ["ERIC:EJ000001", "ERIC:EJ000002"]
    assert plan["selected_count"] == 2


def test_fulltext_plan_respects_max_items_and_priority(cache_dir: Path, tmp_path: Path):
    worklist = tmp_path / "fulltext-worklist.jsonl"
    priority_queue = tmp_path / "screening-pareto-queue.jsonl"
    _write_jsonl(worklist, [
        _retained("ERIC:EJ000010", "include"),
        _retained("ERIC:EJ000011", "include"),
        _retained("ERIC:EJ000012", "probable"),
        _retained("ERIC:EJ000013", "exclude"),
    ])
    _write_jsonl(priority_queue, [
        {"source_identity_reference": "ERIC:EJ000012"},
        {"source_identity_reference": "ERIC:EJ000010"},
    ])
    fetcher = FullTextFetcher(cache_dir=cache_dir)
    plan = fetcher.plan(worklist=worklist, priority_queue=priority_queue, max_items=2)
    assert plan["selected_refs"] == ["ERIC:EJ000012", "ERIC:EJ000010"]


def test_fulltext_dry_run_writes_no_artifacts(cache_dir: Path, tmp_path: Path):
    worklist = tmp_path / "fulltext-worklist.jsonl"
    _write_jsonl(worklist, [_retained("ERIC:EJ000001", "include")])
    fetcher = FullTextFetcher(cache_dir=cache_dir)
    plan = fetcher.plan(worklist=worklist, max_items=20)
    result = fetcher.fetch(plan, dry_run=True)
    assert result["schema"] == "sensiblaw.digital-esd-fulltext-fetch.v0_1"
    assert result["dry_run"] is True
    assert result["downloaded_count"] == 0
    assert result["results"][0]["status"] == "dry-run"
    assert not cache_dir.exists() or not list(cache_dir.rglob("*"))


def test_fulltext_real_fetch_fails_closed_without_transport(cache_dir: Path, tmp_path: Path):
    worklist = tmp_path / "fulltext-worklist.jsonl"
    _write_jsonl(worklist, [_retained("ERIC:EJ000001", "include")])
    fetcher = FullTextFetcher(cache_dir=cache_dir)
    plan = fetcher.plan(worklist=worklist, max_items=20)
    result = fetcher.fetch(plan, dry_run=False)
    assert result["downloaded_count"] == 0
    assert result["failed_count"] == 1
    assert result["total_bytes"] == 0
    assert result["results"][0]["status"] == "download-failed"
    assert not list((cache_dir).glob("*.pdf"))


def test_fetch_never_claims_source_truth(cache_dir: Path, tmp_path: Path):
    worklist = tmp_path / "fulltext-worklist.jsonl"
    _write_jsonl(worklist, [_retained("ERIC:EJ000001", "include")])
    fetcher = FullTextFetcher(cache_dir=cache_dir)
    plan = fetcher.plan(worklist=worklist, max_items=20)
    result = fetcher.fetch(plan, dry_run=True)
    assert "creates_source_truth" not in result or result.get("creates_source_truth") is False