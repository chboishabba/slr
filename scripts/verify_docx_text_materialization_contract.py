#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MODULE = ROOT / "crates/sl-governed-legal-provider/src/docx_text.rs"
EXAMPLE = ROOT / "crates/sl-governed-legal-provider/examples/docx_text_materialization.rs"
LIVE = ROOT / "crates/sl-governed-legal-provider/examples/live_hca_judgment_docx_smoke.rs"
CARGO = ROOT / "crates/sl-governed-legal-provider/Cargo.toml"

module = MODULE.read_text(encoding="utf-8")
example = EXAMPLE.read_text(encoding="utf-8")
live = LIVE.read_text(encoding="utf-8")
cargo = CARGO.read_text(encoding="utf-8")

for needle in (
    "extract_docx_canonical_text",
    "document_xml_to_canonical_text",
    "word/document.xml",
    "paragraph_count",
    "docx_text_materialization_is_semantic_payment() -> bool { false }",
    "docx_text_materialization_is_legal_authority() -> bool { false }",
):
    if needle not in module:
        raise SystemExit(f"DOCX text materialization module missing: {needle}")

for needle in (
    "ZipWriter",
    "word/document.xml",
    "positive operational act",
    "network_requests=0",
    "semantic_payment=false",
    "legal_authority=false",
):
    if needle not in example:
        raise SystemExit(f"DOCX offline fixture missing: {needle}")

for needle in (
    "extract_docx_canonical_text",
    "canonical_text_digest",
    'canonical_text_path = output_dir.join("judgment.txt")',
    "canonical_text_claimed_semantic_payment",
):
    if needle not in live:
        raise SystemExit(f"live DOCX materialization path missing: {needle}")

if 'xml-rs = "0.8"' not in cargo:
    raise SystemExit("DOCX XML reader dependency must remain explicit")
if 'zip = { version = "0.6"' not in cargo:
    raise SystemExit("DOCX ZIP reader dependency must remain explicit")

for forbidden in ("publish(", "auto_admit", "semantic_authority: true", "legal_authority: true"):
    if forbidden in module or forbidden in example or forbidden in live:
        raise SystemExit(f"DOCX materialization acquired forbidden authority shortcut: {forbidden}")

print("DOCX text materialization contract PASS")
