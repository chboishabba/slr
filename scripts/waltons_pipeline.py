#!/usr/bin/env python3
"""Operator workflow for the Waltons/estoppel LegalFollow pipeline.

Human/legal review is intentionally not automated. Commands stop at editable
worksheets and only continue after an operator finalizes explicit decisions.
"""
from __future__ import annotations
import argparse,glob,json,os,subprocess,sys
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
DEFAULT=Path(os.environ.get("SENSIBLAW_OALC_OUTPUT","artifacts/oalc/contracts/waltons"))
ROOT_MNC="[1988] HCA 7"
ROOT_AUTHORITY_REF="case:au:hca:1988:7"

def run(*args:str)->None:
    print("+"," ".join(args))
    subprocess.run(args,cwd=ROOT,check=True)

def paths(base:Path)->dict[str,Path]:
    return {
      "receipt":base/"waltons-oalc-source-receipt.json",
      "text":base/"waltons-stores-v-maher.txt",
      "queue":base/"waltons-oalc-candidate-review-queue.json",
      "worksheet":base/"waltons-review-worksheet.json",
      "decisions":base/"waltons-reviewed-decisions.json",
      "reviewed":base/"waltons-reviewed-proposition-receipts.json",
      "payments":base/"waltons-reviewed-evidence-payments.slrw",
      "frontier":base/"waltons-wrongtype-frontier.json",
      "citedby_manifest":base/"waltons-cited-by-work-manifest.json",
      "citedby_candidates":base/"waltons-cited-by-candidates.json",
      "later_dir":base/"later-authorities",
      "merged_treatment_queue":base/"waltons-treatment-review-queue.json",
      "treatment_worksheet":base/"waltons-treatment-review-worksheet.json",
      "treatment_decisions":base/"waltons-treatment-reviewed-decisions.json",
      "genealogy":base/"waltons-temporal-treatment-genealogy.json",
    }

def status(base:Path)->int:
    p=paths(base)
    order=[
      ("1 source receipt",p["receipt"]),
      ("1 source text",p["text"]),
      ("1 candidate queue",p["queue"]),
      ("2 review worksheet",p["worksheet"]),
      ("2 finalized decisions",p["decisions"]),
      ("3 reviewed receipts",p["reviewed"]),
      ("3 payment wire",p["payments"]),
      ("4/5 WrongType frontier",p["frontier"]),
      ("6 cited-by manifest",p["citedby_manifest"]),
      ("6 cited-by candidates",p["citedby_candidates"]),
      ("7 merged treatment queue",p["merged_treatment_queue"]),
      ("7 treatment worksheet",p["treatment_worksheet"]),
      ("7 treatment decisions",p["treatment_decisions"]),
      ("8 genealogy",p["genealogy"]),
    ]
    for label,path in order:
        print(f"{'OK' if path.exists() else '--'} {label:30} {path}")
    return 0

def main()->int:
    ap=argparse.ArgumentParser()
    ap.add_argument("--base",type=Path,default=DEFAULT)
    sp=ap.add_subparsers(dest="cmd",required=True)
    for name in ["status","acquire","queue","review-template","review-finalize","review-compile","frontier","citedby-manifest","treatment-queues","merge-treatment","treatment-template","treatment-finalize","genealogy"]:
        sp.add_parser(name)
    imp=sp.add_parser("citedby-import"); imp.add_argument("provider_results")
    reacq=sp.add_parser("citedby-oalc"); reacq.add_argument("--execute",action="store_true")
    args=ap.parse_args(); base=args.base; base.mkdir(parents=True,exist_ok=True); p=paths(base)

    if args.cmd=="status": return status(base)
    if args.cmd=="acquire":
        run("cargo","run","-p","sensiblaw-governed-legal-provider","--features","live-network","--example","live_oalc_case_follow","--",
            "--operator-opt-in","--citation",ROOT_MNC,"--court-ref","court:HCA","--oalc-jurisdiction","commonwealth","--output-dir",str(base))
        generic=base/"oalc-source-receipt.json"; generic_text=base/"judgment.txt"
        if generic.exists(): generic.replace(p["receipt"])
        if generic_text.exists(): generic_text.replace(p["text"])
        receipt=json.loads(p["receipt"].read_text()); receipt["local_artifact_ref"]=str(p["text"])
        p["receipt"].write_text(json.dumps(receipt,indent=2)+"\n")
        return 0
    if args.cmd=="queue":
        run("cargo","run","-p","sensiblaw-proof-search-loop","--example","waltons_oalc_materialization","--",str(p["receipt"]),str(p["queue"]))
    elif args.cmd=="review-template":
        run(sys.executable,"scripts/waltons_review_decision_template.py",str(p["queue"]),str(p["worksheet"]))
    elif args.cmd=="review-finalize":
        run(sys.executable,"scripts/waltons_review_decision_finalize.py",str(p["worksheet"]),str(p["decisions"]))
    elif args.cmd=="review-compile":
        run("cargo","run","-p","sensiblaw-proof-search-loop","--example","waltons_reviewed_proposition_compile","--",
            str(p["receipt"]),str(p["decisions"]),str(p["reviewed"]),str(p["payments"]))
    elif args.cmd=="frontier":
        run("cargo","run","-p","sensiblaw-legal-runtime","--example","waltons_wrongtype_frontier","--",
            str(p["receipt"]),str(p["decisions"]),str(p["frontier"]))
    elif args.cmd=="citedby-manifest":
        run("cargo","run","-p","sensiblaw-proof-search-loop","--example","waltons_cited_by_work_manifest","--",
            str(p["receipt"]),str(p["citedby_manifest"]))
    elif args.cmd=="citedby-import":
        run(sys.executable,"scripts/waltons_cited_by_import.py",str(p["citedby_manifest"]),args.provider_results,str(p["citedby_candidates"]))
    elif args.cmd=="citedby-oalc":
        cmd=[sys.executable,"scripts/waltons_cited_by_oalc_worklist.py",str(p["citedby_candidates"]),"--output-dir",str(p["later_dir"])]
        if args.execute: cmd.append("--execute")
        run(*cmd)
    elif args.cmd=="treatment-queues":
        receipts=sorted(p["later_dir"].glob("*/oalc-source-receipt.json"))
        if not receipts: raise SystemExit("no later-authority OALC receipts found")
        for receipt in receipts:
            out=receipt.parent/"waltons-treatment-review-queue.json"
            run("cargo","run","-p","sensiblaw-proof-search-loop","--example","oalc_authority_treatment_review_queue","--",str(receipt),ROOT_MNC,str(out))
    elif args.cmd=="merge-treatment":
        queues=sorted(p["later_dir"].glob("*/waltons-treatment-review-queue.json"))
        if not queues: raise SystemExit("no per-authority treatment queues found")
        run(sys.executable,"scripts/merge_treatment_review_queues.py",*[str(q) for q in queues],"--output",str(p["merged_treatment_queue"]))
    elif args.cmd=="treatment-template":
        run(sys.executable,"scripts/citation_treatment_review_template.py",str(p["merged_treatment_queue"]),ROOT_AUTHORITY_REF,str(p["treatment_worksheet"]))
    elif args.cmd=="treatment-finalize":
        run(sys.executable,"scripts/citation_treatment_review_finalize.py",str(p["treatment_worksheet"]),str(p["treatment_decisions"]))
    elif args.cmd=="genealogy":
        run("cargo","run","-p","sensiblaw-proof-search-loop","--example","citation_treatment_review_compile","--",
            str(p["merged_treatment_queue"]),str(p["treatment_decisions"]),ROOT_AUTHORITY_REF,"2026-09-20",str(p["genealogy"]))
    return 0

if __name__=="__main__":
    raise SystemExit(main())
