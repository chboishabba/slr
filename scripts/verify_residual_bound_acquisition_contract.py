#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
text = (ROOT / "crates/sl-proof-search-loop/src/bound_acquisition.rs").read_text(encoding="utf-8")

required = [
    "pub struct ResidualBoundAuthorityDemand",
    "pub fn bind_authority_demand",
    "ResidualNotOpen",
    "HypothesisResidualMismatch",
    "HypothesisPropositionMismatch",
    "HypothesisProducerMismatch",
    "HypothesisJurisdictionMismatch",
    "DemandPropositionMissing",
    "DemandPropositionMismatch",
    "DemandJurisdictionMismatch",
    "source_route_pays_scheduled_gap",
    "source_route_uses_scheduled_producer",
    "acquisition_is_semantic_payment",
    "acquisition_closes_consumer",
    'BOUND_ACQUISITION_AUTHORITY: &str = "experimental_candidate_only"',
]
missing = [needle for needle in required if needle not in text]
if missing:
    raise SystemExit(f"residual-bound acquisition contract missing: {missing}")

for forbidden in [
    "reqwest",
    "ureq",
    "TcpStream",
    "std::net",
    "publish(",
    "auto_admit",
    "automatic_admission",
]:
    if forbidden in text:
        raise SystemExit(f"residual-bound acquisition gained forbidden capability: {forbidden}")

print("residual-bound governed acquisition contract PASS")
