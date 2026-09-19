#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCHEDULER = ROOT / "crates" / "sl-proof-search-scheduler" / "src" / "lib.rs"
EXAMPLE = ROOT / "crates" / "sl-proof-search-scheduler" / "examples" / "offline_pabai.rs"
WORKSPACE = ROOT / "Cargo.toml"

scheduler = SCHEDULER.read_text(encoding="utf-8")
example = EXAMPLE.read_text(encoding="utf-8")
workspace = WORKSPACE.read_text(encoding="utf-8")

required = [
    "pub enum ExecutionStrategy",
    "PersistedAuthorityReceipt",
    "GovernedLiveReferenceSearch",
    "pub struct ExecutionCostVector",
    "pub struct ProofValueVector",
    "pub struct CandidateMove",
    "pub struct CandidateMoveReceipt",
    "ExperimentalCandidateOnly",
    "GovernedLiveAdapterRequired",
    "pub fn dominates",
    "pub fn threshold_candidates",
    "pub fn pareto_frontier",
    "pub fn schedule",
    "pub fn require_offline_execution",
    "pub fn move_from_legal_source_plan",
    "minimum_proof_reduction_filters_cheap_but_useless_move",
    "equivalent_live_move_is_dominated_by_offline_move",
    "live_high_value_move_can_remain_pareto_but_cannot_execute_here",
]
missing = [needle for needle in required if needle not in scheduler]
if missing:
    raise SystemExit(f"proof-search scheduler contract missing: {missing}")

if '"crates/sl-proof-search-scheduler"' not in workspace:
    raise SystemExit("workspace missing sl-proof-search-scheduler")

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
    "automatic_admission",
    "semantic_authority: true",
    "legal_authority: true",
]:
    if forbidden in scheduler or forbidden in example:
        raise SystemExit(f"offline proof-search scheduler acquired forbidden capability: {forbidden}")

if 'receipt_authority: "experimental_candidate_only"' not in scheduler:
    raise SystemExit("candidate-only receipt authority boundary missing")

if "minimum_pacing_seconds: 4" not in scheduler:
    raise SystemExit("governed-live candidate must expose four-second legal-host pacing cost")

if "require_offline_execution(&receipt)" not in example:
    raise SystemExit("offline example must fail closed before any live execution")

print("offline costed proof-search scheduler contract PASS")
