#!/usr/bin/env python3
"""Merge per-authority treatment queues into one deterministic review queue."""
from __future__ import annotations
import argparse,json
from pathlib import Path

def main()->int:
    p=argparse.ArgumentParser()
    p.add_argument("queues",nargs="+")
    p.add_argument("--output",default="waltons-treatment-review-queue.json")
    a=p.parse_args()
    units={}
    roots=set()
    sources=[]
    for qp in a.queues:
        q=json.loads(Path(qp).read_text())
        if q.get("schema_version")!="sl.authority_treatment_review_queue.v0_1":
            raise SystemExit(f"{qp}: unsupported schema")
        roots.add(q.get("root_authority_citation"))
        sources.append({
          "queue":qp,
          "source_document_ref":q.get("source_document_ref"),
          "source_revision_ref":q.get("source_revision_ref"),
          "canonical_text_sha256":q.get("canonical_text_sha256"),
        })
        for unit in q.get("review_units",[]):
            ref=unit["review_unit_ref"]
            if ref in units and units[ref]!=unit:
                raise SystemExit(f"conflicting review unit {ref}")
            units[ref]=unit
    if len(roots)!=1:
        raise SystemExit(f"queues disagree on root authority citation: {sorted(roots)}")
    merged=sorted(units.values(),key=lambda u:(u.get("source_revision_ref",""),u.get("review_unit_ref","")))
    out={
      "schema_version":"sl.authority_treatment_review_queue.v0_1",
      "root_authority_citation":next(iter(roots)),
      "source_document_ref":"aggregate:later-authorities",
      "source_revision_ref":"aggregate:review-unit-preserved-source-revisions",
      "canonical_text_sha256":"aggregate:per-unit-digests-preserved",
      "source_queues":sources,
      "review_unit_count":len(merged),
      "candidate_only":True,
      "treatment_classified":False,
      "review_units":merged,
    }
    Path(a.output).write_text(json.dumps(out,indent=2)+"\n")
    print(f"merged_treatment_queue={a.output} units={len(merged)} sources={len(sources)}")
    return 0
if __name__=="__main__":
    raise SystemExit(main())
