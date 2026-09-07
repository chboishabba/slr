#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
binding = (ROOT / "crates/sl-proof-search-loop/src/bound_acquisition.rs").read_text(encoding="utf-8")
permit_fixture = (ROOT / "crates/sl-proof-search-loop/examples/residual_bound_hca_acquisition.rs").read_text(encoding="utf-8")
live_runner = (ROOT / "crates/sl-governed-legal-provider/examples/live_hca_judgment_docx_smoke.rs").read_text(encoding="utf-8")
operator_script = (ROOT / "scripts/run_live_hca_judgment_smoke.sh").read_text(encoding="utf-8")

required_binding = [
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
missing = [needle for needle in required_binding if needle not in binding]
if missing:
    raise SystemExit(f"residual-bound acquisition contract missing: {missing}")

required_permit = [
    'sl.residual_bound_authority_demand.v0_1',
    '"residual_ref"',
    '"proposition_ref"',
    '"scheduled_producer_ref"',
    '"hypothesis_ref"',
    '"source_route_pays_scheduled_gap"',
    '"source_route_uses_scheduled_producer"',
    '"acquisition_claimed_semantic_payment"',
    '"acquisition_claimed_consumer_closure"',
]
missing = [needle for needle in required_permit if needle not in permit_fixture]
if missing:
    raise SystemExit(f"residual-bound permit fixture missing: {missing}")

required_live = [
    "SENSIBLAW_BOUND_ACQUISITION_PLAN",
    "sl.residual_bound_authority_demand.v0_1",
    "source_route_pays_scheduled_gap",
    "source_route_uses_scheduled_producer",
    '"binding"',
    '"residual_ref"',
    '"scheduled_producer_ref"',
    '"hypothesis_ref"',
    '"acquisition_claimed_consumer_closure"',
]
missing = [needle for needle in required_live if needle not in live_runner]
if missing:
    raise SystemExit(f"live HCA runner can bypass residual binding: {missing}")

required_operator = [
    "SENSIBLAW_BOUND_ACQUISITION_PLAN",
    "--example residual_bound_hca_acquisition",
    "--example live_hca_judgment_docx_smoke",
]
missing = [needle for needle in required_operator if needle not in operator_script]
if missing:
    raise SystemExit(f"operator live path does not enforce bound permit ordering: {missing}")

for forbidden in [
    "reqwest",
    "ureq",
    "TcpStream",
    "std::net",
    "publish(",
    "auto_admit",
    "automatic_admission",
]:
    if forbidden in binding:
        raise SystemExit(f"residual-bound acquisition gained forbidden capability: {forbidden}")

print("residual-bound governed acquisition contract PASS")
