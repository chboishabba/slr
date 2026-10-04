#!/usr/bin/env python3
"""Explicitly review one Digital-ESD title/abstract packet.

This is a human/operator authority step.  It prints the packet in a compact
form, but emits an overlay only when the operator explicitly supplies:
  --decision include|probable|exclude|unresolved
  --reason <reason-code>
  --reviewer <reviewer/process-ref>

No model/candidate assessment is auto-promoted.
"""

from __future__ import annotations

import argparse
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


ALLOWED = {"include", "probable", "exclude", "unresolved"}


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    rows = []
    with path.open("r", encoding="utf-8") as fh:
        for n, line in enumerate(fh, 1):
            if not line.strip():
                continue
            row = json.loads(line)
            if not isinstance(row, dict):
                raise ValueError(f"{path}:{n}: expected object")
            rows.append(row)
    return rows


def select_packet(rows: list[dict[str, Any]], source_ref: str | None, index: int) -> dict[str, Any]:
    if source_ref:
        matches = [r for r in rows if str(r.get("source_identity_reference") or "") == source_ref]
        if len(matches) != 1:
            raise ValueError(f"expected exactly one packet for {source_ref}, found {len(matches)}")
        return matches[0]
    if not rows:
        raise ValueError("review packet file is empty")
    if index < 0 or index >= len(rows):
        raise IndexError(f"packet index {index} outside [0,{len(rows)-1}]")
    return rows[index]


def compact_view(packet: dict[str, Any]) -> str:
    candidate = packet.get("candidate_assessment") or {}
    return "\n".join(
        [
            f"source: {packet.get('source_identity_reference','')}",
            f"title: {packet.get('title','')}",
            f"authors: {packet.get('authors','')}",
            f"date: {packet.get('publication_date','')}",
            f"journal: {packet.get('journal','')}",
            f"publication_type: {packet.get('publication_type','')}",
            f"query_memberships: {packet.get('query_memberships','')}",
            "",
            "abstract:",
            str(packet.get("abstract") or ""),
            "",
            f"candidate-only suggestion: {candidate.get('candidate_decision')}",
            f"candidate reasons: {candidate.get('candidate_reason_codes')}",
            f"candidate confidence: {candidate.get('confidence_reference')}",
            "",
            "NOTE: candidate assessment is context only and is not authoritative.",
        ]
    )


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--packets", type=Path, required=True)
    ap.add_argument("--source-ref")
    ap.add_argument("--index", type=int, default=0)
    ap.add_argument("--decision", choices=sorted(ALLOWED))
    ap.add_argument("--reason")
    ap.add_argument("--reviewer")
    ap.add_argument("--timestamp")
    ap.add_argument("--output", type=Path)
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args()

    rows = read_jsonl(args.packets)
    packet = select_packet(rows, args.source_ref, args.index)

    if args.decision is None:
        if args.json:
            print(json.dumps(packet, indent=2, ensure_ascii=False, sort_keys=True))
        else:
            print(compact_view(packet))
        return 0

    if not args.reason or not args.reason.strip():
        raise SystemExit("--reason is required when --decision is supplied")
    if not args.reviewer or not args.reviewer.strip():
        raise SystemExit("--reviewer is required when --decision is supplied")

    overlay = {
        "schema": "sensiblaw.digital-esd-screening-decision-overlay.v0_1",
        "source_identity_reference": packet["source_identity_reference"],
        "review_packet_reference": packet["review_packet_reference"],
        "reviewed": True,
        "decision": args.decision,
        "reason_code": args.reason.strip(),
        "reviewer_or_process_reference": args.reviewer.strip(),
        "decision_timestamp": args.timestamp
        or datetime.now(timezone.utc).astimezone().isoformat(),
        "candidate_decision_for_context_only": (
            packet.get("candidate_assessment") or {}
        ).get("candidate_decision"),
        "candidate_reason_codes_for_context_only": (
            packet.get("candidate_assessment") or {}
        ).get("candidate_reason_codes"),
        "candidate_auto_promoted": False,
    }

    encoded = json.dumps(overlay, ensure_ascii=False, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(encoded, encoding="utf-8")
    else:
        print(encoded, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
