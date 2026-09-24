#!/usr/bin/env python3
"""Generic fail-closed document-to-text adapter.

This is deliberately domain-neutral.  It does not know about Digital-ESD,
legal reasoning, or study semantics.

Supported:
  .txt/.md/.csv/.json/.html/.htm  -> UTF-8 text read
  .pdf                            -> external pdftotext executable

The original artifact remains the source revision.  Extracted text receives
its own SHA-256 and extractor receipt so downstream parsing can prove which
text view it consumed.

If a real extractor is unavailable, the adapter fails closed rather than
binary-decoding a PDF.
"""

from __future__ import annotations

import hashlib
import html
import re
import shutil
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path


class DocumentTextError(RuntimeError):
    pass


@dataclass(frozen=True)
class ExtractedText:
    text: str
    extractor_reference: str
    extracted_text_sha256: str
    source_format: str


def sha256_text(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def _strip_html(raw: str) -> str:
    text = re.sub(r"(?is)<(script|style).*?>.*?</\1>", " ", raw)
    text = re.sub(r"(?s)<[^>]+>", " ", text)
    text = html.unescape(text)
    return "\n".join(line.strip() for line in text.splitlines() if line.strip())


def extract_document_text(path: Path) -> ExtractedText:
    if not path.exists() or not path.is_file():
        raise DocumentTextError(f"artifact-missing:{path}")

    ext = path.suffix.lower()

    if ext in {".txt", ".md", ".csv", ".json"}:
        text = path.read_text(encoding="utf-8", errors="strict")
        return ExtractedText(
            text=text,
            extractor_reference="stdlib:utf8-text:v1",
            extracted_text_sha256=sha256_text(text),
            source_format=ext.removeprefix(".") or "text",
        )

    if ext in {".html", ".htm"}:
        raw = path.read_text(encoding="utf-8", errors="replace")
        text = _strip_html(raw)
        return ExtractedText(
            text=text,
            extractor_reference="stdlib:html-strip:v1",
            extracted_text_sha256=sha256_text(text),
            source_format="html",
        )

    if ext == ".pdf":
        pdftotext = shutil.which("pdftotext")
        if not pdftotext:
            raise DocumentTextError("pdf-text-extractor-unavailable:pdftotext")
        with tempfile.TemporaryDirectory(prefix="slr-pdf-text-") as td:
            out = Path(td) / "document.txt"
            proc = subprocess.run(
                [pdftotext, "-layout", "-enc", "UTF-8", str(path), str(out)],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                check=False,
            )
            if proc.returncode != 0:
                raise DocumentTextError(
                    "pdf-text-extraction-failed:"
                    + (proc.stderr.strip() or f"exit-{proc.returncode}")
                )
            text = out.read_text(encoding="utf-8", errors="strict")
        return ExtractedText(
            text=text,
            extractor_reference="external:pdftotext:-layout:utf8:v1",
            extracted_text_sha256=sha256_text(text),
            source_format="pdf",
        )

    raise DocumentTextError(f"unsupported-document-format:{ext or '<none>'}")
