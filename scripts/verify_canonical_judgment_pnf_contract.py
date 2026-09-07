#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MODULE = ROOT / "crates/sl-proof-search-loop/src/judgment_pnf.rs"
EXAMPLE = ROOT / "crates/sl-proof-search-loop/examples/canonical_judgment_pnf_bridge.rs"

module = MODULE.read_text(encoding="utf-8")
example = EXAMPLE.read_text(encoding="utf-8")

for needle in (
    "EvidentialBridgeReceipt",
    "compile_canonical_judgment_text_to_pnf_bridge",
    "canonical_text_sha256",
    "paragraph_count",
    "world_resolution_deferred",
    "semantic_correspondence_required",
    "parser_observation_is_semantic_authority",
    "canonical_text_bridge_is_semantic_correspondence() -> bool { false }",
    "canonical_text_bridge_is_world_truth() -> bool { false }",
    "canonical_text_bridge_is_legal_holding() -> bool { false }",
):
    if needle not in module:
        raise SystemExit(f"canonical judgment PNF module missing: {needle}")

for needle in (
    "canonical_judgment_pnf_bridge=PASS",
    "residual:current-treatment",
    "graph:cullen:source-grounded",
    "semantic_authority=false",
    "correspondence_required=true",
):
    if needle not in example:
        raise SystemExit(f"canonical judgment PNF fixture missing: {needle}")

for forbidden in ("publish(", "auto_admit", "world_truth_claimed: true", "legal_holding_claimed: true"):
    if forbidden in module or forbidden in example:
        raise SystemExit(f"canonical judgment PNF handoff acquired forbidden authority shortcut: {forbidden}")

print("canonical judgment PNF handoff contract PASS")
