#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MODULE = ROOT / "crates/sl-governed-legal-provider/src/official_resource.rs"
EXAMPLE = ROOT / "crates/sl-governed-legal-provider/examples/hca_landing_resource_discovery.rs"
FIXTURE = ROOT / "fixtures/hca_cullen_landing_minimal.html"

module = MODULE.read_text(encoding="utf-8")
example = EXAMPLE.read_text(encoding="utf-8")
fixture = FIXTURE.read_text(encoding="utf-8")

for needle in (
    "JudgmentResourceKind",
    "Docx",
    "Pdf",
    "discover_hca_judgment_resources",
    "preferred_hca_judgment_resource",
    "resource_discovery_is_semantic_payment() -> bool { false }",
    "resource_discovery_is_legal_authority() -> bool { false }",
):
    if needle not in module:
        raise SystemExit(f"HCA resource discovery module missing: {needle}")

for needle in (
    "sl.official_judgment_resource_discovery.v0_1",
    '"network_requests\\\": 0',
    'preferred_kind\\\": \\\"Docx',
    "resource_discovery_claimed_semantic_payment",
    "resource_discovery_claimed_legal_authority",
):
    if needle not in example:
        raise SystemExit(f"HCA resource discovery fixture missing: {needle}")

if ".docx" not in fixture or ".pdf" not in fixture or "[2026] HCA 19" not in fixture:
    raise SystemExit("HCA fixture must retain Cullen MNC plus DOCX and PDF resources")

for forbidden in ("ureq", "reqwest", "TcpStream", "std::net", "publish(", "auto_admit"):
    if forbidden in module or forbidden in example:
        raise SystemExit(f"resource discovery acquired forbidden capability: {forbidden}")

print("HCA judgment resource discovery contract PASS")
