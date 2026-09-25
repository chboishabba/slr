#!/usr/bin/env python3
"""Run one GWB book through the SCALE-1 DB-native cold baseline.

Raw PDF/EPUB bytes are projected deterministically in memory using the existing
GWB source projector. The exact projected UTF-8 text is streamed to the Rust
ingest-book-stdin command; parser state, jobs, tokens, candidates, residuals,
reconciliation, and review pressure remain PostgreSQL-native.

No TSV token stream and no projected-text runtime file is required.
"""

from __future__ import annotations

import argparse
import hashlib
import importlib.metadata
import json
from pathlib import Path
import subprocess
import sys
import platform
import time

from gwb_tranche import project_source, sha256_bytes, sha256_text

SCHEMA = "sensiblaw.gwb-scale1-book-baseline.v0_1"


def projector_runtime_identity(projector: str) -> str:
    if projector == "pypdf":
        try:
            version = importlib.metadata.version("pypdf")
        except importlib.metadata.PackageNotFoundError:
            version = "unknown"
        return f"pypdf:{version}"
    if projector == "pdfminer.six":
        try:
            version = importlib.metadata.version("pdfminer.six")
        except importlib.metadata.PackageNotFoundError:
            version = "unknown"
        return f"pdfminer.six:{version}"
    if projector.startswith("epub-") or projector.startswith("html."):
        return f"{projector}:python-{platform.python_version()}"
    return f"{projector}:python-{platform.python_version()}"


def acquisition_ref(
    raw_sha256: str,
    raw_bytes: int,
    projector: str,
    projected_sha256: str,
) -> str:
    payload = "\x1f".join(
        [
            "gwb-scale1-book-acquisition:v1",
            raw_sha256,
            str(raw_bytes),
            projector,
            projected_sha256,
        ]
    ).encode("utf-8")
    return f"gwb-scale1-book-acquisition:sha256:{hashlib.sha256(payload).hexdigest()}"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--book", required=True)
    ap.add_argument(
        "--scale1-bin",
        default="target/release/examples/scale1_long_document",
    )
    ap.add_argument("--model", default="en_core_web_sm")
    ap.add_argument("--parser-config-json", default="{}")
    ap.add_argument(
        "--parser-script",
        default="scripts/scale1_spacy_json_parser.py",
    )
    ap.add_argument("--batch-size", type=int, default=32)
    ap.add_argument("--output")
    ns = ap.parse_args()

    book = Path(ns.book).resolve()
    scale1_bin = Path(ns.scale1_bin).resolve()
    parser_script = Path(ns.parser_script).resolve()
    if not book.exists():
        raise SystemExit(f"missing book: {book}")
    if not scale1_bin.exists():
        raise SystemExit(f"missing SCALE-1 executable: {scale1_bin}")
    if not parser_script.exists():
        raise SystemExit(f"missing parser adapter: {parser_script}")

    raw = book.read_bytes()
    raw_sha256 = sha256_bytes(raw)

    projection_started = time.perf_counter_ns()
    text, projector = project_source(book)
    projection_ns = time.perf_counter_ns() - projection_started
    projected_sha256 = sha256_text(text)
    projected_bytes = len(text.encode("utf-8"))
    projector_identity = projector_runtime_identity(projector)

    source_ref = f"source:gwb:raw-sha256:{raw_sha256}"
    provider_ref = f"gwb-source-projection:{projector_identity}"
    acquisition_receipt_ref = acquisition_ref(
        raw_sha256,
        len(raw),
        projector_identity,
        projected_sha256,
    )

    proc = subprocess.run(
        [
            str(scale1_bin),
            "ingest-book-stdin",
            source_ref,
            provider_ref,
            acquisition_receipt_ref,
            book.name,
            ns.model,
            ns.parser_config_json,
            str(parser_script),
            str(ns.batch_size),
        ],
        input=text,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if proc.returncode != 0:
        raise SystemExit(
            f"SCALE-1 book ingest failed ({proc.returncode})\n{proc.stderr}"
        )
    try:
        compiler = json.loads(proc.stdout)
    except json.JSONDecodeError as exc:
        raise SystemExit(
            f"SCALE-1 emitted non-JSON stdout:\n{proc.stdout}\n{proc.stderr}"
        ) from exc

    expected_digest = f"sha256:{projected_sha256}"
    source = compiler.get("source", {})
    integrity = compiler.get("integrity", {})
    parser = compiler.get("parser", {})
    if compiler.get("input_transport") != "stdin":
        raise SystemExit("book baseline did not use stdin transport")
    if source.get("content_digest_ref") != expected_digest:
        raise SystemExit(
            "compiler canonical text digest differs from deterministic projection"
        )
    if int(source.get("canonical_bytes", -1)) != projected_bytes:
        raise SystemExit("compiler canonical byte count differs from projection")
    if not bool(source.get("canonical_bytes_reload_identically")):
        raise SystemExit("canonical bytes did not reload identically")

    required_integrity = {
        "unattempted_semantic_regions": 0,
        "source_region_loss_count": 0,
        "every_region_reloaded": True,
        "candidate_pnf_reopen_complete": True,
        "creates_semantic_authority": False,
        "applicability_promoted": False,
        "claim_truth_promoted": False,
    }
    for key, expected in required_integrity.items():
        if integrity.get(key) != expected:
            raise SystemExit(
                f"book integrity gate failed: {key}={integrity.get(key)!r}, "
                f"expected {expected!r}"
            )

    semantic_regions = int(integrity.get("semantic_eligible_regions", -1))
    success = int(integrity.get("parser_success_regions", -1))
    residual = int(integrity.get("parser_residual_regions", -1))
    if semantic_regions < 0 or success + residual != semantic_regions:
        raise SystemExit(
            "semantic region partition mismatch: "
            f"eligible={semantic_regions} success={success} residual={residual}"
        )
    if int(parser.get("deferred_retry_this_run", -1)) != 0:
        raise SystemExit("cold baseline retained deferred parser work")

    receipt = {
        "schema_version": SCHEMA,
        "authority": "projection_plus_db_native_execution_baseline_only",
        "book": {
            "path": str(book),
            "raw_sha256": raw_sha256,
            "raw_bytes": len(raw),
            "projector": projector,
            "projector_identity": projector_identity,
            "python_version": platform.python_version(),
            "projection_ns": projection_ns,
            "projected_sha256": projected_sha256,
            "projected_bytes": projected_bytes,
        },
        "source_ref": source_ref,
        "provider_ref": provider_ref,
        "acquisition_receipt_ref": acquisition_receipt_ref,
        "compiler_receipt": compiler,
        "invariants": {
            "canonical_projection_streamed_directly": True,
            "tsv_runtime_state_required": False,
            "projected_text_runtime_file_required": False,
            "canonical_projection_digest_matches_pg": True,
            "semantic_region_partition_complete": True,
            "zero_source_region_loss": True,
            "zero_unattempted_semantic_regions": True,
            "semantic_authority_created": False,
            "applicability_promoted": False,
            "claim_truth_promoted": False,
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
