#!/usr/bin/env python3
"""Prepare full-text requests for explicitly reviewed-but-unresolved screening rows.

This lane exists only to resolve title/abstract ambiguity. It does NOT convert
an unresolved screening decision into include/probable and it does NOT create
SourceAuditAdmission.

Eligible rows:
  decision == unresolved
  reviewer_or_model_reference != unassigned
  supersedes_decision_reference present
  reason_code in {
      insufficientTitleAbstractEvidence,
      inaccessibleAbstract,
      requiresFullText,
  }

Outputs a candidate-only worklist suitable for retrieval/fetch planning.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
from pathlib import Path
from typing import Any

RESOLUTION_REASONS = {
    "insufficientTitleAbstractEvidence",
    "inaccessibleAbstract",
    "requiresFullText",
}


def read_tsv(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as fh:
        return [dict(row) for row in csv.DictReader(fh, delimiter="\t")]


def explicitly_reviewed(row: dict[str, str]) -> bool:
    reviewer = str(row.get("reviewer_or_model_reference") or "").strip()
    supersedes = str(row.get("supersedes_decision_reference") or "").strip()
    return reviewer not in {"", "unassigned"} and bool(supersedes)


def sha256_json(value: Any) -> str:
    return hashlib.sha256(
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--ledger", type=Path, required=True)
    ap.add_argument("--output", type=Path, required=True)
    ap.add_argument("--manifest", type=Path)
    args = ap.parse_args()

    rows = read_tsv(args.ledger)
    selected: list[dict[str, Any]] = []

    for row in rows:
        if str(row.get("decision") or "") != "unresolved":
            continue
        if not explicitly_reviewed(row):
            continue
        reason = str(row.get("reason_code") or "")
        if reason not in RESOLUTION_REASONS:
            continue

        payload = {
            "source_identity_reference": row["source_identity_reference"],
            "decision_reference": row.get("decision_reference", ""),
            "reason_code": reason,
            "metadata_sha256": row.get("metadata_sha256", ""),
        }
        selected.append(
            {
                "schema": "sensiblaw.digital-esd-screening-resolution-fulltext.v0_1",
                "source_identity_reference": row["source_identity_reference"],
                "decision_reference": row.get("decision_reference", ""),
                "reason_code": reason,
                "reviewer_or_model_reference": row.get("reviewer_or_model_reference", ""),
                "request_reference": "screening-resolution-fulltext:" + sha256_json(payload),
                "purpose": "resolve-title-abstract-screening-ambiguity",
                "candidate_only": True,
                "creates_screening_inclusion": False,
                "creates_source_truth": False,
                "creates_source_audit_admission": False,
            }
        )

    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", encoding="utf-8") as fh:
        for row in selected:
            fh.write(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n")

    manifest = {
        "schema": "sensiblaw.digital-esd-screening-resolution-fulltext-manifest.v0_1",
        "ledger_reference": str(args.ledger.resolve()),
        "ledger_record_count": len(rows),
        "selected_count": len(selected),
        "output_reference": str(args.output.resolve()),
        "reviewed_unresolved_only": True,
        "creates_screening_inclusion": False,
        "creates_source_truth": False,
        "creates_source_audit_admission": False,
    }
    manifest_path = args.manifest or args.output.with_suffix(".manifest.json")
    manifest_path.write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(json.dumps(manifest, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
