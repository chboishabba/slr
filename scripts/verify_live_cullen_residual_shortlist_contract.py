#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
module = (ROOT / "crates/sl-proof-search-loop/src/residual_review_shortlist.rs").read_text(encoding="utf-8")
example = (ROOT / "crates/sl-proof-search-loop/examples/live_cullen_residual_review_shortlist.rs").read_text(encoding="utf-8")
runner = (ROOT / "scripts/run_local_cullen_residual_shortlist.py").read_text(encoding="utf-8")
verifier = (ROOT / "scripts/verify_live_cullen_residual_shortlist.py").read_text(encoding="utf-8")

for needle in [
    "ResidualCitationReviewDemand",
    "ResidualAnchorCriterion",
    "shortlist_anchored_citations_for_residual",
    "CriterionResidualMismatch",
    "CriterionPropositionMismatch",
    "shortlist_membership_is_semantic_payment",
    "shortlist_membership_is_citation_treatment",
    "shortlist_membership_is_current_authority",
]:
    if needle not in module:
        raise SystemExit(f"residual shortlist module missing: {needle}")

for needle in [
    "sl.residual_citation_review_shortlist.v0_1",
    "sl.judgment_citation_review_queue.v0_3",
    "positive negligent conduct",
    "careless acts causing personal injury",
    "positive acts in creating risk",
    "failed to protect her",
    '"network_requests\\\": 0',
    '"shortlist_claimed_semantic_payment\\\": false',
    '"shortlist_claimed_citation_treatment\\\": false',
    '"shortlist_claimed_current_authority\\\": false',
    '"shortlist_claimed_consumer_closure\\\": false',
]:
    if needle not in example:
        raise SystemExit(f"live residual shortlist example missing: {needle}")

for needle in [
    "residual:cullen-positive-operational-act",
    "prop:cullen-positive-operational-duty",
    "judgment.docx",
    "verify_live_cullen_residual_shortlist.py",
]:
    if needle not in runner:
        raise SystemExit(f"residual shortlist runner missing weld: {needle}")

for needle in [
    "[2018] AC 736",
    "(2000) 205 CLR 254",
    "(2024) 98 ALJR 956",
    "candidate_count",
    "matched_criterion_refs",
    "anchor_paragraph_texts",
    "candidate_only",
]:
    if needle not in verifier:
        raise SystemExit(f"residual shortlist verifier missing: {needle}")

for forbidden in ["ureq", "reqwest", "TcpStream", "std::net", "fetch_hca", "austlii_search", "jade_search"]:
    if forbidden in module or forbidden in example or forbidden in runner:
        raise SystemExit(f"residual shortlist gained forbidden network capability: {forbidden}")

for forbidden in ["legal_holding_claimed = true", "current_authority = true", "semantic_payment = true", "consumer_closure = true"]:
    if forbidden in module or forbidden in example:
        raise SystemExit(f"residual shortlist gained forbidden promotion: {forbidden}")

print("live Cullen residual shortlist contract PASS")
