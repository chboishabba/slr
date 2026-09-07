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
    "LocalArtifactPerformedNetworkWork",
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

if "artifact.network_requests != 0" not in loop:
    raise SystemExit("offline loop no longer fails closed on network activity")
if "correspondence.proposition_ref != gap.missing_proposition_ref" not in loop:
    raise SystemExit("offline loop lost exact target-proposition weld")
if "correspondence.graph_ref != bridge.graph_ref" not in loop:
    raise SystemExit("offline loop lost exact graph weld")

print("offline proof-search frontier loop contract PASS")
