#!/usr/bin/env python3
"""Generic scholarly document parser prototype.

This is an application-side prototype, deliberately not yet SLR core.
It opens retained scholarly files and extracts real document structure
plus generic study-facet candidates.

Supported formats:
  TXT / Markdown
  HTML
  DOCX  (requires python-docx)
  PDF   (requires pypdf or PyMuPDF/fitz)

Binary formats are first lowered through interop_scripts.document_text.
PDF extraction uses pypdf, PyMuPDF/fitz, or pdftotext and fails closed if no
trusted text engine produces text. OCR is never implicit.

Output contains document_nodes with anchored:
  heading, paragraph, table_cell, ...

Then runs generic candidate extraction over the actual text.

Every candidate remains:
  candidate_only = true
  creates_study_truth = false
  creates_source_audit_admission = false
"""

from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any

from interop_scripts.document_text import (
    DocumentTextMaterialisation,
    DocumentTextMaterialisationError,
    materialise_document_text,
)

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_PROTOTYPE = ROOT / "interop_scripts" / "digital_esd" / "scholarly_fulltext.prototype.json"


STRUCTURE_PATTERNS = {
    "heading": re.compile(r"^(#{1,6})\s+(.+)$", re.MULTILINE),
    "paragraph": re.compile(r"^(.{20,})$", re.MULTILINE),
    "table_cell": re.compile(r"^\|.*\|$"),
    "list_item": re.compile(r"^\s*[-*]\s+.+$"),
    "numbered_item": re.compile(r"^\s*\d+\.\s+.+$"),
}

STUDY_TERMS = {
    "Population": re.compile(
        r"(?:population|participants?|sample\s+of|n\s*=\s*|N\s*=\s*"
        r"|[\d,]+\s+(?:students?|participants?|subjects?|respondents?))",
        re.IGNORECASE,
    ),
    "Sample": re.compile(
        r"(?:sample\s+(?:of|size|n|N)|random\s+sampling|convenience\s+sampling|"
        r"stratified|purposive|snowball|participants?\s+(?:were|included|recruited))",
        re.IGNORECASE,
    ),
    "Intervention": re.compile(
        r"(?:intervention|treatment|program|curriculum|instructional\s+"
        r"strategy|teaching\s+(?:method|approach)|experimental\s+group|"
        r"control\s+group|implementation)",
        re.IGNORECASE,
    ),
    "Comparator": re.compile(
        r"(?:comparator|comparison\s+group|control\s+(?:group|condition)|"
        r"alternative\s+(?:approach|method)|versus|compared\s+with|"
        r"against\s+(?:the\s+)?(?:traditional|conventional|standard))",
        re.IGNORECASE,
    ),
    "Outcome": re.compile(
        r"(?:outcome|result|finding|effect|impact|improvement|achievement|"
        r"performance\s+(?:gain|score|metric)|statistically\s+significant|"
        r"effect\s+size|improved\s+(?:by|from|to))",
        re.IGNORECASE,
    ),
    "StudyDesign": re.compile(
        r"(?:quasi-experimental|experimental|randomized\s+control|"
        r"longitudinal|cross-sectional|case\s+study|mixed\s+methods|"
        r"qualitative|quantitative|pre-test|post-test|design\s+was)",
        re.IGNORECASE,
    ),
    "Setting": re.compile(
        r"(?:setting|school|university|institution|classroom|district|"
        r"context\s+(?:of|where)|urban|rural|suburban|online\s+learning\s+environment)",
        re.IGNORECASE,
    ),
    "TimePeriod": re.compile(
        r"(?:time\s+period|academic\s+year|semester|duration|period\s+of|"
        r"over\s+(?:a\s+)?(?:\d+\s+)?(?:week|month|semester|year)|"
        r"between\s+\d{4}\s+and\s+\d{4})",
        re.IGNORECASE,
    ),
    "Method": re.compile(
        r"(?:method|methodology|approach|procedure|data\s+collection|"
        r"instrument|survey|interview|observation|assessment\s+tool|"
        r"validated\s+(?:scale|instrument)|reliability|validity)",
        re.IGNORECASE,
    ),
    "Limitation": re.compile(
        r"(?:limitation|limitation|constraint|bias|shortcoming|caveat|"
        r"generalizability|external\s+validity|sample\s+size\s+limitation|"
        r"self-report\s+bias|potential\s+limitation)",
        re.IGNORECASE,
    ),
    "Funding": re.compile(
        r"(?:funding|grant|sponsor|financial\s+support|funded\s+by|"
        r"research\s+grant|funding\s+source|no\s+external\s+funding)",
        re.IGNORECASE,
    ),
    "Institution": re.compile(
        r"(?:institution|university|college|school|department|faculty|"
        r"research\s+center|institute|organization|school\s+of)",
        re.IGNORECASE,
    ),
    "ParticipantGroup": re.compile(
        r"(?:participant\s+group|group\s+(?:of|consisted\s+of)|experimental\s+"
        r"group|control\s+group|treatment\s+group|comparison\s+group|"
        r"cohort|subgroup)",
        re.IGNORECASE,
    ),
    "Measurement": re.compile(
        r"(?:measurement|scale|instrument|survey|questionnaire|test|"
        r"assessment|rubric|rubrics|scoring|metric|index|index)",
        re.IGNORECASE,
    ),
}

CANDIDATE_ROLES = [
    "Population", "Sample", "Intervention", "Comparator", "Outcome",
    "StudyDesign", "Setting", "TimePeriod", "Method", "Limitation",
    "Funding", "Institution", "ParticipantGroup", "Measurement",
]


class ScholarlyParserPrototype:
    """Generic scholarly document parser prototype.

    Does NOT create source truth or semantic authority.
    All outputs are candidate_only.
    """

    def __init__(self, config_path: Path) -> None:
        self.config = self._load_config(config_path)
        self.supported = self.config.get("supported_formats", [])

    @staticmethod
    def _load_config(config_path: Path) -> dict[str, Any]:
        return json.loads(config_path.read_text(encoding="utf-8"))

    def parse_file(self, artifact_path: Path, request: dict[str, Any]) -> dict[str, Any]:
        """Parse a single scholarly document and return structure + candidates."""
        materialised = materialise_document_text(
            artifact_path,
            expected_source_sha256=str(request.get("content_sha256") or "") or None,
        )
        content = materialised.extracted_text
        format_type = materialised.source_format

        document_nodes = self._extract_structure(
            content,
            format_type,
            materialised,
        )
        facets = self._extract_facets(content)

        return {
            "request_reference": request.get("request_reference", ""),
            "source_identity_reference": request.get("source_identity_reference", ""),
            "source_revision_reference": request.get("source_revision_reference", ""),
            "content_sha256": materialised.source_artifact_sha256,
            "artifact_path": str(artifact_path),
            "format_type": format_type,
            "extracted_text_sha256": materialised.extracted_text_sha256,
            "extraction_engine": materialised.extraction_engine,
            "extraction_engine_version": materialised.extraction_engine_version,
            "page_count": materialised.page_count,
            "paragraph_count": materialised.paragraph_count,
            "extraction_receipt": materialised.receipt(),
            "document_nodes": document_nodes,
            "study_facets": facets,
            "candidate_only": True,
            "creates_study_truth": False,
            "creates_source_audit_admission": False,
            "parser_success": True,
        }

    def _detect_format(self, ext: str) -> str:
        mapping = {".txt": "plaintext", ".md": "markdown", ".html": "html",
                   ".htm": "html", ".docx": "docx", ".pdf": "pdf",
                   ".tex": "latex", ".csv": "csv"}
        return mapping.get(ext, "plaintext")

    @staticmethod
    def _page_for_char(materialised: DocumentTextMaterialisation, offset: int) -> int | None:
        for anchor in materialised.anchors:
            if anchor.page_number is None:
                continue
            if anchor.start_char <= offset <= anchor.end_char:
                return anchor.page_number
        return None

    def _extract_structure(
        self,
        content: str,
        format_type: str,
        materialised: DocumentTextMaterialisation,
    ) -> list[dict[str, Any]]:
        """Extract document structure with exact extracted-text anchors."""
        nodes: list[dict[str, Any]] = []
        lines = content.split("\n")
        cursor = 0

        for i, line in enumerate(lines, 1):
            start_char = cursor
            end_char = start_char + len(line)
            page_number = self._page_for_char(materialised, start_char)

            for node_type, pattern in STRUCTURE_PATTERNS.items():
                match = pattern.match(line)
                if match:
                    node: dict[str, Any] = {
                        "node_id": f"{node_type}:{i}",
                        "node_type": node_type,
                        "line": i,
                        "content_ref": f"line:{i}",
                        "format_type": format_type,
                        "start_char": start_char,
                        "end_char": end_char,
                        "page_number": page_number,
                        "extracted_text_sha256": materialised.extracted_text_sha256,
                    }
                    if node_type == "heading":
                        node["heading_level"] = len(match.group(1))
                        node["heading_text"] = match.group(2)
                    elif node_type == "table_cell":
                        node["cell_content"] = line.strip()
                    nodes.append(node)
                    break

            cursor = end_char + 1

        return nodes

    def _extract_facets(self, content: str) -> list[dict[str, Any]]:
        """Extract study-facet candidates from actual text."""
        facets: list[dict[str, Any]] = []

        for role in CANDIDATE_ROLES:
            pattern = STUDY_TERMS[role]
            matches = pattern.findall(content)
            if matches:
                facets.append({
                    "facet_role": role,
                    "candidate_only": True,
                    "creates_study_truth": False,
                    "creates_source_audit_admission": False,
                    "matched_terms": len(matches),
                    "evidence_refs": [f"line:{i}" for i, line in
                                      enumerate(content.split("\n"), 1)
                                      if pattern.search(line)][:5],
                })

        return facets

    def parse_all(
        self,
        requests_path: Path,
        output_path: Path,
    ) -> list[dict[str, Any]]:
        """Parse all requested artifacts and write output."""
        requests = []
        with requests_path.open("r", encoding="utf-8") as fh:
            for line in fh:
                if line.strip():
                    requests.append(json.loads(line))

        results: list[dict[str, Any]] = []
        for req in requests:
            artifact = Path(req.get("artifact_path", ""))
            if not artifact.exists():
                results.append({
                    "source_identity_reference": req.get("source_identity_reference", ""),
                    "parser_success": False,
                    "reason": "artifact-missing",
                    "candidate_only": True,
                })
                continue

            try:
                result = self.parse_file(artifact, req)
            except DocumentTextMaterialisationError as exc:
                results.append({
                    "request_reference": req.get("request_reference", ""),
                    "source_identity_reference": req.get("source_identity_reference", ""),
                    "source_revision_reference": req.get("source_revision_reference", ""),
                    "content_sha256": req.get("content_sha256", ""),
                    "artifact_path": str(artifact),
                    "parser_success": False,
                    "reason": "document-text-materialisation-failed",
                    "failure_reference": str(exc),
                    "candidate_only": True,
                    "creates_study_truth": False,
                    "creates_source_audit_admission": False,
                })
                continue
            results.append(result)

        output_path.parent.mkdir(parents=True, exist_ok=True)
        with output_path.open("w", encoding="utf-8") as fh:
            for r in results:
                fh.write(json.dumps(r, ensure_ascii=False, sort_keys=True) + "\n")

        return results


# ------------------------------------------------------------------
# CLI
# ------------------------------------------------------------------

def main() -> int:
    import argparse

    parser = argparse.ArgumentParser(description="Scholarly document parser prototype")
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--config", type=Path, default=DEFAULT_PROTOTYPE)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--allow-partial", action="store_true")
    args = parser.parse_args()

    config = json.loads(args.config.read_text(encoding="utf-8"))
    parser_obj = ScholarlyParserPrototype(args.config)

    results = parser_obj.parse_all(args.input, args.output)

    success = sum(1 for r in results if r.get("parser_success"))
    failed = sum(1 for r in results if not r.get("parser_success"))

    print(f"parse: {success} succeeded, {failed} failed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
