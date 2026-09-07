#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
text = (ROOT / "crates/sl-proof-search-loop/src/judgment_candidates.rs").read_text(encoding="utf-8")

required = [
    "pub struct CitationOccurrenceCandidate",
    "pub enum LexicalTreatmentHint",
    "pub fn extract_judgment_citation_candidates",
    "pub fn extract_judgment_citation_candidates_with_footnotes",
    "extract_reported_citation_strings",
    "#footnote-",
    "(2024) 98 ALJR 956",
    "418 ALR 639",
    "[2018] AC 736",
    "paragraph_locator_ref",
    "reported_paragraph_label",
    "canonical_text_sha256",
    "source_revision_ref",
    "reviewed: false",
    "candidate_only: true",
    "citation_candidate_is_semantic_correspondence",
    "lexical_hint_is_citation_use",
    "citation_candidate_is_current_authority",
    "footnote_citation_candidate_is_treatment",
]
missing = [needle for needle in required if needle not in text]
if missing:
    raise SystemExit(f"judgment candidate extraction contract missing: {missing}")

for forbidden in [
    "CitationUse::Applied",
    "CitationUse::Followed",
    "CitationUse::Distinguished",
    "ReasoningRole::",
    "world_truth_claimed: true",
    "legal_holding_claimed: true",
]:
    if forbidden in text:
        raise SystemExit(f"pre-review extraction attempted semantic promotion: {forbidden}")

print("source-located judgment candidate extraction contract PASS")
