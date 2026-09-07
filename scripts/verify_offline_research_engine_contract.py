#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / "crates" / "sl-proof-search-loop" / "src"
FILES = [
    BASE / "engine.rs",
    BASE / "frontier.rs",
    BASE / "hypothesis.rs",
    BASE / "planner.rs",
    BASE / "query.rs",
    BASE / "local.rs",
    BASE / "reasoning.rs",
    BASE / "transition.rs",
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
    "AuthorityTreatment",
    "TerminologyExpansion",
    "pub enum QueryExpr",
    "pub fn compile_austlii",
    "pub fn compile_local",
    "pub struct QueryLexicalContext",
    "pub fn synthesize_queries",
    "pub fn execute_local_candidates",
    "pnf-or-consumer-explicit-lexical-context",
    "pub struct DeclaredResearchValue",
    "pub fn plan_local_frontier_research",
    "expected_residual_reduction",
    "pub struct LocalIndex",
    "pub struct PropositionReasoningEdge",
    "pub enum ReasoningRole",
    "pub enum ConditionKind",
    "pub struct ResidualAssessment",
    "pub fn apply_assessments",
    "ClosedCandidate",
    "BudgetExhausted",
    "pub struct SourceRevisionRecord",
    "SourceRevisionRewriteAttempt",
    'pub const ITERATION_SCHEMA: &str = "sl.proof_search_iteration.v0_1"',
    'pub const FRONTIER_ITERATION_SCHEMA_V02: &str = "sl.proof_search_frontier_iteration.v0_2"',
    "pub fn build_frontier_iteration_receipt_v02",
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

planner = (BASE / "planner.rs").read_text(encoding="utf-8")
engine = (BASE / "engine.rs").read_text(encoding="utf-8")
transition = (BASE / "transition.rs").read_text(encoding="utf-8")
if "target_proposition_ref: hypothesis.target_proposition_ref.clone()" not in planner:
    raise SystemExit("query synthesis lost exact target-proposition identity")
if "network_requests: 0" not in planner:
    raise SystemExit("local query execution no longer pins zero network requests")
if "MissingDeclaredValue" not in engine:
    raise SystemExit("whole-frontier planner may infer proof value from retrieval hits")
if "passages.is_empty()" not in engine:
    raise SystemExit("whole-frontier local plan lost explicit local-hit gating")
if "expected_residual_reduction" not in engine:
    raise SystemExit("whole-frontier planner lost declared proof-reduction calibration")
if 'assessment_authority != "experimental_candidate_only"' not in transition:
    raise SystemExit("frontier transition can accept authority-promoting assessments")
if "ResearchTermination::ClosedCandidate" not in transition:
    raise SystemExit("frontier transition lost candidate-only closed state")

fixture = (ROOT / "crates" / "sl-proof-search-loop" / "examples" / "offline_research_engine_v01.rs").read_text(encoding="utf-8")
if "network_requests: 0" not in fixture:
    raise SystemExit("offline research engine fixture lost zero-network receipt")

print("offline research engine v0.1 contract PASS")
