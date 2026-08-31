#!/usr/bin/env python3
"""Canonical full GWB v0.1 certification run.

Preparation remains in ``gwb_tranche.py prepare``. This driver deliberately preloads
and hash-verifies every projected text before the first Rust ``D`` frame so disk I/O
cannot contaminate the parser-relative semantic performance gate. Rust stdout is
sent to DEVNULL and stderr to a file-backed handle, preventing full-corpus pipe
backpressure/deadlock while retaining the authoritative final SL_METRIC receipt.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import subprocess
import tempfile
import time

import spacy

from gwb_tranche import (
    PROFILE_REF,
    PROJECTION_SCHEMA,
    emit_document,
    performance_tier,
    sha256_bytes,
    sha256_text,
)

FULL_RUN_SCHEMA = "sensiblaw.gwb-full-certification-receipt.v0_1"

SL_METRIC_RE = re.compile(
    r"SL_METRIC active_ns=(\d+) pipeline_wall_ns=(\d+) sentences=(\d+) paragraphs=(\d+) "
    r"candidates=(\d+) residuals=(\d+) symbols=(\d+) published=(\d+) parity_checked=(\d+) parity_failed=(\d+)"
)


def preload_verified_documents(manifest: dict, limit: int | None) -> list[tuple[dict, str]]:
    docs = list(manifest.get("documents", []))
    if limit is not None:
        docs = docs[:limit]
    loaded: list[tuple[dict, str]] = []
    for doc in docs:
        path = Path(doc["projected_path"]).resolve()
        text = path.read_text(encoding="utf-8")
        actual_hash = sha256_text(text)
        expected_hash = str(doc["projected_sha256"])
        if actual_hash != expected_hash:
            raise SystemExit(
                f"projected text hash mismatch for document {doc.get('document_ordinal')}: "
                f"expected={expected_hash} actual={actual_hash} path={path}"
            )
        actual_bytes = len(text.encode("utf-8"))
        expected_bytes = int(doc["projected_bytes"])
        if actual_bytes != expected_bytes:
            raise SystemExit(
                f"projected text byte-count mismatch for document {doc.get('document_ordinal')}: "
                f"expected={expected_bytes} actual={actual_bytes} path={path}"
            )
        loaded.append((doc, text))
    return loaded


def run(ns: argparse.Namespace) -> int:
    manifest_path = Path(ns.manifest).resolve()
    manifest_bytes = manifest_path.read_bytes()
    manifest = json.loads(manifest_bytes)
    if manifest.get("schema_version") != PROJECTION_SCHEMA or manifest.get("profile_ref") != PROFILE_REF:
        raise SystemExit("not a GWB v0.1 source projection manifest")
    if manifest.get("authority") != "source_projection_only":
        raise SystemExit("GWB projection manifest has unexpected authority class")
    if manifest.get("derived_inventory_artifacts_reingested") is not False:
        raise SystemExit("GWB projection manifest does not prove derived artifacts were excluded")

    loaded = preload_verified_documents(manifest, ns.limit)
    if not loaded:
        raise SystemExit("GWB run has no projected documents")
    if ns.limit is None and len(loaded) != int(manifest.get("document_count", -1)):
        raise SystemExit("full GWB run document count differs from projection manifest")

    rust_bin = Path(ns.rust_bin).resolve()
    if not rust_bin.exists():
        raise SystemExit(f"missing Rust stream binary: {rust_bin}")

    model_load_start = time.perf_counter_ns()
    nlp = spacy.load(ns.model)
    model_load_ns = time.perf_counter_ns() - model_load_start

    output = Path(ns.output).resolve()
    output.parent.mkdir(parents=True, exist_ok=True)

    with tempfile.NamedTemporaryFile(mode="w+", encoding="utf-8", prefix="gwb-sl-stderr-", delete=False) as err_file:
        err_path = Path(err_file.name)
        proc = subprocess.Popen(
            [str(rust_bin)],
            stdin=subprocess.PIPE,
            stdout=subprocess.DEVNULL,
            stderr=err_file,
            text=True,
            bufsize=1,
        )
        assert proc.stdin is not None
        proc.stdin.write("C\tparity=1\n")
        proc.stdin.flush()

        external_start_ns = time.perf_counter_ns()
        parse_ns = 0
        sentence_id = 0
        paragraph_id = 0
        per_document = []

        for revision_id, (doc, text) in enumerate(loaded, 1):
            before_sentence = sentence_id
            before_paragraph = paragraph_id
            sentence_id, paragraph_id, doc_parse_ns, doc_sentences, doc_paragraphs = emit_document(
                nlp, proc.stdin, text, revision_id, sentence_id, paragraph_id
            )
            parse_ns += doc_parse_ns
            per_document.append(
                {
                    "document_ordinal": doc["document_ordinal"],
                    "projected_sha256": doc["projected_sha256"],
                    "projected_bytes": doc["projected_bytes"],
                    "spacy_parse_ns": doc_parse_ns,
                    "sentences": doc_sentences,
                    "paragraphs": doc_paragraphs,
                    "sentence_id_start": before_sentence,
                    "sentence_id_end_exclusive": sentence_id,
                    "paragraph_id_start": before_paragraph,
                    "paragraph_id_end_exclusive": paragraph_id,
                }
            )

        last_parser_emit_ns = time.perf_counter_ns()
        proc.stdin.close()
        rc = proc.wait()
        end_ns = time.perf_counter_ns()
        err_file.flush()

    stderr = err_path.read_text(encoding="utf-8", errors="replace")
    try:
        err_path.unlink()
    except OSError:
        pass

    metric_matches = list(SL_METRIC_RE.finditer(stderr))
    if len(metric_matches) != 1:
        raise SystemExit(f"expected exactly one authoritative SL_METRIC receipt, found {len(metric_matches)}")
    match = metric_matches[0]
    (
        active_ns,
        pipeline_wall_ns,
        sentences,
        paragraphs,
        candidates,
        residuals,
        symbols,
        published,
        parity_checked,
        parity_failed,
    ) = map(int, match.groups())

    expected_sentences = sum(int(d["sentences"]) for d in per_document)
    expected_paragraphs = sum(int(d["paragraphs"]) for d in per_document)
    identity_ok = sentences == expected_sentences and paragraphs == expected_paragraphs
    parity_ok = parity_failed == 0 and parity_checked == sentences
    publication_ok = published == 0
    performance_ok = pipeline_wall_ns <= 2 * max(parse_ns, 1)
    rust_ok = rc == 0
    gate_pass = rust_ok and identity_ok and parity_ok and publication_ok and performance_ok
    ratio = pipeline_wall_ns / max(parse_ns, 1)

    receipt = {
        "schema_version": FULL_RUN_SCHEMA,
        "authority": "full_gwb_execution_and_parity_certification_receipt",
        "profile_ref": PROFILE_REF,
        "canonical_driver": "python/gwb_full_run.py",
        "projection_manifest": str(manifest_path),
        "projection_manifest_sha256": sha256_bytes(manifest_bytes),
        "document_count": len(loaded),
        "projected_bytes": sum(int(doc["projected_bytes"]) for doc, _ in loaded),
        "spacy_model": ns.model,
        "spacy_version": spacy.__version__,
        "rust_binary": str(rust_bin),
        "rust_return_code": rc,
        "metrics": {
            "spacy_model_cold_load_ns": model_load_ns,
            "spacy_parser_wall_occupancy_ns": parse_ns,
            "sensiblaw_active_ns": active_ns,
            "pipeline_wall_ns": pipeline_wall_ns,
            "external_controller_wall_ns": end_ns - external_start_ns,
            "post_parser_tail_ns": end_ns - last_parser_emit_ns,
            "parser_relative_ratio": ratio,
            "performance_tier": performance_tier(ratio),
            "architectural_2x_gate_pass": performance_ok,
            "sentences": sentences,
            "paragraphs": paragraphs,
            "candidate_deltas": candidates,
            "residuals": residuals,
            "symbols": symbols,
            "published": bool(published),
            "parity_checked": parity_checked,
            "parity_failed": parity_failed,
        },
        "invariants": {
            "all_projected_text_hashes_verified_before_timing": True,
            "projected_byte_counts_verified_before_timing": True,
            "source_projection_timing_excluded_from_parser_relative_gate": True,
            "model_cold_load_excluded_from_parser_relative_gate": True,
            "sentence_paragraph_accounting_matches_controller": identity_ok,
            "direct_reference_parity": parity_ok,
            "parser_did_not_publish": publication_ok,
            "rust_process_success": rust_ok,
            "full_gate_pass": gate_pass,
        },
        "per_document": per_document,
    }
    output.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    print(f"GWB_RUN documents={len(loaded)} sentences={sentences} paragraphs={paragraphs} candidates={candidates} residuals={residuals}")
    print(f"GWB_PARITY checked={parity_checked} failed={parity_failed}")
    print(f"GWB_PUBLICATION published={published}")
    print(f"GWB_PERF total/spacy={ratio:.4f}x tier={performance_tier(ratio)} gate={'PASS' if gate_pass else 'FAIL'}")
    print(output)
    if parity_failed:
        fail_lines = [line for line in stderr.splitlines() if line.startswith("SL_PARITY_FAIL")]
        for line in fail_lines[: ns.max_parity_diagnostics]:
            print(line)
        if len(fail_lines) > ns.max_parity_diagnostics:
            print(f"... {len(fail_lines) - ns.max_parity_diagnostics} additional parity diagnostics omitted")
    return 0 if gate_pass else 1


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--manifest", required=True)
    ap.add_argument("--rust-bin", default="target/release/sensiblaw-stream")
    ap.add_argument("--model", default="en_core_web_sm")
    ap.add_argument("--output", required=True)
    ap.add_argument("--limit", type=int)
    ap.add_argument("--max-parity-diagnostics", type=int, default=20)
    return run(ap.parse_args())


if __name__ == "__main__":
    raise SystemExit(main())
