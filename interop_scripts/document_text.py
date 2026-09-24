#!/usr/bin/env python3
"""Generic binary/text document -> canonical UTF-8 text materialisation.

This module is intentionally domain-neutral.  It converts retained document
artifacts into derived text plus exact extraction provenance.  It does not
create semantic authority, claim truth, applicability, study truth, or review
payment.

Supported source formats:
  TXT / Markdown / HTML / CSV / TeX  -> direct UTF-8 text materialisation
  DOCX                               -> OOXML paragraph extraction
  PDF                                -> pypdf, PyMuPDF/fitz, or pdftotext

PDF extraction fails closed if no trusted text engine is available.  OCR is not
performed implicitly; scanned/image-only PDFs remain an explicit residual.

The original artifact SHA-256 remains the source-artifact identity.  Extracted
text has its own SHA-256 and MUST NOT replace the original source digest.
"""

from __future__ import annotations

import hashlib
import importlib
import importlib.metadata
import io
import json
import shutil
import subprocess
import tempfile
import zipfile
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any
from xml.etree import ElementTree as ET


TEXT_EXTENSIONS = {".txt", ".md", ".html", ".htm", ".csv", ".tex"}
BINARY_EXTENSIONS = {".pdf", ".docx"}


class DocumentTextMaterialisationError(RuntimeError):
    pass


@dataclass(frozen=True)
class TextAnchor:
    anchor_ref: str
    start_char: int
    end_char: int
    page_number: int | None = None
    paragraph_number: int | None = None


@dataclass(frozen=True)
class DocumentTextMaterialisation:
    source_artifact_path: str
    source_artifact_sha256: str
    source_format: str
    extraction_engine: str
    extraction_engine_version: str
    extracted_text: str
    extracted_text_sha256: str
    character_count: int
    page_count: int | None
    paragraph_count: int | None
    anchors: tuple[TextAnchor, ...]
    candidate_only: bool = True
    creates_semantic_authority: bool = False
    applicability_promoted: bool = False
    claim_truth_promoted: bool = False
    creates_source_audit_admission: bool = False

    def receipt(self) -> dict[str, Any]:
        data = asdict(self)
        data.pop("extracted_text", None)
        data["anchors"] = [asdict(anchor) for anchor in self.anchors]
        return data


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def _version_for_distribution(name: str) -> str:
    try:
        return importlib.metadata.version(name)
    except importlib.metadata.PackageNotFoundError:
        return "unknown"


def _normalise_text(text: str) -> str:
    return text.replace("\r\n", "\n").replace("\r", "\n")


def _anchors_from_pages(pages: list[str]) -> tuple[str, tuple[TextAnchor, ...]]:
    parts: list[str] = []
    anchors: list[TextAnchor] = []
    cursor = 0
    for page_number, page in enumerate(pages, 1):
        text = _normalise_text(page)
        if page_number > 1:
            separator = "\n\f\n"
            parts.append(separator)
            cursor += len(separator)
        start = cursor
        parts.append(text)
        cursor += len(text)
        anchors.append(
            TextAnchor(
                anchor_ref=f"page:{page_number}",
                page_number=page_number,
                start_char=start,
                end_char=cursor,
            )
        )
    return "".join(parts), tuple(anchors)


def _materialise_text(path: Path, source_sha: str) -> DocumentTextMaterialisation:
    text = _normalise_text(path.read_text(encoding="utf-8", errors="strict"))
    return DocumentTextMaterialisation(
        source_artifact_path=str(path),
        source_artifact_sha256=source_sha,
        source_format=path.suffix.lower().lstrip(".") or "text",
        extraction_engine="utf8-direct",
        extraction_engine_version="python",
        extracted_text=text,
        extracted_text_sha256=sha256_bytes(text.encode("utf-8")),
        character_count=len(text),
        page_count=None,
        paragraph_count=None,
        anchors=(),
    )


def _materialise_docx(path: Path, source_sha: str) -> DocumentTextMaterialisation:
    try:
        archive = zipfile.ZipFile(path)
    except zipfile.BadZipFile as exc:
        raise DocumentTextMaterialisationError(f"invalid DOCX zip: {exc}") from exc

    try:
        xml = archive.read("word/document.xml")
    except KeyError as exc:
        raise DocumentTextMaterialisationError("DOCX missing word/document.xml") from exc

    try:
        root = ET.fromstring(xml)
    except ET.ParseError as exc:
        raise DocumentTextMaterialisationError(f"invalid DOCX document.xml: {exc}") from exc

    ns = {"w": "http://schemas.openxmlformats.org/wordprocessingml/2006/main"}
    paragraphs: list[str] = []
    anchors: list[TextAnchor] = []
    cursor = 0

    for paragraph_number, paragraph in enumerate(root.findall(".//w:p", ns), 1):
        chunks = [node.text or "" for node in paragraph.findall(".//w:t", ns)]
        text = "".join(chunks)
        if paragraph_number > 1:
            cursor += 1
        start = cursor
        paragraphs.append(text)
        cursor += len(text)
        anchors.append(
            TextAnchor(
                anchor_ref=f"paragraph:{paragraph_number}",
                paragraph_number=paragraph_number,
                start_char=start,
                end_char=cursor,
            )
        )

    text = "\n".join(paragraphs)
    return DocumentTextMaterialisation(
        source_artifact_path=str(path),
        source_artifact_sha256=source_sha,
        source_format="docx",
        extraction_engine="ooxml-document.xml",
        extraction_engine_version="python-stdlib",
        extracted_text=text,
        extracted_text_sha256=sha256_bytes(text.encode("utf-8")),
        character_count=len(text),
        page_count=None,
        paragraph_count=len(paragraphs),
        anchors=tuple(anchors),
    )


def _pdf_with_pypdf(path: Path) -> tuple[str, str, str, tuple[TextAnchor, ...], int]:
    module = importlib.import_module("pypdf")
    reader = module.PdfReader(str(path))
    pages = [(page.extract_text() or "") for page in reader.pages]
    text, anchors = _anchors_from_pages(pages)
    return text, "pypdf", _version_for_distribution("pypdf"), anchors, len(pages)


def _pdf_with_fitz(path: Path) -> tuple[str, str, str, tuple[TextAnchor, ...], int]:
    module = importlib.import_module("fitz")
    document = module.open(str(path))
    try:
        pages = [page.get_text("text") or "" for page in document]
    finally:
        document.close()
    text, anchors = _anchors_from_pages(pages)
    return text, "pymupdf", _version_for_distribution("PyMuPDF"), anchors, len(pages)


def _pdf_with_pdftotext(path: Path) -> tuple[str, str, str, tuple[TextAnchor, ...], int]:
    executable = shutil.which("pdftotext")
    if executable is None:
        raise ImportError("pdftotext executable not found")

    version = "unknown"
    probe = subprocess.run(
        [executable, "-v"], text=True, capture_output=True, check=False
    )
    version_text = (probe.stderr or probe.stdout).strip().splitlines()
    if version_text:
        version = version_text[0]

    with tempfile.TemporaryDirectory(prefix="slr-pdf-text-") as tmp:
        out = Path(tmp) / "document.txt"
        proc = subprocess.run(
            [executable, "-layout", str(path), str(out)],
            text=True,
            capture_output=True,
            check=False,
        )
        if proc.returncode != 0:
            raise DocumentTextMaterialisationError(
                "pdftotext failed: " + (proc.stderr or proc.stdout).strip()
            )
        text = _normalise_text(out.read_text(encoding="utf-8", errors="strict"))

    pages = text.split("\f")
    if pages and pages[-1] == "":
        pages.pop()
    text, anchors = _anchors_from_pages(pages)
    return text, "pdftotext-layout", version, anchors, len(pages)


def _materialise_pdf(
    path: Path,
    source_sha: str,
    *,
    preferred_engine: str | None = None,
) -> DocumentTextMaterialisation:
    engines = {
        "pypdf": _pdf_with_pypdf,
        "pymupdf": _pdf_with_fitz,
        "pdftotext": _pdf_with_pdftotext,
    }
    order = [preferred_engine] if preferred_engine else []
    order += [name for name in ("pypdf", "pymupdf", "pdftotext") if name not in order]

    errors: list[str] = []
    for name in order:
        if name not in engines:
            raise DocumentTextMaterialisationError(f"unknown PDF engine: {name}")
        try:
            text, engine, version, anchors, page_count = engines[name](path)
        except (ImportError, ModuleNotFoundError, DocumentTextMaterialisationError) as exc:
            errors.append(f"{name}: {exc}")
            continue
        except Exception as exc:
            errors.append(f"{name}: {type(exc).__name__}: {exc}")
            continue

        non_whitespace = sum(1 for ch in text if not ch.isspace())
        if non_whitespace == 0:
            errors.append(f"{name}: extracted no text; OCR may be required")
            continue

        return DocumentTextMaterialisation(
            source_artifact_path=str(path),
            source_artifact_sha256=source_sha,
            source_format="pdf",
            extraction_engine=engine,
            extraction_engine_version=version,
            extracted_text=text,
            extracted_text_sha256=sha256_bytes(text.encode("utf-8")),
            character_count=len(text),
            page_count=page_count,
            paragraph_count=None,
            anchors=anchors,
        )

    raise DocumentTextMaterialisationError(
        "PDF text extraction unavailable or empty; OCR not implicitly permitted. "
        + " | ".join(errors)
    )


def materialise_document_text(
    path: Path,
    *,
    expected_source_sha256: str | None = None,
    preferred_pdf_engine: str | None = None,
) -> DocumentTextMaterialisation:
    if not path.exists() or not path.is_file():
        raise DocumentTextMaterialisationError(f"artifact missing: {path}")

    source_sha = sha256_file(path)
    if expected_source_sha256 is not None:
        expected = expected_source_sha256.lower().removeprefix("sha256:")
        if source_sha != expected:
            raise DocumentTextMaterialisationError(
                f"source artifact digest mismatch: expected={expected} observed={source_sha}"
            )

    ext = path.suffix.lower()
    if ext in TEXT_EXTENSIONS:
        return _materialise_text(path, source_sha)
    if ext == ".docx":
        return _materialise_docx(path, source_sha)
    if ext == ".pdf":
        return _materialise_pdf(path, source_sha, preferred_engine=preferred_pdf_engine)

    raise DocumentTextMaterialisationError(
        f"unsupported document format {ext or '<none>'}: {path}"
    )


def main() -> int:
    import argparse

    parser = argparse.ArgumentParser(description="generic document -> canonical text materialisation")
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--expected-sha256")
    parser.add_argument("--pdf-engine", choices=["pypdf", "pymupdf", "pdftotext"])
    parser.add_argument("--text-output", type=Path, required=True)
    parser.add_argument("--receipt-output", type=Path, required=True)
    args = parser.parse_args()

    result = materialise_document_text(
        args.input,
        expected_source_sha256=args.expected_sha256,
        preferred_pdf_engine=args.pdf_engine,
    )
    args.text_output.parent.mkdir(parents=True, exist_ok=True)
    args.text_output.write_text(result.extracted_text, encoding="utf-8")
    args.receipt_output.parent.mkdir(parents=True, exist_ok=True)
    args.receipt_output.write_text(
        json.dumps(result.receipt(), indent=2, ensure_ascii=False, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(json.dumps(result.receipt(), sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
