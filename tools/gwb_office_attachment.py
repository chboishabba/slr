"""Transport of bounded Lean evaluation to the exact native predicate.
This is a candidate handoff to an independent referee, not an issuance path.
"""
import hashlib
from pathlib import Path

def require(ok, reason):
    if not ok:
        raise ValueError(reason)

def attach(v, row, evaluation, inverse, kernel_receipt, evaluation_path):
    require(evaluation["schema"]=="slr.gwb-controlled-office-evaluation.v1" and
            evaluation["bounded_rule_only"] is True and evaluation["world_truth_paid"] is False and
            evaluation["native_attachment_paid"] is False and evaluation["referee_authority_paid"] is False,
            "unexpected controlled evaluation scope")
    hits=[x for x in evaluation["views"] if x["edition"]==v["edition"]]
    require(len(hits)==1,"missing/ambiguous controlled view")
    e=hits[0]
    require(all(e[k]==v[k] for k in ("source_sha256","predicate_span","predicate")),"controlled occurrence mismatch")
    require(e["year_support"]=={"positive":True,"negative":False} and
            e["day_support"]==({"positive":True,"negative":False} if v["edition"]=="es" else None),
            "unexpected bounded answer")
    require(kernel_receipt["edition"]==v["edition"] and kernel_receipt["source_sha256"]==v["source_sha256"]
            and kernel_receipt["source_observation_checked"] is True and kernel_receipt["world_truth_paid"] is False,
            "kernel source mismatch")
    require(inverse["status"]=="unique_checked_candidate" and inverse["truncated"] is False and
            inverse["source_revision_ref"]==v["source_revision_ref"] and inverse["parser_run_constraint"]==v["parser_run_ref"]
            and inverse["source_sha256"]==v["source_sha256"] and inverse["literal"]==v["predicate"]
            and [inverse["source_start_char"],inverse["source_end_char"]]==v["predicate_span"] and
            inverse["world_truth_paid"] is False and inverse["writes_database"] is False and
            len(inverse["candidates"])==1,"checked inverse mismatch")
    c=inverse["candidates"][0]
    o=c["observation"]
    require(c["parser_run_ref"]==v["parser_run_ref"] and c["batch_ref"] in row["batches"] and
            c["predicate_candidate_ref"]==o["predicate_candidate_ref"] and o["source_revision_ref"]==v["source_revision_ref"]
            and o["parser_or_producer_ref"].startswith("db-parser:"+v["parser_run_ref"]+":")
            and "sha256:"+v["source_sha256"] in o["provenance_refs"]
            and "syntax:predicate-span:"+":".join(map(str,v["predicate_span"])) in o["native_metadata_refs"]
            and o["candidate_only"] is True and o["claim_truth_promoted"] is False
            and o["creates_semantic_authority"] is False,"native observation mismatch")
    return {"schema":"slr.gwb-office-attachment-candidate.v1","edition":v["edition"],
            "source_revision_ref":v["source_revision_ref"],"source_sha256":v["source_sha256"],
            "parser_run_ref":v["parser_run_ref"],"predicate_span":v["predicate_span"],"predicate":v["predicate"],
            "indexed_member_ref":row["indexed_member_ref"],"batch_ref":c["batch_ref"],
            "observation":o,"year_support":e["year_support"],"day_support":e["day_support"],
            "evaluation_sha256":hashlib.sha256(Path(evaluation_path).read_bytes()).hexdigest(),
            "kernel_receipt_sha256":hashlib.sha256(Path(row["receipt_path"]).read_bytes()).hexdigest(),
            "candidate_only":True,"formal_premise_paid":False,"world_truth_paid":False}

def bind_family(family, attachments):
    require(family.get("superseded_by") is None,"superseded family")
    sources=family["projection"]["sources"]
    require(len(sources)==len(attachments)==3,"incomplete cohort")
    result=[]
    for a in attachments:
        matches=[s for s in sources if s["observation"]==a["observation"]]
        require(len(matches)==1,"native family observation differs from attached observation")
        result.append(dict(a,selector_ref=matches[0]["selector_ref"]))
    require(len({x["selector_ref"] for x in result})==3,"duplicate family selector")
    return {"schema":"slr.gwb-office-referee-handoff.v1","family_ref":family["family_ref"],
            "consumer_ref":family["projection"]["consumer_ref"],"views":result,
            "candidate_only":True,"formal_premise_paid":False,"world_truth_paid":False}
