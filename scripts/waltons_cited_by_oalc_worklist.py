#!/usr/bin/env python3
"""Print/replay the OALC acquisition worklist for normalized CitedBy candidates.

This script intentionally does not treat the citation-graph provider as the source
of judgment text. Every candidate must be re-acquired through the governed primary-
source path before proposition review.
"""
from __future__ import annotations
import argparse,json,subprocess
from pathlib import Path

def main()->int:
    p=argparse.ArgumentParser()
    p.add_argument("candidates")
    p.add_argument("--execute",action="store_true")
    p.add_argument("--output-dir",default="artifacts/oalc/contracts/waltons/later-authorities")
    a=p.parse_args()
    data=json.loads(Path(a.candidates).read_text())
    if data.get("schema_version")!="sl.cited_by_candidates.v0_1":
        raise SystemExit("unsupported cited-by candidates schema")
    outdir=Path(a.output_dir); outdir.mkdir(parents=True,exist_ok=True)
    work=[]
    for row in data.get("candidates",[]):
        mnc=row["medium_neutral_citation"]
        safe=mnc.replace("[","").replace("]","").replace(" ","-").lower()
        item={"medium_neutral_citation":mnc,"output_dir":str(outdir/safe),"state":"primary_source_acquisition_required"}
        work.append(item)
        if a.execute:
            subprocess.run([
                "cargo","run","-p","sensiblaw-governed-legal-provider",
                "--features","live-network","--example","live_oalc_case_follow","--",
                "--operator-opt-in","--citation",mnc,"--output-dir",str(outdir/safe)
            ],check=True)
            item["state"]="primary_source_acquired"
    path=outdir/"oalc-acquisition-worklist.json"
    path.write_text(json.dumps({
      "schema_version":"sl.oalc_case_acquisition_worklist.v0_1",
      "root_medium_neutral_citation":data["root_medium_neutral_citation"],
      "candidate_only":True,
      "work":work,
    },indent=2)+"\n")
    print(f"oalc_acquisition_worklist={path} count={len(work)}")
    return 0
if __name__=="__main__":
    raise SystemExit(main())
