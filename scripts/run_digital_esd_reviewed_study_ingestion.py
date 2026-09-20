#!/usr/bin/env python3
"""Run the real Digital-ESD reviewed-study retrieval -> ESD-4 parsing slice.

This is deliberately downstream of authoritative title/abstract review.

Pipeline:
  reviewed screening ledger
    -> P0-G full-text worklist
    -> bounded cache plan
    -> opt-in live full-text retrieval
    -> retrieval digest/revision registration
    -> P0-G full-text verification
    -> ESD-4 scholarly full-text parse + verification

If there are no include|probable decisions, the controller stops cleanly.
Nothing here creates screening decisions, source truth, or SourceAuditAdmission.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from interop_scripts.digital_esd.scholarly_fulltext import run as run_scholarly_fulltext
from interop_scripts.digital_esd_fulltext_cache import FullTextCacheWrapper
from scripts.digital_esd_fetcher import FullTextFetcher
from scripts.prepare_digital_esd_fulltext_index import DigitalESDFulltextIndexer


def write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(payload, indent=2, ensure_ascii=False, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def main() -> int:
    ap = argparse.ArgumentParser(
        description="Digital-ESD reviewed-study retrieval -> ESD-4 parser"
    )
    ap.add_argument("--ledger", type=Path, required=True)
    ap.add_argument(
        "--artifact-root",
        type=Path,
        default=Path("artifacts/digital-esd"),
    )
    ap.add_argument("--priority-queue", type=Path)
    ap.add_argument("--url-map", type=Path)
    ap.add_argument("--allow-host", action="append", default=[])
    ap.add_argument("--max-items", type=int, default=20)
    ap.add_argument("--max-cache-gib", type=int, default=2)
    ap.add_argument("--reserve-gib", type=int, default=5)
    ap.add_argument("--planning-size-mb", type=int, default=10)
    ap.add_argument("--live", action="store_true")
    ap.add_argument("--allow-partial-parse", action="store_true")
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args()

    if not args.ledger.exists():
        raise SystemExit(f"reviewed ledger not found: {args.ledger}")

    fulltext_dir = args.artifact_root / "fulltext"
    cache_dir = fulltext_dir / "cache"
    priority_queue = args.priority_queue or (
        args.artifact_root / "screening" / "adaptive" / "screening-pareto-queue.jsonl"
    )

    # First pass creates the authoritative include|probable worklist even when
    # no artifacts have yet been retrieved.
    initial_gate = DigitalESDFulltextIndexer(
        ledger_path=args.ledger,
        output_dir=fulltext_dir,
        retrieved_manifest_path=None,
    ).run()
    worklist = Path(initial_gate["worklist_reference"])

    if int(initial_gate["eligible_for_fulltext"]) == 0:
        result = {
            "schema": "sensiblaw.digital-esd-reviewed-study-ingestion.v1",
            "status": "awaiting-authoritative-include-probable-decisions",
            "ledger_reference": str(args.ledger),
            "eligible_for_fulltext": 0,
            "downloaded": 0,
            "verified_fulltext": 0,
            "parsed_verified": 0,
            "creates_screening_decision": False,
            "creates_source_truth": False,
            "creates_source_audit_admission": False,
        }
        receipt = args.artifact_root / "reviewed-study-ingestion.json"
        write_json(receipt, result)
        if args.json:
            print(json.dumps(result, indent=2, sort_keys=True))
        else:
            print("reviewed-study-ingestion: 0 retained studies; awaiting review decisions")
        return 0

    cache = FullTextCacheWrapper(
        cache_dir=cache_dir,
        worklist=worklist,
        priority_queue=priority_queue,
        max_items=args.max_items,
        max_cache_gib=args.max_cache_gib,
        reserve_gib=args.reserve_gib,
        planning_size_mb=args.planning_size_mb,
    )
    cache_plan = cache.plan()
    if not cache_plan["eligible_for_fetch"]:
        result = {
            "schema": "sensiblaw.digital-esd-reviewed-study-ingestion.v1",
            "status": "cache-envelope-blocked",
            "eligible_for_fulltext": initial_gate["eligible_for_fulltext"],
            "cache_plan": cache_plan,
            "creates_source_truth": False,
            "creates_source_audit_admission": False,
        }
        write_json(args.artifact_root / "reviewed-study-ingestion.json", result)
        if args.json:
            print(json.dumps(result, indent=2, sort_keys=True))
        else:
            print("reviewed-study-ingestion: cache envelope blocked")
        return 2

    selected_refs = [
        str(row["source_identity_reference"])
        for row in cache_plan.get("batch", [])
    ]
    fetch_plan = {
        "schema": "sensiblaw.digital-esd-fulltext-fetch-plan.v0_2",
        "selected_refs": selected_refs,
        "selected_count": len(selected_refs),
        "candidate_only": True,
        "creates_screening_decision": False,
    }

    allowed_hosts = ("files.eric.ed.gov",) + tuple(args.allow_host)
    fetcher = FullTextFetcher(
        cache_dir=cache_dir,
        output_dir=fulltext_dir,
        network_enabled=args.live,
        url_map=args.url_map,
        allowed_hosts=allowed_hosts,
    )
    fetch_result = fetcher.fetch(fetch_plan, dry_run=not args.live)

    if not args.live:
        result = {
            "schema": "sensiblaw.digital-esd-reviewed-study-ingestion.v1",
            "status": "planned-network-disabled",
            "eligible_for_fulltext": initial_gate["eligible_for_fulltext"],
            "selected_for_fetch": len(selected_refs),
            "fetch": fetch_result,
            "creates_source_truth": False,
            "creates_source_audit_admission": False,
        }
        write_json(args.artifact_root / "reviewed-study-ingestion.json", result)
        if args.json:
            print(json.dumps(result, indent=2, sort_keys=True))
        else:
            print(
                f"reviewed-study-ingestion: {len(selected_refs)} planned; "
                "rerun with --live to permit network"
            )
        return 0

    # Register verifies the exact bytes and preserves digest/revision identity.
    registration = cache.register()
    handoff = cache.handoff()

    retrieved_manifest = fulltext_dir / "retrieved-manifest.jsonl"
    verified_gate = DigitalESDFulltextIndexer(
        ledger_path=args.ledger,
        output_dir=fulltext_dir,
        retrieved_manifest_path=retrieved_manifest if retrieved_manifest.exists() else None,
    ).run()

    handoff_manifest = fulltext_dir / "fulltext-handoff-manifest.jsonl"
    scholarly_dir = fulltext_dir / "scholarly"
    scholarly_result: dict[str, Any] | None = None

    if handoff["retained_for_slr"] > 0:
        scholarly_result = run_scholarly_fulltext(
            handoff_manifest,
            Path("interop_scripts/digital_esd/scholarly_fulltext.prototype.json"),
            scholarly_dir,
            allow_partial=args.allow_partial_parse,
        )

    result = {
        "schema": "sensiblaw.digital-esd-reviewed-study-ingestion.v1",
        "status": (
            "parsed"
            if scholarly_result is not None
            else "no-registered-fulltext"
        ),
        "ledger_reference": str(args.ledger),
        "eligible_for_fulltext": initial_gate["eligible_for_fulltext"],
        "selected_for_fetch": len(selected_refs),
        "downloaded": fetch_result["downloaded_count"],
        "fetch_failed": fetch_result["failed_count"],
        "registered": registration["registered_count"],
        "registration_rejected": registration["rejected_count"],
        "verified_fulltext": verified_gate["verified"],
        "fulltext_failed": verified_gate["failed"],
        "handed_to_scholarly_parser": handoff["retained_for_slr"],
        "scholarly": scholarly_result,
        "creates_screening_decision": False,
        "creates_source_truth": False,
        "creates_source_audit_admission": False,
    }
    receipt = args.artifact_root / "reviewed-study-ingestion.json"
    write_json(receipt, result)

    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    else:
        verified_parse = (
            scholarly_result.get("verified_count", 0)
            if scholarly_result
            else 0
        )
        print(
            "reviewed-study-ingestion: "
            f"downloaded={result['downloaded']} "
            f"verified_fulltext={result['verified_fulltext']} "
            f"parsed_verified={verified_parse}"
        )

    return 0 if result["fetch_failed"] == 0 and result["fulltext_failed"] == 0 else 2


if __name__ == "__main__":
    raise SystemExit(main())
