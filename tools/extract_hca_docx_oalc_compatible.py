#!/usr/bin/env python3
"""Extract one official HCA DOCX using the pinned OALC creator semantics.

This intentionally does not crawl or rebuild OALC. The official court DOCX is
the source manifestation; the pinned OALC creator revision is only the text
transformation implementation.

Upstream algorithm/reference:
  https://github.com/isaacus-dev/open-australian-legal-corpus-creator
  commit dbdb9e3752394a87a9c966c9a9199f9685db9af2
  src/oalc_creator/custom_mammoth.py::docx2html
  src/oalc_creator/custom_inscriptis.py::{CustomInscriptis,CustomParserConfig}
  src/oalc_creator/scrapers/high_court_of_australia.py (DOCX branch)
Upstream project license: MIT.
"""

from __future__ import annotations

import argparse
import hashlib
import importlib.metadata
import json
from contextlib import suppress
from io import BytesIO
from pathlib import Path

import lxml.html
import mammoth
from inscriptis import Inscriptis
from inscriptis.css_profiles import CSS_PROFILES
from inscriptis.html_properties import Display
from inscriptis.model.attribute import Attribute
from inscriptis.model.config import ParserConfig
from inscriptis.model.css import CssParse
from inscriptis.model.html_element import HtmlElement

UPSTREAM_REPO = "https://github.com/isaacus-dev/open-australian-legal-corpus-creator"
UPSTREAM_REVISION = "dbdb9e3752394a87a9c966c9a9199f9685db9af2"
UPSTREAM_PATH = "src/oalc_creator/scrapers/high_court_of_australia.py"
UPSTREAM_FUNCTION = "docx2html+CustomInscriptis"
DOCX2HTML_PATH = "src/oalc_creator/custom_mammoth.py"
INSCRIPTIS_PATH = "src/oalc_creator/custom_inscriptis.py"
DOCX_MEDIA_TYPE = "application/vnd.openxmlformats-officedocument.wordprocessingml.document"


def sha256_hex(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def dummy_image_converter(_):
    return {}


def docx2html(doc: BytesIO):
    # Mirrors oalc_creator.custom_mammoth.docx2html at UPSTREAM_REVISION.
    return mammoth.convert_to_html(doc, convert_image=dummy_image_converter)


class CustomInscriptis(Inscriptis):
    def _start_ol(self, ol: dict) -> None:
        self.li_counter.append(int(ol.get("start", 1)))


class CustomCssParse(CssParse):
    @staticmethod
    def attr_style(style_attribute: str, html_element: HtmlElement) -> None:
        for directive in style_attribute.lower().split(";"):
            if ":" not in directive:
                continue
            key, value = (part.strip() for part in directive.split(":", 1))
            try:
                apply_style = getattr(
                    CustomCssParse,
                    "attr_" + key.replace("-webkit-", "").replace("-", "_"),
                )
                apply_style(value, html_element)
            except AttributeError:
                pass

    @staticmethod
    def attr_margin_left(value: str, html_element: HtmlElement) -> None:
        with suppress(ValueError):
            html_element.padding_inline += CssParse._get_em(value)

    @staticmethod
    def attr_padding_left(value: str, html_element: HtmlElement) -> None:
        with suppress(ValueError):
            html_element.padding_inline += CssParse._get_em(value)

    @staticmethod
    def attr_class(classes: str, html_element: HtmlElement) -> None:
        if html_element.tag not in {"p", "div", "li"} or not classes:
            return
        for class_name in classes.split(" "):
            if any(token in class_name for token in {"Head", "Title", "heading"}):
                html_element.display = Display.block
                html_element.margin_before = 1
                html_element.margin_after = 0
                break


class CustomAttribute(Attribute):
    def __init__(self) -> None:
        super().__init__()
        self.attribute_mapping = {
            "style": CustomCssParse.attr_style,
            "align": CustomCssParse.attr_horizontal_align,
            "valign": CustomCssParse.attr_vertical_align,
            "class": CustomCssParse.attr_class,
        }


class CustomParserConfig(ParserConfig):
    def __init__(self, css):
        super().__init__(css, False, False, False, False, None, "  ")
        self.attribute_handler = CustomAttribute()


def extract_docx(docx_bytes: bytes) -> str:
    html = docx2html(BytesIO(docx_bytes))
    etree = lxml.html.fromstring(html.value)
    profile = CSS_PROFILES["strict"].copy()
    profile |= dict.fromkeys(
        ("h1", "h2", "h3", "h4", "h5"),
        HtmlElement(display=Display.block, margin_before=1),
    )
    config = CustomParserConfig(profile)
    return CustomInscriptis(etree, config).get_text()


def package_version(name: str) -> str:
    try:
        return importlib.metadata.version(name)
    except importlib.metadata.PackageNotFoundError:
        return "unknown"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("docx", type=Path)
    parser.add_argument("--text-out", required=True, type=Path)
    parser.add_argument("--receipt-out", required=True, type=Path)
    parser.add_argument("--source-url", required=True)
    parser.add_argument("--citation", required=True)
    parser.add_argument("--expected-source-sha256")
    args = parser.parse_args()

    docx_bytes = args.docx.read_bytes()
    source_sha = sha256_hex(docx_bytes)
    if args.expected_source_sha256 and source_sha != args.expected_source_sha256.lower():
        raise SystemExit(
            f"source SHA-256 mismatch: expected {args.expected_source_sha256.lower()} got {source_sha}"
        )

    text = extract_docx(docx_bytes)
    text_bytes = text.encode("utf-8")
    args.text_out.write_bytes(text_bytes)

    receipt = {
        "schema": "sensiblaw-official-primary-extraction/v1",
        "citation": args.citation,
        "source_provider_ref": "provider:official-hca-docx",
        "source_url": args.source_url,
        "source_media_type": DOCX_MEDIA_TYPE,
        "source_sha256_hex": source_sha,
        "source_size_bytes": len(docx_bytes),
        "extractor_repo": UPSTREAM_REPO,
        "extractor_revision": UPSTREAM_REVISION,
        "extractor_path": UPSTREAM_PATH,
        "extractor_function": UPSTREAM_FUNCTION,
        "extractor_license": "MIT",
        "parameters": {
            "docx2html_path": DOCX2HTML_PATH,
            "inscriptis_path": INSCRIPTIS_PATH,
            "mammoth_images": "discard",
            "inscriptis_profile": "strict+h1-h5-margin-before-1",
            "table_cell_separator": "  ",
            "package_versions": {
                "mammoth": package_version("mammoth"),
                "inscriptis": package_version("inscriptis"),
                "lxml": package_version("lxml"),
            },
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
