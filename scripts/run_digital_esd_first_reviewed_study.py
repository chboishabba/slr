#!/usr/bin/env python3
"""Run the first real reviewed Digital-ESD ERIC study through P0-G + parsing.

Fixture study:
  ERIC:EJ1083370
  Promoting Online Students' Engagement and Learning in Science and
  Sustainability Preservice Teacher Education

This capstone is application-only.  It:
  1. applies the explicit reviewed probable overlay to the exact 43,996 ledger;
  2. preserves the denominator;
  3. runs the ordinary reviewed-study ingestion controller with max_items=1.

Network remains opt-in via --live.
"""

from __future__ import annotations

import argparse
import csv
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BASE_LEDGER = ROOT / "fixtures" / "digital_esd_ledger.tsv"
OVERLAY = ROOT / "fixtures" / "digital_esd_first_reviewed_study_overlay.jsonl"
TARGET = "ERIC:EJ1083370"


def count_rows(path: Path) -> int:
    with path.open(newline="", encoding="utf-8") as fh:
        return sum(1 for _ in csv.DictReader(fh, delimiter="\t"))


def find_target(path: Path) -> dict[str, str]:
    with path.open(newline="", encoding="utf-8") as fh:
        rows = [
            row for row in csv.DictReader(fh, delimiter="\t")
            if row.get("source_identity_reference") == TARGET
        ]
    if len(rows) != 1:
        raise RuntimeError(f"expected exactly one {TARGET} row, found {len(rows)}")
    return rows[0]


def run(cmd: list[str]) -> None:
    proc = subprocess.run(cmd, cwd=ROOT, check=False)
    if proc.returncode != 0:
        raise SystemExit(proc.returncode)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument(
        "--artifact-root",
        type=Path,
        default=ROOT / "artifacts" / "digital-esd" / "first-reviewed-study",
    )
    ap.add_argument("--live", action="store_true")
    ap.add_argument("--allow-host", action="append", default=[])
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args()

    args.artifact_root.mkdir(parents=True, exist_ok=True)
    reviewed_ledger = args.artifact_root / "screening_ledger_reviewed.tsv"
    application_manifest = args.artifact_root / "screening_decision_application.json"

    run([
        sys.executable,
        str(ROOT / "scripts" / "apply_digital_esd_screening_decisions.py"),
        "--ledger", str(BASE_LEDGER),
        "--decisions", str(OVERLAY),
        "--output", str(reviewed_ledger),
        "--manifest", str(application_manifest),
    ])

    before = count_rows(BASE_LEDGER)
    after = count_rows(reviewed_ledger)
    if before != 43_996 or after != before:
        raise RuntimeError(
            f"denominator integrity failed before={before} after={after}"
        )
    target = find_target(reviewed_ledger)
    if target.get("decision") != "probable":
        raise RuntimeError(
            f"{TARGET} was not promoted to reviewed probable"
        )

    cmd = [
        sys.executable,
        str(ROOT / "scripts" / "run_digital_esd_reviewed_study_ingestion.py"),
        "--ledger", str(reviewed_ledger),
        "--artifact-root", str(args.artifact_root),
        "--max-items", "1",
        "--json",
    ]
    for host in args.allow_host:
        cmd.extend(["--allow-host", host])
    if args.live:
        cmd.append("--live")
    run(cmd)

    receipt_path = args.artifact_root / "reviewed-study-ingestion.json"
    receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
    capstone = {
        "schema": "sensiblaw.digital-esd-first-reviewed-study-capstone.v1",
        "source_identity_reference": TARGET,
        "reviewed_decision": "probable",
        "base_denominator": before,
        "reviewed_denominator": after,
        "network_live": args.live,
        "ingestion_status": receipt.get("status"),
        "downloaded": receipt.get("downloaded", 0),
        "verified_fulltext": receipt.get("verified_fulltext", 0),
        "scholarly": receipt.get("scholarly"),
        "creates_source_truth": False,
        "creates_source_audit_admission": False,
    }
    out = args.artifact_root / "first-reviewed-study-capstone.json"
    out.write_text(
        json.dumps(capstone, indent=2, ensure_ascii=False, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    if args.json:
        print(json.dumps(capstone, indent=2, sort_keys=True))
    else:
        print(
            "first-reviewed-study: "
            f"status={capstone['ingestion_status']} "
            f"downloaded={capstone['downloaded']} "
            f"verified_fulltext={capstone['verified_fulltext']}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
