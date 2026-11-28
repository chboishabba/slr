#!/usr/bin/env python3
"""Checkpointed real-source multiview acceptance. No new semantic engine.

Default: export exactly one persisted parser run, reproject and kernel-check
all editions, reopen the candidate predicate and numeric plan. --commit
additionally publishes bounded numeric prefix packets and runs the existing
native family and context capsule owner. Missing proofs remain a HOLD (3).
"""
from __future__ import annotations
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys

from bind_gwb_native_receipt import bind

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / "artifacts/gwb/multiview-native"
ACQ = BASE / "acquired"
LIMIT = int(os.environ.get("GWB_TIMEOUT_SECONDS", "90"))
BIN = Path(os.environ.get("SLR_NATIVE_EXECUTABLE", str(ROOT / "target/debug/sensiblaw"))).resolve()

def load(p):
    return json.loads(p.read_bytes())

def put(p, data):
    p.parent.mkdir(parents=True, exist_ok=True)
    tmp = p.with_suffix(p.suffix + ".tmp")
    tmp.write_text(json.dumps(data, ensure_ascii=False, sort_keys=True, indent=2) + "\n", encoding="utf-8")
    tmp.replace(p)

def require(ok, reason):
    if not ok:
        raise ValueError(reason)

def command(args, log, *, cwd=ROOT, timeout=LIMIT, env=None, allowed=(0,)):
    log.parent.mkdir(parents=True, exist_ok=True)
    try:
        p = subprocess.run([str(a) for a in args], cwd=cwd, env=env, capture_output=True, timeout=timeout)
        log.write_bytes(p.stdout)
        log.with_suffix(log.suffix + ".stderr").write_bytes(p.stderr)
    except subprocess.TimeoutExpired as exc:
        log.write_bytes(exc.stdout or b"")
        log.with_suffix(log.suffix + ".stderr").write_bytes(exc.stderr or b"")
        raise RuntimeError(f"timeout at {log.name}") from exc
    if p.returncode not in allowed:
        raise RuntimeError(f"exit {p.returncode}: {log.name}")
    return p

def native(args, path):
    p = command([BIN, *args], path)
    result = json.loads(p.stdout)
    require(isinstance(result, dict), "native owner did not return object")
    return result

def projection_module(lean):
    spec = importlib.util.spec_from_file_location("existing_gwb_projection", lean / "tools/gwb_article_proof.py")
    require(spec is not None and spec.loader is not None, "missing existing JMD projection")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module

def prepare_view(v, manifest, module, lean, out):
    edition = v["edition"]
    s = next(s for s in manifest["sources"] if s["edition"] == edition)
    a = out / edition
    a.mkdir(parents=True, exist_ok=True)
    raw_path = ACQ / f"{edition}-canonical.txt"
    raw = raw_path.read_bytes()
    text = raw.decode("utf-8")
    require(hashlib.sha256(raw).hexdigest() == v["source_sha256"], "canonical SHA mismatch")
    source = a / "slr-record.json"
    projection = a / "article-candidates.json"
    receipt = a / "indexed-world-fibre.json"
    receipt.unlink(missing_ok=True)
    source.unlink(missing_ok=True)
    title, revid = s["source_ref"].split(":")[3], s["source_ref"].rsplit(":", 1)[1]
    command([sys.executable, ROOT / "tools/export_native_jmd_inputs.py",
             "--revision", v["source_revision_ref"], "--parser-run", v["parser_run_ref"],
             "--source", raw_path, "--output", source, "--language", edition, "--title", title,
             "--revid", revid], a / "export.log")
    record = load(source)
    require(record["parser_receipt"]["parser_run_ref"] == v["parser_run_ref"], "wrong parser export")
    old = load(ACQ / f"{edition}-article-candidates.json")
    require(old["source_text_sha256"] == v["source_sha256"], "section donor mismatch")
    projected = module.derive_projection(record, text, old["sections"])
    require(projected.get("projection_policy") == "native-multilingual-syntax-v2", "wrong projection policy")
    put(projection, projected)
    p = command(["lake", "env", "lean", "--run", "tools/IndexedWorldFibreRunner.lean",
                 "Q207", title.replace("_", " "), raw_path, source, projection,
                 ACQ / "Q207-EntityData.json", receipt, ACQ / "properties.txt"],
                a / "lean.log", cwd=lean)
    require(b"INDEXED_WORLD_FIBRE_KERNEL_OK" in p.stdout, "missing kernel acceptance marker")
    bound = bind(edition, receipt, source_record=source, projection_record=projection)
    put(a / "bound-receipt.json", bound)
    return (text, source, projection, receipt, bound)

def selected(v, view, out, commit, max_steps):
    text, source, projection, receipt, bound = view
    a,b = v["predicate_span"]
    literal = v["predicate"]
    require(0 <= a < b <= len(text) and text[a:b] == literal, "wrong original predicate span")
    if "clause_span" in v:
        c,d = v["clause_span"]
        require(0 <= c <= a < b <= d <= len(text) and text[c:d] == v["clause"], "wrong original clause inverse")
    matches = [e for e in bound["inventory"]["article_lanes"]["event_candidates"]
               if (e["predicate"]["start_char"], e["predicate"]["end_char"], e["predicate"]["surface"]) == (a,b,literal)]
    require(len(matches) == 1, f"indexed predicate multiplicity={len(matches)}")
    rev, parser, sha = v["source_revision_ref"], v["parser_run_ref"], v["source_sha256"]
    inverse = native(["relational-family","discover-checked-predicate",rev,sha,str(a),str(b),literal,parser],
                     out / "checked-inverse.json")
    require(inverse.get("status") == "unique_checked_candidate" and inverse.get("truncated") is False
            and inverse.get("source_revision_ref") == rev and inverse.get("parser_run_constraint") == parser
            and inverse.get("source_sha256") == sha and inverse.get("source_start_char") == a
            and inverse.get("source_end_char") == b and inverse.get("literal") == literal
            and inverse.get("world_truth_paid") is False and inverse.get("writes_database") is False
            and len(inverse.get("candidates", [])) == 1, "checked inverse unavailable or misbound")
    cand = inverse["candidates"][0]
    require(cand["parser_run_ref"] == parser, "foreign candidate parser")
    plan = native(["numeric-pnf-native","plan",rev,parser], out / "numeric-plan.json")
    require(plan["source_revision_ref"] == rev and plan["original_parser_run_ref"] == parser
            and plan["canonical_source_sha256"] == sha and plan["document_ref"] == f"document:sha256:{sha}",
            "numeric plan binding mismatch")
    row = {"edition": v["edition"], "receipt_path": str(receipt), "source_record_path": str(source),
           "projection_path": str(projection), "indexed_member_ref": matches[0]["event_candidate_ref"],
           "revision": rev, "parser_run_ref": parser, "run": plan["numeric_run_ref"],
           "document": plan["document_ref"], "batches": [cand["batch_ref"]],
           "snapshot": "native-numeric-chart:missing", "packet_digest": "native-numeric-chart:missing"}
    if not commit:
        return row
    for step in range(max_steps):
        part = native(["numeric-pnf-native","compile-prefix",rev,parser,str(a),"64","--commit"],
                      out / f"numeric-step-{step:03d}.json")
        require(part["numeric_run_ref"] == row["run"] and part["original_source_sha256"] == sha
                and part["original_source_revision_ref"] == rev and part["original_parser_run_ref"] == parser
                and part["historical_truth_paid"] is False, "foreign numeric publication")
        if (part["sentence_jobs_remaining"] == 0 and part["exact_parser_sentence_receipts"]
                and not any(x.startswith("original_numeric_parser_sentence_bootstrap_pending:") for x in part["residuals"])):
            break
    else:
        raise ValueError("bounded prefix compile still held; persisted steps can be reopened")
    packet = native(["numeric-pnf-native","publish-relation-prefix",rev,parser,str(a),str(b),literal,"--commit"],
                    out / "packet-publication.json")
    require(packet["source_revision_ref"] == rev and packet["parser_run_ref"] == parser
            and packet["source_sha256"] == sha and packet["numeric_run_ref"] == row["run"]
            and packet["document_ref"] == row["document"] and packet["selected_span"] == [a,b]
            and packet["selected_literal"] == literal and packet["world_truth_paid"] is False,
            "packet publication binding mismatch")
    row.update(snapshot=packet["snapshot_ref"], packet_digest=packet["packet_sha256"])
    reopened = native(["numeric-pnf-native","relation-reopen",row["run"],row["document"],
                       row["snapshot"],row["packet_digest"]], out / "packet-reopened.json")
    require(reopened["exact_persisted_packet_readback"] and reopened["writes_database"] is False
            and reopened["packet_sha256"] == row["packet_digest"]
            and reopened["packet"]["comparison"]["source_revision_ref"] == rev
            and reopened["packet"]["comparison"]["parser_run_ref"] == parser, "packet reopen mismatch")
    return row

def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--lean-root", required=True, type=Path)
    ap.add_argument("--out", type=Path, default=BASE / "office-multiview-local")
    ap.add_argument("--commit", action="store_true")
    ap.add_argument("--max-steps", type=int, default=32)
    args=ap.parse_args()
    require(1 <= args.max_steps <= 128, "invalid step limit")
    lean, out = args.lean_root.resolve(), args.out.resolve()
    require(BIN.is_file(), "build native sensiblaw CLI first")
    if args.commit:
        for key in ("GWB_VIEW_CONSUMER_REF","GWB_VIEW_QUERY_REF","GWB_VIEW_ACCESS_REF"):
            require(os.environ.get(key), f"explicit {key} required")
    manifest=load(BASE / "native-acquisition-manifest-v1.json")
    cohort=load(BASE / "gwb-office-query-candidate-cohort-v1.json")
    negative=load(BASE / "gwb-native-view-inputs-v1.json")
    module=projection_module(lean)
    summary={"schema":"slr.gwb-multiview-local-campaign.v1","commit_requested":args.commit,
             "views":[],"cohorts":{},"semantic_alignment_paid":False,"world_truth_paid":False}
    # Always emit the per-view failures, even if another edition is held.
    rows={"positive":[], "negative":[]}
    for v in cohort["views"]:
        edition=v["edition"]
        result={"edition":edition,"stages":{}}
        summary["views"].append(result)
        try:
            view=prepare_view(v,manifest,module,lean,out/"views")
            result["stages"]["projection"]="kernel_accepted"
        except (ValueError, RuntimeError, OSError, KeyError) as exc:
            result["stages"]["projection"]={"held":str(exc)}
            put(out/"campaign.json",summary)
            continue
        old=next(x for x in negative if x["edition"]==edition)
        neg=dict(v,predicate=old["selected_literal"],predicate_span=old["selected_span"])
        neg.pop("clause",None); neg.pop("clause_span",None)
        for name,selection in (("positive",v),("negative",neg)):
            try:
                row=selected(selection,view,out/name/edition,args.commit,args.max_steps)
                rows[name].append(row)
                result["stages"][name]="packet_reopened" if args.commit else "checked_inverse_and_plan"
            except (ValueError, RuntimeError, OSError, KeyError) as exc:
                result["stages"][name]={"held":str(exc)}
        put(out/"campaign.json",summary)
    for name,members in rows.items():
        target=out/name/"view-inputs.json"
        put(target,members)
        if len(members)!=3:
            summary["cohorts"][name]="held_incomplete_views"
        elif not args.commit:
            summary["cohorts"][name]="read_only_preflight_ready"
        else:
            env=dict(os.environ,GWB_NATIVE_VIEW_INPUTS_JSON=str(target),
                     GWB_MULTIVIEW_OUT=str(out/name/"family"))
            # Negative control must never consume the positive proof evidence.
            if name=="negative":
                env.pop("GWB_VIEW_PAIR_EVIDENCE_JSON",None)
            try:
                p=command(["bash",ROOT/"scripts/run_gwb_multiview_native.sh","--commit"],
                          out/name/"family.log",env=env,timeout=None,allowed=(0,3))
                if name=="negative" and p.returncode==0:
                    raise ValueError("negative control unexpectedly reported authenticated parity")
                summary["cohorts"][name]="bounded_parity_reported" if p.returncode==0 else "held_or_partial"
            except (ValueError,RuntimeError,OSError) as exc:
                summary["cohorts"][name]={"held":str(exc)}
        put(out/"campaign.json",summary)
    print(json.dumps(summary,indent=2))
    # This is an implementation/acceptance campaign, not a historical proof.
    return 0 if not args.commit and all(x=="read_only_preflight_ready" for x in summary["cohorts"].values()) else 3

if __name__=="__main__":
    raise SystemExit(main())
