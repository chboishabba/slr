#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path

SCHEMA = "sl.governed_legal_acquisition.v0_1"
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
    if not data.get("runtime_head"):
        raise SystemExit("runtime_head must be pinned")
    if data.get("provider") != "HighCourtAustralia":
        raise SystemExit("first governed live fixture must use the official High Court provider")
    if data.get("provider_access_status") != "Available":
        raise SystemExit("live provider must report Available before the receipt can validate")
    for field in ("source_identity_ref", "proposition_ref", "medium_neutral_citation", "explicit_reference"):
        if not data.get(field):
            raise SystemExit(f"missing receipt field: {field}")
    if not str(data["explicit_reference"]).startswith("https://www.hcourt.gov.au/"):
        raise SystemExit("official HCA receipt must retain an hcourt.gov.au reference")

    first = data.get("first_run") or {}
    replay = data.get("replay_run") or {}
    if first.get("network_requests") != 1:
        raise SystemExit("first run must consume exactly one network request")
    if first.get("locally_ingested") is not True:
        raise SystemExit("first run must persist through the local-ingestion seam")
    if not str(first.get("bytes_digest", "")).startswith("sha256:"):
        raise SystemExit("first run must retain SHA256 bytes identity")
    if not first.get("source_revision_ref"):
        raise SystemExit("first run must produce an immutable source revision")
    if replay.get("network_requests") != 0:
        raise SystemExit("same-demand replay must be zero-network")
    if replay.get("resolution") != "Persisted":
        raise SystemExit("same-demand replay must resolve from persisted material")
    if data.get("search_claimed_semantic_payment") is not False:
        raise SystemExit("search/acquisition receipt cannot claim semantic payment")
    if data.get("acquisition_claimed_authority_receipt") is not False:
        raise SystemExit("acquisition receipt cannot become legal authority")

    print(
        "governed official-source receipt PASS "
        f"head={data['runtime_head']} provider=HighCourtAustralia first_network=1 replay_network=0"
    )


if __name__ == "__main__":
    main()
