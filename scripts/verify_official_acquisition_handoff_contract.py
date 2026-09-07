from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ACQ = ROOT / "crates/sl-proof-search-loop/src/acquisition.rs"
LIB = ROOT / "crates/sl-proof-search-loop/src/lib.rs"
FIXTURE = ROOT / "crates/sl-proof-search-loop/examples/official_acquisition_compounding.rs"
CARGO = ROOT / "crates/sl-proof-search-loop/Cargo.toml"

acq = ACQ.read_text(encoding="utf-8")
lib = LIB.read_text(encoding="utf-8")
fixture = FIXTURE.read_text(encoding="utf-8")
cargo = CARGO.read_text(encoding="utf-8")

for needle in (
    "pub struct AcquiredAuthorityHandoff",
    "handoff_locally_ingested_authority",
    "NotLocallyIngested",
    "NonCandidateAuthority",
    "world.append_source",
    "acquisition_handoff_is_semantic_payment",
):
    if needle not in acq:
        raise SystemExit(f"official acquisition handoff contract lost: {needle}")

if "pub mod acquisition;" not in lib:
    raise SystemExit("proof-search loop no longer exposes acquisition handoff")
if "sensiblaw-governed-legal-provider" not in cargo:
    raise SystemExit("proof-search loop lost governed-provider dependency")

for needle in (
    "LegalProvider::HighCourtAustralia",
    "handoff_locally_ingested_authority",
    "world.apply_reasoning_delta",
    "authority_neighbourhood",
    "query_vocabulary",
    "ResidualAssessmentKind::Narrowed",
    "ResearchTermination::Continue",
    "acquisition_network=1 world_source_added=true reasoning_delta_added=true frontier=Continue",
):
    if needle not in fixture:
        raise SystemExit(f"official acquisition compounding fixture lost: {needle}")

for forbidden in ("semantic_payment = true", "legal_authority = true", "publish(", "auto_admit"):
    if forbidden in acq or forbidden in fixture:
        raise SystemExit(f"forbidden authority shortcut in acquisition handoff: {forbidden}")

print("official acquisition handoff contract PASS")
