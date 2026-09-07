#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
text = (ROOT / "crates/sl-proof-search-loop/src/judgment_review.rs").read_text(encoding="utf-8")

required = [
    "pub struct ReviewedCitationTreatmentDecision",
    "pub fn compile_reviewed_candidate_edge",
    "CandidateNotCandidateOnly",
    "CandidateAlreadyReviewed",
    "LocatorMismatch",
    "CitationMismatch",
    "MissingCitingProposition",
    "MissingCitedDocument",
    "MissingCitedProposition",
    "MissingReviewer",
    "MissingEvidence",
    "reviewed: true",
    "candidate_only: true",
    "lexical_hint_can_auto_compile_reviewed_edge",
    "reviewed_edge_is_binding_authority",
]
missing = [needle for needle in required if needle not in text]
if missing:
    raise SystemExit(f"judgment review gate contract missing: {missing}")

for forbidden in [
    "world_truth_claimed: true",
    "legal_holding_claimed: true",
    "binding_authority: true",
    "automatic_ratio",
]:
    if forbidden in text:
        raise SystemExit(f"judgment review gate attempted forbidden promotion: {forbidden}")

print("explicit judgment review gate contract PASS")
