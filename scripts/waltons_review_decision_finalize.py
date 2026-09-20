#!/usr/bin/env python3
"""Validate an edited Waltons review worksheet and emit compiler decisions."""
from __future__ import annotations
import argparse, json
from pathlib import Path

ALLOWED = {"Supports", "Contests", "ContextOnly"}

def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("worksheet")
    p.add_argument("output", nargs="?", default="waltons-reviewed-decisions.json")
    args = p.parse_args()
    sheet = json.loads(Path(args.worksheet).read_text())
    if sheet.get("schema_version") != "sl.waltons.review_worksheet.v0_1":
        raise SystemExit("unsupported Waltons worksheet schema")

    decisions = []
    for idx, row in enumerate(sheet.get("rows", []), start=1):
        if not row.get("include"):
            continue
        disposition = row.get("disposition")
        reviewer = (row.get("reviewer_ref") or "").strip()
        evidence = [x for x in row.get("review_evidence_refs", []) if str(x).strip()]
        if disposition not in ALLOWED:
            raise SystemExit(f"row {idx}: disposition must be one of {sorted(ALLOWED)}")
        if not reviewer:
            raise SystemExit(f"row {idx}: reviewer_ref required")
        if not evidence:
            raise SystemExit(f"row {idx}: at least one review_evidence_ref required")
        for field in ("paragraph_locator_ref", "source_revision_ref", "canonical_text_sha256", "role"):
            if not row.get(field):
                raise SystemExit(f"row {idx}: missing {field}")
        decisions.append({
            "paragraph_locator_ref": row["paragraph_locator_ref"],
            "source_revision_ref": row["source_revision_ref"],
            "canonical_text_sha256": row["canonical_text_sha256"],
            "role": row["role"],
            "disposition": disposition,
            "reviewer_ref": reviewer,
            "review_evidence_refs": evidence,
        })

    out = {
        "schema_version": "sl.waltons.review_decisions.v0_1",
        "worksheet": str(args.worksheet),
        "decisions": decisions,
    }
    Path(args.output).write_text(json.dumps(out, indent=2) + "\n")
    print(f"waltons_review_decisions={args.output} included={len(decisions)}")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
