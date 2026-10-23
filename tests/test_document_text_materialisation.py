"""Regressions for generic document -> canonical text materialisation."""

from __future__ import annotations

import hashlib
import json
import zipfile
from pathlib import Path

import pytest

import interop_scripts.document_text as dt


def make_docx(path: Path) -> None:
    xml = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Digital education study</w:t></w:r></w:p>
    <w:p><w:r><w:t>Sample of 120 students used online learning.</w:t></w:r></w:p>
  </w:body>
</w:document>"""
    with zipfile.ZipFile(path, "w") as zf:
        zf.writestr("word/document.xml", xml)


def test_direct_text_retains_source_and_derived_digests(tmp_path: Path) -> None:
    path = tmp_path / "study.txt"
    path.write_text("alpha\nbeta\n", encoding="utf-8")

    result = dt.materialise_document_text(path)

    assert result.source_artifact_sha256 == hashlib.sha256(path.read_bytes()).hexdigest()
    assert result.extracted_text_sha256 == hashlib.sha256(result.extracted_text.encode()).hexdigest()
    assert result.extraction_engine == "utf8-direct"
    assert result.candidate_only is True
    assert result.creates_semantic_authority is False
    assert result.claim_truth_promoted is False
    assert result.creates_source_audit_admission is False


def test_digest_mismatch_fails_closed(tmp_path: Path) -> None:
    path = tmp_path / "study.txt"
    path.write_text("alpha", encoding="utf-8")

    with pytest.raises(dt.DocumentTextMaterialisationError, match="digest mismatch"):
        dt.materialise_document_text(path, expected_source_sha256="0" * 64)


def test_docx_materialises_paragraphs_without_semantic_promotion(tmp_path: Path) -> None:
    path = tmp_path / "study.docx"
    make_docx(path)

    result = dt.materialise_document_text(path)

    assert result.source_format == "docx"
    assert result.extraction_engine == "ooxml-document.xml"
    assert result.paragraph_count == 2
    assert "Digital education study" in result.extracted_text
    assert len(result.anchors) == 2
    assert result.anchors[1].paragraph_number == 2
    assert result.creates_semantic_authority is False


def test_pdf_uses_derived_text_engine_and_keeps_page_anchors(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    path = tmp_path / "study.pdf"
    path.write_bytes(b"%PDF-fixture")

    def fake_pypdf(_path: Path):
        text, anchors = dt._anchors_from_pages([
            "Page one study population.",
            "Page two outcome results.",
        ])
        return text, "pypdf", "fixture", anchors, 2

    monkeypatch.setattr(dt, "_pdf_with_pypdf", fake_pypdf)

    result = dt.materialise_document_text(path, preferred_pdf_engine="pypdf")

    assert result.source_format == "pdf"
    assert result.page_count == 2
    assert result.extraction_engine == "pypdf"
    assert [a.page_number for a in result.anchors] == [1, 2]
    assert result.extracted_text != path.read_bytes().decode("utf-8", errors="replace")


def test_empty_pdf_extraction_fails_closed_without_implicit_ocr(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    path = tmp_path / "scan.pdf"
    path.write_bytes(b"%PDF-scan")

    def empty(_path: Path):
        text, anchors = dt._anchors_from_pages([""])
        return text, "pypdf", "fixture", anchors, 1

    monkeypatch.setattr(dt, "_pdf_with_pypdf", empty)
    monkeypatch.setattr(dt, "_pdf_with_fitz", lambda _path: (_ for _ in ()).throw(ImportError("no fitz")))
    monkeypatch.setattr(dt, "_pdf_with_pdftotext", lambda _path: (_ for _ in ()).throw(ImportError("no pdftotext")))

    with pytest.raises(dt.DocumentTextMaterialisationError, match="OCR not implicitly permitted"):
        dt.materialise_document_text(path, preferred_pdf_engine="pypdf")
