#!/usr/bin/env python3
"""S29 source-bound candidate persistence, independent DB readback and sparse reopen.

Uses the operator's EXISTING PostgreSQL database (DATABASE_URL); creates no new
database or service. The opt-in schema names are versioned to avoid modifying
unknown legacy S29 tables. Actual legacy table migration requires schema mapping.

This transport consumes ITIR candidate observations but NEVER verifies upstream
original HTTP provenance, pays P854 source support, or creates edit authority.
No synthetic reviewer is generated: reviewer ref and disposition are optional
explicit operator input, independently retained from immutable observation bytes.
"""
from __future__ import annotations

import argparse
from hashlib import sha256
import json
import os
from pathlib import Path
import re
import sys
import uuid

_SHA = re.compile(r"[0-9a-f]{64}\Z")
_QID = re.compile(r"Q[1-9][0-9]*\Z")
_REV = re.compile(r"[0-9a-f]{40}\Z")

# Execute statements individually to work with psycopg's extended protocol.
DDL = (
    """CREATE TABLE IF NOT EXISTS sl_s29_observations_v1 (
          receipt_digest text PRIMARY KEY CHECK (receipt_digest ~ '^[0-9a-f]{64}$'),
          candidate_json jsonb NOT NULL,
          source_support_paid boolean NOT NULL DEFAULT false CHECK (NOT source_support_paid),
          edit_authority boolean NOT NULL DEFAULT false CHECK (NOT edit_authority),
          inserted_at timestamptz NOT NULL DEFAULT now()
       )""",
    """CREATE TABLE IF NOT EXISTS sl_s29_sources_v1 (
          receipt_digest text NOT NULL REFERENCES sl_s29_observations_v1(receipt_digest),
          source_ref text NOT NULL,
          source_revision text NOT NULL,
          source_digest text NOT NULL CHECK (source_digest ~ '^[0-9a-f]{64}$'),
          PRIMARY KEY (receipt_digest, source_ref)
       )""",
    """CREATE TABLE IF NOT EXISTS sl_s29_dependency_v1 (
          source_ref text NOT NULL,
          consumer_ref text NOT NULL,
          query_ref text NOT NULL,
          policy_ref text NOT NULL,
          pinned_revision text NOT NULL,
          registering_receipt_digest text NOT NULL REFERENCES sl_s29_observations_v1(receipt_digest),
          PRIMARY KEY (source_ref, consumer_ref, query_ref, policy_ref)
       )""",
    """CREATE TABLE IF NOT EXISTS sl_s29_reopen_events_v1 (
          changed_receipt_digest text NOT NULL REFERENCES sl_s29_observations_v1(receipt_digest),
          source_ref text NOT NULL,
          previous_revision text NOT NULL,
          new_revision text NOT NULL,
          consumer_ref text NOT NULL,
          query_ref text NOT NULL,
          policy_ref text NOT NULL,
          minimum_horizon smallint NOT NULL DEFAULT 3 CHECK (minimum_horizon IN (3,6,9)),
          created_at timestamptz NOT NULL DEFAULT now(),
          PRIMARY KEY (changed_receipt_digest, source_ref, consumer_ref, query_ref, policy_ref)
       )""",
    """CREATE TABLE IF NOT EXISTS sl_s29_review_events_v1 (
          event_id text PRIMARY KEY,
          receipt_digest text NOT NULL REFERENCES sl_s29_observations_v1(receipt_digest),
          reviewer_ref text NOT NULL,
          disposition text NOT NULL CHECK (disposition IN ('held','rejected','ambiguous','supported-candidate')),
          rationale text NOT NULL,
          grants_source_authority boolean NOT NULL DEFAULT false CHECK (NOT grants_source_authority),
          grants_world_truth boolean NOT NULL DEFAULT false CHECK (NOT grants_world_truth),
          grants_edit_authority boolean NOT NULL DEFAULT false CHECK (NOT grants_edit_authority),
          created_at timestamptz NOT NULL DEFAULT now()
       )""",
)


def strict_json(raw: bytes) -> dict:
    def pairs(items):
        result = {}
        for key, value in items:
            if key in result:
                raise ValueError("duplicate candidate JSON key")
            result[key] = value
        return result
    def invalid(token):
        raise ValueError("invalid non-JSON number: " + token)
    obj = json.loads(raw.decode("utf-8"), object_pairs_hook=pairs, parse_constant=invalid)
    if type(obj) is not dict:
        raise ValueError("candidate root must be an object")
    return obj


def candidate_sources(data: dict) -> list[tuple[str, str, str]]:
    if data.get("version") != "itir.zelph_wikibase_source_bound_candidate.v1":
        raise ValueError("original same-run ITIR cohort candidate ABI required")
    qid, guid = data.get("qid"), data.get("statement_guid")
    if (type(qid) is not str or not _QID.fullmatch(qid)
            or type(guid) is not str or not guid.startswith(qid + "$")
            or not guid[len(qid)+1:]):
        raise ValueError("native source coordinates missing/mismatched")
    graph_revision, native_rev = data.get("graph_dataset_revision"), data.get("wikibase_native_entity_revision_id")
    if (type(graph_revision) is not str or not _REV.fullmatch(graph_revision)
            or type(native_rev) is not int or native_rev <= 0):
        raise ValueError("exact graph and native revision required")
    graph_digest = data.get("original_bin_sha256_matched_supplied_pin")
    entity_digest = data.get("wikibase_native_raw_entity_sha256")
    if any(type(value) is not str or not _SHA.fullmatch(value)
           for value in (graph_digest, entity_digest)):
        raise ValueError("full original-bin and raw-native SHA256 coordinates required")
    if any(data.get(field) is not False for field in (
        "source_support_paid", "creates_world_truth", "edit_authority",
        "persisted_s29_review_verified",
        "p854_primary_documents_independently_verified",
        "wikipedia_oldid_first_qualified_link_same_as_zelph_edge_proved",
    )):
        raise ValueError("candidate may not assert source/review/FLN authority")
    if (data.get("selected_root_bidirectional_source_adjacency_equal") is not True
            or data.get("declared_lr_query_family_complete") is not True):
        raise ValueError("source-specific selected physical graph coverage required")
    return [
        ("zelph:acrion/zelph:" + qid, graph_revision, graph_digest),
        ("wikibase:" + qid + ":" + guid, str(native_rev), entity_digest),
    ]


def candidate_receipt(data: dict) -> str:
    candidate_sources(data)
    wire = json.dumps(data, sort_keys=True, ensure_ascii=False,
                      separators=(",", ":"), allow_nan=False).encode("utf-8")
    return sha256(wire).hexdigest()


def install(conn) -> None:
    with conn.cursor() as cur:
        for sql in DDL:
            cur.execute(sql)


def persist_candidate(conn, data: dict, *, consumer_ref: str | None = None,
                      query_ref: str | None = None,
                      policy_ref: str | None = None) -> dict:
    """Writes genuine PostgreSQL rows; caller controls review/licence elsewhere."""
    sources = candidate_sources(data)
    digest = candidate_receipt(data)
    if any(x is not None for x in (consumer_ref, query_ref, policy_ref)):
        if not all(type(x) is str and x for x in (consumer_ref, query_ref, policy_ref)):
            raise ValueError("consumer, query and policy must be supplied together")
    with conn.cursor() as cur:
        cur.execute(
            """INSERT INTO sl_s29_observations_v1(receipt_digest, candidate_json)
               VALUES (%s, %s::jsonb) ON CONFLICT (receipt_digest) DO NOTHING""",
            (digest, json.dumps(data, sort_keys=True, ensure_ascii=False, allow_nan=False)))
        woke = []
        for source_ref, revision, raw_sha in sources:
            cur.execute("SELECT pg_advisory_xact_lock(hashtext(%s))", (source_ref,))
            cur.execute(
                """INSERT INTO sl_s29_sources_v1
                   (receipt_digest, source_ref, source_revision, source_digest)
                   VALUES (%s,%s,%s,%s) ON CONFLICT DO NOTHING""",
                (digest, source_ref, revision, raw_sha))
            # Insert only changed-source dependency events; no registered
            # dependency means zero wakes, NOT evidence of a negative answer.
            cur.execute(
                """INSERT INTO sl_s29_reopen_events_v1
                   (changed_receipt_digest,source_ref,previous_revision,new_revision,
                    consumer_ref,query_ref,policy_ref,minimum_horizon)
                   SELECT %s,d.source_ref,d.pinned_revision,%s,
                          d.consumer_ref,d.query_ref,d.policy_ref,3
                   FROM sl_s29_dependency_v1 d
                   WHERE d.source_ref=%s AND d.pinned_revision<>%s
                   ON CONFLICT DO NOTHING
                   RETURNING consumer_ref, query_ref, policy_ref""",
                (digest, revision, source_ref, revision))
            woke.extend((source_ref, *row) for row in cur.fetchall())
            if consumer_ref is not None:
                # Do not silently retarget an older reviewed dependency.
                # The operator must explicitly re-review and update pinning.
                cur.execute(
                    """INSERT INTO sl_s29_dependency_v1
                       (source_ref,consumer_ref,query_ref,policy_ref,
                        pinned_revision,registering_receipt_digest)
                       VALUES (%s,%s,%s,%s,%s,%s) ON CONFLICT DO NOTHING""",
                    (source_ref, consumer_ref, query_ref, policy_ref, revision, digest))
    return {"receipt_digest": digest, "source_coordinates": sources,
            "new_sparse_wake_events": woke, "source_support_paid": False,
            "edit_authority": False}


def add_explicit_review(conn, receipt_digest: str, *,
                        reviewer: str, disposition: str, rationale: str) -> str:
    if (not reviewer.strip() or not rationale.strip()
            or disposition not in ("held","rejected","ambiguous","supported-candidate")):
        raise ValueError("explicit reviewer, rationale and bounded disposition required")
    event_id = str(uuid.uuid4())
    with conn.cursor() as cur:
        cur.execute(
            """INSERT INTO sl_s29_review_events_v1
               (event_id,receipt_digest,reviewer_ref,disposition,rationale)
               VALUES (%s,%s,%s,%s,%s)""",
            (event_id, receipt_digest, reviewer, disposition, rationale))
    return event_id


def readback(conn, digest: str) -> dict:
    with conn.cursor() as cur:
        cur.execute(
            "SELECT candidate_json FROM sl_s29_observations_v1 WHERE receipt_digest=%s",
            (digest,))
        result = cur.fetchone()
        if result is None:
            raise ValueError("committed observation missing on separate connection")
        candidate = result[0]
        if type(candidate) is str:
            candidate = json.loads(candidate)
        if candidate_receipt(candidate) != digest:
            raise ValueError("persisted candidate changed across PostgreSQL readback")
        cur.execute("SELECT source_ref,source_revision,source_digest FROM sl_s29_sources_v1 WHERE receipt_digest=%s ORDER BY source_ref",
                    (digest,))
        sources = cur.fetchall()
        if sorted(tuple(row) for row in sources) != sorted(candidate_sources(candidate)):
            raise ValueError("PostgreSQL source-coordinate equality failed")
        cur.execute(
            """SELECT source_ref,previous_revision,new_revision,consumer_ref,query_ref,policy_ref
               FROM sl_s29_reopen_events_v1 WHERE changed_receipt_digest=%s
               ORDER BY source_ref,consumer_ref,query_ref,policy_ref""",
            (digest,))
        wakes = [tuple(row) for row in cur.fetchall()]
        return {"receipt_digest": digest, "db_readback_verified": True,
                "readback_sources": sources, "reopen_events": wakes,
                "persisted_observation_is_source_authority": False,
                "edit_authority": False}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--apply-schema", action="store_true",
                        help="opt-in create versioned tables in existing database")
    parser.add_argument("--consumer-ref")
    parser.add_argument("--query-ref")
    parser.add_argument("--policy-ref")
    parser.add_argument("--reviewer-ref")
    parser.add_argument("--disposition")
    parser.add_argument("--rationale")
    parser.add_argument("--receipt", type=Path)
    args = parser.parse_args()
    try:
        if any(v is not None for v in (args.reviewer_ref,args.disposition,args.rationale)):
            if not all(v for v in (args.reviewer_ref,args.disposition,args.rationale)):
                raise ValueError("reviewer, disposition and rationale required together")
        if not args.apply_schema:
            raise ValueError("explicit --apply-schema required for first opt-in bridge run")
        database = os.environ.get("DATABASE_URL")
        if not database:
            raise ValueError("existing PostgreSQL DATABASE_URL required; no new database created")
        try:
            import psycopg
        except ImportError as exc:
            raise ValueError("install psycopg v3 in operator environment") from exc
        data = strict_json(args.candidate.read_bytes())
        digest = candidate_receipt(data)
        with psycopg.connect(database) as conn:
            install(conn)
            write = persist_candidate(conn, data, consumer_ref=args.consumer_ref,
                                      query_ref=args.query_ref, policy_ref=args.policy_ref)
            if args.reviewer_ref is not None:
                write["explicit_review_event_id"] = add_explicit_review(
                    conn, digest, reviewer=args.reviewer_ref,
                    disposition=args.disposition, rationale=args.rationale)
        # Required: a new connection *after* committing the transaction.
        with psycopg.connect(database) as other:
            post = readback(other, digest)
        result = {**write, **post, "db_write_committed": True,
                  "s29_bridge_version": "sl_s29_source_observation.v1",
                  "review_authority_not_inferred": True}
        encoded = json.dumps(result, sort_keys=True, default=list, indent=2) + "\n"
        if args.receipt:
            with args.receipt.open("x", encoding="utf-8") as f:
                f.write(encoded)
        else:
            print(encoded, end="")
        return 0
    except (ValueError, OSError, KeyError, TypeError) as exc:
        print(json.dumps({"status": "blocked", "reason": str(exc),
                          "edit_authority": False}), file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
