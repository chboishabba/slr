#!/usr/bin/env python3
"""Run one explicit Digital-ESD review -> ingestion round.

This is a thin orchestration layer over existing authoritative components.

Without --decisions:
  refresh adaptive queue
  prepare bounded review packets
  stop awaiting explicit operator decisions

With --decisions:
  apply only reviewed=true decisions
  assert denominator preservation
  refresh the queue against the reviewed ledger
  run reviewed-study full-text ingestion over include|probable sources

No candidate/model output is promoted automatically.
"""

from __future__ import annotations

import argparse
import csv
import json
import subprocess
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]


def read_tsv(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as fh:
        return [dict(row) for row in csv.DictReader(fh, delimiter="\t")]


def write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(payload, indent=2, ensure_ascii=False, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def run(cmd: list[str]) -> None:
    proc = subprocess.run(cmd, cwd=ROOT, check=False)
    if proc.returncode != 0:
        raise SystemExit(proc.returncode)


def decision_counts(rows: list[dict[str, str]]) -> dict[str, int]:
    allowed = ("include", "probable", "exclude", "unresolved")
    return {
        decision: sum(1 for row in rows if row.get("decision") == decision)
        for decision in allowed
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument(
        "--artifact-dir",
        type=Path,
        default=ROOT / "artifacts" / "digital-esd" / "real-eric",
    )
    ap.add_argument("--ledger", type=Path)
    ap.add_argument("--decisions", type=Path)
    ap.add_argument("--max-packets", type=int, default=50)
    ap.add_argument(
        "--selection",
        choices=("calibration-first", "pareto-first"),
        default="calibration-first",
    )
    ap.add_argument("--max-items", type=int, default=20)
    ap.add_argument("--max-cache-gib", type=int, default=2)
    ap.add_argument("--reserve-gib", type=int, default=5)
    ap.add_argument("--allow-host", action="append", default=[])
    ap.add_argument("--url-map", type=Path)
    ap.add_argument("--live", action="store_true")
    ap.add_argument("--allow-partial-parse", action="store_true")
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args()

    artifact_dir = args.artifact_dir
    base_ledger = args.ledger or (artifact_dir / "screening_ledger.tsv")
    if not base_ledger.exists():
        raise SystemExit(f"authoritative screening ledger not found: {base_ledger}")

    review_dir = artifact_dir / "review"
    adaptive_dir = artifact_dir
    round_dir = artifact_dir / "review-round"
    round_dir.mkdir(parents=True, exist_ok=True)

    before_rows = read_tsv(base_ledger)
    before_count = len(before_rows)
    before_counts = decision_counts(before_rows)

    assessments = artifact_dir / "candidate_assessments.jsonl"
    hypotheses = artifact_dir / "study_family_hypotheses.jsonl"

    run([
        sys.executable,
        str(ROOT / "scripts" / "refresh_digital_esd_review_queue.py"),
        "--ledger", str(base_ledger),
        "--assessments", str(assessments),
        "--hypotheses", str(hypotheses),
        "--output-dir", str(adaptive_dir),
    ])
    run([
        sys.executable,
        str(ROOT / "scripts" / "prepare_digital_esd_review_packets.py"),
        "--artifact-dir", str(artifact_dir),
        "--output-dir", str(review_dir),
        "--max-packets", str(args.max_packets),
        "--selection", args.selection,
    ])

    packet_manifest_path = review_dir / "review_packet_manifest.json"
    packet_manifest = json.loads(packet_manifest_path.read_text(encoding="utf-8"))

    if args.decisions is None:
        result = {
            "schema": "sensiblaw.digital-esd-review-ingestion-round.v1",
            "status": "awaiting-explicit-review-decisions",
            "ledger_reference": str(base_ledger),
            "denominator": before_count,
            "decision_counts": before_counts,
            "review_packet_count": int(packet_manifest.get("packet_count", 0)),
            "review_packets_reference": str(review_dir / "review_packets.jsonl"),
            "overlay_template_reference": str(
                review_dir / "screening_decision_overlay_template.jsonl"
            ),
            "candidate_auto_promoted": False,
            "creates_source_truth": False,
            "creates_source_audit_admission": False,
        }
        write_json(round_dir / "review-round.json", result)
        if args.json:
            print(json.dumps(result, indent=2, sort_keys=True))
        else:
            print(
                "review-round: "
                f"{result['review_packet_count']} packets prepared; "
                "awaiting explicit decisions"
            )
        return 0

    if not args.decisions.exists():
        raise SystemExit(f"decision overlay not found: {args.decisions}")

    reviewed_ledger = round_dir / "screening_ledger_reviewed.tsv"
    apply_manifest = round_dir / "screening_decision_application.json"
    run([
        sys.executable,
        str(ROOT / "scripts" / "apply_digital_esd_screening_decisions.py"),
        "--ledger", str(base_ledger),
        "--decisions", str(args.decisions),
        "--output", str(reviewed_ledger),
        "--manifest", str(apply_manifest),
    ])

    after_rows = read_tsv(reviewed_ledger)
    after_count = len(after_rows)
    after_counts = decision_counts(after_rows)
    if after_count != before_count:
        raise RuntimeError(
            f"denominator integrity failed before={before_count} after={after_count}"
        )

    # Recompute the adaptive frontier after explicit decisions.
    run([
        sys.executable,
        str(ROOT / "scripts" / "refresh_digital_esd_review_queue.py"),
        "--ledger", str(reviewed_ledger),
        "--assessments", str(assessments),
        "--hypotheses", str(hypotheses),
        "--output-dir", str(round_dir / "adaptive"),
    ])

    ingestion_cmd = [
        sys.executable,
        str(ROOT / "scripts" / "run_digital_esd_reviewed_study_ingestion.py"),
        "--ledger", str(reviewed_ledger),
        "--artifact-root", str(artifact_dir),
        "--priority-queue", str(round_dir / "adaptive" / "screening_pareto_queue.jsonl"),
        "--max-items", str(args.max_items),
        "--max-cache-gib", str(args.max_cache_gib),
        "--reserve-gib", str(args.reserve_gib),
        "--json",
    ]
    if args.url_map:
        ingestion_cmd.extend(["--url-map", str(args.url_map)])
    for host in args.allow_host:
        ingestion_cmd.extend(["--allow-host", host])
    if args.live:
        ingestion_cmd.append("--live")
    if args.allow_partial_parse:
        ingestion_cmd.append("--allow-partial-parse")
    run(ingestion_cmd)

    ingestion_receipt_path = artifact_dir / "reviewed-study-ingestion.json"
    ingestion = json.loads(ingestion_receipt_path.read_text(encoding="utf-8"))
    progress_path = artifact_dir / "reviewed-study-progress.json"
    progress = (
        json.loads(progress_path.read_text(encoding="utf-8"))
        if progress_path.exists()
        else {}
    )

    result = {
        "schema": "sensiblaw.digital-esd-review-ingestion-round.v1",
        "status": "reviewed-and-ingestion-run",
        "base_ledger_reference": str(base_ledger),
        "reviewed_ledger_reference": str(reviewed_ledger),
        "base_denominator": before_count,
        "reviewed_denominator": after_count,
        "denominator_preserved": before_count == after_count,
        "decision_counts_before": before_counts,
        "decision_counts_after": after_counts,
        "review_packet_count": int(packet_manifest.get("packet_count", 0)),
        "network_live": args.live,
        "ingestion": ingestion,
        "progress": progress,
        "candidate_auto_promoted": False,
        "creates_source_truth": False,
        "creates_source_audit_admission": False,
    }
    write_json(round_dir / "review-round.json", result)
    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    else:
        print(
            "review-round: "
            f"denominator={after_count} "
            f"include={after_counts['include']} "
            f"probable={after_counts['probable']} "
            f"parsed_cumulative={progress.get('parsed_verified_cumulative', 0)} "
            f"remaining_unparsed={progress.get('remaining_unparsed', 0)}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
