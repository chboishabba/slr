"""Real retained-export regressions for the Digital-ESD ERIC parser."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path

import pytest

from interop_scripts.digital_esd_eric import ERICParser


def _write_query(
    root: Path,
    query_num: int,
    pages: list[list[dict]],
    *,
    num_found: int | None = None,
) -> Path:
    qdir = root / f"Q{query_num}"
    qdir.mkdir(parents=True, exist_ok=True)
    summary_pages = []
    total = sum(len(page) for page in pages)
    expected = total if num_found is None else num_found
    for idx, docs in enumerate(pages):
        path = qdir / f"page-{idx:06d}.json"
        payload = {"response": {"numFound": expected, "docs": docs}}
        raw = json.dumps(payload, sort_keys=True)
        path.write_text(raw, encoding="utf-8")
        summary_pages.append(
            {
                "page_index": idx,
                "path": str(path),
                "sha256": hashlib.sha256(raw.encode("utf-8")).hexdigest(),
            }
        )
    (qdir / "summary.json").write_text(
        json.dumps(
            {
                "query_id": f"Q{query_num}",
                "numFound": expected,
                "pages": summary_pages,
            },
            sort_keys=True,
        ),
        encoding="utf-8",
    )
    return qdir


def _doc(accession: str, title: str = "Digital learning for sustainability") -> dict:
    return {
        "id": accession,
        "title": title,
        "author": ["Example, A.", "Example, B."],
        "source": "Example Journal",
        "publicationdateyear": 2025,
        "description": "A mixed methods study of online learning and sustainability education.",
        "subject": ["Educational Technology", "Sustainable Development"],
        "language": ["English"],
        "publicationtype": ["Journal Articles", "Reports - Research"],
    }


def test_paginated_response_docs_are_normalised_and_query_membership_is_preserved(tmp_path: Path):
    _write_query(tmp_path, 1, [[_doc("EJ000001")], [_doc("EJ000002")]])
    _write_query(tmp_path, 2, [[_doc("EJ000001")]])

    parser = ERICParser(tmp_path, expected_queries=2)
    parser.load_all_pages()
    parsed = parser.parse_all_pages()
    unique = parser.deduplicate(parsed)

    by_id = {row.accession: row for row in unique}
    assert len(parsed) == 3
    assert len(unique) == 2
    assert by_id["EJ000001"].query_memberships == ["Q1", "Q2"]
    assert by_id["EJ000001"].abstract.startswith("A mixed methods study")
    assert by_id["EJ000001"].authors == ["Example, A.", "Example, B."]
    assert by_id["EJ000001"].subjects == [
        "Educational Technology",
        "Sustainable Development",
    ]
    assert by_id["EJ000001"].publication_date == "2025"
    assert by_id["EJ000001"].journal == "Example Journal"
    assert by_id["EJ000001"].publication_type == "Journal Articles; Reports - Research"
    assert by_id["EJ000001"].language == "English"


def test_summary_page_digest_drift_fails_closed(tmp_path: Path):
    qdir = _write_query(tmp_path, 1, [[_doc("EJ000003")]])
    page = qdir / "page-000000.json"
    page.write_text('{"response":{"numFound":1,"docs":[]}}', encoding="utf-8")

    parser = ERICParser(tmp_path, expected_queries=1)
    with pytest.raises(ValueError, match="SHA-256 mismatch"):
        parser.load_all_pages()


def test_incomplete_pagination_fails_closed(tmp_path: Path):
    _write_query(tmp_path, 1, [[_doc("EJ000004")]], num_found=2)

    parser = ERICParser(tmp_path, expected_queries=1)
    parser.load_all_pages()
    with pytest.raises(ValueError, match="pagination incomplete"):
        parser.parse_all_pages()


def test_conflicting_same_accession_metadata_fails_closed(tmp_path: Path):
    _write_query(tmp_path, 1, [[_doc("EJ000005", "Title A")]])
    _write_query(tmp_path, 2, [[_doc("EJ000005", "Title B")]])

    parser = ERICParser(tmp_path, expected_queries=2)
    parser.load_all_pages()
    parsed = parser.parse_all_pages()
    with pytest.raises(ValueError, match="conflicting metadata"):
        parser.deduplicate(parsed)


def test_legacy_precombined_entries_remain_supported(tmp_path: Path):
    for q in (1, 2):
        (tmp_path / f"q{q}.json").write_text(
            json.dumps(
                {
                    "entries": [
                        {
                            "accession_id": f"EJ00000{q + 5}",
                            "title": f"Legacy {q}",
                            "abstract": "Legacy retained export fixture.",
                            "authors": ["Legacy, A."],
                            "subjects": ["Education"],
                            "publication_date": "2024",
                            "journal": "Legacy Journal",
                            "publication_type": "Journal Article",
                            "language": "English",
                        }
                    ]
                }
            ),
            encoding="utf-8",
        )

    parser = ERICParser(tmp_path, expected_queries=2)
    parser.load_all_pages()
    parsed = parser.parse_all_pages()
    assert [row.query_memberships for row in parsed] == [["Q1"], ["Q2"]]
