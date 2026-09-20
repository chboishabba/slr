#!/usr/bin/env python3
"""Real ERIC study metadata parser — Digital-ESD P0-A screening universe.

Official ERIC API reference:
  Institute of Education Sciences / ERIC,
  "Using the ERIC API for Research Topics",
  https://eric.ed.gov/pdf/Using_ERIC_API_for_Research_Topics.pdf

This parser consumes the retained ERIC API exports produced by the Digital-ESD
execution lane and normalizes the documented ERIC API fields.

Supported retained layouts:

  <root>/Q1/page-000000.json
  <root>/Q1/page-000001.json
  <root>/Q1/summary.json
  ...
  <root>/Q7/...

and the earlier precombined compatibility form:

  <root>/q1.json
  ...
  <root>/q7.json

For raw ERIC API responses the parser reads payload["response"]["docs"].

Key invariants:
- every retained page may be SHA-256 checked against summary.json;
- all seven query families must be present;
- all document occurrences are counted;
- query membership is derived from the retained Q1..Q7 source context;
- deduplication is only by stable ERIC accession id;
- conflicting metadata for the same accession fails closed;
- metadata/abstract parsing never counts as full-text retrieval or parsing.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import logging
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable

logger = logging.getLogger("digital_esd_eric")
logger.setLevel(logging.INFO)
if not logger.handlers:
    logger.addHandler(logging.StreamHandler(sys.stderr))

QUERY_OCCURRENCE_EXPECTED = 46597
UNIQUE_RECORD_EXPECTED = 43996
QUERY_COUNT_EXPECTED = 7


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_text(text: str) -> str:
    return sha256_bytes(text.encode("utf-8"))


def _as_list(value: Any) -> list[str]:
    if value is None:
        return []
    if isinstance(value, list):
        return [str(x).strip() for x in value if str(x).strip()]
    if isinstance(value, tuple):
        return [str(x).strip() for x in value if str(x).strip()]
    text = str(value).strip()
    return [text] if text else []


def _as_text(value: Any) -> str:
    if value is None:
        return ""
    if isinstance(value, list):
        return "; ".join(str(x).strip() for x in value if str(x).strip())
    return str(value).strip()


def _first(entry: dict[str, Any], *keys: str) -> Any:
    for key in keys:
        if key in entry and entry[key] not in (None, "", []):
            return entry[key]
    return None


@dataclass(frozen=True)
class ERICPage:
    query_num: int
    page_index: int
    raw_path: Path
    content: str
    sha256: str
    expected_sha256: str | None = None
    expected_num_found: int | None = None

    @classmethod
    def load(
        cls,
        *,
        query_num: int,
        page_index: int,
        raw_path: Path,
        expected_sha256: str | None = None,
        expected_num_found: int | None = None,
    ) -> "ERICPage":
        content = raw_path.read_text(encoding="utf-8")
        digest = sha256_text(content)
        if expected_sha256 and digest != expected_sha256:
            raise ValueError(
                f"Q{query_num} page {page_index} SHA-256 mismatch: "
                f"expected {expected_sha256}, observed {digest}"
            )
        return cls(
            query_num=query_num,
            page_index=page_index,
            raw_path=raw_path,
            content=content,
            sha256=digest,
            expected_sha256=expected_sha256,
            expected_num_found=expected_num_found,
        )


@dataclass
class ERICRecord:
    accession: str
    title: str
    abstract: str
    authors: list[str]
    subjects: list[str]
    publication_date: str
    journal: str
    publication_type: str
    language: str
    query_memberships: list[str]
    source_page: int

    def __post_init__(self) -> None:
        self.canonical_hash = sha256_text(
            json.dumps(
                {
                    "accession": self.accession,
                    "title": self.title,
                    "abstract": self.abstract,
                    "authors": self.authors,
                    "subjects": self.subjects,
                    "publication_date": self.publication_date,
                    "journal": self.journal,
                    "publication_type": self.publication_type,
                    "language": self.language,
                },
                ensure_ascii=False,
                sort_keys=True,
                separators=(",", ":"),
            )
        )


class ERICParser:
    def __init__(self, export_root: Path, expected_queries: int = QUERY_COUNT_EXPECTED) -> None:
        self.export_root = export_root
        self.expected_queries = expected_queries
        self.pages: list[ERICPage] = []
        self.records: dict[str, ERICRecord] = {}
        self.query_occurrences = 0
        self.conflicts: list[str] = []
        self.query_occurrence_counts: dict[int, int] = {}
        self.query_expected_counts: dict[int, int] = {}

    # ------------------------------------------------------------------
    # Retained export discovery / integrity
    # ------------------------------------------------------------------

    def _query_dir(self, query_num: int) -> Path | None:
        for name in (f"Q{query_num}", f"q{query_num}"):
            path = self.export_root / name
            if path.is_dir():
                return path
        return None

    def _summary_metadata(self, query_dir: Path) -> tuple[dict[Path, str], int | None]:
        summary_path = query_dir / "summary.json"
        if not summary_path.exists():
            return {}, None
        payload = json.loads(summary_path.read_text(encoding="utf-8"))
        expected_num_found = payload.get("numFound")
        if expected_num_found is not None:
            expected_num_found = int(expected_num_found)

        hashes: dict[Path, str] = {}
        for page in payload.get("pages", []):
            path_text = page.get("path")
            digest = page.get("sha256")
            if not path_text or not digest:
                continue
            candidate = Path(path_text)
            if not candidate.is_absolute():
                # Prefer the retained query directory when summary paths are
                # relative or were emitted from another working directory.
                candidate = query_dir / candidate.name
            hashes[candidate.resolve()] = str(digest)
        return hashes, expected_num_found

    def discover_query_pages(self, query_num: int) -> list[ERICPage]:
        query_dir = self._query_dir(query_num)
        if query_dir is not None:
            summary_hashes, expected_num_found = self._summary_metadata(query_dir)
            page_paths = sorted(query_dir.glob("page-*.json"))
            if not page_paths:
                raise FileNotFoundError(
                    f"Q{query_num}: retained query directory has no page-*.json files: {query_dir}"
                )
            pages = [
                ERICPage.load(
                    query_num=query_num,
                    page_index=index,
                    raw_path=path,
                    expected_sha256=summary_hashes.get(path.resolve()),
                    expected_num_found=expected_num_found,
                )
                for index, path in enumerate(page_paths)
            ]
            return pages

        # Compatibility with a precombined per-query export.
        for name in (f"q{query_num}.json", f"Q{query_num}.json"):
            path = self.export_root / name
            if path.exists():
                return [
                    ERICPage.load(
                        query_num=query_num,
                        page_index=0,
                        raw_path=path,
                    )
                ]

        raise FileNotFoundError(
            f"Q{query_num}: no retained export found under {self.export_root}"
        )

    def load_all_pages(self, expected_hashes: dict[int, str] | None = None) -> list[ERICPage]:
        self.pages = []
        for query_num in range(1, self.expected_queries + 1):
            pages = self.discover_query_pages(query_num)
            # Compatibility: a caller-supplied hash applies only to a single
            # precombined page, never to a paginated query directory.
            if expected_hashes and query_num in expected_hashes and len(pages) == 1:
                expected = expected_hashes[query_num]
                if pages[0].sha256 != expected:
                    raise ValueError(
                        f"Q{query_num} SHA-256 mismatch: expected {expected}, "
                        f"observed {pages[0].sha256}"
                    )
            self.pages.extend(pages)

        observed_queries = {page.query_num for page in self.pages}
        expected_queries = set(range(1, self.expected_queries + 1))
        if observed_queries != expected_queries:
            raise ValueError(
                f"query coverage mismatch: observed={sorted(observed_queries)} "
                f"expected={sorted(expected_queries)}"
            )
        logger.info(
            "Loaded %d retained pages across Q1-Q%d",
            len(self.pages),
            self.expected_queries,
        )
        return self.pages

    # ------------------------------------------------------------------
    # API normalization
    # ------------------------------------------------------------------

    def _docs_from_payload(self, payload: Any) -> tuple[list[dict[str, Any]], int | None]:
        if isinstance(payload, list):
            docs = [row for row in payload if isinstance(row, dict)]
            return docs, None
        if not isinstance(payload, dict):
            raise ValueError("ERIC page payload must be an object or array")

        response = payload.get("response")
        if isinstance(response, dict):
            docs = response.get("docs", [])
            if not isinstance(docs, list):
                raise ValueError("ERIC response.docs must be a list")
            num_found = response.get("numFound")
            return [row for row in docs if isinstance(row, dict)], (
                int(num_found) if num_found is not None else None
            )

        entries = payload.get("entries")
        if isinstance(entries, list):
            return [row for row in entries if isinstance(row, dict)], payload.get("numFound")

        # One more compatibility form used by some retained exports.
        docs = payload.get("docs")
        if isinstance(docs, list):
            return [row for row in docs if isinstance(row, dict)], payload.get("numFound")

        raise ValueError("ERIC page has neither response.docs nor entries/docs")

    def _record_from_entry(self, entry: dict[str, Any], page: ERICPage) -> ERICRecord:
        accession = _as_text(_first(entry, "id", "accession_id", "eric_accession", "ERICNumber"))
        if not accession:
            raise ValueError(
                f"Q{page.query_num} page {page.page_index}: record missing ERIC id"
            )

        title = _as_text(_first(entry, "title", "Title"))
        abstract = _as_text(_first(entry, "description", "abstract", "Abstract"))
        authors = _as_list(_first(entry, "author", "authors", "Author", "Authors"))
        subjects = _as_list(_first(entry, "subject", "subjects", "Subject"))
        publication_date = _as_text(
            _first(
                entry,
                "publicationdateyear",
                "publication_date",
                "PublicationDate",
                "year",
            )
        )
        journal = _as_text(_first(entry, "source", "journal", "Source", "Journal"))
        publication_type = _as_text(
            _first(entry, "publicationtype", "publication_type", "PublicationType")
        )
        language = _as_text(_first(entry, "language", "Language"))

        memberships = _as_list(entry.get("query_memberships"))
        memberships.append(f"Q{page.query_num}")
        memberships = sorted(set(memberships))

        return ERICRecord(
            accession=accession,
            title=title,
            abstract=abstract,
            authors=authors,
            subjects=subjects,
            publication_date=publication_date,
            journal=journal,
            publication_type=publication_type,
            language=language,
            query_memberships=memberships,
            source_page=page.page_index,
        )

    def parse_page(self, page: ERICPage) -> list[ERICRecord]:
        payload = json.loads(page.content)
        docs, observed_num_found = self._docs_from_payload(payload)

        expected_num_found = page.expected_num_found
        if (
            expected_num_found is not None
            and observed_num_found is not None
            and observed_num_found != expected_num_found
        ):
            raise ValueError(
                f"Q{page.query_num}: numFound drift between summary/page: "
                f"{expected_num_found} != {observed_num_found}"
            )

        records = [self._record_from_entry(entry, page) for entry in docs]
        self.query_occurrences += len(records)
        self.query_occurrence_counts[page.query_num] = (
            self.query_occurrence_counts.get(page.query_num, 0) + len(records)
        )
        if expected_num_found is not None:
            self.query_expected_counts[page.query_num] = expected_num_found
        elif observed_num_found is not None:
            self.query_expected_counts[page.query_num] = observed_num_found
        return records

    def parse_all_pages(self) -> list[ERICRecord]:
        self.query_occurrences = 0
        self.query_occurrence_counts = {}
        self.query_expected_counts = {}
        all_records: list[ERICRecord] = []
        for page in self.pages:
            all_records.extend(self.parse_page(page))

        for query_num, expected in sorted(self.query_expected_counts.items()):
            observed = self.query_occurrence_counts.get(query_num, 0)
            if observed != expected:
                raise ValueError(
                    f"Q{query_num}: pagination incomplete: "
                    f"observed {observed} docs, expected {expected}"
                )
        return all_records

    # ------------------------------------------------------------------
    # Same-object deduplication
    # ------------------------------------------------------------------

    def _metadata_signature(self, record: ERICRecord) -> tuple[Any, ...]:
        return (
            record.title,
            record.abstract,
            tuple(record.authors),
            tuple(record.subjects),
            record.publication_date,
            record.journal,
            record.publication_type,
            record.language,
        )

    def deduplicate(self, records: list[ERICRecord]) -> list[ERICRecord]:
        self.records = {}
        self.conflicts = []
        for record in records:
            existing = self.records.get(record.accession)
            if existing is None:
                self.records[record.accession] = record
                continue

            if self._metadata_signature(existing) != self._metadata_signature(record):
                conflict = (
                    f"conflicting metadata for ERIC accession {record.accession}: "
                    f"{existing.source_page} vs {record.source_page}"
                )
                self.conflicts.append(conflict)
                raise ValueError(conflict)

            existing.query_memberships = sorted(
                set(existing.query_memberships + record.query_memberships)
            )

        logger.info(
            "Deduplicated %d query occurrences to %d unique ERIC records",
            len(records),
            len(self.records),
        )
        return list(self.records.values())

    def validate_counts(self, unique_records: list[ERICRecord]) -> dict[str, Any]:
        observed_queries = sorted({page.query_num for page in self.pages})
        result = {
            "expected_raw_occurrences": QUERY_OCCURRENCE_EXPECTED,
            "observed_raw_occurrences": self.query_occurrences,
            "expected_unique_records": UNIQUE_RECORD_EXPECTED,
            "observed_unique_records": len(unique_records),
            "expected_queries": QUERY_COUNT_EXPECTED,
            "observed_queries": len(observed_queries),
            "query_numbers": observed_queries,
            "retained_page_count": len(self.pages),
            "page_sha_verified": all(
                page.expected_sha256 is None or page.sha256 == page.expected_sha256
                for page in self.pages
            ),
            "pagination_complete": all(
                self.query_occurrence_counts.get(query_num, 0) == expected
                for query_num, expected in self.query_expected_counts.items()
            ),
            "conflicts": len(self.conflicts),
            "counts_match": (
                self.query_occurrences == QUERY_OCCURRENCE_EXPECTED
                and len(unique_records) == UNIQUE_RECORD_EXPECTED
            ),
        }
        return result

    def export_records(self, output_path: Path, records: list[ERICRecord]) -> None:
        fields = [
            "accession",
            "title",
            "abstract",
            "authors",
            "subjects",
            "publication_date",
            "journal",
            "publication_type",
            "language",
            "query_memberships",
            "source_page",
            "canonical_hash",
        ]
        with output_path.open("w", newline="", encoding="utf-8") as fh:
            writer = csv.DictWriter(fh, fieldnames=fields, delimiter="\t")
            writer.writeheader()
            for record in records:
                writer.writerow(
                    {
                        "accession": record.accession,
                        "title": record.title,
                        "abstract": record.abstract,
                        "authors": ";".join(record.authors),
                        "subjects": ";".join(record.subjects),
                        "publication_date": record.publication_date,
                        "journal": record.journal,
                        "publication_type": record.publication_type,
                        "language": record.language,
                        "query_memberships": ";".join(record.query_memberships),
                        "source_page": record.source_page,
                        "canonical_hash": record.canonical_hash,
                    }
                )


def main() -> int:
    parser = argparse.ArgumentParser(description="Parse retained real ERIC API exports")
    parser.add_argument("--export-root", type=Path, required=True)
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("artifacts/digital-esd/real-eric/digital_esd_eric_metadata.tsv"),
    )
    parser.add_argument("--expected-hashes", type=Path)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    expected_hashes = None
    if args.expected_hashes:
        expected_hashes = {
            int(key): value
            for key, value in json.loads(args.expected_hashes.read_text()).items()
        }

    eric = ERICParser(args.export_root)
    eric.load_all_pages(expected_hashes)
    parsed = eric.parse_all_pages()
    unique = eric.deduplicate(parsed)
    counts = eric.validate_counts(unique)

    args.output.parent.mkdir(parents=True, exist_ok=True)
    eric.export_records(args.output, unique)

    if args.json:
        print(json.dumps(counts, indent=2, sort_keys=True))
    return 0 if counts["counts_match"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
