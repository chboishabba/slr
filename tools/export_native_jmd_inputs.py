#!/usr/bin/env python3
"""Export source-bound JMD-compatible inputs from persisted native SLR rows.

This is a transport adapter only: it reopens the native parser jobs, tokens and
entities for one immutable source revision. It never adds semantic promotion,
identity, or truth fields. The resulting record is consumed by the existing
JMD projection/checker lane.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
from pathlib import Path


def psql_json(sql: str) -> object:
    args = ["psql", os.environ["DATABASE_URL"], "-X", "-At", "-v", "ON_ERROR_STOP=1"]
    args += ["-c", sql]
    out = subprocess.check_output(args, text=True)
    return json.loads(out)


def sha(text: str) -> str:
    return hashlib.sha256(text.encode()).hexdigest()


def sections(text: str) -> list[dict]:
    rows = []
    cursor = 0
    for part in text.split("\n\n"):
        if not part:
            continue
        start = cursor
        end = start + len(part)
        kind = "heading" if part.startswith(("==", "# ")) else "paragraph"
        rows.append({"kind": kind, "heading": part if kind == "heading" else (rows[-1]["heading"] if rows else "Lead"),
                     "start_char": start, "end_char": end})
        cursor = end + 2
    if cursor != len(text) + 2:
        raise SystemExit(f"section projection does not cover source: {cursor} != {len(text)+2}")
    return rows


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--revision", required=True)
    ap.add_argument("--parser-run", required=True, help="Exact persisted parser run; never merge revision histories")
    ap.add_argument("--source", type=Path, required=True)
    ap.add_argument("--output", type=Path, required=True)
    ap.add_argument("--language", required=True)
    ap.add_argument("--title", required=True)
    ap.add_argument("--revid", required=True)
    args = ap.parse_args()
    text = args.source.read_text(encoding="utf-8")
    content_sha = sha(text)
    # The revision is an internally generated content address, not user SQL.
    # Quote it explicitly because psql variable interpolation is disabled in
    # some operator installations.
    rev = args.revision.replace("'", "''")
    run = args.parser_run.replace("'", "''")
    def scoped(sql):
        return psql_json(sql.replace(":'rev'", "'" + rev + "'").replace(":'run'", "'" + run + "'"))

    meta = scoped("""
      SELECT row_to_json(x) FROM (
        SELECT r.source_ref, r.source_revision_ref, r.document_ref,
               r.content_digest_ref, r.provider_ref,
               COALESCE((SELECT parser_run_ref FROM ingest.parser_job j
                         WHERE j.source_revision_ref=r.source_revision_ref
                         AND j.parser_run_ref=:'run' AND j.status='succeeded'
                         LIMIT 1), '') AS parser_run_ref
        FROM ingest.generic_source_revision r
        WHERE r.source_revision_ref=:'rev'
      ) x
    """)
    if not meta:
        raise SystemExit("native source revision not found")
    if isinstance(meta, list):
        meta = meta[0]
    if meta["parser_run_ref"] != args.parser_run:
        raise SystemExit("requested parser run not found for revision")
    expected_unit = f"source:wikipedia:{args.language}:{args.title.replace(' ', '_')}:revid:{args.revid}"
    if meta["source_ref"] != expected_unit:
        raise SystemExit("source edition/title/revision mismatch")
    if meta["content_digest_ref"] != f"sha256:{content_sha}":
        raise SystemExit(f"source digest mismatch: {content_sha} != {meta['content_digest_ref']}")

    jobs = scoped("""
      SELECT COALESCE(jsonb_agg(row_to_json(x) ORDER BY x.region_start_char), '[]'::jsonb)
      FROM (
        SELECT j.region_start_char, j.region_end_char, j.compilation_key,
          (SELECT COALESCE(jsonb_agg(jsonb_build_object(
             'i',t.token_ordinal, 'idx',t.start_char, 'text',t.surface,
             'lemma',t.lemma, 'pos',t.pos, 'tag',COALESCE(t.original_tag,''),
             'dep',t.dependency_ref, 'head_i',t.head_ordinal)
             ORDER BY t.token_ordinal), '[]'::jsonb)
           FROM ingest.parser_token t WHERE t.compilation_key=j.compilation_key) AS tokens
        FROM ingest.parser_job j
        WHERE j.source_revision_ref=:'rev' AND j.parser_run_ref=:'run' AND j.status='succeeded'
      ) x
    """)
    entities = scoped("""
      SELECT COALESCE(jsonb_agg(jsonb_build_object(
        'surface',e.surface,'start_char',e.start_char,'end_char',e.end_char,
        'label_ref',e.label_ref,'candidate_only',true,'semantic_promotion',false)
        ORDER BY e.start_char,e.end_char,e.entity_ordinal), '[]'::jsonb)
      FROM ingest.parser_entity e
      JOIN ingest.parser_job j ON j.compilation_key=e.compilation_key
      WHERE j.source_revision_ref=:'rev' AND j.parser_run_ref=:'run' AND j.status='succeeded'
    """)

    if not jobs:
        raise SystemExit("empty parser run")
    candidates = []
    for index, job in enumerate(jobs):
        start, end = int(job["region_start_char"]), int(job["region_end_char"])
        sentence = text[start:end]
        rows = []
        for token in job["tokens"]:
            token = dict(token)
            token["head_i"] = int(token["head_i"])
            rows.append(token)
        candidates.append({
            "claim_candidate_id": f"native-pnf-candidate:{job['compilation_key']}",
            "sentence_index": index, "source_span_start": start, "source_span_end": end,
            "sentence_text_sha256": sha(sentence), "dependency_rows": rows,
            "candidate_only": True, "semantic_promotion": False,
            "claim_truth_promoted": False,
        })

    record = {
        "schema": "slr-source-unit-pnf-record-v1",
        "source_unit_ref": meta["source_ref"],
        "revision_ref": meta["source_revision_ref"],
        "manifestation": {
            "candidate_only": True, "semantic_promotion": False,
            "source_text_sha256": content_sha, "source_kind": "wikipedia-article",
            "source_unit_ref": meta["source_ref"], "revision_ref": meta["source_revision_ref"],
            "language": args.language, "document_ref": meta["document_ref"],
        },
        "parser_receipt": {"dependency_capable": True, "parser_backend": "native-slr-persisted",
                           "parser_run_ref": meta["parser_run_ref"], "candidate_only": True},
        "candidate_only": True, "semantic_promotion": False,
        "parser_output_creates_ontology_truth": False,
        "source_role_creates_claim_truth": False,
        "named_entities": entities,
        "pnf_candidates": candidates,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(record, indent=2, sort_keys=True) + "\n")
    print(json.dumps({"output": str(args.output), "source_sha256": content_sha,
                      "parser_run_ref": meta["parser_run_ref"], "jobs": len(candidates)}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
