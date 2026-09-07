from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LIB = ROOT / "crates/sl-governed-legal-provider/src/lib.rs"
CARGO = ROOT / "crates/sl-governed-legal-provider/Cargo.toml"
REPLAY = ROOT / "crates/sl-governed-legal-provider/examples/offline_replay_fixture.rs"
LIVE = ROOT / "crates/sl-governed-legal-provider/examples/live_austlii_smoke.rs"

lib = LIB.read_text(encoding="utf-8")
cargo = CARGO.read_text(encoding="utf-8")
replay = REPLAY.read_text(encoding="utf-8")
live = LIVE.read_text(encoding="utf-8")

required = {
    "historical pacing": "minimum_pacing_seconds: 4",
    "burst one": "burst: 1",
    "bounded citation depth": "max_depth: 1",
    "bounded new documents": "max_new_documents: 5",
    "known authority resolver": "resolve_known_authority",
    "persisted first": "ResolutionStage::Persisted",
    "AustLII SINO": "AUSTLII_SINO_ENDPOINT",
    "deterministic MNC lowering": "deterministic_mnc_to_austlii",
    "JADE search": "jade_search_url",
    "citation treatment intent": "CitationTreatmentIntent",
    "HTTP isolated in provider crate": "trait HttpTransport",
    "explicit operator opt in": "OperatorOptInRequired",
    "request budget": "RequestBudgetExceeded",
    "candidate authority": 'experimental_candidate_only',
    "local ingestion seam": "mark_locally_ingested",
}
for label, needle in required.items():
    if needle not in lib:
        raise SystemExit(f"missing governed-provider contract: {label}: {needle}")

if 'live-network = ["dep:ureq"]' not in cargo:
    raise SystemExit("live HTTP must remain behind the explicit live-network feature")
if 'ureq = ' not in cargo or 'optional = true' not in cargo:
    raise SystemExit("ureq transport must be optional")

for needle in (
    "first_network_requests=1 replay_network_requests=0",
    "ResolutionStage::Persisted",
    "mark_locally_ingested",
):
    if needle not in replay:
        raise SystemExit(f"offline replay fixture lost: {needle}")

for needle in (
    "SENSIBLAW_LIVE_LEGAL_OPT_IN",
    "max_network_requests: 1",
    "max_new_documents: 1",
    "--features live-network",
):
    if needle not in live:
        raise SystemExit(f"live smoke opt-in/bound lost: {needle}")

# The proof scheduler itself must remain network-free. Live transport belongs
# only in the governed provider crate.
scheduler = (ROOT / "crates/sl-proof-search-scheduler/src/lib.rs").read_text(encoding="utf-8")
for forbidden in ("ureq", "reqwest", "TcpStream", "std::net", "hyper::"):
    if forbidden in scheduler:
        raise SystemExit(f"scheduler acquired network capability: {forbidden}")

# Search/fetch still do not grant semantic/legal authority.
for forbidden in ("semantic_authority: true", "legal_authority: true", "publish(", "auto_admit"):
    if forbidden in lib:
        raise SystemExit(f"forbidden authority shortcut in provider crate: {forbidden}")

print("governed legal provider source contract PASS")
