#!/usr/bin/env python3
"""Compile an exact Digital-ESD study-processing census.

The census is deliberately fail-closed.  Counts are derived only from explicit
artifacts supplied to this command.  A later stage is never inferred from an
earlier one:

metadata != screened
retained != materialised
materialised != handed to SLR
handoff != parsed
parsed != reviewed
reviewed != SourceAuditAdmission

Supported stages:
  L0 metadata universe
  L1 authoritative title/abstract screening
  L2 retained include/probable worklist
  L3 verified full-text materialisation
  L4 handed to SLR
  L5 successfully parsed by SLR
  L6 reviewed canonical evidence
  L7 SourceAuditAdmission-complete
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
from pathlib import Path
from typing import Any, Iterable


DECISIONS = {"include", "probable", "exclude", "unresolved"}
RETAINED = {"include", "probable"}


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read_tsv(path: Path | None) -> list[dict[str, str]]:
    if path is None or not path.exists():
        return []
    with path.open(newline="", encoding="utf-8") as fh:
        return [dict(row) for row in csv.DictReader(fh, delimiter="	")]


def read_json(path: Path | None) -> Any:
    if path is None or not path.exists():
        return None
    return json.loads(path.read_text(encoding="utf-8"))


def read_jsonl(path: Path | None) -> list[dict[str, Any]]:
    if path is None or not path.exists():
        return []
    out: list[dict[str, Any]] = []
    with path.open("r", encoding="utf-8") as fh:
        for n, line in enumerate(fh, 1):
            if not line.strip():
                continue
            row = json.loads(line)
            if not isinstance(row, dict):
                raise ValueError(f"{path}:{n}: expected JSON object")
            out.append(row)
    return out


def ref_of(row: dict[str, Any]) -> str:
    for key in (
        "source_identity_reference",
        "source_ref",
        "source_unit_ref",
        "sourceReference",
        "sourceIdentityReference",
    ):
        value = row.get(key)
        if isinstance(value, str) and value.strip():
            return value.strip()
    return ""


def explicit_success(row: dict[str, Any], stage: str) -> bool:
    """Return true only when a row explicitly records the stage as successful."""
    if stage == "handoff":
        return (
            row.get("handoff_status") in {"pending-slr-evidence", "handed-to-slr", "handed_off"}
            or row.get("handed_to_slr") is True
        )
    if stage == "parsed":
        return (
            row.get("parsed") is True
            or row.get("parse_success") is True
            or row.get("status") in {"parsed", "parse-success", "success"}
            or row.get("candidate_only") is True and row.get("source_text_sha256") not in (None, "")
        )
    if stage == "reviewed":
        return (
            row.get("reviewed") is True
            or row.get("review_complete") is True
            or row.get("status") in {"reviewed", "review-complete", "paid"}
            or bool(row.get("review_ref"))
        )
    if stage == "admitted":
        return (
            row.get("source_audit_admission_complete") is True
            or row.get("corpus_audited_source") is True
            or row.get("status") in {"source-audit-admitted", "corpus-audited"}
        )
    raise ValueError(stage)


def unique_success_refs(rows: Iterable[dict[str, Any]], stage: str) -> set[str]:
    refs: set[str] = set()
    for row in rows:
        if not explicit_success(row, stage):
            continue
        ref = ref_of(row)
        if not ref:
            raise ValueError(f"{stage} receipt lacks source identity: {row}")
        refs.add(ref)
    return refs


def artifact_entry(path: Path | None) -> dict[str, Any] | None:
    if path is None:
        return None
    return {
        "path": str(path.resolve()),
        "exists": path.exists(),
        "sha256": sha256_file(path) if path.exists() and path.is_file() else None,
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--screening-ledger", type=Path, required=True)
    ap.add_argument("--fulltext-index", type=Path)
    ap.add_argument("--slr-handoff", type=Path)
    ap.add_argument("--slr-parse-receipts", type=Path)
    ap.add_argument("--slr-review-receipts", type=Path)
    ap.add_argument("--source-audit-receipts", type=Path)
    ap.add_argument("--output", type=Path, required=True)
    ap.add_argument("--expected-metadata-count", type=int, default=43996)
    args = ap.parse_args()

    ledger = read_tsv(args.screening_ledger)
    if not ledger:
        raise ValueError("screening ledger is empty")

    ledger_refs = {row.get("source_identity_reference", "") for row in ledger}
    if "" in ledger_refs:
        raise ValueError("screening ledger contains blank source identity")
    if len(ledger_refs) != len(ledger):
        raise ValueError("screening ledger contains duplicate source identities")

    for row in ledger:
        decision = str(row.get("decision") or "")
        if decision not in DECISIONS:
            raise ValueError(
                "screening ledger is not authoritative decision schema; "
                f"invalid/missing decision={decision!r}"
            )

    decision_counts = {
        decision: sum(1 for row in ledger if row["decision"] == decision)
        for decision in sorted(DECISIONS)
    }
    reviewed_refs = {
        row["source_identity_reference"]
        for row in ledger
        if row["decision"] != "unresolved"
        or str(row.get("reviewer_or_model_reference") or "") not in {"", "unassigned"}
    }
    retained_refs = {
        row["source_identity_reference"]
        for row in ledger
        if row["decision"] in RETAINED
    }

    fulltext_rows = read_tsv(args.fulltext_index)
    fulltext_refs = {
        row["source_identity_reference"]
        for row in fulltext_rows
        if row.get("status") == "verified"
    }
    failed_fulltext_refs = {
        row["source_identity_reference"]
        for row in fulltext_rows
        if str(row.get("status") or "").startswith("failed-")
    }
    pending_fulltext_refs = {
        row["source_identity_reference"]
        for row in fulltext_rows
        if row.get("status") == "pending"
    }

    if fulltext_refs - retained_refs:
        raise ValueError("verified full text exists for non-retained records")

    handoff_rows = read_jsonl(args.slr_handoff)
    parsed_rows = read_jsonl(args.slr_parse_receipts)
    reviewed_rows = read_jsonl(args.slr_review_receipts)
    admitted_rows = read_jsonl(args.source_audit_receipts)

    handed_refs = unique_success_refs(handoff_rows, "handoff")
    parsed_refs = unique_success_refs(parsed_rows, "parsed")
    canonical_reviewed_refs = unique_success_refs(reviewed_rows, "reviewed")
    admitted_refs = unique_success_refs(admitted_rows, "admitted")

    if handed_refs - fulltext_refs:
        raise ValueError("SLR handoff contains source without verified full text")
    if parsed_refs - handed_refs:
        raise ValueError("SLR parse receipt exists without explicit handoff")
    if canonical_reviewed_refs - parsed_refs:
        raise ValueError("SLR review receipt exists without explicit parse receipt")
    if admitted_refs - canonical_reviewed_refs:
        raise ValueError("SourceAuditAdmission receipt exists without explicit review receipt")

    census = {
        "schema": "sensiblaw.digital-esd-study-processing-census.v0_1",
        "expected_metadata_records": args.expected_metadata_count,
        "metadata_records": len(ledger),
        "metadata_count_matches_expected": len(ledger) == args.expected_metadata_count,

        "genuinely_screened_records": len(reviewed_refs),
        "screening_decision_counts": decision_counts,
        "include_probable_eligible_for_materialisation": len(retained_refs),

        "verified_fulltext_artifacts": len(fulltext_refs),
        "fulltext_pending": len(pending_fulltext_refs),
        "fulltext_failed": len(failed_fulltext_refs),

        "handed_to_slr": len(handed_refs),
        "successfully_parsed_by_slr": len(parsed_refs),
        "reviewed_canonical_evidence": len(canonical_reviewed_refs),
        "source_audit_admission_complete": len(admitted_refs),

        "denominator_integrity": sum(decision_counts.values()) == len(ledger),
        "later_stage_inference_used": False,
        "metadata_counts_as_fulltext": False,
        "fulltext_counts_as_parsed": False,
        "parse_counts_as_reviewed": False,
        "review_counts_as_source_audit_admission": False,

        "artifacts": {
            "screening_ledger": artifact_entry(args.screening_ledger),
            "fulltext_index": artifact_entry(args.fulltext_index),
            "slr_handoff": artifact_entry(args.slr_handoff),
            "slr_parse_receipts": artifact_entry(args.slr_parse_receipts),
            "slr_review_receipts": artifact_entry(args.slr_review_receipts),
            "source_audit_receipts": artifact_entry(args.source_audit_receipts),
        },
    }

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(census, indent=2, ensure_ascii=False, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(json.dumps(census, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
