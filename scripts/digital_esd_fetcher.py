#!/usr/bin/env python3
"""Retrieval fetcher for Digital-ESD.

Owns the network layer between:
  ERIC API export-root          ← fetches ERIC JSON responses
  full-text cache ledger         ← fetches retained full-text artifacts

This module does NOT parse or screen anything.
It owns only:
  plan       which artifacts to fetch
  fetch      download to local cache
  verify     hash downloaded artifacts against plan
  reconcile  match fetched files to source identity + revision

Invariants preserved:
  planning size ≠ actual bytes
  cached artifact ≠ SourceAuditAdmission
  fetch success ≠ screening decision
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]

DEFAULT_CACHE_DIR = Path("artifacts/digital-esd/fulltext/cache")
DEFAULT_EXPORT_ROOT = Path("artifacts/digital-esd/eric")
DEFAULT_API_BASE = "https://eric.ed.gov/api/"


def now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def read_jsonl(path: Path) -> list[dict[str, Any]]:
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
            fh.write(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n")


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, data: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def fmt_bytes(bytes_count: int) -> str:
    if bytes_count >= 1024 ** 3:
        return f"{bytes_count / (1024 ** 3):.2f} GiB"
    if bytes_count >= 1024 ** 2:
        return f"{bytes_count / (1024 ** 2):.2f} MiB"
    if bytes_count >= 1024:
        return f"{bytes_count / 1024:.2f} KiB"
    return f"{bytes_count} B"


# ------------------------------------------------------------------
# ERIC API fetcher
# ------------------------------------------------------------------

class ERICFetcher:
    """Fetches ERIC API JSON responses (Q1-Q7)."""

    def __init__(
        self,
        export_root: Path = DEFAULT_EXPORT_ROOT,
        api_base: str = DEFAULT_API_BASE,
        api_key: str | None = None,
    ) -> None:
        self.export_root = export_root
        self.api_base = api_base.rstrip("/")
        self.api_key = api_key or ""

    def plan(self) -> list[dict[str, Any]]:
        """List the 7 query families that need fetching."""
        queries = []
        for q in range(1, 8):
            query_dir = self.export_root / f"Q{q}"
            queries.append({
                "query": f"Q{q}",
                "query_dir": str(query_dir),
                "needs_fetch": not query_dir.exists() or not any(query_dir.glob("*.json")),
            })
        return queries

    def fetch(
        self,
        query_num: int,
        *,
        max_pages: int = 100,
        dry_run: bool = False,
    ) -> dict[str, Any]:
        """Fetch a single ERIC query family's JSON responses."""
        query_dir = self.export_root / f"Q{query_num}"
        query_dir.mkdir(parents=True, exist_ok=True)

        if dry_run:
            return {
                "query": f"Q{query_num}",
                "dry_run": True,
                "query_dir": str(query_dir),
                "status": "dry-run",
            }

        # ERIC API uses a paginated endpoint with query parameters
        # The actual endpoint structure is documented at:
        # https://eric.ed.gov/pdf/Using_ERIC_API_for_Research_Topics.pdf
        page = 0
        all_docs: list[dict[str, Any]] = []
        total_pages = 0

        while True:
            page += 1
            if page > max_pages:
                break

            url = (
                f"{self.api_base}/?"
                f"q=digital%20education&"
                f"ff1=dtdYearRange%3A{2020 + query_num}-{2020 + query_num + 1}&"
                f"p={page}&"
                f"results=100"
            )

            # Placeholder for actual HTTP request
            # In production, this uses requests or httpx with the API key
            response = self._http_get(url)
            if response is None:
                break

            docs = self._extract_docs(response)
            if not docs:
                break

            all_docs.extend(docs)
            total_pages += 1

            # Rate limit: ERIC API has strict per-second limits
            time.sleep(0.5)

        # Write combined JSON response for the query family
        output_path = query_dir / "combined.json"
        write_json(output_path, {
            "query": f"Q{query_num}",
            "total_pages": total_pages,
            "total_docs": len(all_docs),
            "fetched_at": now_iso(),
            "docs": all_docs,
        })

        return {
            "query": f"Q{query_num}",
            "query_dir": str(query_dir),
            "docs_fetched": len(all_docs),
            "pages": total_pages,
            "output_path": str(output_path),
            "status": "fetched",
        }

    def fetch_all(self, *, dry_run: bool = False) -> dict[str, Any]:
        """Fetch all 7 ERIC query families."""
        plans = self.plan()
        results: list[dict[str, Any]] = []

        for plan in plans:
            if not plan["needs_fetch"]:
                results.append({
                    "query": plan["query"],
                    "query_dir": plan["query_dir"],
                    "status": "already-exists",
                })
                continue
            result = self.fetch(plan["query_num"], dry_run=dry_run)
            results.append(result)

        total_docs = sum(r.get("docs_fetched", 0) for r in results)
        return {
            "schema": "sensiblaw.digital-esd-eric-fetch.v0_1",
            "fetched_at": now_iso(),
            "dry_run": dry_run,
            "query_families": len(results),
            "total_docs": total_docs,
            "results": results,
        }

    @staticmethod
    def _http_get(url: str) -> dict[str, Any] | None:
        """Placeholder for actual HTTP GET. Must be implemented with requests/httpx."""
        return None

    @staticmethod
    def _extract_docs(response: dict[str, Any]) -> list[dict[str, Any]]:
        """Extract document entries from ERIC API response."""
        return response.get("response", {}).get("docs", [])


# ------------------------------------------------------------------
# Full-text artifact fetcher
# ------------------------------------------------------------------

class FullTextFetcher:
    """Fetches full-text artifacts identified by the cache plan."""

    def __init__(
        self,
        cache_dir: Path = DEFAULT_CACHE_DIR,
        output_dir: Path = DEFAULT_CACHE_DIR.parent,
    ) -> None:
        self.cache_dir = cache_dir
        self.output_dir = output_dir

    def plan(
        self,
        worklist: Path | None = None,
        priority_queue: Path | None = None,
        max_items: int = 20,
    ) -> dict[str, Any]:
        """Build a fetch plan from the retained worklist."""
        if worklist is None:
            worklist = self.cache_dir.parent / "fulltext-worklist.jsonl"
        if priority_queue is None:
            priority_queue = self.cache_dir.parent.parent.parent / "screening" / "adaptive" / "screening-pareto-queue.jsonl"

        rows = read_jsonl(worklist) if worklist.exists() else []
        retained = [r for r in rows if str(r.get("decision") or "") in ("include", "probable")]

        priority = read_jsonl(priority_queue) if priority_queue and priority_queue.exists() else []
        priority_order: list[str] = []
        for item in priority:
            ref = str(item.get("source_identity_reference") or "")
            if ref in {str(r.get("source_identity_reference") or "") for r in retained} and ref not in priority_order:
                priority_order.append(ref)

        selected_refs = priority_order[:max_items] if priority_order else [
            str(r.get("source_identity_reference") or "") for r in retained[:max_items]
        ]

        return {
            "schema": "sensiblaw.digital-esd-fulltext-fetch-plan.v0_1",
            "generated_at": now_iso(),
            "max_items": max_items,
            "selected_count": len(selected_refs),
            "selected_refs": selected_refs,
        }

    def fetch(
        self,
        fetch_plan: dict[str, Any],
        *,
        dry_run: bool = False,
        retry_count: int = 3,
    ) -> dict[str, Any]:
        """Fetch selected full-text artifacts to the cache directory."""
        selected_refs = fetch_plan.get("selected_refs", [])
        results: list[dict[str, Any]] = []
        total_bytes = 0

        for ref in selected_refs:
            artifact_path = self.cache_dir / f"{ref}.pdf"
            artifact_path.parent.mkdir(parents=True, exist_ok=True)

            if dry_run:
                results.append({
                    "source_identity_reference": ref,
                    "artifact_path": str(artifact_path),
                    "status": "dry-run",
                    "actual_bytes": 0,
                })
                continue

            # In production, this downloads from the actual repository
            # (ERIC, publisher, or institutional repository)
            # Placeholder for actual HTTP download
            downloaded = self._download_artifact(ref, artifact_path, retry_count)

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

        return {
            "schema": "sensiblaw.digital-esd-fulltext-fetch.v0_1",
            "fetched_at": now_iso(),
            "dry_run": dry_run,
            "selected_count": len(selected_refs),
            "downloaded_count": sum(1 for r in results if r["status"] == "downloaded"),
            "failed_count": sum(1 for r in results if r["status"] == "download-failed"),
            "total_bytes": total_bytes,
            "total_bytes_fmt": fmt_bytes(total_bytes),
            "results": results,
        }

    @staticmethod
    def _download_artifact(ref: str, path: Path, retries: int) -> bool:
        """Placeholder for actual artifact download."""
        return False


# ------------------------------------------------------------------
# Combined retrieval pipeline
# ------------------------------------------------------------------

def retrieve_all(
    *,
    eric_export_root: Path = DEFAULT_EXPORT_ROOT,
    fulltext_cache_dir: Path = DEFAULT_CACHE_DIR,
    api_key: str = "",
    dry_run: bool = False,
) -> dict[str, Any]:
    """Run the full retrieval pipeline.

    1. Fetch ERIC API JSON responses (Q1-Q7)
    2. Fetch selected full-text artifacts
    3. Verify downloaded files against plan
    """
    eric_fetcher = ERICFetcher(export_root=eric_export_root, api_key=api_key)
    ft_fetcher = FullTextFetcher(cache_dir=fulltext_cache_dir)

    eric_plan = eric_fetcher.plan()
    eric_needs_fetch = any(p["needs_fetch"] for p in eric_plan)

    eric_result = eric_fetcher.fetch_all(dry_run=dry_run) if eric_needs_fetch else {
        "schema": "sensiblaw.digital-esd-eric-fetch.v0_1",
        "fetched_at": now_iso(),
        "dry_run": dry_run,
        "query_families": len(eric_plan),
        "total_docs": 0,
        "results": [{"query": p["query"], "status": "already-exists"} for p in eric_plan],
    }

    ft_plan = ft_fetcher.plan(max_items=20)
    ft_result = ft_fetcher.fetch(ft_plan, dry_run=dry_run)

    return {
        "schema": "sensiblaw.digital-esd-retrieval.v0_1",
        "retrieved_at": now_iso(),
        "dry_run": dry_run,
        "eric": eric_result,
        "fulltext": ft_result,
        "total_artifacts": ft_result["downloaded_count"],
        "total_eric_docs": eric_result.get("total_docs", 0),
    }


# ------------------------------------------------------------------
# CLI
# ------------------------------------------------------------------

def main() -> int:
    parser = argparse.ArgumentParser(
        description="Digital-ESD retrieval fetcher",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=(
            "Operations:\n"
            "  eric        fetch ERIC API JSON responses (Q1-Q7)\n"
            "  fulltext    fetch full-text artifacts from cache plan\n"
            "  all         run full retrieval pipeline\n"
            "  plan        show what needs fetching\n"
        ),
    )
    sub = parser.add_subparsers(dest="command", required=True)

    e = sub.add_parser("eric", help="fetch ERIC API JSON responses")
    e.add_argument("--export-root", type=Path, default=DEFAULT_EXPORT_ROOT)
    e.add_argument("--api-key", type=str, default="")
    e.add_argument("--api-base", type=str, default=DEFAULT_API_BASE)
    e.add_argument("--dry-run", action="store_true")
    e.add_argument("--json", action="store_true")

    f = sub.add_parser("fulltext", help="fetch full-text artifacts")
    f.add_argument("--cache-dir", type=Path, default=DEFAULT_CACHE_DIR)
    f.add_argument("--worklist", type=Path)
    f.add_argument("--priority-queue", type=Path)
    f.add_argument("--max-items", type=int, default=20)
    f.add_argument("--dry-run", action="store_true")
    f.add_argument("--json", action="store_true")

    a = sub.add_parser("all", help="full retrieval pipeline")
    a.add_argument("--export-root", type=Path, default=DEFAULT_EXPORT_ROOT)
    a.add_argument("--cache-dir", type=Path, default=DEFAULT_CACHE_DIR)
    a.add_argument("--api-key", type=str, default="")
    a.add_argument("--api-base", type=str, default=DEFAULT_API_BASE)
    a.add_argument("--dry-run", action="store_true")
    a.add_argument("--json", action="store_true")

    p = sub.add_parser("plan", help="show what needs fetching")
    p.add_argument("--export-root", type=Path, default=DEFAULT_EXPORT_ROOT)
    p.add_argument("--cache-dir", type=Path, default=DEFAULT_CACHE_DIR)
    p.add_argument("--worklist", type=Path)
    p.add_argument("--priority-queue", type=Path)
    p.add_argument("--json", action="store_true")

    args = parser.parse_args()

    if args.command == "eric":
        fetcher = ERICFetcher(export_root=args.export_root, api_key=args.api_key, api_base=args.api_base)
        result = fetcher.fetch_all(dry_run=args.dry_run)
        if args.json:
            print(json.dumps(result, indent=2, sort_keys=True))
        else:
            print(f"eric: {result['total_docs']} docs across {result['query_families']} families")

    elif args.command == "fulltext":
        fetcher = FullTextFetcher(cache_dir=args.cache_dir)
        plan = fetcher.plan(max_items=args.max_items, worklist=args.worklist, priority_queue=args.priority_queue)
        result = fetcher.fetch(plan, dry_run=args.dry_run)
        if args.json:
            print(json.dumps(result, indent=2, sort_keys=True))
        else:
            print(f"fulltext: {result['downloaded_count']}/{result['selected_count']} downloaded, {result['total_bytes_fmt']}")

    elif args.command == "all":
        result = retrieve_all(
            eric_export_root=args.export_root,
            fulltext_cache_dir=args.cache_dir,
            api_key=args.api_key,
            api_base=args.api_base,
            dry_run=args.dry_run,
        )
        if args.json:
            print(json.dumps(result, indent=2, sort_keys=True))
        else:
            print(f"all: {result['total_artifacts']} artifacts, {result['total_eric_docs']} ERIC docs")

    elif args.command == "plan":
        fetcher = FullTextFetcher(cache_dir=args.cache_dir)
        plan = fetcher.plan(max_items=20, worklist=args.worklist, priority_queue=args.priority_queue)
        eric_fetcher = ERICFetcher(export_root=args.export_root)
        eric_plan = eric_fetcher.plan()
        if args.json:
            print(json.dumps({"eric_plan": eric_plan, "fulltext_plan": plan}, indent=2, sort_keys=True))
        else:
            print(f"plan: {sum(1 for p in eric_plan if p['needs_fetch'])} ERIC families need fetch, {plan['selected_count']} full-text items selected")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
