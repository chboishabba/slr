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
    load_eric_query_config,
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
    assert result["schema"] == "sensiblaw.digital-esd-eric-fetch.v0_2"
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
    assert plan["schema"] == "sensiblaw.digital-esd-fulltext-fetch-plan.v0_2"
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


def test_fulltext_real_fetch_fails_closed_without_live_network(cache_dir: Path, tmp_path: Path):
    worklist = tmp_path / "fulltext-worklist.jsonl"
    _write_jsonl(worklist, [_retained("ERIC:EJ000001", "include")])
    fetcher = FullTextFetcher(cache_dir=cache_dir)
    plan = fetcher.plan(worklist=worklist, max_items=20)
    result = fetcher.fetch(plan, dry_run=False)
    assert result["downloaded_count"] == 0
    assert result["failed_count"] == 1
    assert result["total_bytes"] == 0
    assert result["results"][0]["status"] == "network-disabled"
    assert not list((cache_dir).glob("*.pdf"))


def test_fetch_never_claims_source_truth(cache_dir: Path, tmp_path: Path):
    worklist = tmp_path / "fulltext-worklist.jsonl"
    _write_jsonl(worklist, [_retained("ERIC:EJ000001", "include")])
    fetcher = FullTextFetcher(cache_dir=cache_dir)
    plan = fetcher.plan(worklist=worklist, max_items=20)
    result = fetcher.fetch(plan, dry_run=True)
    assert "creates_source_truth" not in result or result.get("creates_source_truth") is False


def test_eric_query_config_is_exact_seven_family_surface():
    config = load_eric_query_config()
    families = config["query_families"]
    assert list(sorted(families)) == [f"Q{n}" for n in range(1, 8)]
    assert '"education for sustainable development"' in families["Q1"]
    assert '"life cycle assessment"' in families["Q4"]
    assert '"right to repair"' in families["Q7"]


def test_eric_live_mock_writes_page_hashes_and_complete_summary(
    export_root: Path,
    monkeypatch: pytest.MonkeyPatch,
):
    fetcher = ERICFetcher(
        export_root=export_root,
        network_enabled=True,
        request_interval_seconds=0,
    )

    payload = {
        "response": {
            "numFound": 2,
            "docs": [
                {"id": "EJ900001", "title": "One"},
                {"id": "EJ900002", "title": "Two"},
            ],
        }
    }

    def fake_page(query: str, *, start: int):
        assert start == 0
        assert query == fetcher._query(1)
        return payload, "https://api.ies.ed.gov/eric/?fixture=1"

    monkeypatch.setattr(fetcher, "_request_page", fake_page)
    result = fetcher.fetch(1)
    assert result["status"] == "fetched"
    assert result["docs_fetched"] == 2
    summary = json.loads((export_root / "Q1" / "summary.json").read_text())
    assert summary["pagination_complete"] is True
    assert summary["numFound"] == 2
    assert summary["api_key_used"] is False
    assert len(summary["pages"]) == 1
    assert len(summary["pages"][0]["sha256"]) == 64
    assert (export_root / "Q1" / "page-000000.json").exists()


def test_fulltext_live_mock_persists_p0g_retrieved_manifest(
    cache_dir: Path,
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
):
    worklist = tmp_path / "fulltext-worklist.jsonl"
    _write_jsonl(worklist, [_retained("ERIC:EJ000001", "include")])
    fetcher = FullTextFetcher(
        cache_dir=cache_dir,
        output_dir=tmp_path / "fulltext",
        network_enabled=True,
        request_interval_seconds=0,
    )

    def fake_download(url: str, path: Path, retries: int) -> bool:
        assert url == "https://files.eric.ed.gov/fulltext/EJ000001.pdf"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(b"%PDF-fixture")
        return True

    monkeypatch.setattr(fetcher, "_download_artifact", fake_download)
    plan = fetcher.plan(worklist=worklist, max_items=20)
    result = fetcher.fetch(plan)
    assert result["downloaded_count"] == 1
    manifest_path = Path(result["retrieved_manifest_path"])
    rows = [json.loads(line) for line in manifest_path.read_text().splitlines()]
    assert rows[0]["source_identity_reference"] == "ERIC:EJ000001"
    assert len(rows[0]["sha256"]) == 64
    assert rows[0]["source_revision_reference"].startswith("fulltext-sha256:")
    assert rows[0]["creates_source_audit_admission"] is False


def test_fulltext_external_url_requires_explicit_allowlist(
    cache_dir: Path,
    tmp_path: Path,
):
    worklist = tmp_path / "fulltext-worklist.jsonl"
    url_map = tmp_path / "urls.jsonl"
    _write_jsonl(worklist, [_retained("ERIC:EJ000001", "include")])
    _write_jsonl(url_map, [{
        "source_identity_reference": "ERIC:EJ000001",
        "url": "https://publisher.example/paper.pdf",
    }])
    fetcher = FullTextFetcher(
        cache_dir=cache_dir,
        network_enabled=True,
        url_map=url_map,
        request_interval_seconds=0,
    )
    plan = fetcher.plan(worklist=worklist, max_items=20)
    result = fetcher.fetch(plan)
    assert result["downloaded_count"] == 0
    assert result["results"][0]["status"] == "host-not-allowlisted"
