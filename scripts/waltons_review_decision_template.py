#!/usr/bin/env python3
"""Build an editable Waltons paragraph-review worksheet from the candidate queue.

This script never decides legal meaning. It copies exact source coordinates/text into
an operator worksheet and leaves inclusion/disposition/reviewer fields unset.
"""
from __future__ import annotations
import argparse, json
from pathlib import Path

ROLE_BY_REQUIREMENT = {
    "requirement:estoppel:assumption": "AssumptionOrExpectation",
    "requirement:estoppel:reliance": "Reliance",
    "requirement:estoppel:detriment": "Detriment",
    "requirement:estoppel:unconscionability": "Unconscionability",
}

def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("queue")
    p.add_argument("output", nargs="?", default="waltons-review-worksheet.json")
    args = p.parse_args()

    queue = json.loads(Path(args.queue).read_text())
    rows = []
    for paragraph in queue.get("matched_paragraphs", []):
        for requirement in paragraph.get("matched_research_criterion_refs", []):
            role = ROLE_BY_REQUIREMENT.get(requirement)
            if role is None:
                continue
            rows.append({
                "include": False,
                "paragraph_locator_ref": paragraph["paragraph_locator_ref"],
                "reported_paragraph_label": paragraph.get("reported_paragraph_label"),
                "paragraph_text": paragraph.get("paragraph_text", ""),
                "source_revision_ref": queue["source_revision_ref"],
                "canonical_text_sha256": queue["canonical_text_sha256"],
                "requirement_ref": requirement,
                "role": role,
                "disposition": None,
                "reviewer_ref": "",
                "review_evidence_refs": [],
                "review_notes": "",
            })

    out = {
        "schema_version": "sl.waltons.review_worksheet.v0_1",
        "source_queue": str(args.queue),
        "candidate_only": True,
        "instructions": {
            "include": "Set true only after reading the exact paragraph in context.",
            "disposition": ["Supports", "Contests", "ContextOnly"],
            "reviewer_ref": "Required for included rows.",
            "review_evidence_refs": "At least one review-note/record reference required.",
            "warning": "A Supports decision pays an evidence-coordinate obligation only; it does not establish proposition truth or binding authority.",
        },
        "rows": rows,
    }
    Path(args.output).write_text(json.dumps(out, indent=2, ensure_ascii=False) + "\n")
    print(f"waltons_review_worksheet={args.output} rows={len(rows)}")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
