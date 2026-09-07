#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path

SCHEMA = "sl.residual_citation_review_shortlist.v0_1"
AUTHORITY = "experimental_candidate_only"
ROBINSON = "[2018] AC 736"
MODBURY = "(2000) 205 CLR 254"
MALLONLAND = "(2024) 98 ALJR 956"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("receipt", type=Path)
    args = parser.parse_args()

    data = json.loads(args.receipt.read_text(encoding="utf-8"))
    if data.get("schema_version") != SCHEMA:
        raise SystemExit(f"unexpected schema: {data.get('schema_version')!r}")
    if data.get("authority") != AUTHORITY:
        raise SystemExit("shortlist must remain candidate-only")
    if data.get("network_requests") != 0:
        raise SystemExit("shortlist must be zero-network")
    if data.get("source_queue_schema") != "sl.judgment_citation_review_queue.v0_3":
        raise SystemExit("shortlist must derive from anchored v0.3 observer semantics")
    if data.get("residual_ref") != "residual:cullen-positive-operational-act":
        raise SystemExit("shortlist lost live residual binding")
    if data.get("proposition_ref") != "prop:cullen-positive-operational-duty":
        raise SystemExit("shortlist lost live proposition binding")
    if data.get("candidate_count") != 190:
        raise SystemExit("shortlist must project from the observed 190-candidate Cullen carrier")

    for forbidden in [
        "shortlist_claimed_semantic_payment",
        "shortlist_claimed_citation_treatment",
        "shortlist_claimed_current_authority",
        "shortlist_claimed_consumer_closure",
    ]:
        if data.get(forbidden) is not False:
            raise SystemExit(f"shortlist made forbidden claim: {forbidden}")

    shortlist = data.get("shortlist") or []
    if data.get("shortlist_count") != len(shortlist) or not shortlist:
        raise SystemExit("shortlist count mismatch or empty shortlist")

    citations = set()
    robinson_contexts = []
    modbury_contexts = []
    for item in shortlist:
        if item.get("reviewed") is not False or item.get("candidate_only") is not True:
            raise SystemExit("shortlist item must remain unreviewed candidate-only")
        criteria = item.get("matched_criterion_refs") or []
        anchors = item.get("anchor_paragraph_texts") or []
        locators = item.get("anchor_paragraph_locator_refs") or []
        if not criteria or not anchors or not locators:
            raise SystemExit("shortlist item lost residual criterion or source anchor provenance")
        citation = str(item.get("citation_text", ""))
        citations.add(citation)
        if citation == ROBINSON:
            robinson_contexts.extend(anchors)
        if citation == MODBURY:
            modbury_contexts.extend(anchors)

    if ROBINSON not in citations:
        raise SystemExit("shortlist failed to retain Robinson")
    if MODBURY not in citations:
        raise SystemExit("shortlist failed to retain Modbury")
    if MALLONLAND in citations:
        raise SystemExit("Mallonland calibration citation should not enter the positive-operational-act shortlist merely because it was recovered")

    combined_robinson = " ".join(robinson_contexts).lower()
    combined_modbury = " ".join(modbury_contexts).lower()
    if "positive negligent conduct" not in combined_robinson and "positive acts in creating risk" not in combined_robinson:
        raise SystemExit("Robinson shortlist entry lacks positive-act anchor context")
    if "careless acts causing personal injury" not in combined_modbury and "positive acts in creating risk" not in combined_modbury:
        raise SystemExit("Modbury shortlist entry lacks act/omission discriminator context")

    print(
        "live Cullen residual shortlist PASS "
        f"shortlist={len(shortlist)} network=0 authority={AUTHORITY}"
    )


if __name__ == "__main__":
    main()
