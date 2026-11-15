#!/usr/bin/env python3
"""Read-only SLR native PostgreSQL parser -> Leech original syntax observations.

The production source is the Rust sensiblaw-pg-source-store's EXISTING
corpus.canonical_content + ingest.generic_source_revision and
ingest.parser_run/job/token tables. No parallel token store, migrations,
spaCy replay, or semantic promotion. Python is export I/O only.

Manifest: [{source_revision_ref,parser_run_ref,split,source_sha256,
            document_ref(optional)}] with ONLY train/development splits.
This format deliberately needs NO external text files: source bytes are
reopened from the canonical PostgreSQL content store.

Launch from the SLR repository:
  DATABASE_URL=... python3 -m python.slr_pg_leech_grammar_export \\
    --manifest PATH --output PATH
"""
from __future__ import annotations
import argparse,hashlib,json,os
from pathlib import Path

SCHEMA="slr.numeric_spacy.leech_grammar.v1"
DOMAIN="slr-native-pg-grammar-observation-v1"
SOURCE_SQL="""
SELECT r.parser_run_ref, r.source_revision_ref, r.parser_family,
       r.parser_version, r.model_ref, r.config_digest_ref,
       r.status, r.candidate_only, r.creates_semantic_authority,
       r.applicability_promoted, r.claim_truth_promoted,
       g.document_ref, g.content_digest_ref, g.candidate_only AS source_candidate_only,
       g.creates_semantic_authority AS source_authority,
       g.applicability_promoted AS source_applicability,
       g.claim_truth_promoted AS source_truth,
       c.payload
FROM ingest.parser_run r
JOIN ingest.generic_source_revision g
  ON g.source_revision_ref = r.source_revision_ref
JOIN corpus.document d ON d.document_ref = g.document_ref
JOIN corpus.canonical_content c ON c.canonical_ref = d.canonical_ref
WHERE r.parser_run_ref = %s AND r.source_revision_ref = %s
"""
JOB_SQL="""
SELECT compilation_key, source_revision_ref, region_ref,
       region_start_char, region_end_char, status,
       candidate_only, creates_semantic_authority,
       applicability_promoted, claim_truth_promoted
FROM ingest.parser_job
WHERE parser_run_ref = %s ORDER BY region_start_char, compilation_key
"""
TOKEN_SQL="""
SELECT t.compilation_key, t.token_ordinal, t.start_char, t.end_char,
       t.surface, t.lemma, t.pos, t.morph_json, t.head_ordinal,
       t.dependency_ref
FROM ingest.parser_job j
JOIN ingest.parser_token t ON t.compilation_key = j.compilation_key
WHERE j.parser_run_ref = %s
ORDER BY t.start_char, t.end_char, t.compilation_key, t.token_ordinal
"""


def _sha(b):
    return hashlib.sha256(b).hexdigest()


def _canon(obj):
    return json.dumps(obj,sort_keys=True,ensure_ascii=False,
                      separators=(",",":"),allow_nan=False)


def _digest(domain,text):
    return _sha((domain+"\0"+text).encode("utf-8"))


def _yes_candidate(row, authority, prefix):
    if not row.get("candidate_only") or any(row.get(k) for k in authority):
        raise ValueError(prefix+" is not candidate-only/nonpromoting")


def validate_manifest(rows):
    if not isinstance(rows,list) or not rows:
        raise ValueError("nonempty manifest array required")
    seen=set()
    seen_revisions=set()
    split_for_sha={}
    out=[]
    for row in rows:
        if not isinstance(row,dict) or row.get("split") not in ("train","development"):
            raise ValueError("reserved/holdout/F5 source forbidden before PG access")
        if any(not isinstance(row.get(k),str) or not row[k] for k in (
                "source_revision_ref","parser_run_ref")):
            raise ValueError("original SLR native source/parser run ref required")
        sha=row.get("source_sha256")
        if not isinstance(sha,str) or len(sha)!=64 or any(
                c not in "0123456789abcdef" for c in sha):
            raise ValueError("pinned canonical source SHA256 required")
        if row.get("document_ref") is not None and (
            not isinstance(row["document_ref"],str) or not row["document_ref"]):
            raise ValueError("invalid canonical document reference")
        key=(row["source_revision_ref"],row["parser_run_ref"])
        if key in seen or row["source_revision_ref"] in seen_revisions:
            raise ValueError("duplicate original SLR source revision/run")
        seen.add(key)
        seen_revisions.add(row["source_revision_ref"])
        if (sha in split_for_sha and split_for_sha[sha]!=row["split"]):
            raise ValueError("same canonical source across train/development manifest")
        split_for_sha[sha]=row["split"]
        out.append(row)
    if {r["split"] for r in out}!={"train","development"}:
        raise ValueError("source-disjoint train/development both required")
    return out


def project_native_pg_document(spec,source,jobs,tokens):
    """Pure projection from ACTUAL Rust-PG row names and source receipt."""
    if source is None:
        raise ValueError("SLR PG source/run row missing")
    for key in ("parser_run_ref","source_revision_ref"):
        if source[key]!=spec[key]:
            raise ValueError("parser run mismatch: source revision identity differs")
    if source["status"]!="complete":
        raise ValueError("native SLR parser run not complete")
    if not source["candidate_only"] or any(source[k] for k in (
            "creates_semantic_authority","applicability_promoted",
            "claim_truth_promoted","source_authority",
            "source_applicability","source_truth")) or not source["source_candidate_only"]:
        raise ValueError("native SLR source/parser cannot promote truth (nonpromoting required)")
    if spec.get("document_ref") and spec["document_ref"]!=source["document_ref"]:
        raise ValueError("canonical document identity mismatch")
    raw=source["payload"]
    if not isinstance(raw,(bytes,bytearray,memoryview)):
        raise ValueError("canonical corpus payload must be bytes")
    raw=bytes(raw)
    if _sha(raw)!=spec["source_sha256"] or (
        source["content_digest_ref"]!="sha256:"+_sha(raw)):
        raise ValueError("immutable SLR source bytes/digest differ")
    try:
        content=raw.decode("utf-8")
    except UnicodeDecodeError as exc:
        raise ValueError("original SLR source is not UTF8") from exc
    if not content or not jobs or not tokens:
        raise ValueError("no completed SLR parser tokens/regions")
    by_job={}
    for j in jobs:
        if (j["source_revision_ref"]!=spec["source_revision_ref"] or
            j["status"]!="succeeded" or not j["candidate_only"] or
            any(j[k] for k in ("creates_semantic_authority",
                              "applicability_promoted","claim_truth_promoted"))):
            raise ValueError("parser residual/queued/job authority prevents export")
        lo,hi=j["region_start_char"],j["region_end_char"]
        if not all(type(x) is int for x in (lo,hi)) or not 0<=lo<hi<=len(content):
            raise ValueError("source parser region coordinates invalid")
        if j["compilation_key"] in by_job:
            raise ValueError("duplicate compilation key")
        by_job[j["compilation_key"]]=j
    # IDs are export-local positive occurrence IDs. The ACTUAL SLR primary
    # key is (compilation_key,token_ordinal), retained explicitly below.
    seen={}
    all_rows=[]
    last_end=0
    for n,t in enumerate(tokens,1):
        key=(t["compilation_key"],t["token_ordinal"])
        if (t["compilation_key"] not in by_job or key in seen or
            type(t["token_ordinal"]) is not int or t["token_ordinal"]<0):
            raise ValueError("unknown or duplicate original PG token occurrence")
        j=by_job[t["compilation_key"]]
        lo,hi=t["start_char"],t["end_char"]
        if not all(type(x) is int for x in (lo,hi)) or not (
            0<=last_end<=lo<hi<=len(content) and
            j["region_start_char"]<=lo<hi<=j["region_end_char"]):
            raise ValueError("original PG tokens must be ordered, nonoverlapping source spans")
        if content[lo:hi]!=t["surface"]:
            raise ValueError("native parser token surface differs from canonical source")
        if not isinstance(t["pos"],str) or not t["pos"] or not isinstance(
            t["dependency_ref"],str) or not t["dependency_ref"]:
            raise ValueError("native POS/dependency label missing")
        morph=t["morph_json"]
        morph_label=None
        if morph is not None:
            try:morph_label=_canon(json.loads(morph))
            except (ValueError,TypeError) as e:
                raise ValueError("invalid source native morph JSON") from e
        labels=dict(pos=t["pos"],dependency=t["dependency_ref"],morph=morph_label)
        seen[key]=n
        all_rows.append((n,t,key,labels))
        last_end=hi
    for n,t,key,labels in all_rows:
        ordinal=t["head_ordinal"]
        if ordinal is not None and (type(ordinal) is not int or
                                    (key[0],ordinal) not in seen):
            raise ValueError("dependency head not found in same SLR parser region")
    # Parser contract is model/family/config identity shared across
    # distinct source revisions. Run and revision belong to RECEIPT, not
    # contract; otherwise identical parsers on train/dev look unrelated.
    contract=dict(parser_family=source["parser_family"],
        parser_version=source["parser_version"],
        model_ref=source["model_ref"],
        config_digest_ref=source["config_digest_ref"])
    contract_sha=_sha(_canon(contract).encode("utf-8"))
    digest_lists={}
    for kind in ("pos","dependency","morph"):
        digest_lists[kind]=sorted({_digest(
            "slr-pg-"+kind+":v1:"+contract_sha,labels[kind])
            for _,_,_,labels in all_rows if labels[kind] is not None})
    # Small stable (within-document) positive label IDs; cross-run identity
    # is the full SHA256 digest, NOT these compact ephemeral ranks.
    numbers={kind:{d:i+1 for i,d in enumerate(digest_lists[kind])}
             for kind in digest_lists}
    out=[]
    for n,t,key,labels in all_rows:
        digests={kind:(_digest("slr-pg-"+kind+":v1:"+contract_sha,labels[kind])
                       if labels[kind] is not None else None)
                 for kind in labels}
        out.append(dict(token_id=n,source_occurrence_ref=f"{key[0]}:{key[1]}",
            source_compilation_key=key[0],source_token_ordinal=key[1],
            start_char=t["start_char"],end_char=t["end_char"],
            surface=t["surface"],lemma=t["lemma"],
            head_token_id=(seen[(key[0],t["head_ordinal"])]
                           if t["head_ordinal"] is not None else None),
            pos_symbol_id=numbers["pos"][digests["pos"]],
            pos_symbol_digest=digests["pos"],
            dependency_id=numbers["dependency"][digests["dependency"]],
            dependency_digest=digests["dependency"],
            morph_set_id=(numbers["morph"][digests["morph"]]
                          if digests["morph"] is not None else None),
            morph_set_digest=digests["morph"]))

    receipt_sha=_sha(_canon(dict(domain=DOMAIN,source_sha256=_sha(raw),
        parser_contract_sha256=contract_sha,
        parser_run_ref=source["parser_run_ref"],
        source_revision_ref=source["source_revision_ref"],
        jobs=[(j["compilation_key"],j["region_ref"]) for j in jobs],
        token_observations=out)).encode("utf-8"))
    return dict(schema_version=SCHEMA,
        document_id=source["document_ref"],split=spec["split"],
        source_revision_ref=source["source_revision_ref"],
        slr_parser_run_ref=source["parser_run_ref"],
        source_text=content,source_sha256=_sha(raw),
        parser_contract_sha256=contract_sha,
        parser_receipt_sha256=receipt_sha,
        visibility="full_context",slr_tokens=out,
        world_resolution_deferred=True,
        parser_observation_is_semantic_authority=False,
        source_coordinate_unit="unicode_codepoints",
        source_persisted_by="sensiblaw-pg-source-store",
        source_token_occurrence_primary_key="(compilation_key,token_ordinal)",
        label_identity="stable_SHA256_full; numeric_local_per_document",
        non_promoting_native_parser_projection=True)


def _read(conn,query,params):
    with conn.cursor() as cursor:
        cursor.execute(query,params)
        names=[c[0] for c in cursor.description]
        return [dict(zip(names,row)) for row in cursor.fetchall()]


def export_native_pg_manifest(manifest,output,connect):
    # Complete F5/split gate BEFORE ANY database/network request.
    specs=validate_manifest(json.loads(Path(manifest).read_text(encoding="utf-8")))
    dest=Path(output)
    if dest.exists() or dest.with_suffix(dest.suffix+".tmp").exists():
        raise FileExistsError("native SLR export output already exists")
    conn=connect()
    try:
        conn.set_session(isolation_level="REPEATABLE READ",readonly=True,
                         autocommit=False)
        records=[];content_splits={}
        active_parser_contract=None
        for spec in specs:
            selector=(spec["parser_run_ref"],spec["source_revision_ref"])
            source=_read(conn,SOURCE_SQL,selector)
            if len(source)!=1:
                raise ValueError("ambiguous native SLR source revision/PG parser")
            jobs=_read(conn,JOB_SQL,(spec["parser_run_ref"],))
            tokens=_read(conn,TOKEN_SQL,(spec["parser_run_ref"],))
            doc=project_native_pg_document(spec,source[0],jobs,tokens)
            if active_parser_contract is None:
                active_parser_contract=doc["parser_contract_sha256"]
            elif active_parser_contract!=doc["parser_contract_sha256"]:
                raise ValueError("native SLR train/development parser model/config contracts differ")
            sha=doc["source_sha256"]
            if sha in content_splits and content_splits[sha]!=spec["split"]:
                raise ValueError("same canonical SLR source across train/development")
            content_splits[sha]=spec["split"]
            records.append(doc)
        conn.commit()
    except Exception:
        conn.rollback()
        raise
    finally:
        conn.close()
    dest.parent.mkdir(parents=True,exist_ok=True)
    tmp=dest.with_suffix(dest.suffix+".tmp")
    try:
        with tmp.open("x",encoding="utf-8") as handle:
            for doc in records:
                handle.write(_canon(doc)+"\n")
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(tmp,dest)
    finally:
        if tmp.exists():tmp.unlink()
    return dict(kind="slr_native_pg_leech_source_export",
        source_count=len(records),
        train_count=sum(r["split"]=="train" for r in records),
        development_count=sum(r["split"]=="development" for r in records),
        output_sha256=_sha(dest.read_bytes()),F5_source_read=False,
        candidate_parser_promoted_to_truth=False)


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument("--manifest",type=Path,required=True)
    p.add_argument("--output",type=Path,required=True)
    args=p.parse_args()
    url=os.environ.get("DATABASE_URL")
    if not url:p.error("SLR native DATABASE_URL must be set in environment")
    try:import psycopg2
    except ImportError as err:
        p.error("psycopg2 required for read-only SLR PG export: "+str(err))
    receipt=export_native_pg_manifest(args.manifest,args.output,
        connect=lambda:psycopg2.connect(url))
    print(json.dumps(receipt,sort_keys=True,indent=2))


if __name__=="__main__":
    main()
