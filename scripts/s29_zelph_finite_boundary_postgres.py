#!/usr/bin/env python3
"""Read-only S29 transport of ITIR source-reachable finite-word certificates.

Store on the EXISTING S29 PostgreSQL database. Preserve unknown source
boundaries and node/direction multiplicities. No replay of raw HTTP206,
C++ decoder, Agda kernel, source-support payment or reviewer authority here.
"""
from __future__ import annotations

from hashlib import sha256
import json
import re

_VERSION = "itir.zelph.source_reachable_lr_finite_word_certificate.v1"
_SHA = re.compile(r"[0-9a-f]{64}\Z")
_REV = re.compile(r"[0-9a-f]{40}\Z")
_QID = re.compile(r"Q[1-9][0-9]*\Z")
_MAX = 1000000

DDL = """CREATE TABLE IF NOT EXISTS sl_s29_zelph_finite_boundaries_v1 (
  receipt_digest text PRIMARY KEY CHECK (receipt_digest ~ '^[0-9a-f]{64}$'),
  certificate_json jsonb NOT NULL,
  graph_revision text NOT NULL CHECK (graph_revision ~ '^[0-9a-f]{40}$'),
  query_family_sha256 text NOT NULL CHECK (query_family_sha256 ~ '^[0-9a-f]{64}$'),
  conditional_boundary_complete boolean NOT NULL,
  source_authenticity_proved boolean NOT NULL DEFAULT false CHECK (NOT source_authenticity_proved),
  source_support_paid boolean NOT NULL DEFAULT false CHECK (NOT source_support_paid),
  edit_authority boolean NOT NULL DEFAULT false CHECK (NOT edit_authority),
  inserted_at timestamptz NOT NULL DEFAULT now()
)"""


def _hex(value: object, pattern: re.Pattern) -> bool:
    return type(value) is str and bool(pattern.fullmatch(value))


def _nodes(values: object) -> bool:
    return (type(values) is list and len(values) <= _MAX and
            all(type(x) is int and 0 <= x <= 2**64 - 1 for x in values)
            and values == sorted(set(values)))


def _digest(certificate: dict) -> str:
    wire = json.dumps(certificate, ensure_ascii=False, sort_keys=True,
                      separators=(",", ":"), allow_nan=False).encode("utf-8")
    return sha256(wire).hexdigest()


def check_boundary_certificate(cert: dict) -> dict:
    if type(cert) is not dict or cert.get("version") != _VERSION:
        raise ValueError("exact ITIR source-reachable finite-word certificate required")
    if not _hex(cert.get("dataset_revision"), _REV):
        raise ValueError("pinned exact graph revision required")
    for key in ("manifest_sha256", "original_index_sha256",
                "native_positive_roots_receipt_sha256", "native_decoder_sha256",
                "query_family_raw_sha256", "finite_result_raw_sha256"):
        if not _hex(cert.get(key), _SHA):
            raise ValueError(f"missing exact certificate source pin: {key}")
    if any(cert.get(key) is not False for key in (
            "physical_http206_bytes_independently_replayed_here",
            "native_capnp_cpp_semantics_formally_refined_here",
            "agda_source_reachable_boundary_term_generated_here",
            "s29_persistence_paid", "p854_source_support_paid",
            "nora_review_authority", "edit_authority")):
        raise ValueError("candidate certificate attempts upstream or review authority")
    if cert.get("evidence_only") is not True:
        raise ValueError("finite boundary is evidence-only")
    deps=cert.get("accepted_audit_dependencies")
    if (type(deps) is not list or not 1 <= len(deps) <= 64
            or any(type(d) is not dict or not _hex(d.get("sha256"),_SHA)
                   or type(d.get("used_for_direction_equality")) is not bool
                   for d in deps)):
        raise ValueError("exact original native scan dependency digests required")
    ids=[d["sha256"] for d in deps]
    if len(ids) != len(set(ids)):
        raise ValueError("duplicate native audit digest in dependency set")
    accepted={d["sha256"] for d in deps if d["used_for_direction_equality"]}
    queries=cert.get("query_certificate_entries")
    if (type(queries) is not list or not 1 <= len(queries) <= 10000
            or type(cert.get("query_count")) is not int
            or len(queries) != cert["query_count"]):
        raise ValueError("nonempty exact finite query list required")
    query_ids=set()
    pairs=set()
    all_closed=True
    total_steps=0
    for entry in queries:
        if type(entry) is not dict:
            raise ValueError("finite word entry must be an object")
        label,qid,node,word=(entry.get(k) for k in
                    ("query_id","qid","original_numeric_root","relation_word"))
        if (type(label) is not str or not label or label in query_ids
                or not _hex(qid, _QID) or type(node) is not int
                or not 0 <= node <= 2**64 - 1 or type(word) is not list
                or len(word)>64 or any(x not in ("left","right") for x in word)
                or (qid,tuple(word)) in pairs):
            raise ValueError("invalid or duplicated QID/relation-word contract")
        query_ids.add(label)
        pairs.add((qid,tuple(word)))
        steps=entry.get("source_reachable_steps")
        missing=entry.get("missing_source_boundaries")
        if type(steps) is not list or type(missing) is not list:
            raise ValueError("finite step/missing source boundary arrays required")
        frontier=[node]
        if len(steps)>len(word):
            raise ValueError("too many steps for declared finite word")
        for idx,step in enumerate(steps):
            if type(step) is not dict or step.get("step")!=idx:
                raise ValueError("wrong order or missing source induction step")
            direction=word[idx]
            if step.get("direction")!=direction or step.get("source_reachable_input_nodes")!=frontier:
                raise ValueError("source frontier or direction differs from inductive boundary")
            node_rows=step.get("per_node_direction_witnesses")
            if type(node_rows) is not list or len(node_rows)!=len(frontier):
                raise ValueError("one row witness required per reached source node")
            children=set()
            refs=set()
            for n,row in zip(frontier,node_rows):
                if type(row) is not dict or row.get("node_id")!=n or row.get("direction")!=direction:
                    raise ValueError("per-node source direction witness mismatch")
                neighbors=row.get("native_original_neighbors_sorted_unique")
                count=row.get("native_original_values_seen")
                hashes=row.get("exact_native_audit_shas")
                if (not _nodes(neighbors) or type(count) is not int
                        or count < len(neighbors) or count < 0
                        or type(hashes) is not list or not hashes
                        or hashes!=sorted(set(hashes))
                        or any(h not in accepted for h in hashes)):
                    raise ValueError("unverified native row or lost multiplicity")
                children.update(neighbors)
                refs.update(hashes)
            next_frontier=sorted(children)
            if (step.get("original_and_selected_next_nodes")!=next_frontier
                    or step.get("source_equal_native_scan_receipt_shas")!=sorted(refs)):
                raise ValueError("inductive frontier result or source dependency drift")
            frontier=next_frontier
            total_steps+=1
            if total_steps>100000:
                raise ValueError("finite-word transport exceeds bounded step count")
        closed=(entry.get("status")=="conditional-source-reachable-steps-complete")
        incomplete=(entry.get("status")=="blocked-by-unknown-original-source-direction")
        if closed:
            if len(steps)!=len(word) or missing or entry.get("end_nodes")!=frontier:
                raise ValueError("false source word completion")
        elif incomplete:
            all_closed=False
            if len(steps)>=len(word) or entry.get("end_nodes") is not None or not missing:
                raise ValueError("missing original source direction not retained")
            expected_index=len(steps)
            unseen=[]
            for gap in missing:
                if (type(gap) is not dict or gap.get("step")!=expected_index
                        or gap.get("direction")!=word[expected_index]
                        or type(gap.get("node_id")) is not int
                        or gap["node_id"] not in frontier):
                    raise ValueError("missing source boundary outside reached frontier")
                unseen.append(gap["node_id"])
            if unseen!=sorted(set(unseen)):
                raise ValueError("duplicate or unordered missing source boundaries")
        else:
            raise ValueError("unrecognized finite source boundary state")
    if cert.get("all_declared_words_conditional_boundary_complete") is not all_closed:
        raise ValueError("aggregate finite boundary completion differs from entries")
    twenty=(all_closed and len(queries)==20 and len(pairs)==20)
    if cert.get("all_twenty_distinct_words_conditional_boundary_complete") is not twenty:
        raise ValueError("twenty-word completion must mean twenty distinct complete contracts")
    return {"receipt_digest":_digest(cert),"graph_revision":cert["dataset_revision"],
            "query_family_sha256":cert["query_family_raw_sha256"],
            "conditional_boundary_complete":all_closed,"unknown_boundaries_visible":not all_closed,
            "graph_only_evidence":True,"source_support_paid":False,"edit_authority":False}


def install(conn) -> None:
    with conn.cursor() as cur:
        cur.execute(DDL)


def persist_boundary(conn, cert: dict) -> dict:
    projection=check_boundary_certificate(cert)
    with conn.cursor() as cur:
        cur.execute(
            """INSERT INTO sl_s29_zelph_finite_boundaries_v1
                 (receipt_digest,certificate_json,graph_revision,query_family_sha256,
                  conditional_boundary_complete)
                 VALUES (%s,%s::jsonb,%s,%s,%s)
                 ON CONFLICT (receipt_digest) DO NOTHING""",
            (projection["receipt_digest"],json.dumps(cert,sort_keys=True,allow_nan=False),
             projection["graph_revision"],projection["query_family_sha256"],
             projection["conditional_boundary_complete"]))
    return projection


def readback_boundary(conn, digest: str) -> dict:
    if not _hex(digest,_SHA):
        raise ValueError("invalid S29 finite certificate digest")
    with conn.cursor() as cur:
        cur.execute(
            """SELECT certificate_json,graph_revision,query_family_sha256,
                      conditional_boundary_complete,source_authenticity_proved,
                      source_support_paid,edit_authority
                 FROM sl_s29_zelph_finite_boundaries_v1 WHERE receipt_digest=%s""",
            (digest,))
        row=cur.fetchone()
    if row is None:
        raise ValueError("source boundary not committed/readable in S29")
    cert=row[0]
    if isinstance(cert,str):
        cert=json.loads(cert)
    expected=check_boundary_certificate(cert)
    if (expected["receipt_digest"]!=digest or
            (row[1],row[2],row[3]) !=
            (expected["graph_revision"],expected["query_family_sha256"],
             expected["conditional_boundary_complete"]) or
            any(row[i] for i in (4,5,6))):
        raise ValueError("persisted finite boundary provenance/authority mismatch")
    return {**expected,"db_readback_verified":True,
            "physical_original_bin_provenance_verified":False,
            "agda_kernel_witness_instantiated":False}
