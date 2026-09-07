#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
example = (ROOT / "crates/sl-proof-search-loop/examples/live_cullen_citation_review_queue.rs").read_text(encoding="utf-8")
runner = (ROOT / "scripts/run_local_cullen_review_queue.py").read_text(encoding="utf-8")
verifier = (ROOT / "scripts/verify_live_cullen_review_queue.py").read_text(encoding="utf-8")
candidates = (ROOT / "crates/sl-proof-search-loop/src/judgment_candidates.rs").read_text(encoding="utf-8")
observer = (ROOT / "crates/sl-governed-legal-provider/src/docx_text.rs").read_text(encoding="utf-8")

required_example = [
    "sl.judgment_citation_review_queue.v0_3",
    "extract_docx_canonical_judgment",
    "extract_judgment_citation_candidates_with_footnotes_and_anchors",
    "FootnoteAnchorObservation",
    "body_only_canonical_text_sha256",
    "body_footnotes_preserved",
    "body_footnote_anchors_preserved",
    "footnote_anchor_count",
    "anchored_footnote_candidate_count",
    "SENSIBLAW_BOUND_RESIDUAL_REF",
    "SENSIBLAW_BOUND_PROPOSITION_REF",
    "SENSIBLAW_BOUND_PRODUCER_REF",
    "SENSIBLAW_BOUND_HYPOTHESIS_REF",
    '"network_requests\\\": 0',
    '"all_candidates_reviewed\\\": false',
    '"candidate_extraction_claimed_semantic_correspondence\\\": false',
    '"candidate_extraction_claimed_citation_treatment\\\": false',
    '"candidate_extraction_claimed_current_authority\\\": false',
    '"anchor_observation_claimed_residual_payment\\\": false',
]
missing = [needle for needle in required_example if needle not in example]
if missing:
    raise SystemExit(f"live Cullen review queue example missing contract fields: {missing}")

required_runner = [
    "governed-official-judgment-acquisition-v01.json",
    "judgment.docx",
    "cullen-citation-review-queue-v03.json",
    "landing_page_network_requests",
    "document_fetch",
    "replay_run",
    "SENSIBLAW_BOUND_RESIDUAL_REF",
    "SENSIBLAW_CANONICAL_TEXT_SHA256",
    "verify_live_cullen_review_queue.py",
]
missing = [needle for needle in required_runner if needle not in runner]
if missing:
    raise SystemExit(f"live Cullen review queue runner missing receipt weld: {missing}")

required_verifier = [
    "residual:cullen-positive-operational-act",
    "prop:cullen-positive-operational-duty",
    "producer:exact-primary-authority",
    "body_footnotes_preserved",
    "body_footnote_anchors_preserved",
    "footnote_anchor_count",
    "(2024) 98 ALJR 956",
    "418 ALR 639",
    "[2018] AC 736",
    "(2000) 205 CLR 254",
    "positive acts in creating risk",
    "anchored_footnote_candidate_count",
    "anchor_paragraph_locator_refs",
    "reviewed",
    "candidate_only",
]
missing = [needle for needle in required_verifier if needle not in verifier]
if missing:
    raise SystemExit(f"live Cullen review queue verifier missing fail-closed checks: {missing}")

for needle in (
    "document_xml_to_canonical_paragraphs",
    "footnoteReference",
    "CanonicalDocxParagraph",
    "word/footnotes.xml",
    "CanonicalDocxFootnote",
    "footnote_anchor_is_residual_relevance",
):
    if needle not in observer:
        raise SystemExit(f"anchored observer lost required carrier: {needle}")

for needle in (
    "extract_reported_citation_strings",
    "extract_judgment_citation_candidates_with_footnotes_and_anchors",
    "FootnoteAnchorObservation",
    "anchor_paragraph_locator_refs",
    "anchor_paragraph_texts",
    "footnote_anchor_is_residual_payment",
):
    if needle not in candidates:
        raise SystemExit(f"citation extractor lost anchor/reporter support: {needle}")

for forbidden in ["ureq", "reqwest", "TcpStream", "std::net", "fetch_hca", "austlii_search", "jade_search"]:
    if forbidden in example or forbidden in runner:
        raise SystemExit(f"zero-network Cullen review queue gained forbidden network capability: {forbidden}")

for forbidden in ["legal_holding_claimed = true", "current_authority = true", "semantic_payment = true", "residual_payment = true"]:
    if forbidden in example or forbidden in candidates:
        raise SystemExit(f"review queue gained forbidden semantic promotion: {forbidden}")

print("live Cullen anchored citation review queue contract PASS")
