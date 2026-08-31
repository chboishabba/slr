#!/usr/bin/env python3
"""Strict GWB v0.1 certification for the post-baseline semantic-expansion lane.

The same prepared corpus and one loaded spaCy model are run twice:

1. parity pass: expanded direct + row/reference compilers;
2. performance pass: expanded direct compiler only.

Certification compares a canonical parser-observation digest over D/P/S/T/E/Q frames
only. Runtime telemetry such as M\tspacy_parse_ns=... is deliberately excluded from
that digest: timing belongs to the performance receipt, not to semantic observation
identity. Reference-certification cost is excluded from the production-speed claim;
the direct-only pass owns the parser-relative performance gate.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tempfile
import time

import spacy

from gwb_full_run import preload_verified_documents
from gwb_tranche import (
    PROFILE_REF,
    PROJECTION_SCHEMA,
    emit_document,
    performance_tier,
    sha256_bytes,
)

SCHEMA = "sensiblaw.gwb-expanded-semantic-certification-receipt.v0_2"
SEMANTIC_FRAME_KINDS = {"D", "P", "S", "T", "E", "Q"}
METRIC_RE = re.compile(
    r"SL_EXPANDED_METRIC parity_mode=(\d+) framing_active_ns=(\d+) direct_active_ns=(\d+) "
    r"reference_active_ns=(\d+) pipeline_wall_ns=(\d+) sentences=(\d+) paragraphs=(\d+) "
    r"candidates=(\d+) residuals=(\d+) alternatives=(\d+) projection_failures=(\d+) "
    r"symbols=(\d+) publication_effects=(\d+) parity_checked=(\d+) parity_failed=(\d+)"
)


class CanonicalObservationHashingSink:
    """Forward every frame, but hash only the semantic parser-observation language."""

    def __init__(self, delegate) -> None:
        self.delegate = delegate
        self.digest = hashlib.sha256()
        self.bytes_hashed = 0
        self.telemetry_frames_excluded = 0

    def write(self, text: str):
        frame_kind = text.split("\t", 1)[0].rstrip("\n")
        if frame_kind in SEMANTIC_FRAME_KINDS:
            encoded = text.encode("utf-8")
            self.digest.update(encoded)
            self.bytes_hashed += len(encoded)
        else:
            self.telemetry_frames_excluded += 1
        return self.delegate.write(text)

    def flush(self):
        return self.delegate.flush()

    def hexdigest(self) -> str:
        return self.digest.hexdigest()


def parse_metric(stderr: str) -> dict[str, int]:
    matches = list(METRIC_RE.finditer(stderr))
    if len(matches) != 1:
        raise SystemExit(f"expected exactly one SL_EXPANDED_METRIC receipt, found {len(matches)}")
    keys = [
        "parity_mode",
        "framing_active_ns",
        "direct_active_ns",
        "reference_active_ns",
        "pipeline_wall_ns",
        "sentences",
        "paragraphs",
        "candidates",
        "residuals",
        "alternatives",
        "projection_failures",
        "symbols",
        "publication_effects",
        "parity_checked",
        "parity_failed",
    ]
    return dict(zip(keys, map(int, matches[0].groups())))


def run_pass(nlp, loaded, rust_bin: Path, parity: bool, diagnostics: int) -> dict:
    with tempfile.NamedTemporaryFile(mode="w+", encoding="utf-8", prefix="gwb-expanded-stderr-", delete=False) as err_file:
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
        proc.stdin.write(f"C\tparity={1 if parity else 0}\n")
        proc.stdin.flush()
        sink = CanonicalObservationHashingSink(proc.stdin)

        external_start = time.perf_counter_ns()
        parse_ns = 0
        sentence_id = 0
        paragraph_id = 0
        per_document = []
        for revision_id, (doc, text) in enumerate(loaded, 1):
            before_sentence = sentence_id
            before_paragraph = paragraph_id
            sentence_id, paragraph_id, doc_parse_ns, doc_sentences, doc_paragraphs = emit_document(
                nlp, sink, text, revision_id, sentence_id, paragraph_id
            )
            parse_ns += doc_parse_ns
            per_document.append({
                "document_ordinal": doc["document_ordinal"],
                "projected_sha256": doc["projected_sha256"],
                "spacy_parse_ns": doc_parse_ns,
                "sentences": doc_sentences,
                "paragraphs": doc_paragraphs,
                "sentence_id_start": before_sentence,
                "sentence_id_end_exclusive": sentence_id,
                "paragraph_id_start": before_paragraph,
                "paragraph_id_end_exclusive": paragraph_id,
            })
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
    metric = parse_metric(stderr)
    expected_sentences = sum(item["sentences"] for item in per_document)
    expected_paragraphs = sum(item["paragraphs"] for item in per_document)
    identity_ok = metric["sentences"] == expected_sentences and metric["paragraphs"] == expected_paragraphs
    if parity:
        parity_ok = metric["parity_checked"] == metric["sentences"] and metric["parity_failed"] == 0
    else:
        parity_ok = metric["parity_checked"] == 0 and metric["parity_failed"] == 0
    publication_ok = metric["publication_effects"] == 0
    rust_ok = rc == 0
    ratio = metric["pipeline_wall_ns"] / max(parse_ns, 1)

    if parity and metric["parity_failed"]:
        failures = [line for line in stderr.splitlines() if line.startswith("SL_EXPANDED_PARITY_FAIL")]
        for line in failures[:diagnostics]:
            print(line)
        if len(failures) > diagnostics:
            print(f"... {len(failures) - diagnostics} additional expanded parity diagnostics omitted")

    return {
        "mode": "parity" if parity else "direct_only",
        "rust_return_code": rc,
        "canonical_parser_observation_sha256": sink.hexdigest(),
        "canonical_parser_observation_bytes": sink.bytes_hashed,
        "runtime_telemetry_frames_excluded_from_observation_digest": sink.telemetry_frames_excluded,
        "spacy_parser_wall_occupancy_ns": parse_ns,
        "external_controller_wall_ns": end_ns - external_start,
        "post_parser_tail_ns": end_ns - last_parser_emit_ns,
        "parser_relative_ratio": ratio,
        "performance_tier": performance_tier(ratio),
        "metrics": metric,
        "invariants": {
            "sentence_paragraph_accounting_matches_controller": identity_ok,
            "parity_mode_contract_holds": parity_ok,
            "publication_effects_zero": publication_ok,
            "rust_process_success": rust_ok,
        },
        "per_document": per_document,
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--manifest", required=True)
    ap.add_argument("--rust-bin", default="target/release/sensiblaw-expanded-cert")
    ap.add_argument("--model", default="en_core_web_sm")
    ap.add_argument("--output", required=True)
    ap.add_argument("--max-parity-diagnostics", type=int, default=20)
    ns = ap.parse_args()

    manifest_path = Path(ns.manifest).resolve()
    manifest_bytes = manifest_path.read_bytes()
    manifest = json.loads(manifest_bytes)
    if manifest.get("schema_version") != PROJECTION_SCHEMA or manifest.get("profile_ref") != PROFILE_REF:
        raise SystemExit("not a GWB v0.1 source projection manifest")
    if manifest.get("authority") != "source_projection_only":
        raise SystemExit("GWB projection manifest has unexpected authority class")
    if manifest.get("derived_inventory_artifacts_reingested") is not False:
        raise SystemExit("GWB projection manifest does not prove derived artifacts were excluded")
    loaded = preload_verified_documents(manifest, None)
    if len(loaded) != 10 or len(loaded) != int(manifest.get("document_count", -1)):
        raise SystemExit("expanded certification requires the complete 10-document GWB v0.1 corpus")

    rust_bin = Path(ns.rust_bin).resolve()
    if not rust_bin.exists():
        raise SystemExit(f"missing expanded Rust certification binary: {rust_bin}")

    model_load_start = time.perf_counter_ns()
    nlp = spacy.load(ns.model)
    model_load_ns = time.perf_counter_ns() - model_load_start

    parity_pass = run_pass(nlp, loaded, rust_bin, True, ns.max_parity_diagnostics)
    direct_pass = run_pass(nlp, loaded, rust_bin, False, ns.max_parity_diagnostics)

    same_observation_stream = (
        parity_pass["canonical_parser_observation_sha256"]
        == direct_pass["canonical_parser_observation_sha256"]
        and parity_pass["canonical_parser_observation_bytes"]
        == direct_pass["canonical_parser_observation_bytes"]
    )
    same_direct_accounting = all(
        parity_pass["metrics"][key] == direct_pass["metrics"][key]
        for key in (
            "sentences",
            "paragraphs",
            "candidates",
            "residuals",
            "alternatives",
            "projection_failures",
            "symbols",
            "publication_effects",
        )
    )
    parity_ok = all(parity_pass["invariants"].values())
    direct_integrity_ok = all(direct_pass["invariants"].values())
    performance_ok = (
        direct_pass["metrics"]["pipeline_wall_ns"]
        <= 2 * max(direct_pass["spacy_parser_wall_occupancy_ns"], 1)
    )
    gate_pass = same_observation_stream and same_direct_accounting and parity_ok and direct_integrity_ok and performance_ok

    receipt = {
        "schema_version": SCHEMA,
        "authority": "bounded_gwb_expanded_semantic_parity_and_performance_receipt",
        "profile_ref": PROFILE_REF,
        "projection_manifest": str(manifest_path),
        "projection_manifest_sha256": sha256_bytes(manifest_bytes),
        "document_count": len(loaded),
        "projected_bytes": sum(int(doc["projected_bytes"]) for doc, _ in loaded),
        "spacy_model": ns.model,
        "spacy_version": spacy.__version__,
        "spacy_model_cold_load_ns": model_load_ns,
        "rust_binary": str(rust_bin),
        "parser_observation_digest_contract": {
            "included_frame_kinds": sorted(SEMANTIC_FRAME_KINDS),
            "excluded_runtime_telemetry_frame_kinds": ["M"],
            "control_frames_excluded": True,
        },
        "parity_pass": parity_pass,
        "direct_only_performance_pass": direct_pass,
        "invariants": {
            "all_projected_text_hashes_verified_before_both_passes": True,
            "same_canonical_parser_observation_stream_across_passes": same_observation_stream,
            "runtime_timing_telemetry_excluded_from_semantic_observation_identity": True,
            "same_direct_semantic_accounting_across_passes": same_direct_accounting,
            "expanded_direct_reference_parity": parity_ok,
            "direct_only_integrity": direct_integrity_ok,
            "direct_only_architectural_2x_gate_pass": performance_ok,
            "reference_certification_cost_excluded_from_production_speed_claim": True,
            "full_expanded_gate_pass": gate_pass,
        },
    }

    output = Path(ns.output).resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    pmetric = parity_pass["metrics"]
    dmetric = direct_pass["metrics"]
    print(
        f"GWB_EXPANDED_PARITY checked={pmetric['parity_checked']} failed={pmetric['parity_failed']} "
        f"candidates={pmetric['candidates']} residuals={pmetric['residuals']} alternatives={pmetric['alternatives']}"
    )
    print(
        f"GWB_EXPANDED_OBSERVATION digest_match={same_observation_stream} "
        f"sha256={direct_pass['canonical_parser_observation_sha256']} "
        f"bytes={direct_pass['canonical_parser_observation_bytes']}"
    )
    print(
        f"GWB_EXPANDED_DIRECT active_ns={dmetric['direct_active_ns']} framing_ns={dmetric['framing_active_ns']} "
        f"total/spacy={direct_pass['parser_relative_ratio']:.4f}x "
        f"tier={direct_pass['performance_tier']} gate={'PASS' if performance_ok else 'FAIL'}"
    )
    print(f"GWB_EXPANDED_CERTIFICATION {'PASS' if gate_pass else 'FAIL'} receipt={output}")
    return 0 if gate_pass else 1


if __name__ == "__main__":
    raise SystemExit(main())
