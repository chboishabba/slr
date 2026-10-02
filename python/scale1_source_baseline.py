#!/usr/bin/env python3
"""Source-family-neutral SCALE-1 canonical-text baseline.

This is the execution boundary *after* a source adapter has produced canonical
text. Raw-source projection, authorship, timeline, and revision preservation
remain the responsibility of that adapter; this runner cannot certify them.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import time


SCHEMA = "sensiblaw.scale1.canonical-source-baseline.v0_1"


def validate_compiler_receipt(compiled: dict, family: str, canonical: bytes) -> None:
    """Fail closed on identity, loss, incomplete parsing, and authority promotion."""
    source = compiled.get("source", {})
    integrity = compiled.get("integrity", {})
    parser = compiled.get("parser", {})
    if compiled.get("source_family") != family:
        raise ValueError("compiler source family differs from requested family")
    if compiled.get("input_transport") != "stdin":
        raise ValueError("compiler did not use stdin transport")
    if source.get("content_digest_ref") != "sha256:" + hashlib.sha256(canonical).hexdigest():
        raise ValueError("canonical digest mismatch")
    if source.get("canonical_bytes") != len(canonical):
        raise ValueError("canonical byte count mismatch")
    required = {
        "canonical_bytes_reload_identically": (source, True),
        "source_region_loss_count": (integrity, 0),
        "unattempted_semantic_regions": (integrity, 0),
        "every_region_reloaded": (integrity, True),
        "candidate_pnf_reopen_complete": (integrity, True),
        "creates_semantic_authority": (integrity, False),
        "applicability_promoted": (integrity, False),
        "claim_truth_promoted": (integrity, False),
    }
    for key, (owner, expected) in required.items():
        if owner.get(key) != expected:
            raise ValueError(f"compiler integrity violation: {key}")
    eligible = integrity.get("semantic_eligible_regions")
    successes = integrity.get("parser_success_regions")
    residuals = integrity.get("parser_residual_regions")
    if any(type(v) is not int or v < 0 for v in (eligible, successes, residuals)):
        raise ValueError("semantic region counters missing or invalid")
    if successes + residuals != eligible:
        raise ValueError("semantic region partition mismatch")
    if parser.get("deferred_retry_this_run") != 0:
        raise ValueError("deferred parser jobs remain")


def compile_canonical(
    *, binary: str, family: str, source_ref: str, provider_ref: str,
    acquisition_ref: str, label: str, model: str, config: str,
    parser_script: str, batch_size: int, canonical: bytes,
) -> dict:
    if not canonical:
        raise ValueError("canonical source is empty")
    if not family or not source_ref or not provider_ref or not acquisition_ref:
        raise ValueError("source family and provenance identifiers are required")
    try:
        canonical.decode("utf-8", errors="strict")
    except UnicodeDecodeError as exc:
        raise ValueError("canonical bytes must be valid UTF-8") from exc
    args = [
        binary, "compile-source-stdin", family, source_ref, provider_ref,
        acquisition_ref, label, model, config, parser_script, str(batch_size),
    ]
    started = time.perf_counter_ns()
    proc = subprocess.run(args, input=canonical, capture_output=True, check=False)
    elapsed_ns = time.perf_counter_ns() - started
    if proc.returncode:
        raise RuntimeError(
            f"compiler failed ({proc.returncode}): "
            + proc.stderr.decode("utf-8", errors="replace")
        )
    try:
        compiled = json.loads(proc.stdout)
    except (ValueError, UnicodeDecodeError) as exc:
        raise ValueError("compiler returned invalid JSON") from exc
    validate_compiler_receipt(compiled, family, canonical)
    return {
        "schema": SCHEMA,
        "authority": "canonical_text_execution_only_not_source_adapter_parity",
        "source_family": family,
        "source_ref": source_ref,
        "provider_ref": provider_ref,
        "acquisition_receipt_ref": acquisition_ref,
        "canonical_sha256": hashlib.sha256(canonical).hexdigest(),
        "canonical_bytes": len(canonical),
        "execution_elapsed_ns": elapsed_ns,
        "compiler_receipt": compiled,
    }


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--canonical-input", required=True, help="adapter-produced UTF-8 text")
    ap.add_argument("--source-family", required=True)
    ap.add_argument("--source-ref", required=True)
    ap.add_argument("--provider-ref", required=True)
    ap.add_argument("--acquisition-receipt-ref", required=True)
    ap.add_argument("--label", required=True)
    ap.add_argument("--scale1-bin", default="target/release/examples/scale1_source_compiler")
    ap.add_argument("--model", default="en_core_web_sm")
    ap.add_argument("--parser-config-json", default="{}")
    ap.add_argument("--parser-script", default="scripts/scale1_spacy_json_parser.py")
    ap.add_argument("--batch-size", type=int, default=32)
    ap.add_argument("--output")
    ns = ap.parse_args()
    if ns.batch_size < 1:
        ap.error("--batch-size must be positive")
    try:
        json.loads(ns.parser_config_json)
    except ValueError as exc:
        ap.error(f"invalid parser configuration JSON: {exc}")
    for candidate in (ns.scale1_bin, ns.parser_script, ns.canonical_input):
        if not Path(candidate).is_file():
            ap.error(f"required file missing: {candidate}")
    try:
        receipt = compile_canonical(
            binary=str(Path(ns.scale1_bin).resolve()),
            family=ns.source_family,
            source_ref=ns.source_ref,
            provider_ref=ns.provider_ref,
            acquisition_ref=ns.acquisition_receipt_ref,
            label=ns.label,
            model=ns.model,
            config=ns.parser_config_json,
            parser_script=str(Path(ns.parser_script).resolve()),
            batch_size=ns.batch_size,
            canonical=Path(ns.canonical_input).read_bytes(),
        )
    except (RuntimeError, ValueError) as exc:
        ap.error(str(exc))
    encoded = json.dumps(receipt, sort_keys=True, indent=2) + "\n"
    if ns.output:
        output = Path(ns.output)
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(encoded, encoding="utf-8")
    sys.stdout.write(encoded)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
