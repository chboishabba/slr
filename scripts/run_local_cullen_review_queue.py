#!/usr/bin/env python3
from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
LIVE_DIR = Path(os.environ.get("SENSIBLAW_LIVE_RECEIPT_DIR", "/tmp/sensiblaw-live-legal"))
RECEIPT = Path(os.environ.get(
    "SENSIBLAW_HCA_JUDGMENT_RECEIPT",
    str(LIVE_DIR / "governed-official-judgment-acquisition-v01.json"),
))
DOCX = Path(os.environ.get("SENSIBLAW_JUDGMENT_DOCX", str(LIVE_DIR / "judgment.docx")))
OUTPUT = Path(os.environ.get("SENSIBLAW_CULLEN_REVIEW_QUEUE", str(LIVE_DIR / "cullen-citation-review-queue-v03.json")))


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def main() -> None:
    require(RECEIPT.is_file(), f"missing full-judgment receipt: {RECEIPT}")
    require(DOCX.is_file(), f"missing retained official judgment DOCX: {DOCX}")
    data = json.loads(RECEIPT.read_text(encoding="utf-8"))
    require(data.get("schema_version") == "sl.governed_official_judgment_acquisition.v0_1", "unexpected full-judgment receipt schema")
    require(data.get("authority") == "experimental_candidate_only", "full-judgment receipt must remain candidate-only")
    require(data.get("provider") == "HighCourtAustralia", "Cullen review queue requires official HCA receipt")
    require(data.get("medium_neutral_citation") == "[2026] HCA 19", "unexpected Cullen calibration")
    require(data.get("landing_page_network_requests") == 0, "review queue must reuse persisted landing state")
    require((data.get("document_fetch") or {}).get("network_requests") == 1, "full judgment receipt must retain one DOCX request")
    require((data.get("replay_run") or {}).get("network_requests") == 0, "full judgment replay must be zero-network")

    binding = data.get("binding") or {}
    canonical = data.get("canonical_text") or {}
    document = data.get("document_fetch") or {}
    required = {
        "SENSIBLAW_JUDGMENT_DOCX": str(DOCX),
        "SENSIBLAW_CULLEN_REVIEW_QUEUE": str(OUTPUT),
        "SENSIBLAW_DOCUMENT_REF": data.get("document_source_identity_ref") or data.get("source_identity_ref") or "document:hca:[2026]-HCA-19:docx",
        "SENSIBLAW_SOURCE_REVISION_REF": document.get("source_revision_ref", ""),
        "SENSIBLAW_CANONICAL_TEXT_SHA256": canonical.get("sha256", ""),
        "SENSIBLAW_BOUND_RESIDUAL_REF": binding.get("residual_ref", ""),
        "SENSIBLAW_BOUND_PROPOSITION_REF": binding.get("proposition_ref", ""),
        "SENSIBLAW_BOUND_PRODUCER_REF": binding.get("scheduled_producer_ref", ""),
        "SENSIBLAW_BOUND_HYPOTHESIS_REF": binding.get("hypothesis_ref", ""),
    }
    for name, value in required.items():
        require(bool(value), f"full-judgment receipt missing required queue input: {name}")

    env = os.environ.copy()
    env.update(required)
    subprocess.run(
        ["cargo", "run", "-p", "sensiblaw-proof-search-loop", "--example", "live_cullen_citation_review_queue"],
        cwd=ROOT,
        env=env,
        check=True,
    )
    subprocess.run(
        ["python3", "scripts/verify_live_cullen_review_queue.py", str(OUTPUT)],
        cwd=ROOT,
        check=True,
    )
    print(f"LOCAL CULLEN ANCHORED REVIEW QUEUE PASS output={OUTPUT}")


if __name__ == "__main__":
    main()
