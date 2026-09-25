#!/usr/bin/env python3
"""SCALE-1 DB-native GWB ingest.

This is the production-style replacement for the historical
projection-text -> TSV/stream handoff path.

For every raw GWB v0.1 source:
  raw HTML/EPUB/PDF
    -> deterministic canonical UTF-8 projection in memory
    -> scale1_long_document prepare-stdin
    -> Postgres parser jobs
    -> scale1 worker (spaCy JSON is process IPC only)
    -> durable M12 Statement/PNF
    -> corpus reconciliation
    -> S29 review projection
    -> final receipt

No projected .txt file and no TSV parser state is required.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys
from typing import Any

from gwb_prepare import collect_memberships, source_kind, validate_v01
from gwb_tranche import (
    PROFILE_REF,
    load_inventory,
    project_source,
    sha256_bytes,
    sha256_text,
)

RECEIPT_SCHEMA = "sensiblaw.gwb-scale1-db-native.v0_1"


def run_json(
    argv: list[str],
    *,
    stdin_text: str | None = None,
) -> dict[str, Any]:
    proc = subprocess.run(
        argv,
        input=stdin_text,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if proc.returncode != 0:
        raise SystemExit(
            f"command failed ({proc.returncode}): {' '.join(argv)}\n{proc.stderr}"
        )
    stdout = proc.stdout.strip()
    if not stdout:
        raise SystemExit(f"command emitted no JSON: {' '.join(argv)}")
    try:
        return json.loads(stdout)
    except json.JSONDecodeError as exc:
        raise SystemExit(
            f"command emitted non-JSON stdout: {' '.join(argv)}\n{stdout}"
        ) from exc


def stable_acquisition_ref(
    inventory_sha256: str,
    ordinal: int,
    raw_sha256: str,
    raw_bytes: int,
    projector: str,
) -> str:
    payload = "\x1f".join(
        [
            "gwb-scale1-acquisition:v1",
            PROFILE_REF,
            inventory_sha256,
            str(ordinal),
            raw_sha256,
            str(raw_bytes),
            projector,
        ]
    ).encode("utf-8")
    return f"gwb-scale1-acquisition:sha256:{hashlib.sha256(payload).hexdigest()}"


def ingest_document(
    *,
    scale1_bin: Path,
    parser_script: Path,
    model: str,
    parser_config_json: str,
    inventory_sha256: str,
    ordinal: int,
    path: Path,
) -> dict[str, Any]:
    raw = path.read_bytes()
    raw_sha256 = sha256_bytes(raw)
    text, projector = project_source(path)
    projected_sha256 = sha256_text(text)
    projected_bytes = len(text.encode("utf-8"))

    source_ref = f"source:gwb:raw-sha256:{raw_sha256}"
    provider_ref = f"gwb-source-projection:{projector}"
    acquisition_ref = stable_acquisition_ref(
        inventory_sha256,
        ordinal,
        raw_sha256,
        len(raw),
        projector,
    )

    prepare = run_json(
        [
            str(scale1_bin),
            "prepare-stdin",
            source_ref,
            provider_ref,
            acquisition_ref,
            path.name,
            model,
            parser_config_json,
            str(parser_script),
        ],
        stdin_text=text,
    )
    parser_run_ref = str(prepare["parser_run_ref"])

    worker = run_json(
        [
            str(scale1_bin),
            "worker",
            parser_run_ref,
            f"worker:gwb-scale1:{ordinal}",
            "64",
            str(parser_script),
        ]
    )
    if (
        int(worker.get("queued", 0)) != 0
        or int(worker.get("leased", 0)) != 0
        or int(worker.get("unattempted_semantic_regions", 0)) != 0
    ):
        raise SystemExit(
            f"GWB document {ordinal} has retryable/unattempted parser work: "
            f"{json.dumps(worker, sort_keys=True)}"
        )

    final = run_json(
        [str(scale1_bin), "finalize", parser_run_ref]
    )

    if str(final.get("source_revision_ref")) != str(prepare.get("source_revision_ref")):
        raise SystemExit(f"GWB document {ordinal} source revision drifted")
    if int(final.get("source_region_loss_count", -1)) != 0:
        raise SystemExit(f"GWB document {ordinal} lost source regions")
    if int(final.get("unattempted_semantic_regions", -1)) != 0:
        raise SystemExit(f"GWB document {ordinal} retained unattempted semantic regions")
    if not bool(final.get("canonical_bytes_reload_identically")):
        raise SystemExit(f"GWB document {ordinal} canonical bytes did not reopen identically")
    if not bool(final.get("candidate_pnf_reopen_complete")):
        raise SystemExit(f"GWB document {ordinal} candidate PNF did not reopen completely")
    if bool(final.get("creates_semantic_authority")) or bool(final.get("claim_truth_promoted")):
        raise SystemExit(f"GWB document {ordinal} crossed semantic authority boundary")
    if bool(final.get("reconciliation_creates_event_identity")):
        raise SystemExit(f"GWB document {ordinal} automatic reconciliation created event identity")
    if bool(final.get("reconciliation_review_creates_event_identity")):
        raise SystemExit(f"GWB document {ordinal} review projection created event identity")

    return {
        "document_ordinal": ordinal,
        "source_kind": source_kind(path),
        "source_path": str(path),
        "source_sha256": raw_sha256,
        "source_bytes": len(raw),
        "projector": projector,
        "projected_sha256": projected_sha256,
        "projected_bytes": projected_bytes,
        "source_ref": prepare["source_ref"],
        "source_revision_ref": prepare["source_revision_ref"],
        "canonical_ref": prepare["canonical_ref"],
        "document_ref": prepare["document_ref"],
        "parser_run_ref": parser_run_ref,
        "prepare": {
            "semantic_regions": prepare["semantic_regions"],
            "structural_regions": prepare["structural_regions"],
            "new_jobs": prepare["new_jobs"],
            "reused_jobs": prepare["reused_jobs"],
        },
        "worker": worker,
        "final": final,
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--inventory", required=True)
    ap.add_argument("--source-root", required=True)
    ap.add_argument(
        "--scale1-bin",
        default="target/release/examples/scale1_long_document",
    )
    ap.add_argument(
        "--parser-script",
        default="scripts/scale1_spacy_json_parser.py",
    )
    ap.add_argument("--model", default="en_core_web_sm")
    ap.add_argument("--parser-config-json", default="{}")
    ap.add_argument("--output")
    ns = ap.parse_args()

    inventory_path = Path(ns.inventory).resolve()
    source_root = Path(ns.source_root).resolve()
    scale1_bin = Path(ns.scale1_bin).resolve()
    parser_script = Path(ns.parser_script).resolve()

    if not scale1_bin.exists():
        raise SystemExit(
            f"missing SCALE-1 executable {scale1_bin}; build with "
            "cargo build --release -p sensiblaw-pg-source-store "
            "--example scale1_long_document"
        )
    if not parser_script.exists():
        raise SystemExit(f"missing parser adapter {parser_script}")

    inventory = load_inventory(inventory_path)
    memberships = collect_memberships(inventory, source_root)
    ordered_paths = validate_v01(memberships)
    inventory_bytes = inventory_path.read_bytes()
    inventory_sha256 = sha256_bytes(inventory_bytes)

    documents = []
    for ordinal, path in enumerate(ordered_paths, 1):
        documents.append(
            ingest_document(
                scale1_bin=scale1_bin,
                parser_script=parser_script,
                model=ns.model,
                parser_config_json=ns.parser_config_json,
                inventory_sha256=inventory_sha256,
                ordinal=ordinal,
                path=path,
            )
        )
        print(
            f"GWB_SCALE1 document={ordinal}/{len(ordered_paths)} "
            f"source={path.name} "
            f"statements={documents[-1]['final']['compiled_statement_count']} "
            f"pnf={documents[-1]['final']['candidate_pnf_count']} "
            f"reviews={documents[-1]['final']['reconciliation_review_items']}",
            file=sys.stderr,
        )

    receipt = {
        "schema_version": RECEIPT_SCHEMA,
        "authority": "db_native_compilation_and_review_projection_receipt_only",
        "profile_ref": PROFILE_REF,
        "canonical_driver": "python/gwb_scale1_ingest.py",
        "source_inventory": str(inventory_path),
        "source_inventory_sha256": inventory_sha256,
        "document_count": len(documents),
        "documents": documents,
        "totals": {
            "raw_bytes": sum(int(d["source_bytes"]) for d in documents),
            "projected_bytes": sum(int(d["projected_bytes"]) for d in documents),
            "semantic_regions": sum(int(d["prepare"]["semantic_regions"]) for d in documents),
            "parser_success_regions": sum(int(d["final"]["parser_success_regions"]) for d in documents),
            "parser_residual_regions": sum(int(d["final"]["parser_residual_regions"]) for d in documents),
            "compiled_statements": sum(int(d["final"]["compiled_statement_count"]) for d in documents),
            "candidate_pnf_factors": sum(int(d["final"]["candidate_pnf_count"]) for d in documents),
            "named_entity_mentions": sum(int(d["final"]["named_entity_mention_candidates"]) for d in documents),
            "temporal_mentions": sum(int(d["final"]["temporal_mention_candidates"]) for d in documents),
            "proposition_occurrences": sum(int(d["final"]["proposition_occurrence_candidates"]) for d in documents),
            "event_occurrences": sum(int(d["final"]["event_occurrence_candidates"]) for d in documents),
            "polarity_conflict_candidates": sum(int(d["final"]["polarity_conflict_candidates"]) for d in documents),
            "review_items": sum(int(d["final"]["reconciliation_review_items"]) for d in documents),
        },
        "invariants": {
            "runtime_parser_state_is_postgres": True,
            "projected_text_files_required": False,
            "tsv_parser_state_required": False,
            "all_semantic_regions_attempted": all(
                int(d["final"]["unattempted_semantic_regions"]) == 0
                for d in documents
            ),
            "zero_source_region_loss": all(
                int(d["final"]["source_region_loss_count"]) == 0
                for d in documents
            ),
            "canonical_bytes_reload_identically": all(
                bool(d["final"]["canonical_bytes_reload_identically"])
                for d in documents
            ),
            "candidate_pnf_reopen_complete": all(
                bool(d["final"]["candidate_pnf_reopen_complete"])
                for d in documents
            ),
            "automatic_reconciliation_creates_event_identity": any(
                bool(d["final"]["reconciliation_creates_event_identity"])
                for d in documents
            ),
            "review_projection_creates_event_identity": any(
                bool(d["final"]["reconciliation_review_creates_event_identity"])
                for d in documents
            ),
            "semantic_authority_created": any(
                bool(d["final"]["creates_semantic_authority"])
                for d in documents
            ),
            "claim_truth_promoted": any(
                bool(d["final"]["claim_truth_promoted"])
                for d in documents
            ),
        },
    }

    encoded = json.dumps(receipt, indent=2, sort_keys=True) + "\n"
    if ns.output:
        output = Path(ns.output).resolve()
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(encoded, encoding="utf-8")
        print(output, file=sys.stderr)
    sys.stdout.write(encoded)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
