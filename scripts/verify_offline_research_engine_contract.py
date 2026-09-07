#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / "crates" / "sl-proof-search-loop" / "src"
FILES = [
    BASE / "frontier.rs",
    BASE / "hypothesis.rs",
    BASE / "query.rs",
    BASE / "local.rs",
    BASE / "reasoning.rs",
    BASE / "world.rs",
    BASE / "receipt.rs",
]
text = "\n".join(p.read_text(encoding="utf-8") for p in FILES)

required = [
    "pub struct ProofFrontier",
    "pub struct ProofResidual",
    "pub fn select_frontier_move",
    "pub enum SearchHypothesisKind",
    "Defeater",
    "Comparator",
    "Contradiction",
    "pub enum QueryExpr",
    "pub fn compile_austlii",
    "pub fn compile_local",
    "pub struct LocalIndex",
    "pub struct PropositionReasoningEdge",
    "pub enum ReasoningRole",
    "pub enum ConditionKind",
    "pub struct SourceRevisionRecord",
    "SourceRevisionRewriteAttempt",
    'pub const ITERATION_SCHEMA: &str = "sl.proof_search_iteration.v0_1"',
    'authority_boundary: "experimental_candidate_only"',
]
missing = [needle for needle in required if needle not in text]
if missing:
    raise SystemExit(f"offline research engine contract missing: {missing}")

for forbidden in [
    "reqwest",
    "ureq",
    "hyper::",
    "TcpStream",
    "std::net",
    "urlopen",
    "curl",
    "publish(",
    "automatic_admission",
    "auto_admit",
]:
    if forbidden in text:
        raise SystemExit(f"offline research engine acquired forbidden capability: {forbidden}")

if "network_requests: 0" not in (ROOT / "crates" / "sl-proof-search-loop" / "examples" / "offline_research_engine_v01.rs").read_text(encoding="utf-8"):
    raise SystemExit("offline research engine fixture lost zero-network receipt")

print("offline research engine v0.1 contract PASS")
