#!/usr/bin/env python3
"""Verify the pinned cross-repo DASHI authority for the ERIC Q1-Q7 fixture.

Offline mode validates that the fixture carries an immutable repository/commit/
blob/path pin.  --live fetches the exact immutable GitHub contents API object at
that commit, checks its blob SHA, extracts ericQ1..ericQ7 exactTranslatedQuery
strings, and compares them byte-for-byte with the local fixture.

This is provenance verification only; it does not execute ERIC searches.
"""

from __future__ import annotations

import argparse
import base64
import json
import re
import urllib.request
from pathlib import Path
from typing import Any

DEFAULT_FIXTURE = Path("fixtures/digital_esd_eric_queries.json")

ERIC_NAMES = {
    "Q1": "ericQ1DigitalEducationESD",
    "Q2": "ericQ2Transformation",
    "Q3": "ericQ3ReflexiveSustainability",
    "Q4": "ericQ4LifecycleCircularity",
    "Q5": "ericQ5ParticipantGovernance",
    "Q6": "ericQ6LongitudinalInstitutional",
    "Q7": "ericQ7OpenInteroperableRepairable",
}


def load_fixture(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError("fixture must be a JSON object")
    return value


def validate_pin(fixture: dict[str, Any]) -> None:
    required = (
        "source_repository",
        "source_commit",
        "source_blob_sha",
        "source_path",
        "query_families",
    )
    missing = [k for k in required if not str(fixture.get(k) or "").strip()]
    if missing:
        raise ValueError(f"fixture lacks immutable authority fields: {missing}")
    commit = str(fixture["source_commit"])
    blob = str(fixture["source_blob_sha"])
    if not re.fullmatch(r"[0-9a-f]{40}", commit):
        raise ValueError("source_commit must be an exact 40-hex git commit")
    if not re.fullmatch(r"[0-9a-f]{40}", blob):
        raise ValueError("source_blob_sha must be an exact 40-hex git blob SHA")


def fetch_exact_owner(fixture: dict[str, Any]) -> tuple[str, str]:
    repo = str(fixture["source_repository"])
    commit = str(fixture["source_commit"])
    path = str(fixture["source_path"])
    url = f"https://api.github.com/repos/{repo}/contents/{path}?ref={commit}"
    req = urllib.request.Request(
        url,
        headers={
            "Accept": "application/vnd.github+json",
            "User-Agent": "DASHI-Digital-ESD-query-authority-verifier/1.0",
        },
    )
    with urllib.request.urlopen(req, timeout=30) as response:
        payload = json.loads(response.read().decode("utf-8"))
    observed_sha = str(payload.get("sha") or "")
    content_b64 = str(payload.get("content") or "").replace("\n", "")
    if not content_b64:
        raise RuntimeError("GitHub contents response lacks source content")
    text = base64.b64decode(content_b64).decode("utf-8")
    return observed_sha, text


def agda_unescape(literal: str) -> str:
    # Exact query literals in this owner use only JSON/Python-compatible
    # backslash escapes for quotes/backslashes.
    return json.loads(literal)


def extract_query(owner_text: str, binding: str) -> str:
    # Bind to the named ERIC declaration and then to the first quoted literal
    # after Syntax.ericSyntaxReceipt. This avoids accidentally reading a
    # neighbouring translationBoundary string.
    pattern = re.compile(
        rf"(?ms)^{re.escape(binding)}\s*:\s*TranslatedQueryReceipt.*?"
        rf"Syntax\.ericSyntaxReceipt\s*\n\s*(\"(?:[^\"\\]|\\.)*\")"
    )
    match = pattern.search(owner_text)
    if not match:
        raise RuntimeError(f"could not extract exact query for {binding}")
    return agda_unescape(match.group(1))


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--fixture", type=Path, default=DEFAULT_FIXTURE)
    ap.add_argument("--live", action="store_true")
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args()

    fixture = load_fixture(args.fixture)
    validate_pin(fixture)

    result: dict[str, Any] = {
        "schema": "sensiblaw.digital-esd-query-authority-verification.v1",
        "fixture": str(args.fixture),
        "source_repository": fixture["source_repository"],
        "source_commit": fixture["source_commit"],
        "source_blob_sha": fixture["source_blob_sha"],
        "source_path": fixture["source_path"],
        "immutable_pin_valid": True,
        "live_checked": False,
        "blob_sha_matches": None,
        "queries_match": None,
    }

    if args.live:
        observed_sha, owner_text = fetch_exact_owner(fixture)
        expected_sha = str(fixture["source_blob_sha"])
        if observed_sha != expected_sha:
            raise RuntimeError(
                f"authority blob mismatch expected={expected_sha} observed={observed_sha}"
            )
        local_queries = fixture["query_families"]
        mismatches: list[str] = []
        for qid, binding in ERIC_NAMES.items():
            owner_query = extract_query(owner_text, binding)
            if owner_query != local_queries.get(qid):
                mismatches.append(qid)
        if mismatches:
            raise RuntimeError(
                "local ERIC query fixture differs from pinned DASHI owner: "
                + ", ".join(mismatches)
            )
        result.update(
            {
                "live_checked": True,
                "blob_sha_matches": True,
                "queries_match": True,
            }
        )

    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    else:
        status = "pin-ok"
        if args.live:
            status += " live-owner-ok"
        print(f"digital-esd-query-authority: {status}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
