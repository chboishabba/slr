#!/usr/bin/env python3
"""Normalize sanctioned CitedBy provider results for the Waltons treatment lane.

This is an import boundary, not a treatment classifier. Expected provider JSON:
{
  "provider": "Jade",
  "operation": "CitedBy",
  "root_medium_neutral_citation": "[1988] HCA 7",
  "candidates": [
    {"medium_neutral_citation":"[2014] HCA 19","explicit_reference":"...","provider_record_ref":"..."}
  ]
}
"""
from __future__ import annotations
import argparse,json,re
from pathlib import Path

MNC=re.compile(r"^\[(\d{4})\]\s+([A-Z][A-Z0-9]*)\s+(\d+)$")

def main()->int:
    p=argparse.ArgumentParser()
    p.add_argument("manifest")
    p.add_argument("provider_results")
    p.add_argument("output",nargs="?",default="waltons-cited-by-candidates.json")
    a=p.parse_args()
    manifest=json.loads(Path(a.manifest).read_text())
    raw=json.loads(Path(a.provider_results).read_text())
    root=manifest["root_authority"]["medium_neutral_citation"]
    if raw.get("operation")!="CitedBy": raise SystemExit("provider result operation must be CitedBy")
    if raw.get("root_medium_neutral_citation")!=root: raise SystemExit("provider result root citation mismatch")
    candidates=[]
    seen=set()
    for i,row in enumerate(raw.get("candidates",[]),1):
        mnc=(row.get("medium_neutral_citation") or "").strip()
        if not MNC.match(mnc): raise SystemExit(f"candidate {i}: invalid neutral citation {mnc!r}")
        if mnc in seen: continue
        seen.add(mnc)
        candidates.append({
          "medium_neutral_citation":mnc,
          "explicit_reference":row.get("explicit_reference"),
          "provider_record_ref":row.get("provider_record_ref"),
          "candidate_only":True,
          "treatment_classified":False,
          "creates_legal_authority":False,
        })
    out={
      "schema_version":"sl.cited_by_candidates.v0_1",
      "provider":raw.get("provider"),
      "operation":"CitedBy",
      "root_medium_neutral_citation":root,
      "manifest":str(a.manifest),
      "candidate_count":len(candidates),
      "candidate_only":True,
      "provider_result_is_treatment":False,
      "candidates":candidates,
    }
    Path(a.output).write_text(json.dumps(out,indent=2)+"\n")
    print(f"cited_by_candidates={a.output} count={len(candidates)}")
    return 0
if __name__=="__main__":
    raise SystemExit(main())
