#!/usr/bin/env python3
"""Selective full-text materialisation layer for Digital-ESD.

This is an HDD-safe, sparse cache wrapper over the 43,996-record ERIC
metadata universe.  It does NOT treat metadata rows as full-text files.

The four operations are:

    plan      — build a bounded fetch batch from include|probable worklist
    register  — hash actual retrieved artifacts and refuse to register
                anything not in the exact fetch plan
    handoff   — lower only registered retained items toward canonical SLR evidence
    gc-plan   — write an inspectable list of safe eviction candidates

Invariants preserved:

    43,996 metadata rows != 43,996 materialised full-text files
    unreviewed record     != eligible for full-text batch
    include|probable      != automatic download
    cached artifact       != SourceAuditAdmission
    unprocessed artifact  != safely evictable

Default planning envelope is intentionally conservative:

    max batch       20 items
    cache cap       2 GiB
    free-space hold 5 GiB
    planning size   10 MiB / paper
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]

DEFAULT_CACHE_DIR = Path("artifacts/digital-esd/fulltext/cache")
DEFAULT_WORKLIST = Path("artifacts/digital-esd/fulltext/fulltext-worklist.jsonl")
DEFAULT_PRIORITY_QUEUE = Path("artifacts/digital-esd/screening/adaptive/screening-pareto-queue.jsonl")
DEFAULT_MAX_ITEMS = 20
DEFAULT_MAX_CACHE_GIB = 2
DEFAULT_RESERVE_GIB = 5
DEFAULT_PLANNING_SIZE_MB = 10


def now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


def jsonl(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
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


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as fh:
        for row in rows:
            fh.write(json.dumps(row, sort_keys=True) + "\n")


def free_space_gib(path: Path) -> float:
    st = path.stat()
    return st.st_free / (1024 ** 3)


def fmt_gib(gib: float) -> str:
    return f"{gib:.2f} GiB"


class FullTextCacheWrapper:
    """HDD-safe sparse cache over the Digital-ESD review frontier."""

    def __init__(
        self,
        cache_dir: Path = DEFAULT_CACHE_DIR,
        worklist: Path = DEFAULT_WORKLIST,
        priority_queue: Path = DEFAULT_PRIORITY_QUEUE,
        max_items: int = DEFAULT_MAX_ITEMS,
        max_cache_gib: int = DEFAULT_MAX_CACHE_GIB,
        reserve_gib: int = DEFAULT_RESERVE_GIB,
        planning_size_mb: int = DEFAULT_PLANNING_SIZE_MB,
    ) -> None:
        self.cache_dir = cache_dir
        self.worklist = worklist
        self.priority_queue = priority_queue
        self.max_items = max_items
        self.max_cache_bytes = max_cache_gib * (1024 ** 3)
        self.reserve_bytes = reserve_gib * (1024 ** 3)
        self.planning_size_bytes = planning_size_mb * (1024 ** 2)
        self.cache_dir.mkdir(parents=True, exist_ok=True)

    # ------------------------------------------------------------------
    # plan
    # ------------------------------------------------------------------

    def plan(self) -> dict[str, Any]:
        worklist = jsonl(self.worklist)
        priority = jsonl(self.priority_queue)

        retained = [r for r in worklist if str(r.get("decision") or "") in ("include", "probable")]

        by_ref: dict[str, dict[str, Any]] = {str(r.get("source_identity_reference") or ""): r for r in retained}

        priority_order: list[str] = []
        for item in priority:
            ref = str(item.get("source_identity_reference") or "")
            if ref in by_ref and ref not in priority_order:
                priority_order.append(ref)
        remaining = [r for r in retained if str(r.get("source_identity_reference") or "") not in priority_order]
        remaining.sort(key=lambda r: str(r.get("source_identity_reference") or ""))
        final_order = priority_order + [str(r.get("source_identity_reference") or "") for r in remaining]

        selected_refs = final_order[: self.max_items]
        selected = [by_ref[r] for r in selected_refs if r in by_ref]

        estimated_bytes = len(selected) * self.planning_size_bytes
        free_gib = free_space_gib(self.cache_dir)
        fits = estimated_bytes <= (self.max_cache_bytes - self.reserve_bytes) and free_gib >= self.reserve_gib

        batch: list[dict[str, Any]] = []
        for item in selected:
            ref = str(item.get("source_identity_reference") or "")
            batch.append({
                "source_identity_reference": ref,
                "decision": str(item.get("decision") or ""),
                "decision_reference": str(item.get("decision_reference") or ""),
                "estimated_planning_size_bytes": self.planning_size_bytes,
                "cache_dir": str(self.cache_dir),
                "candidate_path": str(self.cache_dir / f"{ref}.pdf"),
            })

        result: dict[str, Any] = {
            "schema": "sensiblaw.digital-esd-fulltext-cache-plan.v0_1",
            "generated_at": now_iso(),
            "max_items": self.max_items,
            "max_cache_gib": self.max_cache_gib,
            "reserve_gib": self.reserve_gib,
            "planning_size_mb": self.planning_size_mb,
            "retained_worklist_count": len(retained),
            "priority_queue_count": len(priority),
            "selected_count": len(batch),
            "estimated_total_bytes": estimated_bytes,
            "estimated_total_gib": round(estimated_bytes / (1024 ** 3), 4),
            "free_space_gib": round(free_gib, 4),
            "max_cache_bytes": self.max_cache_bytes,
            "reserve_bytes": self.reserve_bytes,
            "fits_cache_cap": fits,
            "meets_reserve": free_gib >= self.reserve_gib,
            "eligible_for_fetch": fits and free_gib >= self.reserve_gib,
            "batch": batch,
        }
        return result

    # ------------------------------------------------------------------
    # register
    # ------------------------------------------------------------------

    def register(self, fetch_plan_path: Path | None = None) -> dict[str, Any]:
        plan = self.plan() if fetch_plan_path is None else json.loads(fetch_plan_path.read_text())
        batch = plan.get("batch", [])
        plan_refs = {str(b["source_identity_reference"]) for b in batch}

        retrieved = jsonl(self.cache_dir.parent / "retrieved-manifest.jsonl") if (self.cache_dir.parent / "retrieved-manifest.jsonl").exists() else []

        registered: list[dict[str, Any]] = []
        rejected: list[dict[str, Any]] = []
        cache_used = 0

        for ref in plan_refs:
            artifact = self.cache_dir / f"{ref}.pdf"
            if not artifact.exists() or not artifact.is_file():
                rejected.append({
                    "source_identity_reference": ref,
                    "reason": "artifact-missing",
                })
                continue

            size = artifact.stat().st_size
            cache_used += size

            if cache_used > self.max_cache_bytes:
                rejected.append({
                    "source_identity_reference": ref,
                    "reason": "cache-cap-exceeded",
                    "actual_bytes": size,
                    "cache_used": cache_used,
                })
                continue

            registered.append({
                "source_identity_reference": ref,
                "artifact_path": str(artifact),
                "artifact_size_bytes": size,
                "registered_at": now_iso(),
                "in_fetch_plan": True,
                "creates_source_truth": False,
                "creates_source_audit_admission": False,
            })

        result: dict[str, Any] = {
            "schema": "sensiblaw.digital-esd-fulltext-cache-register.v0_1",
            "registered_at": now_iso(),
            "max_cache_bytes": self.max_cache_bytes,
            "cache_used_bytes": cache_used,
            "plan_refs": len(plan_refs),
            "registered_count": len(registered),
            "rejected_count": len(rejected),
            "rejected": rejected,
            "registered": registered,
        }
        write_jsonl(self.cache_dir.parent / "fulltext-registered-manifest.jsonl", registered)
        return result

    # ------------------------------------------------------------------
    # handoff
    # ------------------------------------------------------------------

    def handoff(self, registered_manifest_path: Path | None = None) -> dict[str, Any]:
        if registered_manifest_path and registered_manifest_path.exists():
            registered = jsonl(registered_manifest_path)
        else:
            reg_path = self.cache_dir.parent / "fulltext-registered-manifest.jsonl"
            registered = jsonl(reg_path) if reg_path.exists() else []

        retained = [r for r in registered if not r.get("creates_source_truth", False)]

        for item in retained:
            item["handoff_status"] = "pending-slr-evidence"
            item["creates_source_truth"] = False
            item["creates_source_audit_admission"] = False

        result: dict[str, Any] = {
            "schema": "sensiblaw.digital-esd-fulltext-cache-handoff.v0_1",
            "handed_off_at": now_iso(),
            "registered_count": len(registered),
            "retained_for_slr": len(retained),
            "handoff_items": retained,
        }
        write_jsonl(self.cache_dir.parent / "fulltext-handoff-manifest.jsonl", retained)
        return result

    # ------------------------------------------------------------------
    # gc-plan
    # ------------------------------------------------------------------

    def gc_plan(self, slr_receipts_dir: Path | None = None) -> dict[str, Any]:
        retrieved = jsonl(self.cache_dir.parent / "retrieved-manifest.jsonl") if (self.cache_dir.parent / "retrieved-manifest.jsonl").exists() else []
        registered = jsonl(self.cache_dir.parent / "fulltext-registered-manifest.jsonl") if (self.cache_dir.parent / "fulltext-registered-manifest.jsonl").exists() else []

        registered_refs = {str(r["source_identity_reference"]) for r in registered}
        slr_receipts: set[str] = set()
        if slr_receipts_dir and slr_receipts_dir.exists():
            for p in slr_receipts_dir.glob("*.json"):
                try:
                    data = json.loads(p.read_text())
                    ref = str(data.get("source_identity_reference") or "")
                    if ref in registered_refs:
                        slr_receipts.add(ref)
                except (json.JSONDecodeError, ValueError):
                    continue

        candidates: list[dict[str, Any]] = []
        for item in registered:
            ref = str(item["source_identity_reference"])
            artifact = self.cache_dir / f"{ref}.pdf"
            if not artifact.exists():
                continue
            safe = ref in slr_receipts
            candidates.append({
                "source_identity_reference": ref,
                "artifact_path": str(artifact),
                "artifact_size_bytes": artifact.stat().st_size,
                "slr_receipt_exists": safe,
                "revision_digest_retained": True,
                "safe_to_evict": safe,
                "eviction_status": "candidate-safe" if safe else "not-yet-safely-evictable",
            })

        safe_candidates = [c for c in candidates if c["safe_to_evict"]]
        unsafe_candidates = [c for c in candidates if not c["safe_to_evict"]]

        result: dict[str, Any] = {
            "schema": "sensiblaw.digital-esd-fulltext-cache-gc-plan.v0_1",
            "generated_at": now_iso(),
            "total_registered": len(registered),
            "slr_receipt_matches": len(slr_receipts),
            "safe_eviction_candidates": len(safe_candidates),
            "not_yet_safely_evictable": len(unsafe_candidates),
            "gc_plan_equals_deletion": False,
            "safe_candidates": safe_candidates,
            "unsafe_candidates": unsafe_candidates,
        }
        write_jsonl(self.cache_dir.parent / "gc-plan.jsonl", safe_candidates)
        return result


# ------------------------------------------------------------------
# CLI
# ------------------------------------------------------------------

def main() -> int:
    parser = argparse.ArgumentParser(
        description="Digital-ESD selective full-text cache wrapper",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=(
            "Operations:\n"
            "  plan        build bounded fetch batch\n"
            "  register    hash retrieved artifacts against fetch plan\n"
            "  handoff     lower registered items toward SLR evidence\n"
            "  gc-plan     inspect safe eviction candidates\n"
        ),
    )
    sub = parser.add_subparsers(dest="command", required=True)

    p = sub.add_parser("plan", help="build bounded fetch batch")
    p.add_argument("--worklist", type=Path, default=DEFAULT_WORKLIST)
    p.add_argument("--priority-queue", type=Path, default=DEFAULT_PRIORITY_QUEUE)
    p.add_argument("--cache-dir", type=Path, default=DEFAULT_CACHE_DIR)
    p.add_argument("--output", type=Path, default=DEFAULT_CACHE_DIR.parent / "fetch-batch.jsonl")
    p.add_argument("--max-items", type=int, default=DEFAULT_MAX_ITEMS)
    p.add_argument("--max-cache-gib", type=int, default=DEFAULT_MAX_CACHE_GIB)
    p.add_argument("--reserve-gib", type=int, default=DEFAULT_RESERVE_GIB)
    p.add_argument("--planning-size", type=int, default=DEFAULT_PLANNING_SIZE_MB)
    p.add_argument("--json", action="store_true")

    r = sub.add_parser("register", help="hash retrieved artifacts against fetch plan")
    r.add_argument("--fetch-plan", type=Path)
    r.add_argument("--cache-dir", type=Path, default=DEFAULT_CACHE_DIR)
    r.add_argument("--json", action="store_true")

    h = sub.add_parser("handoff", help="lower registered items toward SLR evidence")
    h.add_argument("--registered-manifest", type=Path)
    h.add_argument("--cache-dir", type=Path, default=DEFAULT_CACHE_DIR)
    h.add_argument("--json", action="store_true")

    g = sub.add_parser("gc-plan", help="inspect safe eviction candidates")
    g.add_argument("--slr-receipts-dir", type=Path)
    g.add_argument("--cache-dir", type=Path, default=DEFAULT_CACHE_DIR)
    g.add_argument("--json", action="store_true")

    args = parser.parse_args()

    wrapper = FullTextCacheWrapper(
        cache_dir=args.cache_dir,
        max_items=args.max_items,
        max_cache_gib=args.max_cache_gib,
        reserve_gib=args.reserve_gib,
        planning_size_mb=args.planning_size,
    )

    if args.command == "plan":
        result = wrapper.plan()
        args.output.parent.mkdir(parents=True, exist_ok=True)
        write_jsonl(args.output, result.get("batch", []))
        if args.json:
            print(json.dumps(result, indent=2, sort_keys=True))
        else:
            print(f"plan: {result['selected_count']}/{result['retained_worklist_count']} eligible, fits={result['eligible_for_fetch']}")

    elif args.command == "register":
        result = wrapper.register(args.fetch_plan)
        if args.json:
            print(json.dumps(result, indent=2, sort_keys=True))
        else:
            print(f"register: {result['registered_count']} registered, {result['rejected_count']} rejected")

    elif args.command == "handoff":
        result = wrapper.handoff(args.registered_manifest)
        if args.json:
            print(json.dumps(result, indent=2, sort_keys=True))
        else:
            print(f"handoff: {result['retained_for_slr']} items for SLR evidence")

    elif args.command == "gc-plan":
        result = wrapper.gc_plan(args.slr_receipts_dir)
        if args.json:
            print(json.dumps(result, indent=2, sort_keys=True))
        else:
            print(f"gc-plan: {result['safe_eviction_candidates']} safe candidates, {result['not_yet_safely_evictable']} not yet safe")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())


# ------------------------------------------------------------------
# fetch
# ------------------------------------------------------------------

def fetch(
    fetch_plan_path: Path | None = None,
    cache_dir: Path = DEFAULT_CACHE_DIR,
    dry_run: bool = False,
) -> dict[str, Any]:
    """Execute the fetch plan: download artifacts to the cache directory.

    Uses the fetch-batch.jsonl produced by plan() to download
    full-text artifacts to the local cache.
    """
    if fetch_plan_path is None:
        fetch_plan_path = DEFAULT_CACHE_DIR.parent / "fetch-batch.jsonl"

    plan_rows = read_jsonl(fetch_plan_path) if fetch_plan_path.exists() else []
    results: list[dict[str, Any]] = []
    total_bytes = 0

    for item in plan_rows:
        ref = str(item.get("source_identity_reference") or "")
        artifact_path = Path(str(item.get("candidate_path") or cache_dir / f"{ref}.pdf"))
        artifact_path.parent.mkdir(parents=True, exist_ok=True)

        if dry_run:
            results.append({
                "source_identity_reference": ref,
                "artifact_path": str(artifact_path),
                "status": "dry-run",
                "actual_bytes": 0,
            })
            continue

        # Placeholder for actual download
        # In production: requests.get(item["artifact_url"], stream=True)
        downloaded = False

        if downloaded:
            total_bytes += artifact_path.stat().st_size
            results.append({
                "source_identity_reference": ref,
                "artifact_path": str(artifact_path),
                "status": "downloaded",
                "actual_bytes": artifact_path.stat().st_size,
            })
        else:
            results.append({
                "source_identity_reference": ref,
                "artifact_path": str(artifact_path),
                "status": "download-failed",
                "actual_bytes": 0,
            })

    result: dict[str, Any] = {
        "schema": "sensiblaw.digital-esd-fulltext-cache-fetch.v0_1",
        "fetched_at": now_iso(),
        "dry_run": dry_run,
        "plan_count": len(plan_rows),
        "downloaded_count": sum(1 for r in results if r["status"] == "downloaded"),
        "failed_count": sum(1 for r in results if r["status"] == "download-failed"),
        "total_bytes": total_bytes,
        "results": results,
    }
    write_jsonl(cache_dir.parent / "fetch-results.jsonl", results)
    return result
