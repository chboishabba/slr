from __future__ import annotations

import csv
import json
from pathlib import Path

import pytest

from interop_scripts.document_text import DocumentTextError, extract_document_text
from scripts.apply_digital_esd_screening_decisions import apply_decisions, write_tsv
from scripts.prepare_digital_esd_fulltext_index import DigitalESDFulltextIndexer


ROOT = Path(__file__).resolve().parents[1]
LEDGER = ROOT / "artifacts" / "digital-esd" / "real-eric" / "screening_ledger.tsv"
OVERLAY = ROOT / "fixtures" / "digital_esd_first_reviewed_study_overlay.jsonl"
TARGET = "ERIC:EJ1083370"

REAL_LEDGER_MISSING = (
    "real ERIC screening ledger not retained; build it via "
    "`python3 scripts/digital_esd_fetcher.py eric --live` and "
    "`python3 -m scripts.run_digital_esd_real_eric --export-root "
    "artifacts/digital-esd/eric --output-root artifacts/digital-esd/real-eric`"
)


def read_tsv(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as fh:
        return [dict(row) for row in csv.DictReader(fh, delimiter="\t")]


def _ledger_present() -> bool:
    return LEDGER.exists() and LEDGER.stat().st_size > 0


@pytest.mark.skipif(not _ledger_present(), reason=REAL_LEDGER_MISSING)
def test_first_reviewed_study_belongs_to_exact_denominator() -> None:
    rows = read_tsv(LEDGER)
    assert len(rows) == 43_996
    matches = [row for row in rows if row["source_identity_reference"] == TARGET]
    assert len(matches) == 1
    assert matches[0]["decision"] == "unresolved"


@pytest.mark.skipif(not _ledger_present(), reason=REAL_LEDGER_MISSING)
def test_first_review_overlay_preserves_denominator_and_opens_p0g(tmp_path: Path) -> None:
    ledger_rows = read_tsv(LEDGER)
    overlay = [
        json.loads(line)
        for line in OVERLAY.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]

    reviewed_rows, counts = apply_decisions(ledger_rows, overlay)

    assert len(reviewed_rows) == 43_996
    assert sum(counts.values()) == 43_996
    target = next(
        row for row in reviewed_rows
        if row["source_identity_reference"] == TARGET
    )
    assert target["decision"] == "probable"
    assert counts["probable"] == 1

    reviewed_ledger = tmp_path / "screening_ledger_reviewed.tsv"
    write_tsv(reviewed_ledger, reviewed_rows)

    gate = DigitalESDFulltextIndexer(
        ledger_path=reviewed_ledger,
        output_dir=tmp_path / "fulltext",
        retrieved_manifest_path=None,
    ).run()

    assert gate["eligible_for_fulltext"] == 1
    worklist = Path(gate["worklist_reference"])
    rows = [
        json.loads(line)
        for line in worklist.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]
    assert [row["source_identity_reference"] for row in rows] == [TARGET]


def test_pdf_text_extraction_fails_closed_without_real_extractor(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    pdf = tmp_path / "fixture.pdf"
    pdf.write_bytes(b"%PDF-1.4\nnot-a-real-pdf\n")

    monkeypatch.setattr("interop_scripts.document_text.shutil.which", lambda _: None)

    with pytest.raises(
        DocumentTextError, match="pdf-text-extractor-unavailable:pdftotext"
    ):
        extract_document_text(pdf)


def test_plaintext_adapter_retains_extracted_text_digest(tmp_path: Path) -> None:
    path = tmp_path / "study.txt"
    path.write_text("Methods\nParticipants: 12\n", encoding="utf-8")
    extracted = extract_document_text(path)

    assert extracted.text == "Methods\nParticipants: 12\n"
    assert extracted.extractor_reference == "stdlib:utf8-text:v1"
    assert len(extracted.extracted_text_sha256) == 64
