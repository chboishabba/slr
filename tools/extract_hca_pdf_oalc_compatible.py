#!/usr/bin/env python3
"""Extract one official HCA PDF using the OALC creator's PDF semantics.

This intentionally does *not* crawl or rebuild OALC. The official court PDF is
input/source evidence; the pinned OALC creator revision is only the text
transformation implementation.

Upstream algorithm/reference:
  https://github.com/isaacus-dev/open-australian-legal-corpus-creator
  commit dbdb9e3752394a87a9c966c9a9199f9685db9af2
  src/oalc_creator/ocr.py::pdf2txt
  src/oalc_creator/scrapers/high_court_of_australia.py (PDF scale=2)
Upstream project license: MIT.
"""

from __future__ import annotations

import argparse
import asyncio
import hashlib
import json
import multiprocessing
import re
from concurrent.futures import ThreadPoolExecutor
from io import BytesIO
from pathlib import Path

import pypdfium2
import tesserocr

UPSTREAM_REPO = "https://github.com/isaacus-dev/open-australian-legal-corpus-creator"
UPSTREAM_REVISION = "dbdb9e3752394a87a9c966c9a9199f9685db9af2"
UPSTREAM_PATH = "src/oalc_creator/ocr.py"
UPSTREAM_FUNCTION = "pdf2txt"
PARAGRAPH_NUMBER_PATTERN = r"(^|\n)\d{1,4}[^\S\n]*\n"
PDF_SCALE = 2


def sha256_hex(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def batches(items, size: int):
    for start in range(0, len(items), size):
        yield items[start : start + size]


async def extract_pdf(pdf_bytes: bytes, workers: int | None = None) -> str:
    max_workers = workers or max(multiprocessing.cpu_count() - 1, 1)
    pdf = pypdfium2.PdfDocument(BytesIO(pdf_bytes))
    pages = [pdf[index] for index in range(len(pdf))]
    text: list[str] = []
    loop = asyncio.get_running_loop()
    with ThreadPoolExecutor(max_workers=max_workers) as executor:
        for page_batch in batches(pages, max_workers):
            images = [page.render(scale=PDF_SCALE).to_pil() for page in page_batch]
            text.extend(
                await asyncio.gather(
                    *[
                        loop.run_in_executor(executor, tesserocr.image_to_text, image)
                        for image in images
                    ]
                )
            )
    joined = "\n".join(text)
    return re.sub(PARAGRAPH_NUMBER_PATTERN, "", joined)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("pdf", type=Path)
    parser.add_argument("--text-out", required=True, type=Path)
    parser.add_argument("--receipt-out", required=True, type=Path)
    parser.add_argument("--source-url", required=True)
    parser.add_argument("--citation", required=True)
    parser.add_argument("--expected-source-sha256")
    parser.add_argument("--workers", type=int)
    args = parser.parse_args()

    pdf_bytes = args.pdf.read_bytes()
    source_sha = sha256_hex(pdf_bytes)
    if args.expected_source_sha256 and source_sha != args.expected_source_sha256.lower():
        raise SystemExit(
            f"source SHA-256 mismatch: expected {args.expected_source_sha256.lower()} got {source_sha}"
        )

    text = asyncio.run(extract_pdf(pdf_bytes, args.workers))
    text_bytes = text.encode("utf-8")
    args.text_out.write_bytes(text_bytes)

    receipt = {
        "schema": "sensiblaw-official-primary-extraction/v1",
        "citation": args.citation,
        "source_provider_ref": "provider:official-hca-pdf",
        "source_url": args.source_url,
        "source_media_type": "application/pdf",
        "source_sha256_hex": source_sha,
        "source_size_bytes": len(pdf_bytes),
        "extractor_repo": UPSTREAM_REPO,
        "extractor_revision": UPSTREAM_REVISION,
        "extractor_path": UPSTREAM_PATH,
        "extractor_function": UPSTREAM_FUNCTION,
        "extractor_license": "MIT",
        "parameters": {
            "pdf_scale": PDF_SCALE,
            "paragraph_number_cleanup_regex": PARAGRAPH_NUMBER_PATTERN,
            "ocr_engine": "tesserocr.image_to_text",
            "renderer": "pypdfium2",
        },
        "output_sha256_hex": sha256_hex(text_bytes),
        "output_size_bytes": len(text_bytes),
        "authority": "source-observation-only",
        "creates_semantic_authority": False,
        "creates_legal_authority": False,
        "applicability_promoted": False,
        "claim_truth_promoted": False,
    }
    args.receipt_out.write_text(
        json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(json.dumps(receipt, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
