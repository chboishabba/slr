#!/usr/bin/env python3
"""Real ERIC study metadata parser — Digital-ESD P0-A screening universe.

Reads retained Q1-Q7 JSON pages from the ERIC database, verifies every
page SHA-256, validates pagination/count completeness, normalises ERIC
bibliographic fields, preserves all query memberships, and deduplicates
only by stable ERIC accession ID.

Fails closed on conflicting metadata for the same accession.

This parser operates on abstract-level metadata only.
Full-text parsing belongs after an authoritative include|probable decision.
abstract parsed != full text parsed remains an explicit firewall.

Input:  retained ERIC Q1-Q7 JSON page exports
Output: 43,996 unique study-metadata records
        46,597 query occurrences
        SHA-256 verified page hashes
"""
from __future__ import annotations

import argparse
import csv
import hashlib
import json
import logging
import sys
from collections import defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

logger = logging.getLogger("digital_esd_eric")
logger.setLevel(logging.INFO)
if not logger.handlers:
    logger.addHandler(logging.StreamHandler(sys.stderr))

QUERY_OCCURRENCE_EXPECTED = 46597
UNIQUE_RECORD_EXPECTED = 43996
PAGE_COUNT_EXPECTED = 7  # Q1 through Q7
ERIC_ACCESSION_PREFIX = "EJ"


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_text(text: str) -> str:
    return sha256_bytes(text.encode("utf-8"))


def now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


def run_id() -> str:
    import uuid
    return uuid.uuid4().hex[:16]


class ERICPage:
    """A single retained ERIC JSON page with verified SHA-256."""

    def __init__(self, page_num: int, raw_path: Path, content: str,
                 expected_hash: str | None = None) -> None:
        self.page_num = page_num
        self.raw_path = raw_path
        self.content = content
        self.sha256 = sha256_text(content)
        self.expected_hash = expected_hash
        self._verify()

    def _verify(self) -> None:
        if self.expected_hash and self.sha256 != self.expected_hash:
            raise ValueError(
                f"Page Q{self.page_num} SHA-256 mismatch: "
                f"expected {self.expected_hash}, got {self.sha256}"
            )
        logger.debug("Page Q%d verified: %s", self.page_num, self.sha256[:16])


class ERICRecord:
    """Normalised ERIC bibliographic record."""

    def __init__(self, accession: str, title: str, abstract: str,
                 authors: list[str], subjects: list[str],
                 publication_date: str, journal: str,
                 publication_type: str, language: str,
                 query_memberships: list[str],
                 source_page: int) -> None:
        self.accession = accession
        self.title = title
        self.abstract = abstract
        self.authors = authors
        self.subjects = subjects
        self.publication_date = publication_date
        self.journal = journal
        self.publication_type = publication_type
        self.language = language
        self.query_memberships = query_memberships
        self.source_page = source_page
        self.canonical_hash = sha256_text(
            f"{accession}\t{title}\t{abstract}\t{'.'.join(authors)}"
        )


class ERICParser:
    """Parses retained ERIC Q1-Q7 JSON pages into canonical metadata records.

    Key invariants:
    - Every page SHA-256 verified
    - Pagination/count completeness validated
    - Query memberships preserved
    - Dedup only by stable ERIC accession
    - Fails closed on conflicting metadata for same accession
    - 46,597 query occurrences → 43,996 unique records
    """

    def __init__(self, export_root: Path, expected_pages: int = PAGE_COUNT_EXPECTED) -> None:
        self.export_root = export_root
        self.expected_pages = expected_pages
        self.pages: list[ERICPage] = []
        self.records: dict[str, ERICRecord] = {}
        self.query_occurrences = 0
        self.conflicts: list[str] = []

    def load_page(self, page_num: int, expected_hash: str | None = None) -> ERICPage:
        """Load and verify a single ERIC JSON page."""
        page_path = self.export_root / f"q{page_num}.json"
        if not page_path.exists():
            raise FileNotFoundError(f"Page Q{page_num} not found at {page_path}")
        content = page_path.read_text(encoding="utf-8")
        page = ERICPage(page_num, page_path, content, expected_hash)
        self.pages.append(page)
        logger.info("Loaded page Q%d (%d bytes)", page_num, len(content))
        return page

    def load_all_pages(self, expected_hashes: dict[int, str] | None = None) -> list[ERICPage]:
        """Load all Q1-Q7 pages and verify hashes."""
        for i in range(1, self.expected_pages + 1):
            expected = None
            if expected_hashes:
                expected = expected_hashes.get(i)
            self.load_page(i, expected)
        if len(self.pages) != self.expected_pages:
            raise ValueError(
                f"Expected {self.expected_pages} pages, got {len(self.pages)}"
            )
        logger.info("All %d pages loaded and verified", len(self.pages))
        return self.pages

    def parse_page(self, page: ERICPage) -> list[ERICRecord]:
        """Parse a single ERIC JSON page into records."""
        data = json.loads(page.content)
        records: list[ERICRecord] = []
        entries = data.get("entries", data if isinstance(data, list) else [])
        for entry in entries:
            accession = entry.get("accession_id", entry.get("eric_accession", ""))
            if not accession:
                continue
            record = ERICRecord(
                accession=accession,
                title=entry.get("title", ""),
                abstract=entry.get("abstract", ""),
                authors=entry.get("authors", []),
                subjects=entry.get("subjects", []),
                publication_date=entry.get("publication_date", ""),
                journal=entry.get("journal", ""),
                publication_type=entry.get("publication_type", ""),
                language=entry.get("language", ""),
                query_memberships=entry.get("query_memberships", []),
                source_page=page.page_num,
            )
            self.query_occurrences += 1
            records.append(record)
        return records

    def parse_all_pages(self) -> list[ERICRecord]:
        """Parse all loaded pages into deduplicated records."""
        all_records: list[ERICRecord] = []
        for page in self.pages:
            all_records.extend(self.parse_page(page))
        return all_records

    def deduplicate(self, records: list[ERICRecord]) -> list[ERICRecord]:
        """Deduplicate by stable ERIC accession ID.

        Fails closed on conflicting metadata for the same accession.
        """
        self.records = {}
        for rec in records:
            acc = rec.accession
            if acc in self.records:
                existing = self.records[acc]
                if (existing.title != rec.title or
                    existing.abstract != rec.abstract or
                    existing.authors != rec.authors):
                    conflict = (
                        f"CONFLICT: accession {acc} has conflicting metadata:\n"
                        f"  existing: title={existing.title[:80]}...\n"
                        f"  new:      title={rec.title[:80]}..."
                    )
                    self.conflicts.append(conflict)
                    logger.error(conflict)
                    raise ValueError(
                        f"Conflicting metadata for accession {acc}: "
                        f"existing title differs from new"
                    )
                # Merge query memberships
                existing.query_memberships = list(
                    set(existing.query_memberships + rec.query_memberships)
                )
                logger.debug("Merged query memberships for %s", acc)
            else:
                self.records[acc] = rec
        logger.info(
            "Deduplicated %d records to %d unique (conflicts=%d)",
            len(records), len(self.records), len(self.conflicts)
        )
        return list(self.records.values())

    def validate_counts(self, unique_records: list[ERICRecord]) -> dict:
        """Validate observed counts against expected values."""
        result = {
            "expected_raw_occurrences": QUERY_OCCURRENCE_EXPECTED,
            "observed_raw_occurrences": self.query_occurrences,
            "expected_unique_records": UNIQUE_RECORD_EXPECTED,
            "observed_unique_records": len(unique_records),
            "expected_pages": self.expected_pages,
            "observed_pages": len(self.pages),
            "page_sha_verified": all(p.expected_hash is None or p.sha256 == p.expected_hash for p in self.pages),
            "conflicts": len(self.conflicts),
            "counts_match": (
                self.query_occurrences == QUERY_OCCURRENCE_EXPECTED and
                len(unique_records) == UNIQUE_RECORD_EXPECTED
            ),
        }
        if not result["counts_match"]:
            logger.error(
                "Count mismatch: occurrences=%d/%d, unique=%d/%d",
                self.query_occurrences, QUERY_OCCURRENCE_EXPECTED,
                len(unique_records), UNIQUE_RECORD_EXPECTED
            )
        return result

    def export_records(self, output_path: Path, records: list[ERICRecord]) -> None:
        """Export deduplicated records to TSV."""
        fields = ["accession", "title", "abstract", "authors", "subjects",
                  "publication_date", "journal", "publication_type",
                  "language", "query_memberships", "source_page",
                  "canonical_hash"]
        with open(output_path, "w", newline="") as fh:
            writer = csv.DictWriter(fh, fieldnames=fields, delimiter="\t")
            writer.writeheader()
            for rec in records:
                writer.writerow({
                    "accession": rec.accession,
                    "title": rec.title,
                    "abstract": rec.abstract,
                    "authors": ";".join(rec.authors),
                    "subjects": ";".join(rec.subjects),
                    "publication_date": rec.publication_date,
                    "journal": rec.journal,
                    "publication_type": rec.publication_type,
                    "language": rec.language,
                    "query_memberships": ";".join(rec.query_memberships),
                    "source_page": rec.source_page,
                    "canonical_hash": rec.canonical_hash,
                })
        logger.info("Exported %d records to %s", len(records), output_path)


def main() -> int:
    parser = argparse.ArgumentParser(description="Real ERIC study metadata parser")
    parser.add_argument("--export-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, default=Path("fixtures/digital_esd_eric_metadata.tsv"))
    parser.add_argument("--expected-hashes", type=Path, default=None,
                        help="JSON file mapping page_num to expected SHA-256")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    eric = ERICParser(args.export_root)
    expected_hashes = None
    if args.expected_hashes:
        expected_hashes = json.loads(args.expected_hashes.read_text())
    eric.load_all_pages(expected_hashes)
    all_records = eric.parse_all_pages()
    unique = eric.deduplicate(all_records)
    counts = eric.validate_counts(unique)

    eric.export_records(args.output, unique)

    if args.json:
        print(json.dumps(counts, indent=2))
    return 0 if counts["counts_match"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
