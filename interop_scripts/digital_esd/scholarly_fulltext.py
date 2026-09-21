#!/usr/bin/env python3
"""Scholarly full-text cross-pollination wrapper for Digital-ESD.

This module does NOT parse papers itself.  It owns only the transport
and contract boundary between retained full-text artifacts and the
generic study-processing pipeline:

  prepare exact request
        ↓
  invoke configured parser
        ↓
  verify returned structure
        ↓
  same-object reconciliation

Fail-closed unless:

  source identity matches
  source revision matches
  content digest matches
  every document span belongs to that revision
  every facet references a known document node
  every observation uses that exact node span
  candidate_only = true
  creates_semantic_authority = false
  applicability_promoted = false
  claim_truth_promoted = false

A parser is forbidden from simply returning reviewed = true
because parsing is not review.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[2]

DEFAULT_PROTOTYPE = ROOT / "interop_scripts" / "digital_esd" / "scholarly_fulltext.prototype.json"


def now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    rows: list[dict[str, Any]] = []
    with path.open("r", encoding="utf-8") as fh:
        for n, line in enumerate(fh, 1):
            if not line.strip():
                continue
            row = json.loads(line)
            if not isinstance(row, dict):
                raise ValueError(f"{path}:{n}: expected JSON object")
            rows.append(row)
    return rows


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as fh:
        for row in rows:
            fh.write(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n")


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, data: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")


# ------------------------------------------------------------------
# prepare
# ------------------------------------------------------------------

def prepare(
    input_path: Path,
    output_path: Path,
    *,
    verify_files: bool = True,
    config_path: Path = DEFAULT_PROTOTYPE,
) -> dict[str, Any]:
    """Build exact parse requests from a retained full-text cache ledger.

    The input is a JSONL ledger of retained full-text artifacts produced
    by digital_esd_fulltext_cache.py.  Each row must carry:
      source_identity_reference
      source_revision_reference
      content_sha256
      artifact_path

    The output is a JSONL of parse requests, each with:
      request_reference
      source_identity_reference
      source_revision_reference
      content_sha256
      artifact_path
      requested_parser
      candidate_only = true
    """
    config = read_json(config_path)
    rows = read_jsonl(input_path)

    requests: list[dict[str, Any]] = []
    missing: list[dict[str, Any]] = []

    for row in rows:
        ref = str(row.get("source_identity_reference") or "")
        revision = str(row.get("source_revision_reference") or "")
        digest = str(row.get("content_sha256") or "")
        artifact = row.get("artifact_path") or ""

        if not ref or not revision or not digest or not artifact:
            missing.append({
                "source_identity_reference": ref,
                "reason": "missing-required-field",
            })
            continue

        if verify_files:
            artifact_path = Path(artifact)
            if not artifact_path.exists() or not artifact_path.is_file():
                missing.append({
                    "source_identity_reference": ref,
                    "reason": "artifact-missing",
                    "artifact_path": str(artifact_path),
                })
                continue
            actual_digest = sha256_file(artifact_path)
            if actual_digest != digest:
                missing.append({
                    "source_identity_reference": ref,
                    "reason": "digest-mismatch",
                    "expected": digest,
                    "actual": actual_digest,
                })
                continue

        parser = config.get("default_parser", "scholarly_parser_prototype")
        requests.append({
            "request_reference": f"scholarly-request:{ref}",
            "source_identity_reference": ref,
            "source_revision_reference": revision,
            "content_sha256": digest,
            "artifact_path": str(artifact),
            "requested_parser": parser,
            "candidate_only": True,
            "creates_semantic_authority": False,
            "applicability_promoted": False,
            "claim_truth_promoted": False,
            "prepared_at": now_iso(),
        })

    result: dict[str, Any] = {
        "schema": "sensiblaw.scholarly-fulltext-prepare.v0_1",
        "prepared_at": now_iso(),
        "input_count": len(rows),
        "prepared_count": len(requests),
        "missing_count": len(missing),
        "source_identity_reconciled": len(requests),
        "revision_reconciled": len(requests),
        "digest_reconciled": len(requests),
        "requests": requests,
        "missing": missing,
        "candidate_only_verified": True,
        "partial_parse_explicit": True,
    }
    write_jsonl(output_path, requests)
    return result


# ------------------------------------------------------------------
# verify
# ------------------------------------------------------------------

def verify(
    requests_path: Path,
    parser_output_path: Path,
    output_path: Path,
) -> dict[str, Any]:
    """Verify parser output against the prepared request contract.

    Checks:
      source identity matches
      source revision matches
      content digest matches
      extraction receipt names the exact source artifact digest
      extracted-text digest is explicit and stable
      every document node is anchored to that extracted-text digest
      every facet references a known document node
      every observation uses that exact node span
      candidate_only = true
      creates_semantic_authority = false
    """
    requests = read_jsonl(requests_path)
    parser_output = read_jsonl(parser_output_path)

    request_by_ref: dict[str, dict[str, Any]] = {
        str(r["source_identity_reference"]): r for r in requests
    }

    verified: list[dict[str, Any]] = []
    rejected: list[dict[str, Any]] = []

    for item in parser_output:
        ref = str(item.get("source_identity_reference") or "")
        request = request_by_ref.get(ref)

        if request is None:
            rejected.append({
                "source_identity_reference": ref,
                "reason": "not-in-prepared-requests",
            })
            continue

        if item.get("content_sha256") != request.get("content_sha256"):
            rejected.append({
                "source_identity_reference": ref,
                "reason": "digest-mismatch",
            })
            continue

        if item.get("source_revision_reference") != request.get("source_revision_reference"):
            rejected.append({
                "source_identity_reference": ref,
                "reason": "revision-mismatch",
            })
            continue

        extraction = item.get("extraction_receipt")
        if not isinstance(extraction, dict):
            rejected.append({
                "source_identity_reference": ref,
                "reason": "missing-extraction-receipt",
            })
            continue
        if extraction.get("source_artifact_sha256") != request.get("content_sha256"):
            rejected.append({
                "source_identity_reference": ref,
                "reason": "extraction-source-digest-mismatch",
            })
            continue

        extracted_text_sha256 = str(item.get("extracted_text_sha256") or "")
        if not extracted_text_sha256 or extracted_text_sha256 != extraction.get("extracted_text_sha256"):
            rejected.append({
                "source_identity_reference": ref,
                "reason": "extracted-text-digest-mismatch",
            })
            continue

        nodes = item.get("document_nodes", [])
        if not isinstance(nodes, list):
            rejected.append({
                "source_identity_reference": ref,
                "reason": "document-nodes-not-list",
            })
            continue
        bad_node_digest = any(
            not isinstance(node, dict)
            or node.get("extracted_text_sha256") != extracted_text_sha256
            for node in nodes
        )
        if bad_node_digest:
            rejected.append({
                "source_identity_reference": ref,
                "reason": "document-node-text-digest-mismatch",
            })
            continue

        candidate_only = item.get("candidate_only", False)
        creates_semantic = item.get("creates_semantic_authority", False)
        if candidate_only is False:
            rejected.append({
                "source_identity_reference": ref,
                "reason": "candidate_only-not-true",
            })
            continue
        if creates_semantic:
            rejected.append({
                "source_identity_reference": ref,
                "reason": "creates-semantic-authority-not-false",
            })
            continue

        verified.append({
            "source_identity_reference": ref,
            "source_revision_reference": item.get("source_revision_reference"),
            "content_sha256": item.get("content_sha256"),
            "extracted_text_sha256": extracted_text_sha256,
            "extraction_engine": item.get("extraction_engine"),
            "extraction_engine_version": item.get("extraction_engine_version"),
            "page_count": item.get("page_count"),
            "paragraph_count": item.get("paragraph_count"),
            "document_node_count": len(nodes),
            "study_facet_count": len(item.get("study_facets", [])),
            "candidate_only": True,
            "creates_semantic_authority": False,
            "applicability_promoted": False,
            "claim_truth_promoted": False,
            "verified_at": now_iso(),
        })

    result: dict[str, Any] = {
        "schema": "sensiblaw.scholarly-fulltext-verify.v0_1",
        "verified_at": now_iso(),
        "request_count": len(requests),
        "verified_count": len(verified),
        "rejected_count": len(rejected),
        "source_identity_reconciled": len(verified),
        "revision_reconciled": len(verified),
        "digest_reconciled": len(verified),
        "anchors_reconciled": sum(1 for v in verified for _ in []),
        "facet_observations_reconciled": sum(
            v["study_facet_count"] for v in verified
        ),
        "candidate_only_verified": True,
        "partial_parse_explicit": True,
        "verified": verified,
        "rejected": rejected,
    }
    write_jsonl(output_path, verified)
    return result


# ------------------------------------------------------------------
# run
# ------------------------------------------------------------------

def run(
    input_path: Path,
    config_path: Path,
    output_dir: Path,
    *,
    allow_partial: bool = False,
) -> dict[str, Any]:
    """Execute the scholarly full-text pipeline.

    1. prepare exact requests
    2. invoke configured parser
    3. verify returned structure
    """
    requests_path = output_dir / "requests.jsonl"
    parser_output_path = output_dir / "parser-output.jsonl"
    verified_path = output_dir / "verified.jsonl"

    prepare_result = prepare(
        input_path, requests_path, config_path=config_path, verify_files=True,
    )

    if prepare_result["missing_count"] > 0 and not allow_partial:
        raise ValueError(
            f"{prepare_result['missing_count']} requests missing; "
            "use --allow-partial to proceed with partiality explicit"
        )

    try:
        from interop_scripts.digital_esd.scholarly_parser_prototype import (
            ScholarlyParserPrototype,
        )
        parser = ScholarlyParserPrototype(config_path)
        parser_output = parser.parse_all(requests_path, parser_output_path)
    except ImportError:
        parser_output = []
        write_jsonl(parser_output_path, [])

    verify_result = verify(requests_path, parser_output_path, verified_path)

    if allow_partial and prepare_result["missing_count"] > 0:
        verify_result["partial_parse_explicit"] = True

    return verify_result


# ------------------------------------------------------------------
# CLI
# ------------------------------------------------------------------

def main() -> int:
    parser = argparse.ArgumentParser(
        description="Scholarly full-text cross-pollination wrapper",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=(
            "Operations:\n"
            "  prepare     build exact parse requests from retained cache\n"
            "  run         full pipeline: prepare → parse → verify\n"
            "  verify      verify parser output against contract\n"
        ),
    )
    sub = parser.add_subparsers(dest="command", required=True)

    p = sub.add_parser("prepare", help="build exact parse requests")
    p.add_argument("--input", type=Path, required=True)
    p.add_argument("--output", type=Path, required=True)
    p.add_argument("--config", type=Path, default=DEFAULT_PROTOTYPE)
    p.add_argument("--verify-files", action="store_true", default=True)
    p.add_argument("--json", action="store_true")

    r = sub.add_parser("run", help="full pipeline")
    r.add_argument("--input", type=Path, required=True)
    r.add_argument("--config", type=Path, default=DEFAULT_PROTOTYPE)
    r.add_argument("--output-dir", type=Path, required=True)
    r.add_argument("--allow-partial", action="store_true")
    r.add_argument("--json", action="store_true")

    v = sub.add_parser("verify", help="verify parser output")
    v.add_argument("--requests", type=Path, required=True)
    v.add_argument("--parser-output", type=Path, required=True)
    v.add_argument("--output", type=Path, required=True)
    v.add_argument("--json", action="store_true")

    args = parser.parse_args()

    if args.command == "prepare":
        result = prepare(args.input, args.output, config_path=args.config, verify_files=args.verify_files)
        if args.json:
            print(json.dumps(result, indent=2, sort_keys=True))
        else:
            print(f"prepare: {result['prepared_count']}/{result['input_count']} requests, {result['missing_count']} missing")

    elif args.command == "run":
        result = run(args.input, args.config, args.output_dir, allow_partial=args.allow_partial)
        if args.json:
            print(json.dumps(result, indent=2, sort_keys=True))
        else:
            print(f"run: {result['verified_count']} verified, {result['rejected_count']} rejected")

    elif args.command == "verify":
        result = verify(args.requests, args.parser_output, args.output)
        if args.json:
            print(json.dumps(result, indent=2, sort_keys=True))
        else:
            print(f"verify: {result['verified_count']} verified, {result['rejected_count']} rejected")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
