#!/usr/bin/env python3
"""Build an editable proposition-level treatment review worksheet.

Input is sl.authority_treatment_review_queue.v0_1. Nothing is pre-classified:
the operator must select an anchor and legal citation use/role explicitly.
"""
from __future__ import annotations
import argparse, json
from pathlib import Path

def main() -> int:
    p=argparse.ArgumentParser()
    p.add_argument("queue")
    p.add_argument("root_authority_ref")
    p.add_argument("output", nargs="?", default="citation-treatment-review-worksheet.json")
    a=p.parse_args()
    q=json.loads(Path(a.queue).read_text())
    if q.get("schema_version")!="sl.authority_treatment_review_queue.v0_1":
        raise SystemExit("unsupported treatment queue schema")
    rows=[]
    for unit in q.get("review_units",[]):
        rows.append({
            "include": False,
            "review_unit_ref": unit["review_unit_ref"],
            "source_revision_ref": unit["source_revision_ref"],
            "citation_text": unit["citation_text"],
            "citation_locator_refs": unit.get("citation_locator_refs",[]),
            "anchor_paragraph_locator_refs": unit.get("anchor_paragraph_locator_refs",[]),
            "anchor_paragraph_texts": unit.get("anchor_paragraph_texts",[]),
            "selected_anchor_paragraph_locator_ref": None,
            "citing_proposition_ref": "",
            "cited_document_ref": a.root_authority_ref,
            "cited_proposition_ref": "",
            "citation_use": None,
            "reasoning_role": None,
            "condition_coordinates": [],
            "judge_or_speaker_ref": None,
            "court_ref": None,
            "jurisdiction_ref": None,
            "temporal_ref": None,
            "outcome_ref": None,
            "remedy_ref": None,
            "burden_refs": [],
            "exception_refs": [],
            "lexical_realisation": "",
            "reviewer_ref": "",
            "evidence_refs": [],
            "review_notes": "",
        })
    out={
      "schema_version":"sl.citation_treatment_review_worksheet.v0_1",
      "source_queue":str(a.queue),
      "root_authority_ref":a.root_authority_ref,
      "candidate_only":True,
      "allowed_citation_use":[
        "Mentioned","Quoted","ReliedOn","Adopted","Applied","Followed",
        "Distinguished","Criticised","Rejected","Overruled","PartySubmission",
        "HistoricalBackground","Unresolved"
      ],
      "allowed_reasoning_role":[
        "Rule","Premise","Exception","Analogy","Distinction","Policy",
        "FactualFinding","Burden","Remedy","Unresolved"
      ],
      "rows":rows,
    }
    Path(a.output).write_text(json.dumps(out,indent=2,ensure_ascii=False)+"\n")
    print(f"treatment_review_worksheet={a.output} rows={len(rows)}")
    return 0
if __name__=="__main__":
    raise SystemExit(main())
