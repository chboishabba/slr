#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
example = (ROOT / "crates/sl-proof-search-loop/examples/live_cullen_citation_review_queue.rs").read_text(encoding="utf-8")
runner = (ROOT / "scripts/run_local_cullen_review_queue.py").read_text(encoding="utf-8")
verifier = (ROOT / "scripts/verify_live_cullen_review_queue.py").read_text(encoding="utf-8")

required_example = [
    "sl.judgment_citation_review_queue.v0_1",
    "extract_judgment_citation_candidates",
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
    "judgment.txt",
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
    "candidate_count",
    "paragraph_locator_ref",
    "reviewed",
    "candidate_only",
]
missing = [needle for needle in required_verifier if needle not in verifier]
if missing:
    raise SystemExit(f"live Cullen review queue verifier missing fail-closed checks: {missing}")

for forbidden in ["ureq", "reqwest", "TcpStream", "std::net", "fetch_hca", "austlii_search", "jade_search"]:
    if forbidden in example or forbidden in runner:
        raise SystemExit(f"zero-network Cullen review queue gained forbidden network capability: {forbidden}")

for forbidden in ["legal_holding_claimed = true", "current_authority = true", "semantic_payment = true"]:
    if forbidden in example:
        raise SystemExit(f"review queue gained forbidden semantic promotion: {forbidden}")

print("live Cullen citation review queue contract PASS")
