#!/usr/bin/env python3
"""Validate an edited treatment worksheet into Rust-deserializable decisions."""
from __future__ import annotations
import argparse,json
from pathlib import Path

USES={"Mentioned","Quoted","ReliedOn","Adopted","Applied","Followed","Distinguished","Criticised","Rejected","Overruled","PartySubmission","HistoricalBackground","Unresolved"}
ROLES={"Rule","Premise","Exception","Analogy","Distinction","Policy","FactualFinding","Burden","Remedy","Unresolved"}
KINDS={"Factual","Legal","Jurisdictional","Temporal","Procedural","Evidential","Exception"}

def main()->int:
    p=argparse.ArgumentParser()
    p.add_argument("worksheet")
    p.add_argument("output",nargs="?",default="citation-treatment-reviewed-decisions.json")
    a=p.parse_args()
    w=json.loads(Path(a.worksheet).read_text())
    if w.get("schema_version")!="sl.citation_treatment_review_worksheet.v0_1":
        raise SystemExit("unsupported treatment worksheet schema")
    decisions=[]
    for i,row in enumerate(w.get("rows",[]),1):
        if not row.get("include"): continue
        anchors=row.get("anchor_paragraph_locator_refs",[])
        selected=row.get("selected_anchor_paragraph_locator_ref")
        if selected not in anchors: raise SystemExit(f"row {i}: selected anchor must belong to review unit")
        if row.get("citation_use") not in USES: raise SystemExit(f"row {i}: invalid citation_use")
        if row.get("reasoning_role") not in ROLES: raise SystemExit(f"row {i}: invalid reasoning_role")
        if not (row.get("reviewer_ref") or "").strip(): raise SystemExit(f"row {i}: reviewer_ref required")
        if not row.get("evidence_refs"): raise SystemExit(f"row {i}: evidence_refs required")
        for fld in ("review_unit_ref","source_revision_ref","citation_text","citing_proposition_ref","cited_document_ref","cited_proposition_ref","lexical_realisation"):
            if not row.get(fld): raise SystemExit(f"row {i}: {fld} required")
        coords=[]
        for c in row.get("condition_coordinates",[]):
            if c.get("kind") not in KINDS or not c.get("condition_ref"):
                raise SystemExit(f"row {i}: invalid condition coordinate")
            coords.append({"kind":c["kind"],"condition_ref":c["condition_ref"]})
        decisions.append({
          "review_unit_ref":row["review_unit_ref"],
          "source_revision_ref":row["source_revision_ref"],
          "citation_text":row["citation_text"],
          "selected_anchor_paragraph_locator_ref":selected,
          "citing_proposition_ref":row["citing_proposition_ref"],
          "cited_document_ref":row["cited_document_ref"],
          "cited_proposition_ref":row["cited_proposition_ref"],
          "citation_use":row["citation_use"],
          "reasoning_role":row["reasoning_role"],
          "condition_coordinates":coords,
          "judge_or_speaker_ref":row.get("judge_or_speaker_ref"),
          "court_ref":row.get("court_ref"),
          "jurisdiction_ref":row.get("jurisdiction_ref"),
          "temporal_ref":row.get("temporal_ref"),
          "outcome_ref":row.get("outcome_ref"),
          "remedy_ref":row.get("remedy_ref"),
          "burden_refs":row.get("burden_refs",[]),
          "exception_refs":row.get("exception_refs",[]),
          "lexical_realisation":row["lexical_realisation"],
          "reviewer_ref":row["reviewer_ref"],
          "evidence_refs":row["evidence_refs"],
        })
    out={"schema_version":"sl.citation_treatment_review_decisions.v0_1","worksheet":str(a.worksheet),"decisions":decisions}
    Path(a.output).write_text(json.dumps(out,indent=2)+"\n")
    print(f"treatment_review_decisions={a.output} included={len(decisions)}")
    return 0
if __name__=="__main__":
    raise SystemExit(main())
