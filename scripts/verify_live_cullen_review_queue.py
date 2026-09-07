#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path

SCHEMA = "sl.judgment_citation_review_queue.v0_1"
AUTHORITY = "experimental_candidate_only"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("queue", type=Path)
    args = parser.parse_args()

    data = json.loads(args.queue.read_text(encoding="utf-8"))
    if data.get("schema_version") != SCHEMA:
        raise SystemExit(f"unexpected schema: {data.get('schema_version')!r}")
    if data.get("authority") != AUTHORITY:
        raise SystemExit("citation review queue must remain candidate-only")
    if data.get("network_requests") != 0:
        raise SystemExit("citation review queue extraction must be zero-network")

    binding = data.get("binding") or {}
    expected_binding = {
        "residual_ref": "residual:cullen-positive-operational-act",
        "proposition_ref": "prop:cullen-positive-operational-duty",
        "scheduled_producer_ref": "producer:exact-primary-authority",
    }
    for key, expected in expected_binding.items():
        if binding.get(key) != expected:
            raise SystemExit(f"unexpected residual binding {key}: {binding.get(key)!r}")
    if not str(binding.get("hypothesis_ref", "")).endswith(":support"):
        raise SystemExit("Cullen citation review queue must remain bound to support hypothesis")

    if not str(data.get("canonical_text_sha256", "")).startswith("sha256:"):
        raise SystemExit("review queue must retain canonical text SHA256")
    if not str(data.get("source_revision_ref", "")).startswith("source-revision:sha256:"):
        raise SystemExit("review queue must retain immutable source revision")

    candidates = data.get("candidates") or []
    if data.get("candidate_count") != len(candidates):
        raise SystemExit("candidate_count does not match serialized candidates")
    if data.get("all_candidates_reviewed") is not False:
        raise SystemExit("new extraction queue must remain pre-review")
    for forbidden_claim in [
        "candidate_extraction_claimed_semantic_correspondence",
        "candidate_extraction_claimed_citation_treatment",
        "candidate_extraction_claimed_current_authority",
    ]:
        if data.get(forbidden_claim) is not False:
            raise SystemExit(f"review queue made forbidden claim: {forbidden_claim}")

    for candidate in candidates:
        if candidate.get("reviewed") is not False or candidate.get("candidate_only") is not True:
            raise SystemExit("every extracted citation occurrence must remain unreviewed candidate-only")
        if candidate.get("canonical_text_sha256") != data.get("canonical_text_sha256"):
            raise SystemExit("candidate lost canonical text identity")
        if candidate.get("source_revision_ref") != data.get("source_revision_ref"):
            raise SystemExit("candidate lost source revision identity")
        if not candidate.get("paragraph_locator_ref"):
            raise SystemExit("candidate missing stable paragraph locator")
        if not candidate.get("citation_text"):
            raise SystemExit("candidate missing citation text")

    print(
        "live Cullen citation review queue PASS "
        f"candidates={len(candidates)} network=0 authority={AUTHORITY}"
    )


if __name__ == "__main__":
    main()
