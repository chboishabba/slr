from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LIB = ROOT / "crates/sl-governed-legal-provider/src/lib.rs"
EXAMPLE = ROOT / "crates/sl-governed-legal-provider/examples/source_lineage_candidates.rs"
WORKSPACE = ROOT / "Cargo.toml"

lib = LIB.read_text()
example = EXAMPLE.read_text()
workspace = WORKSPACE.read_text()

required = [
    "pub enum LegalProvider",
    "pub enum ProviderOperation",
    "pub enum AcquisitionAuthority",
    "ExperimentalCandidateOnly",
    "pub enum PropositionUseIntent",
    "pub enum CitationTreatmentIntent",
    "pub struct KnownAuthorityDemand",
    "pub struct PersistedAuthorityReceipt",
    "pub struct LiveGovernanceBounds",
    "minimum_pacing_seconds: 4",
    "burst: 1",
    "max_depth: 1",
    "max_new_documents: 5",
    "pub enum ResolutionStage",
    "Persisted",
    "ExplicitAustLII",
    "JadeExactMnc",
    "DeterministicMncToAustLII",
    "AustLIIReferenceSearch",
    "Unresolved",
    "pub fn resolve_known_authority",
    "pub fn citation_traversal_plan",
    "search_is_semantic_payment",
    "acquisition_is_authority_receipt",
    'receipt_authority: "experimental_candidate_only"',
]

for needle in required:
    if needle not in lib:
        raise SystemExit(f"missing governed-provider contract marker: {needle}")

for forbidden in [
    "reqwest",
    "ureq",
    "hyper::",
    "TcpStream",
    "tokio::net",
    "std::net",
    "curl",
    "urlopen",
    "publish(",
    "auto_admit",
    "semantic_authority: true",
    "legal_authority: true",
]:
    if forbidden in lib or forbidden in example:
        raise SystemExit(f"forbidden live/authority primitive in governed provider contract: {forbidden}")

if '"crates/sl-governed-legal-provider"' not in workspace:
    raise SystemExit("governed legal provider crate missing from workspace")

for source_marker in [
    "case:Mallonland-v-Advanta-Seeds-2024-HCA-25",
    "case:Cullen-v-State-of-Queensland-2026-HCA-19",
    "case:Pabai-v-Commonwealth-2025-FCA-796",
]:
    if source_marker not in example:
        raise SystemExit(f"source-lineage fixture missing source carrier: {source_marker}")

print("governed legal provider contract: ok")
