#!/usr/bin/env python3
"""Apply explicit reviewed title/abstract decisions to the Digital-ESD ledger.

This is the authority-changing step in the screening loop.

Candidate assessments and Pareto priorities are not accepted directly.  Every
overlay row must explicitly say reviewed=true and name a reviewer/process ref.

The output ledger preserves exactly the same source-identity denominator as the
input. Unmentioned records are copied unchanged.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


ALLOWED = {"include", "probable", "exclude", "unresolved"}


def now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read_tsv(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as fh:
        return [dict(row) for row in csv.DictReader(fh, delimiter="\t")]


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    with path.open("r", encoding="utf-8") as fh:
        for n, line in enumerate(fh, 1):
            if not line.strip():
                continue
            row = json.loads(line)
            if not isinstance(row, dict):
                raise ValueError(f"{path}:{n}: expected JSON object")
            rows.append(row)
    return rows


def write_tsv(path: Path, rows: list[dict[str, Any]]) -> None:
    if not rows:
        raise ValueError("screening ledger cannot be empty")
    fields = list(rows[0].keys())
    with path.open("w", newline="", encoding="utf-8") as fh:
        writer = csv.DictWriter(fh, fieldnames=fields, delimiter="\t", extrasaction="ignore")
        writer.writeheader()
        writer.writerows(rows)


def apply_decisions(
    ledger_rows: list[dict[str, str]],
    decision_rows: list[dict[str, Any]],
) -> tuple[list[dict[str, Any]], dict[str, int]]:
    by_ref = {
        str(row.get("source_identity_reference") or ""): dict(row)
        for row in ledger_rows
    }
    if "" in by_ref:
        raise ValueError("screening ledger contains blank source identity")
    if len(by_ref) != len(ledger_rows):
        raise ValueError("screening ledger contains duplicate source identities")

    seen: set[str] = set()
    for decision in decision_rows:
        ref = str(decision.get("source_identity_reference") or "")
        if not ref:
            raise ValueError("decision row lacks source_identity_reference")
        if ref in seen:
            raise ValueError(f"multiple decision overlay rows for {ref}")
        seen.add(ref)
        if ref not in by_ref:
            raise ValueError(f"decision references source outside exact denominator: {ref}")

        reviewed = decision.get("reviewed")
        if reviewed is not True:
            raise ValueError(f"{ref}: decision overlay must have reviewed=true")

        value = str(decision.get("decision") or "")
        if value not in ALLOWED:
            raise ValueError(f"{ref}: invalid decision {value!r}")

        reviewer = str(
            decision.get("reviewer_or_process_reference")
            or decision.get("reviewer_reference")
            or ""
        ).strip()
        if not reviewer:
            raise ValueError(f"{ref}: explicit reviewer/process reference required")

        reason = str(decision.get("reason_code") or "").strip()
        if not reason:
            raise ValueError(f"{ref}: reason_code required")

        row = by_ref[ref]
        prior_ref = row.get("decision_reference", "")
        timestamp = str(decision.get("decision_timestamp") or now_iso())
        identity_payload = json.dumps(
            {
                "source_identity_reference": ref,
                "metadata_sha256": row.get("metadata_sha256", ""),
                "decision": value,
                "reason_code": reason,
                "reviewer_or_process_reference": reviewer,
                "decision_timestamp": timestamp,
                "supersedes": prior_ref,
            },
            sort_keys=True,
            separators=(",", ":"),
        ).encode("utf-8")
        decision_ref = "screening-decision:" + hashlib.sha256(identity_payload).hexdigest()

        row["decision"] = value
        row["reason_code"] = reason
        row["reviewer_or_model_reference"] = reviewer
        row["decision_timestamp"] = timestamp
        row["supersedes_decision_reference"] = prior_ref
        row["decision_reference"] = decision_ref
        by_ref[ref] = row

    output = [by_ref[str(row["source_identity_reference"])] for row in ledger_rows]
    counts = {key: 0 for key in ALLOWED}
    for row in output:
        value = str(row.get("decision") or "")
        if value not in ALLOWED:
            raise ValueError(f"ledger contains invalid decision state {value!r}")
        counts[value] += 1
    if sum(counts.values()) != len(ledger_rows):
        raise AssertionError("denominator integrity failure")
    return output, counts


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--ledger", type=Path, required=True)
    ap.add_argument("--decisions", type=Path, required=True)
    ap.add_argument("--output", type=Path, required=True)
    ap.add_argument("--manifest", type=Path)
    args = ap.parse_args()

    before_hash = sha256_file(args.ledger)
    ledger_rows = read_tsv(args.ledger)
    decision_rows = read_jsonl(args.decisions)
    output_rows, counts = apply_decisions(ledger_rows, decision_rows)

    args.output.parent.mkdir(parents=True, exist_ok=True)
    write_tsv(args.output, output_rows)
    after_hash = sha256_file(args.output)

    manifest_path = args.manifest or args.output.with_suffix(".manifest.json")
    manifest = {
        "schema": "sensiblaw.digital-esd-screening-decision-application.v0_1",
        "input_ledger": str(args.ledger.resolve()),
        "input_ledger_sha256": before_hash,
        "decision_overlay": str(args.decisions.resolve()),
        "decision_overlay_sha256": sha256_file(args.decisions),
        "output_ledger": str(args.output.resolve()),
        "output_ledger_sha256": after_hash,
        "record_count": len(output_rows),
        "applied_decision_count": len(decision_rows),
        "decision_counts": counts,
        "denominator_preserved": len(output_rows) == len(ledger_rows),
        "candidate_assessment_auto_promoted": False,
        "pareto_priority_auto_promoted": False,
    }
    manifest_path.write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(json.dumps(manifest, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
