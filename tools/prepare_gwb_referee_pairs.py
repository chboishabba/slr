#!/usr/bin/env python3
"""Prepare every direct candidate pair for each explicit consumer query.
No certification, payment or transitive licence. Native binding owner
computes digests; the independent referee must check the exact theorem.
"""
import argparse
import itertools
import json
import os
from pathlib import Path
import subprocess

def require(ok,reason):
    if not ok: raise ValueError(reason)

def put(path,obj):
    temp=path.with_suffix(path.suffix+".tmp")
    temp.write_text(json.dumps(obj,sort_keys=True,indent=2)+"\n",encoding="utf-8")
    temp.replace(path)

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument("--handoff",type=Path,required=True)
    p.add_argument("--coordinates",type=Path,required=True)
    p.add_argument("--out",type=Path,required=True)
    args=p.parse_args()
    args.out.mkdir(parents=True,exist_ok=True)
    h=json.loads(args.handoff.read_bytes())
    coords=json.loads(args.coordinates.read_bytes())
    require(h["schema"]=="slr.gwb-office-referee-handoff.v1" and h["candidate_only"] is True
            and h["formal_premise_paid"] is False and h["world_truth_paid"] is False,"invalid handoff")
    views={v["edition"]:v for v in h["views"]}
    require(len(h["views"])==3 and set(views)=={"en","simple","es"},"incomplete cohort")
    require(set(coords)=={"year_support","day_support"},"explicit distinct query manifests required")
    require(coords["year_support"]["query_ref"]!=coords["day_support"]["query_ref"],"query collision")
    binary=os.environ.get("SLR_NATIVE_EXECUTABLE","target/debug/sensiblaw")
    report={"schema":"slr.gwb-referee-pair-candidates.v1","family_ref":h["family_ref"],
            "queries":{},"candidate_only":True,"formal_premise_paid":False,"world_truth_paid":False}
    evidence=set()
    for lane,config in sorted(coords.items()):
        query=config["query_ref"]
        require(isinstance(query,str) and query.strip(),"empty query")
        expected={"|".join(x) for x in itertools.combinations(sorted(views),2)}
        require(set(config["pairs"])==expected,"every direct pair required")
        rows=[]
        report["queries"][lane]={"query_ref":query,"pairs":rows}
        for key in sorted(expected):
            left,right=(views[e] for e in key.split("|"))
            c=config["pairs"][key]
            require(c["evidence_ref"] not in evidence and c["evidence_ref"],"duplicate or empty evidence coordinate")
            evidence.add(c["evidence_ref"])
            require(c["statement_digest"].startswith("sha256:") and len(c["statement_digest"])==71,"invalid statement digest")
            answer={"schema":"slr.signed-view-query-answer.v1","query_ref":query,
                    "left_support":left[lane],"right_support":right[lane]}
            stem=lane+"-"+key.replace("|","-")
            answer_path=args.out/(stem+"-answer.json")
            put(answer_path,answer)
            row={"pair":key,"left_selector_ref":left["selector_ref"],"right_selector_ref":right["selector_ref"],
                 "evidence_ref":c["evidence_ref"],"statement_digest":c["statement_digest"],"candidate_only":True}
            rows.append(row)
            try:
                result=subprocess.run([binary,"relational-family","prepare-reciprocal-referee-binding",
                    h["family_ref"],left["selector_ref"],right["selector_ref"],query,
                    c["statement_digest"],c["evidence_ref"],str(answer_path)],
                    capture_output=True,timeout=90,check=True)
                binding=json.loads(result.stdout)
                require(binding["schema"]=="slr.reciprocal-referee-binding-candidate.v1" and
                        binding["candidate_only"] is True and binding["formal_premise_paid"] is False and
                        binding["world_truth_paid"] is False and binding["writes_database"] is False and
                        binding["proposed_answer"]==answer,"unexpected native binding")
                b=binding["proposed_binding"]
                require(b["family_ref"]==h["family_ref"] and b["consumer_ref"]==h["consumer_ref"] and
                        b["query_ref"]==query and b["left_selector_ref"]==left["selector_ref"] and
                        b["right_selector_ref"]==right["selector_ref"] and b["statement_digest"]==c["statement_digest"]
                        and b["evidence_ref"]==c["evidence_ref"],"misbound pair")
                put(args.out/(stem+"-binding.json"),binding)
                row["status"]="candidate_binding_prepared"
            except (OSError,ValueError,KeyError,subprocess.SubprocessError) as exc:
                row.update(status="held",reason=str(exc))
            put(args.out/"manifest.json",report)
    print(json.dumps(report,indent=2))
    return 0 if all(row["status"]=="candidate_binding_prepared" for q in report["queries"].values() for row in q["pairs"]) else 3

if __name__=="__main__":
    raise SystemExit(main())
