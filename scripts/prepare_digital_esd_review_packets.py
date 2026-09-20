#!/usr/bin/env python3
"""Compile human-review packets from the Digital-ESD calibration/Pareto queue.

This is the L1->L2 review handoff.  It deliberately does NOT emit an
authoritative screening decision.

Inputs:
  screening_ledger.tsv
  candidate_assessments.jsonl
  calibration_selection.jsonl
  screening_pareto_queue.jsonl

Outputs:
  review_packets.jsonl
  screening_decision_overlay_template.jsonl
  review_packet_manifest.json

The overlay template contains decision=null and reviewed=false.  It must be
explicitly completed before apply_digital_esd_screening_decisions.py accepts it.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
from pathlib import Path
from typing import Any


def read_tsv(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as fh:
        return [dict(row) for row in csv.DictReader(fh, delimiter="	")]


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    out: list[dict[str, Any]] = []
    with path.open("r", encoding="utf-8") as fh:
        for n, line in enumerate(fh, 1):
            if not line.strip():
                continue
            row = json.loads(line)
            if not isinstance(row, dict):
                raise ValueError(f"{path}:{n}: expected object")
            out.append(row)
    return out


def explicitly_reviewed(row: dict[str, str]) -> bool:
    reviewer = str(row.get("reviewer_or_model_reference") or "").strip()
    supersedes = str(row.get("supersedes_decision_reference") or "").strip()
    return reviewer not in {"", "unassigned"} and bool(supersedes)


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    with path.open("w", encoding="utf-8") as fh:
        for row in rows:
            fh.write(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n")


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def sha256_json(value: Any) -> str:
    return hashlib.sha256(
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--artifact-dir", type=Path, required=True)
    ap.add_argument("--output-dir", type=Path)
    ap.add_argument("--max-packets", type=int, default=250)
    ap.add_argument(
        "--selection",
        choices=("calibration-first", "pareto-first"),
        default="calibration-first",
    )
    args = ap.parse_args()

    root = args.artifact_dir
    out = args.output_dir or (root / "review")
    out.mkdir(parents=True, exist_ok=True)

    ledger_path = root / "screening_ledger.tsv"
    assessment_path = root / "candidate_assessments.jsonl"
    calibration_path = root / "calibration_selection.jsonl"
    pareto_path = root / "screening_pareto_queue.jsonl"

    ledger_rows = read_tsv(ledger_path)
    ledger_by_ref = {row["source_identity_reference"]: row for row in ledger_rows}
    assessments = read_jsonl(assessment_path)
    assessment_by_ref = {row["source_identity_reference"]: row for row in assessments}
    calibration = read_jsonl(calibration_path)
    pareto = read_jsonl(pareto_path)

    calibration_refs = [row["source_identity_reference"] for row in calibration]
    pareto_refs = [row["source_identity_reference"] for row in pareto]

    ordered: list[str] = []
    seen: set[str] = set()
    sources = (calibration_refs, pareto_refs) if args.selection == "calibration-first" else (pareto_refs, calibration_refs)
    for refs in sources:
        for ref in refs:
            if ref in seen:
                continue
            seen.add(ref)
            ordered.append(ref)

    packets: list[dict[str, Any]] = []
    overlays: list[dict[str, Any]] = []

    for ref in ordered:
        if len(packets) >= args.max_packets:
            break
        ledger = ledger_by_ref.get(ref)
        if ledger is None:
            raise RuntimeError(f"queue references source outside ledger denominator: {ref}")
        if ledger.get("decision") != "unresolved":
            continue
        if explicitly_reviewed(ledger):
            continue
        assessment = assessment_by_ref.get(ref)
        if assessment is None:
            raise RuntimeError(f"missing candidate assessment for queued source: {ref}")

        packet_payload = {
            "source_identity_reference": ref,
            "metadata_revision_reference": ledger.get("metadata_revision_reference", ""),
            "title_abstract_snapshot_reference": ledger.get("title_abstract_snapshot_reference", ""),
            "candidate_assessment_reference": assessment.get("assessment_reference", ""),
        }
        packet = {
            "schema": "sensiblaw.digital-esd-screening-review-packet.v0_1",
            "review_packet_reference": "review-packet:" + sha256_json(packet_payload),
            "source_identity_reference": ref,
            "eric_accession": ledger.get("eric_accession", ""),
            "metadata_revision_reference": ledger.get("metadata_revision_reference", ""),
            "metadata_sha256": ledger.get("metadata_sha256", ""),
            "title_abstract_snapshot_reference": ledger.get("title_abstract_snapshot_reference", ""),
            "title_abstract_snapshot_sha256": ledger.get("title_abstract_snapshot_sha256", ""),
            "title": ledger.get("title", ""),
            "abstract": ledger.get("abstract", ""),
            "authors": ledger.get("authors", ""),
            "subjects": ledger.get("subjects", ""),
            "publication_date": ledger.get("publication_date", ""),
            "journal": ledger.get("journal", ""),
            "publication_type": ledger.get("publication_type", ""),
            "language": ledger.get("language", ""),
            "query_memberships": ledger.get("query_memberships", ""),
            "current_decision": ledger.get("decision", ""),
            "current_decision_reference": ledger.get("decision_reference", ""),
            "candidate_assessment": {
                "candidate_decision": assessment.get("candidate_decision"),
                "candidate_reason_codes": assessment.get("candidate_reason_codes"),
                "confidence_reference": assessment.get("confidence_reference"),
                "margin_reference": assessment.get("margin_reference"),
                "feature_evidence": assessment.get("feature_evidence"),
                "candidate_only": True,
            },
            "review_required": True,
            "candidate_assessment_is_authoritative": False,
            "review_packet_creates_screening_decision": False,
        }
        packets.append(packet)

        overlays.append({
            "schema": "sensiblaw.digital-esd-screening-decision-overlay-template.v0_1",
            "source_identity_reference": ref,
            "review_packet_reference": packet["review_packet_reference"],
            "reviewed": False,
            "decision": None,
            "reason_code": None,
            "reviewer_or_process_reference": None,
            "decision_timestamp": None,
            "candidate_decision_for_context_only": assessment.get("candidate_decision"),
            "candidate_reason_codes_for_context_only": assessment.get("candidate_reason_codes"),
            "candidate_auto_promoted": False,
        })

    packet_path = out / "review_packets.jsonl"
    overlay_path = out / "screening_decision_overlay_template.jsonl"
    write_jsonl(packet_path, packets)
    write_jsonl(overlay_path, overlays)

    manifest = {
        "schema": "sensiblaw.digital-esd-screening-review-packet-manifest.v0_1",
        "selection": args.selection,
        "max_packets": args.max_packets,
        "ledger_record_count": len(ledger_rows),
        "packet_count": len(packets),
        "all_packets_unresolved_before_review": all(
            ledger_by_ref[p["source_identity_reference"]].get("decision") == "unresolved"
            for p in packets
        ),
        "overlay_rows_reviewed": 0,
        "overlay_rows_with_authoritative_decision": 0,
        "candidate_auto_promoted": False,
        "packet_reference": str(packet_path.resolve()),
        "packet_sha256": sha256_file(packet_path),
        "overlay_template_reference": str(overlay_path.resolve()),
        "overlay_template_sha256": sha256_file(overlay_path),
    }
    (out / "review_packet_manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(json.dumps(manifest, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
