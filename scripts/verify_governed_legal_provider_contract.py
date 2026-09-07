from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LIB = ROOT / "crates/sl-governed-legal-provider/src/lib.rs"
CARGO = ROOT / "crates/sl-governed-legal-provider/Cargo.toml"
REPLAY = ROOT / "crates/sl-governed-legal-provider/examples/offline_replay_fixture.rs"
LIVE = ROOT / "crates/sl-governed-legal-provider/examples/live_austlii_smoke.rs"
RUNNER = ROOT / "scripts/run_live_legal_smoke.sh"
VALIDATOR = ROOT / "scripts/verify_live_legal_receipt.py"

lib = LIB.read_text(encoding="utf-8")
cargo = CARGO.read_text(encoding="utf-8")
replay = REPLAY.read_text(encoding="utf-8")
live = LIVE.read_text(encoding="utf-8")
runner = RUNNER.read_text(encoding="utf-8")
validator = VALIDATOR.read_text(encoding="utf-8")

required = {
    "historical pacing": "minimum_pacing_seconds: 4",
    "burst one": "burst: 1",
    "bounded citation depth": "max_depth: 1",
    "bounded new documents": "max_new_documents: 5",
    "known authority resolver": "resolve_known_authority",
    "whole resolution candidates": "resolution_candidates",
    "persisted first": "ResolutionStage::Persisted",
    "OALC exact MNC": "lookup_oalc_exact_mnc",
    "OALC provider": "LegalProvider::Oalc",
    "official HCA provider": "LegalProvider::HighCourtAustralia",
    "official FCA provider": "LegalProvider::FederalCourtAustralia",
    "official HCA calibration": "official_hca_known_reference",
    "official FCA calibration": "official_fca_known_reference",
    "provider availability": "ProviderAccessStatus",
    "403 policy classification": "PolicyBlocked",
    "TLS classification": "TlsInvalid",
    "provider failure not evidence": "provider_failure_is_negative_legal_evidence",
    "AustLII sanctioned mode": "AustliiAccessMode",
    "AustLII SINO": "AUSTLII_SINO_ENDPOINT",
    "deterministic MNC lowering": "deterministic_mnc_to_austlii",
    "JADE search": "jade_search_url",
    "citation treatment intent": "CitationTreatmentIntent",
    "HTTP isolated in provider crate": "trait HttpTransport",
    "explicit operator opt in": "OperatorOptInRequired",
    "request budget": "RequestBudgetExceeded",
    "candidate authority": "experimental_candidate_only",
    "search returns references": "SearchReferenceReceipt",
    "local ingestion seam": "mark_locally_ingested",
}
for label, needle in required.items():
    if needle not in lib:
        raise SystemExit(f"missing governed-provider contract: {label}: {needle}")

if 'live-network = ["dep:ureq"]' not in cargo:
    raise SystemExit("live HTTP must remain behind the explicit live-network feature")
if 'ureq = ' not in cargo or 'optional = true' not in cargo:
    raise SystemExit("ureq transport must be optional")
if 'sha2 = ' not in cargo:
    raise SystemExit("live acquisition receipts must retain SHA256 bytes identity")

for needle in (
    "provider=HighCourtAustralia first_network_requests=1 replay_network_requests=0",
    "ResolutionStage::Persisted",
    "ResolutionStage::OfficialHighCourt",
    "mark_locally_ingested",
):
    if needle not in replay:
        raise SystemExit(f"offline replay fixture lost: {needle}")

for needle in (
    "SENSIBLAW_LIVE_LEGAL_OPT_IN",
    "SENSIBLAW_RUNTIME_HEAD",
    "max_network_requests: 1",
    "max_new_documents: 1",
    "official_hca_known_reference",
    "fetch_hca",
    "HighCourtAustralia",
    "Sha256::digest",
    "sl.governed_legal_acquisition.v0_1",
    "replay_run",
    "search_claimed_semantic_payment",
    "acquisition_claimed_authority_receipt",
):
    if needle not in live:
        raise SystemExit(f"live smoke acquisition/receipt contract lost: {needle}")

for needle in (
    "git rev-parse HEAD",
    "--features live-network",
    "verify_live_legal_receipt.py",
):
    if needle not in runner:
        raise SystemExit(f"explicit live runner contract lost: {needle}")

for needle in (
    'SCHEMA = "sl.governed_legal_acquisition.v0_1"',
    'AUTHORITY = "experimental_candidate_only"',
    'data.get("provider") != "HighCourtAustralia"',
    'first.get("network_requests") != 1',
    'replay.get("network_requests") != 0',
    'replay.get("resolution") != "Persisted"',
):
    if needle not in validator:
        raise SystemExit(f"live receipt validator contract lost: {needle}")

scheduler = (ROOT / "crates/sl-proof-search-scheduler/src/lib.rs").read_text(encoding="utf-8")
for forbidden in ("ureq", "reqwest", "TcpStream", "std::net", "hyper::"):
    if forbidden in scheduler:
        raise SystemExit(f"scheduler acquired network capability: {forbidden}")

for forbidden in ("semantic_authority: true", "legal_authority: true", "publish(", "auto_admit"):
    if forbidden in lib:
        raise SystemExit(f"forbidden authority shortcut in provider crate: {forbidden}")

print("governed legal provider source contract PASS")
