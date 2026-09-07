#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LOOP = ROOT / "crates" / "sl-proof-search-loop" / "src" / "lib.rs"
WORKSPACE = ROOT / "Cargo.toml"

loop = LOOP.read_text(encoding="utf-8")
workspace = WORKSPACE.read_text(encoding="utf-8")

required = [
    "pub struct LocalArtifactReceipt",
    "pub enum AssessmentGrade",
    "pub enum FrontierDisposition",
    "pub struct FrontierDelta",
    "pub fn assess_offline_result",
    "pub fn wake_from_delta",
    "pub fn reschedule_after_delta",
    "SelectedMoveReceiptMismatch",
    "SelectedSourceRevisionMismatch",
    "LocalArtifactPerformedNetworkWork",
    "BridgeDigestMismatch",
    "CorrespondenceClaimsAuthority",
    'delta_authority: "experimental_candidate_only"',
    'iteration_authority: "experimental_candidate_only"',
    "world_truth_claimed",
    "legal_holding_claimed",
    "require_offline_execution",
    "sparse_wake",
]
missing = [needle for needle in required if needle not in loop]
if missing:
    raise SystemExit(f"proof-search loop contract missing: {missing}")

if '"crates/sl-proof-search-loop"' not in workspace:
    raise SystemExit("workspace missing sl-proof-search-loop")

for forbidden in [
    "reqwest",
    "ureq",
    "hyper::",
    "TcpStream",
    "std::net",
    "urlopen",
    "curl",
    "publish(",
    "auto_admit",
    "automatic_admission",
]:
    if forbidden in loop:
        raise SystemExit(f"proof-search loop acquired forbidden capability: {forbidden}")

required_welds = [
    "selected_receipt.selected_move_ref != selected_move.move_ref",
    "selected_move.source_ref.as_deref() != Some(artifact.source_revision_ref.as_str())",
    "artifact.network_requests != 0",
    "bridge.canonical_text_sha256 != artifact.bytes_digest_ref",
    "bridge.parser_observation_is_semantic_authority",
    "!bridge.semantic_correspondence_required",
    "correspondence.proposition_ref != gap.missing_proposition_ref",
    "correspondence.graph_ref != bridge.graph_ref",
    "correspondence.world_truth_claimed || correspondence.legal_holding_claimed",
]
missing_welds = [needle for needle in required_welds if needle not in loop]
if missing_welds:
    raise SystemExit(f"offline proof-search weld missing: {missing_welds}")

print("offline proof-search frontier loop contract PASS")
