#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path

SCHEMA = "sl.judgment_citation_review_queue.v0_2"
AUTHORITY = "experimental_candidate_only"
SELF_CITATION = "[2026] HCA 19"
MALLONLAND_REPORTED = "(2024) 98 ALJR 956"
MALLONLAND_PARALLEL = "418 ALR 639"


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

    old_digest = str(data.get("body_only_canonical_text_sha256", ""))
    new_digest = str(data.get("canonical_text_sha256", ""))
    if not old_digest.startswith("sha256:") or not new_digest.startswith("sha256:"):
        raise SystemExit("review queue must retain both body-only and refined observer SHA256")
    if old_digest == new_digest:
        raise SystemExit("footnote-preserving observer refinement must have a distinct digest")
    if not str(data.get("source_revision_ref", "")).startswith("source-revision:sha256:"):
        raise SystemExit("review queue must retain immutable source revision")

    refinement = data.get("observer_refinement") or {}
    if refinement.get("body_footnotes_preserved") is not True:
        raise SystemExit("refined observer must preserve DOCX footnotes")
    if int(refinement.get("body_paragraph_count", 0)) <= 0:
        raise SystemExit("refined observer lost body paragraphs")
    if int(refinement.get("footnote_count", 0)) <= 0:
        raise SystemExit("refined observer found no material footnotes")

    candidates = data.get("candidates") or []
    if data.get("candidate_count") != len(candidates):
        raise SystemExit("candidate_count does not match serialized candidates")
    if len(candidates) <= 1:
        raise SystemExit("refined Cullen observer still exposes only the judgment self-citation")
    if data.get("all_candidates_reviewed") is not False:
        raise SystemExit("new extraction queue must remain pre-review")
    for forbidden_claim in [
        "candidate_extraction_claimed_semantic_correspondence",
        "candidate_extraction_claimed_citation_treatment",
        "candidate_extraction_claimed_current_authority",
    ]:
        if data.get(forbidden_claim) is not False:
            raise SystemExit(f"review queue made forbidden claim: {forbidden_claim}")

    citations = set()
    footnote_candidates = 0
    for candidate in candidates:
        if candidate.get("reviewed") is not False or candidate.get("candidate_only") is not True:
            raise SystemExit("every extracted citation occurrence must remain unreviewed candidate-only")
        if candidate.get("canonical_text_sha256") != new_digest:
            raise SystemExit("candidate lost refined observer identity")
        if candidate.get("source_revision_ref") != data.get("source_revision_ref"):
            raise SystemExit("candidate lost source revision identity")
        locator = str(candidate.get("paragraph_locator_ref", ""))
        if not locator:
            raise SystemExit("candidate missing stable source locator")
        if "#footnote-" in locator:
            footnote_candidates += 1
        citation = str(candidate.get("citation_text", ""))
        if not citation:
            raise SystemExit("candidate missing citation text")
        citations.add(citation)

    if footnote_candidates == 0:
        raise SystemExit("refined queue exposed no footnote-located authority candidates")
    if not any(citation != SELF_CITATION for citation in citations):
        raise SystemExit("refined queue still exposes no substantive non-self authority")
    if MALLONLAND_REPORTED not in citations or MALLONLAND_PARALLEL not in citations:
        raise SystemExit("refined Cullen queue did not recover Mallonland's reported/parallel citations")

    print(
        "live Cullen citation review queue PASS "
        f"candidates={len(candidates)} footnote_candidates={footnote_candidates} "
        f"network=0 authority={AUTHORITY}"
    )


if __name__ == "__main__":
    main()
