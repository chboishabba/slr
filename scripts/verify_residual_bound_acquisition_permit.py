#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path

SCHEMA = "sl.residual_bound_authority_demand.v0_1"
AUTHORITY = "experimental_candidate_only"


def require_nonempty_string(data: dict, key: str) -> str:
    value = data.get(key)
    if not isinstance(value, str) or not value:
        raise SystemExit(f"missing non-empty string: {key}")
    return value


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("permit", type=Path)
    args = parser.parse_args()

    data = json.loads(args.permit.read_text(encoding="utf-8"))
    if data.get("schema_version") != SCHEMA:
        raise SystemExit(f"unexpected schema: {data.get('schema_version')!r}")
    if data.get("authority") != AUTHORITY:
        raise SystemExit(f"unexpected authority: {data.get('authority')!r}")

    residual_ref = require_nonempty_string(data, "residual_ref")
    proposition_ref = require_nonempty_string(data, "proposition_ref")
    producer_ref = require_nonempty_string(data, "scheduled_producer_ref")
    hypothesis_ref = require_nonempty_string(data, "hypothesis_ref")
    require_nonempty_string(data, "source_identity_ref")
    require_nonempty_string(data, "medium_neutral_citation")
    if data.get("jurisdiction_ref") != "AU":
        raise SystemExit("unexpected jurisdiction calibration")
    if data.get("source_route_pays_scheduled_gap") is not True:
        raise SystemExit("permit is not bound to the scheduled gap")
    if data.get("source_route_uses_scheduled_producer") is not True:
        raise SystemExit("permit is not bound to the scheduled producer")
    if data.get("acquisition_claimed_semantic_payment") is not False:
        raise SystemExit("permit cannot claim semantic payment")
    if data.get("acquisition_claimed_consumer_closure") is not False:
        raise SystemExit("permit cannot claim consumer closure")

    print(
        "residual-bound acquisition permit PASS "
        f"residual={residual_ref} proposition={proposition_ref} "
        f"producer={producer_ref} hypothesis={hypothesis_ref}"
    )


if __name__ == "__main__":
    main()
