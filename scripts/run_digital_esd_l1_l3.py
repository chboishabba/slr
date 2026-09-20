#!/usr/bin/env python3
"""Run the real Digital-ESD L1->L3 study-processing path.

This is a thin orchestrator over existing owners.  It does not implement
screening semantics itself.

Stages:
  L0/L1  retained ERIC JSON -> parsed/deduplicated metadata -> unresolved ledger
  L1.5   candidate assessment/Pareto -> human review packets
  L2     optional explicit reviewed overlay -> authoritative screening ledger
  L3     optional retrieved-manifest -> verified full-text index
  census exact stage counts, fail-closed for SLR parse/review/admission

If --decision-overlay is omitted, the run intentionally stops with all
authoritative decisions unresolved.  Candidate assessments are never promoted.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Iterable


def run(cmd: list[str]) -> None:
    print("+", " ".join(cmd), file=sys.stderr)
    subprocess.run(cmd, check=True)


def existing(path: Path | None) -> Path | None:
    if path is None:
        return None
    return path if path.exists() else None


def append_optional(args: list[str], flag: str, value: Path | None) -> None:
    if value is not None:
        args.extend([flag, str(value)])


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--export-root", type=Path, required=True)
    ap.add_argument(
        "--artifact-root",
        type=Path,
        default=Path("artifacts/digital-esd/real-eric"),
    )
    ap.add_argument("--decision-overlay", type=Path)
    ap.add_argument("--retrieved-manifest", type=Path)
    ap.add_argument("--slr-handoff", type=Path)
    ap.add_argument("--slr-parse-receipts", type=Path)
    ap.add_argument("--slr-review-receipts", type=Path)
    ap.add_argument("--source-audit-receipts", type=Path)
    ap.add_argument("--review-packets", type=int, default=250)
    args = ap.parse_args()

    root = args.artifact_root
    root.mkdir(parents=True, exist_ok=True)

    # L0/L1: real retained ERIC exports -> exact metadata + unresolved ledger
    run([
        sys.executable,
        "scripts/run_digital_esd_real_eric.py",
        "--export-root",
        str(args.export_root),
        "--output-root",
        str(root),
    ])

    # Candidate/Pareto queue -> explicit review packets. No decision authority.
    run([
        sys.executable,
        "scripts/prepare_digital_esd_review_packets.py",
        "--artifact-dir",
        str(root),
        "--max-packets",
        str(args.review_packets),
    ])

    unresolved_ledger = root / "screening_ledger.tsv"
    authoritative_ledger = unresolved_ledger

    # L2: only explicit reviewed overlay may change authority state.
    if args.decision_overlay is not None:
        if not args.decision_overlay.exists():
            raise FileNotFoundError(args.decision_overlay)
        authoritative_ledger = root / "screening_ledger_reviewed.tsv"
        run([
            sys.executable,
            "scripts/apply_digital_esd_screening_decisions.py",
            "--ledger",
            str(unresolved_ledger),
            "--decisions",
            str(args.decision_overlay),
            "--output",
            str(authoritative_ledger),
        ])

    # L3: only include/probable rows are eligible for full-text verification.
    fulltext_dir = root / "fulltext"
    fulltext_index: Path | None = None
    if args.retrieved_manifest is not None or args.decision_overlay is not None:
        ft_cmd = [
            sys.executable,
            "scripts/prepare_digital_esd_fulltext_index.py",
            "--ledger",
            str(authoritative_ledger),
            "--output-dir",
            str(fulltext_dir),
        ]
        if args.retrieved_manifest is not None:
            if not args.retrieved_manifest.exists():
                raise FileNotFoundError(args.retrieved_manifest)
            ft_cmd.extend(["--retrieved-manifest", str(args.retrieved_manifest)])
        run(ft_cmd)
        candidate_index = fulltext_dir / "digital_esd_fulltext_index.tsv"
        if candidate_index.exists():
            fulltext_index = candidate_index

    # Fail-closed census: absent later receipts -> zero, never inferred.
    census_path = root / "study_processing_census.json"
    census_cmd = [
        sys.executable,
        "scripts/census_digital_esd_study_processing.py",
        "--screening-ledger",
        str(authoritative_ledger),
        "--output",
        str(census_path),
    ]
    append_optional(census_cmd, "--fulltext-index", fulltext_index)
    append_optional(census_cmd, "--slr-handoff", existing(args.slr_handoff))
    append_optional(census_cmd, "--slr-parse-receipts", existing(args.slr_parse_receipts))
    append_optional(census_cmd, "--slr-review-receipts", existing(args.slr_review_receipts))
    append_optional(census_cmd, "--source-audit-receipts", existing(args.source_audit_receipts))
    run(census_cmd)

    census = json.loads(census_path.read_text(encoding="utf-8"))
    print(json.dumps({
        "metadata_records": census["metadata_records"],
        "genuinely_screened_records": census["genuinely_screened_records"],
        "include_probable_eligible_for_materialisation": census["include_probable_eligible_for_materialisation"],
        "verified_fulltext_artifacts": census["verified_fulltext_artifacts"],
        "handed_to_slr": census["handed_to_slr"],
        "successfully_parsed_by_slr": census["successfully_parsed_by_slr"],
        "reviewed_canonical_evidence": census["reviewed_canonical_evidence"],
        "source_audit_admission_complete": census["source_audit_admission_complete"],
        "authoritative_ledger": str(authoritative_ledger),
        "census": str(census_path),
    }, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
