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
import urllib.error
import urllib.parse
import urllib.request
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]

DEFAULT_CACHE_DIR = Path("artifacts/digital-esd/fulltext/cache")
DEFAULT_EXPORT_ROOT = Path("artifacts/digital-esd/eric")
DEFAULT_API_BASE = "https://api.ies.ed.gov/eric/"
DEFAULT_QUERY_CONFIG = Path("fixtures/digital_esd_eric_queries.json")
DEFAULT_REQUEST_INTERVAL_SECONDS = 4.0
DEFAULT_TIMEOUT_SECONDS = 45.0
DEFAULT_ROWS = 200
DEFAULT_ALLOWED_FULLTEXT_HOSTS = ("files.eric.ed.gov",)


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


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def load_eric_query_config(path: Path = DEFAULT_QUERY_CONFIG) -> dict[str, Any]:
    payload = read_json(path)
    families = payload.get("query_families")
    if not isinstance(families, dict):
        raise ValueError(f"{path}: missing query_families object")
    expected = {f"Q{n}" for n in range(1, 8)}
    if set(families) != expected:
        raise ValueError(
            f"{path}: expected exactly Q1..Q7, got {sorted(families)}"
        )
    for name, query in families.items():
        if not isinstance(query, str) or not query.strip():
            raise ValueError(f"{path}: {name} query is empty")
    return payload


def _safe_http_json(url: str, *, timeout: float) -> dict[str, Any]:
    request = urllib.request.Request(
        url,
        headers={
            "User-Agent": "SensibLaw-DigitalESD/1.0 (+https://github.com/chboishabba/slr)",
            "Accept": "application/json",
        },
    )
    with urllib.request.urlopen(request, timeout=timeout) as response:
        raw = response.read()
    payload = json.loads(raw.decode("utf-8"))
    if not isinstance(payload, dict):
        raise ValueError("HTTP JSON response is not an object")
    return payload


# ------------------------------------------------------------------
# ERIC API fetcher
# ------------------------------------------------------------------

class ERICFetcher:
    """Fetch the seven frozen ERIC query families through the public API.

    Network access is opt-in.  The authoritative on-disk representation is the
    retained page/summary layout already consumed by digital_esd_eric.py.
    """

    def __init__(
        self,
        export_root: Path = DEFAULT_EXPORT_ROOT,
        api_base: str = DEFAULT_API_BASE,
        api_key: str | None = None,
        *,
        query_config: Path = DEFAULT_QUERY_CONFIG,
        network_enabled: bool = False,
        request_interval_seconds: float = DEFAULT_REQUEST_INTERVAL_SECONDS,
        timeout_seconds: float = DEFAULT_TIMEOUT_SECONDS,
        rows: int = DEFAULT_ROWS,
    ) -> None:
        self.export_root = export_root
        self.api_base = api_base.rstrip("/") + "/"
        # Kept only for CLI/backward compatibility.  The documented public ERIC
        # API path used here does not require or transmit a secret API key.
        self.api_key = api_key or ""
        self.query_config_path = query_config
        self.query_config = load_eric_query_config(query_config)
        self.network_enabled = network_enabled
        self.request_interval_seconds = max(0.0, request_interval_seconds)
        self.timeout_seconds = timeout_seconds
        self.rows = rows
        if self.rows < 20 or self.rows > 200:
            raise ValueError("ERIC rows must be in [20, 200]")

    def _query(self, query_num: int) -> str:
        key = f"Q{query_num}"
        try:
            return str(self.query_config["query_families"][key])
        except KeyError as exc:
            raise ValueError(f"unknown ERIC query family {key}") from exc

    def plan(self) -> list[dict[str, Any]]:
        queries: list[dict[str, Any]] = []
        for q in range(1, 8):
            query_dir = self.export_root / f"Q{q}"
            summary = query_dir / "summary.json"
            pages = sorted(query_dir.glob("page-*.json")) if query_dir.exists() else []
            complete = False
            if summary.exists() and pages:
                try:
                    summary_payload = read_json(summary)
                    complete = bool(summary_payload.get("pagination_complete"))
                except (OSError, ValueError, json.JSONDecodeError):
                    complete = False
            query = self._query(q)
            queries.append({
                "query_num": q,
                "query": f"Q{q}",
                "canonical_search": query,
                "canonical_search_sha256": sha256_bytes(query.encode("utf-8")),
                "query_dir": str(query_dir),
                "needs_fetch": not complete,
            })
        return queries

    def _request_page(self, query: str, *, start: int) -> tuple[dict[str, Any], str]:
        if not self.network_enabled:
            raise RuntimeError("live ERIC network disabled; pass --live explicitly")
        params = {
            "search": query,
            "rows": self.rows,
            "format": "json",
            "start": start,
        }
        url = self.api_base + "?" + urllib.parse.urlencode(params)
        return _safe_http_json(url, timeout=self.timeout_seconds), url

    def fetch(
        self,
        query_num: int,
        *,
        max_pages: int = 1000,
        dry_run: bool = False,
    ) -> dict[str, Any]:
        query = self._query(query_num)
        query_dir = self.export_root / f"Q{query_num}"

        if dry_run:
            return {
                "query": f"Q{query_num}",
                "dry_run": True,
                "query_dir": str(query_dir),
                "canonical_search": query,
                "canonical_search_sha256": sha256_bytes(query.encode("utf-8")),
                "status": "dry-run",
            }

        if not self.network_enabled:
            return {
                "query": f"Q{query_num}",
                "query_dir": str(query_dir),
                "canonical_search": query,
                "status": "network-disabled",
                "docs_fetched": 0,
                "pages": 0,
            }

        query_dir.mkdir(parents=True, exist_ok=True)
        start = 0
        page_index = 0
        num_found: int | None = None
        fetched = 0
        page_receipts: list[dict[str, Any]] = []
        started_at = now_iso()

        while num_found is None or fetched < num_found:
            if page_index >= max_pages:
                raise RuntimeError(
                    f"Q{query_num}: pagination exceeded max_pages={max_pages}"
                )
            try:
                payload, url = self._request_page(query, start=start)
            except (urllib.error.URLError, urllib.error.HTTPError, TimeoutError) as exc:
                raise RuntimeError(
                    f"Q{query_num}: ERIC request failed at start={start}: {exc}"
                ) from exc

            response = payload.get("response")
            if not isinstance(response, dict):
                raise ValueError(f"Q{query_num}: response object missing")
            observed = response.get("numFound")
            docs = response.get("docs")
            if not isinstance(observed, int):
                raise ValueError(f"Q{query_num}: numFound missing/non-integer")
            if not isinstance(docs, list):
                raise ValueError(f"Q{query_num}: docs missing/non-list")
            if num_found is None:
                num_found = observed
            elif observed != num_found:
                raise RuntimeError(
                    f"Q{query_num}: numFound drift during pagination "
                    f"{num_found} -> {observed}"
                )

            raw = (
                json.dumps(payload, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
                + "\n"
            ).encode("utf-8")
            page_path = query_dir / f"page-{page_index:06d}.json"
            page_path.write_bytes(raw)
            page_receipts.append({
                "page_index": page_index,
                "start": start,
                "rows_requested": self.rows,
                "docs_returned": len(docs),
                "request_url": url,
                "path": str(page_path),
                "sha256": sha256_bytes(raw),
            })

            fetched += len(docs)
            if not docs:
                break
            start += len(docs)
            page_index += 1
            if fetched < num_found and self.request_interval_seconds:
                time.sleep(self.request_interval_seconds)

        pagination_complete = num_found is not None and fetched >= num_found
        if not pagination_complete:
            raise RuntimeError(
                f"Q{query_num}: pagination incomplete fetched={fetched} numFound={num_found}"
            )

        historical = self.query_config.get(
            "historical_observed_counts_2026_09_19", {}
        ).get(f"Q{query_num}")

        summary = {
            "schema": "sensiblaw.digital-esd-eric-export-summary.v1",
            "query_id": f"Q{query_num}",
            "canonical_unencoded_query": query,
            "canonical_search_sha256": sha256_bytes(query.encode("utf-8")),
            "endpoint": self.api_base,
            "format": "json",
            "rows_requested": self.rows,
            "execution_started": started_at,
            "execution_completed": now_iso(),
            "numFound": num_found,
            "fetched_docs": fetched,
            "pagination_complete": True,
            "historical_numFound_2026_09_19": historical,
            "historical_count_matches": historical == num_found if historical is not None else None,
            "pages": page_receipts,
            "api_key_used": False,
            "candidate_only": True,
            "creates_source_truth": False,
            "creates_source_audit_admission": False,
        }
        summary_path = query_dir / "summary.json"
        write_json(summary_path, summary)

        return {
            "query": f"Q{query_num}",
            "query_dir": str(query_dir),
            "docs_fetched": fetched,
            "pages": len(page_receipts),
            "numFound": num_found,
            "summary_path": str(summary_path),
            "status": "fetched",
        }

    def fetch_all(self, *, dry_run: bool = False) -> dict[str, Any]:
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
            results.append(self.fetch(plan["query_num"], dry_run=dry_run))

        total_docs = sum(r.get("docs_fetched", 0) for r in results)
        return {
            "schema": "sensiblaw.digital-esd-eric-fetch.v0_2",
            "fetched_at": now_iso(),
            "dry_run": dry_run,
            "network_enabled": self.network_enabled,
            "query_families": len(results),
            "total_docs": total_docs,
            "api_key_used": False,
            "results": results,
        }

    def _http_get(self, url: str) -> dict[str, Any] | None:
        """Compatibility hook used by older tests/callers."""
        if not self.network_enabled:
            return None
        return _safe_http_json(url, timeout=self.timeout_seconds)

    @staticmethod
    def _extract_docs(response: dict[str, Any]) -> list[dict[str, Any]]:
        docs = response.get("response", {}).get("docs", [])
        return docs if isinstance(docs, list) else []


# ------------------------------------------------------------------
# Full-text artifact fetcher
# ------------------------------------------------------------------

class FullTextFetcher:
    """Fetch full-text artifacts for already retained include|probable records.

    By default only ERIC's public full-text host is allowed.  External publisher
    or institutional URLs must be explicitly supplied and allowlisted.
    """

    def __init__(
        self,
        cache_dir: Path = DEFAULT_CACHE_DIR,
        output_dir: Path = DEFAULT_CACHE_DIR.parent,
        *,
        network_enabled: bool = False,
        request_interval_seconds: float = DEFAULT_REQUEST_INTERVAL_SECONDS,
        timeout_seconds: float = DEFAULT_TIMEOUT_SECONDS,
        url_map: Path | None = None,
        allowed_hosts: tuple[str, ...] = DEFAULT_ALLOWED_FULLTEXT_HOSTS,
    ) -> None:
        self.cache_dir = cache_dir
        self.output_dir = output_dir
        self.network_enabled = network_enabled
        self.request_interval_seconds = max(0.0, request_interval_seconds)
        self.timeout_seconds = timeout_seconds
        self.allowed_hosts = tuple(h.lower() for h in allowed_hosts)
        self.url_by_ref: dict[str, str] = {}
        if url_map is not None and url_map.exists():
            for row in read_jsonl(url_map):
                ref = str(row.get("source_identity_reference") or "")
                url = str(row.get("url") or row.get("fulltext_url") or "")
                if ref and url:
                    self.url_by_ref[ref] = url

    def plan(
        self,
        worklist: Path | None = None,
        priority_queue: Path | None = None,
        max_items: int = 20,
    ) -> dict[str, Any]:
        if worklist is None:
            worklist = self.cache_dir.parent / "fulltext-worklist.jsonl"
        if priority_queue is None:
            priority_queue = (
                self.cache_dir.parent.parent.parent
                / "screening" / "adaptive" / "screening-pareto-queue.jsonl"
            )

        rows = read_jsonl(worklist) if worklist.exists() else []
        retained = [
            r for r in rows
            if str(r.get("decision") or "") in ("include", "probable")
        ]
        retained_by_ref = {
            str(r.get("source_identity_reference") or ""): r for r in retained
        }

        priority = (
            read_jsonl(priority_queue)
            if priority_queue and priority_queue.exists()
            else []
        )
        priority_order: list[str] = []
        for item in priority:
            ref = str(item.get("source_identity_reference") or "")
            if ref in retained_by_ref and ref not in priority_order:
                priority_order.append(ref)

        if priority_order:
            selected_refs = priority_order[:max_items]
        else:
            selected_refs = list(retained_by_ref)[:max_items]

        return {
            "schema": "sensiblaw.digital-esd-fulltext-fetch-plan.v0_2",
            "generated_at": now_iso(),
            "max_items": max_items,
            "selected_count": len(selected_refs),
            "selected_refs": selected_refs,
            "candidate_only": True,
            "creates_screening_decision": False,
        }

    @staticmethod
    def _accession_from_ref(ref: str) -> str | None:
        if ref.startswith("ERIC:"):
            accession = ref.split(":", 1)[1].strip()
            if accession.startswith(("ED", "EJ")) and accession[2:].isdigit():
                return accession
        return None

    def _resolve_url(self, ref: str) -> str | None:
        explicit = self.url_by_ref.get(ref)
        if explicit:
            return explicit
        accession = self._accession_from_ref(ref)
        if accession:
            return f"https://files.eric.ed.gov/fulltext/{accession}.pdf"
        return None

    def _url_allowed(self, url: str) -> bool:
        parsed = urllib.parse.urlparse(url)
        return parsed.scheme == "https" and (parsed.hostname or "").lower() in self.allowed_hosts

    def fetch(
        self,
        fetch_plan: dict[str, Any],
        *,
        dry_run: bool = False,
        retry_count: int = 3,
    ) -> dict[str, Any]:
        selected_refs = fetch_plan.get("selected_refs", [])
        results: list[dict[str, Any]] = []
        total_bytes = 0

        for ref in selected_refs:
            artifact_path = self.cache_dir / f"{ref}.pdf"
            url = self._resolve_url(ref)

            if dry_run:
                results.append({
                    "source_identity_reference": ref,
                    "artifact_path": str(artifact_path),
                    "resolved_url": url,
                    "status": "dry-run",
                    "actual_bytes": 0,
                })
                continue

            if not self.network_enabled:
                results.append({
                    "source_identity_reference": ref,
                    "artifact_path": str(artifact_path),
                    "resolved_url": url,
                    "status": "network-disabled",
                    "actual_bytes": 0,
                })
                continue

            if not url:
                results.append({
                    "source_identity_reference": ref,
                    "artifact_path": str(artifact_path),
                    "status": "no-fulltext-url",
                    "actual_bytes": 0,
                })
                continue
            if not self._url_allowed(url):
                results.append({
                    "source_identity_reference": ref,
                    "artifact_path": str(artifact_path),
                    "resolved_url": url,
                    "status": "host-not-allowlisted",
                    "actual_bytes": 0,
                })
                continue

            artifact_path.parent.mkdir(parents=True, exist_ok=True)
            downloaded = self._download_artifact(
                url, artifact_path, retry_count
            )
            if downloaded:
                size = artifact_path.stat().st_size
                total_bytes += size
                digest = sha256_file(artifact_path)
                results.append({
                    "source_identity_reference": ref,
                    "artifact_path": str(artifact_path),
                    "resolved_url": url,
                    "status": "downloaded",
                    "actual_bytes": size,
                    "sha256": digest,
                    "source_revision_reference": f"fulltext-sha256:{digest}",
                    "retrieval_reference": url,
                    "retrieval_timestamp": now_iso(),
                    "candidate_only": True,
                    "creates_source_truth": False,
                    "creates_source_audit_admission": False,
                })
                if self.request_interval_seconds:
                    time.sleep(self.request_interval_seconds)
            else:
                results.append({
                    "source_identity_reference": ref,
                    "artifact_path": str(artifact_path),
                    "resolved_url": url,
                    "status": "download-failed",
                    "actual_bytes": 0,
                })

        failed_statuses = {
            "download-failed", "network-disabled", "no-fulltext-url",
            "host-not-allowlisted",
        }
        return {
            "schema": "sensiblaw.digital-esd-fulltext-fetch.v0_2",
            "fetched_at": now_iso(),
            "dry_run": dry_run,
            "network_enabled": self.network_enabled,
            "selected_count": len(selected_refs),
            "downloaded_count": sum(
                1 for r in results if r["status"] == "downloaded"
            ),
            "failed_count": sum(
                1 for r in results if r["status"] in failed_statuses
            ),
            "total_bytes": total_bytes,
            "total_bytes_fmt": fmt_bytes(total_bytes),
            "results": results,
            "creates_source_truth": False,
            "creates_source_audit_admission": False,
        }

    def _download_artifact(self, url: str, path: Path, retries: int) -> bool:
        if not self.network_enabled:
            return False
        temp = path.with_suffix(path.suffix + ".part")
        for attempt in range(max(1, retries)):
            try:
                request = urllib.request.Request(
                    url,
                    headers={
                        "User-Agent": "SensibLaw-DigitalESD/1.0 (+https://github.com/chboishabba/slr)",
                        "Accept": "application/pdf,application/octet-stream;q=0.8,*/*;q=0.1",
                    },
                )
                with urllib.request.urlopen(
                    request, timeout=self.timeout_seconds
                ) as response:
                    with temp.open("wb") as fh:
                        while True:
                            chunk = response.read(1024 * 1024)
                            if not chunk:
                                break
                            fh.write(chunk)
                if temp.stat().st_size == 0:
                    temp.unlink(missing_ok=True)
                    return False
                with temp.open("rb") as fh:
                    prefix = fh.read(5)
                if prefix != b"%PDF-":
                    temp.unlink(missing_ok=True)
                    return False
                temp.replace(path)
                return True
            except (urllib.error.URLError, urllib.error.HTTPError, TimeoutError, OSError):
                temp.unlink(missing_ok=True)
                if attempt + 1 < max(1, retries) and self.request_interval_seconds:
                    time.sleep(self.request_interval_seconds)
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
    live: bool = False,
    query_config: Path = DEFAULT_QUERY_CONFIG,
    fulltext_url_map: Path | None = None,
    allowed_fulltext_hosts: tuple[str, ...] = DEFAULT_ALLOWED_FULLTEXT_HOSTS,
) -> dict[str, Any]:
    """Run the full retrieval pipeline.

    1. Fetch ERIC API JSON responses (Q1-Q7)
    2. Fetch selected full-text artifacts
    3. Verify downloaded files against plan
    """
    eric_fetcher = ERICFetcher(
        export_root=eric_export_root,
        api_key=api_key,
        query_config=query_config,
        network_enabled=live,
    )
    ft_fetcher = FullTextFetcher(
        cache_dir=fulltext_cache_dir,
        network_enabled=live,
        url_map=fulltext_url_map,
        allowed_hosts=allowed_fulltext_hosts,
    )

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
    e.add_argument("--api-key", type=str, default="", help="deprecated; public ERIC API path uses no key")
    e.add_argument("--api-base", type=str, default=DEFAULT_API_BASE)
    e.add_argument("--query-config", type=Path, default=DEFAULT_QUERY_CONFIG)
    e.add_argument("--live", action="store_true", help="explicitly permit network requests")
    e.add_argument("--request-interval-seconds", type=float, default=DEFAULT_REQUEST_INTERVAL_SECONDS)
    e.add_argument("--dry-run", action="store_true")
    e.add_argument("--json", action="store_true")

    f = sub.add_parser("fulltext", help="fetch full-text artifacts")
    f.add_argument("--cache-dir", type=Path, default=DEFAULT_CACHE_DIR)
    f.add_argument("--worklist", type=Path)
    f.add_argument("--priority-queue", type=Path)
    f.add_argument("--max-items", type=int, default=20)
    f.add_argument("--url-map", type=Path)
    f.add_argument("--allow-host", action="append", default=[])
    f.add_argument("--live", action="store_true", help="explicitly permit network requests")
    f.add_argument("--request-interval-seconds", type=float, default=DEFAULT_REQUEST_INTERVAL_SECONDS)
    f.add_argument("--dry-run", action="store_true")
    f.add_argument("--json", action="store_true")

    a = sub.add_parser("all", help="full retrieval pipeline")
    a.add_argument("--export-root", type=Path, default=DEFAULT_EXPORT_ROOT)
    a.add_argument("--cache-dir", type=Path, default=DEFAULT_CACHE_DIR)
    a.add_argument("--api-key", type=str, default="", help="deprecated; public ERIC API path uses no key")
    a.add_argument("--api-base", type=str, default=DEFAULT_API_BASE)
    a.add_argument("--query-config", type=Path, default=DEFAULT_QUERY_CONFIG)
    a.add_argument("--fulltext-url-map", type=Path)
    a.add_argument("--allow-host", action="append", default=[])
    a.add_argument("--live", action="store_true", help="explicitly permit network requests")
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
        fetcher = ERICFetcher(
            export_root=args.export_root,
            api_key=args.api_key,
            api_base=args.api_base,
            query_config=args.query_config,
            network_enabled=args.live,
            request_interval_seconds=args.request_interval_seconds,
        )
        result = fetcher.fetch_all(dry_run=args.dry_run)
        if args.json:
            print(json.dumps(result, indent=2, sort_keys=True))
        else:
            print(f"eric: {result['total_docs']} docs across {result['query_families']} families")

    elif args.command == "fulltext":
        allowed_hosts = tuple(DEFAULT_ALLOWED_FULLTEXT_HOSTS) + tuple(args.allow_host)
        fetcher = FullTextFetcher(
            cache_dir=args.cache_dir,
            network_enabled=args.live,
            request_interval_seconds=args.request_interval_seconds,
            url_map=args.url_map,
            allowed_hosts=allowed_hosts,
        )
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
            query_config=args.query_config,
            fulltext_url_map=args.fulltext_url_map,
            allowed_fulltext_hosts=tuple(DEFAULT_ALLOWED_FULLTEXT_HOSTS) + tuple(args.allow_host),
            live=args.live,
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
