#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path

SCHEMA = "sl.governed_official_judgment_acquisition.v0_1"
AUTHORITY = "experimental_candidate_only"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("receipt", type=Path)
    args = parser.parse_args()

    data = json.loads(args.receipt.read_text(encoding="utf-8"))
    if data.get("schema_version") != SCHEMA:
        raise SystemExit(f"unexpected schema: {data.get('schema_version')!r}")
    if data.get("authority") != AUTHORITY:
        raise SystemExit(f"unexpected authority: {data.get('authority')!r}")
    if data.get("provider") != "HighCourtAustralia":
        raise SystemExit("official judgment fixture must use HighCourtAustralia")
    if data.get("medium_neutral_citation") != "[2026] HCA 19":
        raise SystemExit("unexpected HCA judgment calibration")
    if data.get("landing_page_network_requests") != 0:
        raise SystemExit("landing page must be reused locally without a new request")
    if data.get("resource_discovery_network_requests") != 0:
        raise SystemExit("resource discovery must be zero-network")
    if data.get("document_kind") != "Docx":
        raise SystemExit("preferred HCA judgment resource must be DOCX")
    if not str(data.get("document_reference", "")).startswith("https://www.hcourt.gov.au/"):
        raise SystemExit("judgment document reference must stay on official HCA host")

    document = data.get("document_fetch") or {}
    replay = data.get("replay_run") or {}
    if document.get("network_requests") != 1:
        raise SystemExit("judgment document fetch must consume exactly one request")
    if document.get("locally_ingested") is not True:
        raise SystemExit("judgment document must pass through local ingestion")
    if not str(document.get("bytes_digest", "")).startswith("sha256:"):
        raise SystemExit("judgment document must retain SHA256 bytes identity")
    if replay.get("network_requests") != 0 or replay.get("resolution") != "Persisted":
        raise SystemExit("judgment document replay must resolve persisted with zero network")
    if data.get("document_fetch_claimed_semantic_payment") is not False:
        raise SystemExit("document acquisition cannot claim semantic payment")
    if data.get("document_fetch_claimed_legal_authority") is not False:
        raise SystemExit("document acquisition cannot claim legal authority")

    print(
        "official HCA judgment acquisition receipt PASS "
        f"head={data['runtime_head']} landing=0 document=1 replay=0"
    )


if __name__ == "__main__":
    main()
