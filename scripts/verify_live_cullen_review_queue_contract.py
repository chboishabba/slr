#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
example = (ROOT / "crates/sl-proof-search-loop/examples/live_cullen_citation_review_queue.rs").read_text(encoding="utf-8")
runner = (ROOT / "scripts/run_local_cullen_review_queue.py").read_text(encoding="utf-8")
verifier = (ROOT / "scripts/verify_live_cullen_review_queue.py").read_text(encoding="utf-8")
candidates = (ROOT / "crates/sl-proof-search-loop/src/judgment_candidates.rs").read_text(encoding="utf-8")
observer = (ROOT / "crates/sl-governed-legal-provider/src/docx_text.rs").read_text(encoding="utf-8")

required_example = [
    "sl.judgment_citation_review_queue.v0_2",
    "extract_docx_canonical_judgment",
    "extract_judgment_citation_candidates_with_footnotes",
    "body_only_canonical_text_sha256",
    "body_footnotes_preserved",
    "footnote_count",
    "SENSIBLAW_BOUND_RESIDUAL_REF",
    "SENSIBLAW_BOUND_PROPOSITION_REF",
    "SENSIBLAW_BOUND_PRODUCER_REF",
    "SENSIBLAW_BOUND_HYPOTHESIS_REF",
    '"network_requests\\\": 0',
    '"all_candidates_reviewed\\\": false',
    '"candidate_extraction_claimed_semantic_correspondence\\\": false',
    '"candidate_extraction_claimed_citation_treatment\\\": false',
    '"candidate_extraction_claimed_current_authority\\\": false',
]
missing = [needle for needle in required_example if needle not in example]
if missing:
    raise SystemExit(f"live Cullen review queue example missing contract fields: {missing}")

required_runner = [
    "governed-official-judgment-acquisition-v01.json",
    "judgment.docx",
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
    "footnote_count",
    "(2024) 98 ALJR 956",
    "418 ALR 639",
    "candidate_count",
    "paragraph_locator_ref",
    "reviewed",
    "candidate_only",
]
missing = [needle for needle in required_verifier if needle not in verifier]
if missing:
    raise SystemExit(f"live Cullen review queue verifier missing fail-closed checks: {missing}")

for needle in (
    "footnotes_xml_to_canonical_footnotes",
    "word/footnotes.xml",
    "CanonicalDocxFootnote",
):
    if needle not in observer:
        raise SystemExit(f"refined observer lost footnote carrier: {needle}")

for needle in (
    "extract_reported_citation_strings",
    "extract_judgment_citation_candidates_with_footnotes",
    "#footnote-",
    "footnote_citation_candidate_is_treatment",
):
    if needle not in candidates:
        raise SystemExit(f"citation extractor lost footnote/reporter support: {needle}")

for forbidden in ["ureq", "reqwest", "TcpStream", "std::net", "fetch_hca", "austlii_search", "jade_search"]:
    if forbidden in example or forbidden in runner:
        raise SystemExit(f"zero-network Cullen review queue gained forbidden network capability: {forbidden}")

for forbidden in ["legal_holding_claimed = true", "current_authority = true", "semantic_payment = true"]:
    if forbidden in example or forbidden in candidates:
        raise SystemExit(f"review queue gained forbidden semantic promotion: {forbidden}")

print("live Cullen citation review queue contract PASS")
