#!/usr/bin/env python3
"""WIKI-1 producer-owned CSV -> SLR diagnostic packet adapter.

Consumes *actual* JMD RequestProject.RepairReview.reviewCsv output, not
synthetically recreated ontology results. Caller supplies independently
captured execution receipt and source revision/canonical bytes. This does
NOT run Lean, verify that receipt, or authorize edits. Missing native
statement GUIDs become explicit undecided obligations, never inferred QID
statement identities. Outputs JSONL of schema itir.wikidata.ontology-diagnostic.v1.

Original Lean author: JMD (github.com/meta-introspector).
This adapter is a DASHI integration, not an attribution transfer.
"""
import argparse
import csv
import hashlib
import io
import json
from pathlib import Path

SCHEMA = "itir.wikidata.ontology-diagnostic.v1"
HEADER = [
    "code", "severity", "subject", "object", "extra", "issue",
    "change", "rationale", "verdict", "score before", "score after",
]


def sha(blob: bytes) -> str:
    return "sha256:" + hashlib.sha256(blob).hexdigest()


def nonempty(value: str) -> str:
    if not value.strip():
        raise ValueError("empty required producer/source identity")
    return value


def adapt(
    raw_csv: bytes,
    canonical_source: bytes,
    source_revision_ref: str,
    wikidata_entity_ref: str,
    lean_source_commit: str,
    producer_run_ref: str,
    producer_receipt_ref: str,
    graph_view: str,
    graph_slice_ref: str | None,
    statement_map: dict,
):
    if graph_view not in {"full_statements", "truthy_rank", "scoped_slice"}:
        raise ValueError("unsupported graph view")
    if (graph_view == "scoped_slice") != bool(graph_slice_ref):
        raise ValueError("slice coordinate required iff scoped")
    if not canonical_source:
        raise ValueError("canonical source bytes are required")
    for val in (
        source_revision_ref, wikidata_entity_ref, lean_source_commit,
        producer_run_ref, producer_receipt_ref,
    ):
        nonempty(val)
    text = raw_csv.decode("utf-8")
    parsed = list(csv.DictReader(io.StringIO(text)))
    if not parsed or list(parsed[0].keys()) != HEADER:
        raise ValueError("input is not the JMD RepairReview.reviewCsv schema")
    packets = []
    for ix, row in enumerate(parsed):
        if not row["code"] or not row["issue"] or row["verdict"] not in {"proven", "rejected"}:
            raise ValueError(f"invalid CSV row {ix}")
        row_payload = json.dumps(row, sort_keys=True, ensure_ascii=False,
                                 separators=(",", ":"))
        # The entire row is retained byte-for-byte in the JSON source packet;
        # digest is over UTF-8 of this explicitly serialized row, not a claim
        # about the independent full CSV archive.
        record = statement_map.get(str(ix), {})
        native_statements = record.get("native_statements", [])
        if not isinstance(native_statements, list):
            raise ValueError(f"invalid native statement mapping for row {ix}")
        refs = [item["statement_ref"] for item in native_statements]
        if len(set(refs)) != len(refs):
            raise ValueError(f"duplicate native statement refs at row {ix}")
        rule = nonempty(row["code"])
        obligations = []
        if not native_statements:
            obligations.append(
                "No source-native Wikidata statement GUIDs supplied for this Lean report row; "
                "the QIDs in the finite model are not a statement-level provenance witness"
            )
        witnesses = []
        if native_statements:
            witnesses = [{
                "witness_ref": f"lean-review-row:{producer_run_ref}:{ix}",
                "statement_refs": refs,
                "native_statements": native_statements,
                "rule_ref": rule,
                "explanation": row["issue"],
                "evidence_refs": record.get("evidence_refs", []),
            }]
        if row["verdict"] == "rejected":
            obligations.append(
                "Lean modeled repair candidate did not pass its finite no-regression check"
            )
        disposition = "undetermined" if not native_statements else (
            "warning" if row["severity"].lower() in {"warning", "style"} else "violation"
        )
        repairs = []
        if row["change"] and row["change"] != "(no change)":
            repairs.append({
                "candidate_ref": f"jmd-repair:{producer_run_ref}:{ix}",
                "proposed_edit_description": row["change"],
                "rationale": row["rationale"] or "No detailed producer rationale",
                "modeled_verdict_ref": row["verdict"],
                "modeled_before_ref": row["score before"] or "unknown",
                "modeled_after_ref": row["score after"] or "unknown",
                "changes_wikidata": False,
                "grants_edit_authority": False,
            })
        packets.append({
            "schema": SCHEMA,
            "source_revision_ref": source_revision_ref,
            "source_snapshot_digest_ref": sha(canonical_source),
            "wikidata_entity_ref": wikidata_entity_ref,
            "graph_view": graph_view,
            "graph_slice_ref": graph_slice_ref,
            "checker_kind": "advisory_repair_review",
            "lean_owner_ref": "RequestProject.RepairReview",
            "lean_source_commit": lean_source_commit,
            "producer_run_ref": f"{producer_run_ref}:row:{ix}",
            "producer_receipt_ref": producer_receipt_ref,
            "producer_output_digest_ref": sha(row_payload.encode("utf-8")),
            "raw_checker_output": row_payload,
            "executed_checker": True,
            "lean_kernel_checked": False,
            "lean_kernel_receipt_ref": None,
            "disposition": disposition,
            "witnesses": witnesses,
            "missing_obligations": obligations,
            "repair_candidates": repairs,
            "original_author_ref": "JMD (github.com/meta-introspector)",
            "integration_ref": "DASHI-WIKI-1:jmd-review-csv-adapter:v1",
            "candidate_only": True,
            "creates_semantic_authority": False,
            "claim_truth_promoted": False,
            "grants_wikidata_edit_authority": False,
        })
    return packets


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--lean-review-csv", type=Path, required=True)
    ap.add_argument("--canonical-source", type=Path, required=True)
    ap.add_argument("--statement-map", type=Path, required=True,
                    help="JSON object keyed by 0-based CSV row with native_statements/evidence_refs")
    ap.add_argument("--source-revision-ref", required=True)
    ap.add_argument("--wikidata-entity-ref", required=True)
    ap.add_argument("--lean-source-commit", required=True)
    ap.add_argument("--producer-run-ref", required=True)
    ap.add_argument("--producer-receipt-ref", required=True)
    ap.add_argument("--graph-view", choices=["full_statements", "truthy_rank", "scoped_slice"],
                    required=True)
    ap.add_argument("--graph-slice-ref")
    ap.add_argument("--output-jsonl", type=Path, required=True)
    args = ap.parse_args()
    mapping = json.loads(args.statement_map.read_text(encoding="utf-8"))
    if not isinstance(mapping, dict):
        raise ValueError("statement-map must be a JSON object")
    packets = adapt(
        args.lean_review_csv.read_bytes(), args.canonical_source.read_bytes(),
        args.source_revision_ref, args.wikidata_entity_ref, args.lean_source_commit,
        args.producer_run_ref, args.producer_receipt_ref, args.graph_view,
        args.graph_slice_ref, mapping,
    )
    args.output_jsonl.write_text(
        "".join(json.dumps(packet, ensure_ascii=False, sort_keys=True) + "\n"
                for packet in packets), encoding="utf-8",
    )


if __name__ == "__main__":
    main()
