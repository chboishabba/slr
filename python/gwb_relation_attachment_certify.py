#!/usr/bin/env python3
"""Bounded certification for the candidate-only relation-attachment producer.

This is intentionally a producer-specific pass over the already-certified GWB v0.4
source observation. It does not re-authorize the expanded semantic carrier and does
not promote dependency syntax into legal meaning.
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

from gwb_expanded_certify import CanonicalObservationHashingSink
from gwb_full_run import preload_verified_documents
from gwb_tranche import PROFILE_REF, PROJECTION_SCHEMA, emit_document, sha256_bytes

SCHEMA = "sensiblaw.gwb-relation-attachment-candidate-certification-receipt.v0_1"
SOURCE_SCHEMA = "sensiblaw.gwb-expanded-semantic-certification-receipt.v0_4"
KINDS = (
    "preposition",
    "prepositional_object",
    "prepositional_complement",
    "passive_agent_marker",
    "dative",
    "case_marker",
    "particle",
)
SOURCE_LABELS = {
    "prep": "preposition",
    "pobj": "prepositional_object",
    "pcomp": "prepositional_complement",
    "agent": "passive_agent_marker",
    "dative": "dative",
    "case": "case_marker",
    "prt": "particle",
}
RELATION_METRIC_RE = re.compile(
    r"^SL_RELATION_ATTACHMENT_METRIC parity_mode=(\d+) candidates=(\d+) "
    r"parity_checked=(\d+) parity_failed=(\d+) semantic_authority=(\d+) publication_effects=(\d+)$",
    re.MULTILINE,
)
RELATION_KIND_RE = re.compile(
    r"^SL_RELATION_ATTACHMENT kind=([a-z_]+) count=(\d+)$", re.MULTILINE
)


def parse_relation(stderr: str) -> tuple[dict[str, int], dict[str, int]]:
    metrics = list(RELATION_METRIC_RE.finditer(stderr))
    if len(metrics) != 1:
        raise SystemExit(f"expected one relation metric line, found {len(metrics)}")
    names = (
        "parity_mode", "candidates", "parity_checked", "parity_failed",
        "semantic_authority", "publication_effects",
    )
    metric = dict(zip(names, map(int, metrics[0].groups())))
    counts: dict[str, int] = {}
    for kind, count in RELATION_KIND_RE.findall(stderr):
        if kind in counts:
            raise SystemExit(f"duplicate relation kind line: {kind}")
        counts[kind] = int(count)
    if set(counts) != set(KINDS):
        raise SystemExit(
            f"relation kind mismatch missing={sorted(set(KINDS)-set(counts))} "
            f"extra={sorted(set(counts)-set(KINDS))}"
        )
    return metric, {kind: counts[kind] for kind in KINDS}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--manifest", required=True)
    ap.add_argument("--source-receipt", required=True)
    ap.add_argument("--rust-bin", default="target/release/sensiblaw-expanded-cert")
    ap.add_argument("--model", default="en_core_web_sm")
    ap.add_argument("--output", required=True)
    ns = ap.parse_args()

    source_path = Path(ns.source_receipt).resolve()
    source_bytes = source_path.read_bytes()
    source = json.loads(source_bytes)
    if source.get("schema_version") != SOURCE_SCHEMA:
        raise SystemExit("relation certification requires a v0.4 expanded source receipt")
    if not source.get("invariants", {}).get("full_expanded_gate_pass"):
        raise SystemExit("source v0.4 expanded gate did not pass")
    if not source.get("invariants", {}).get(
        "same_unsupported_dependency_relative_fibre_across_passes"
    ):
        raise SystemExit("source unsupported-dependency fibre was not stable across passes")

    source_direct = source["direct_only_performance_pass"]
    source_fibre = source_direct["unsupported_dependency_relative_fibre"]
    expected_counts = {kind: 0 for kind in KINDS}
    for label, kind in SOURCE_LABELS.items():
        expected_counts[kind] += int(source_fibre.get(label, 0))
    expected_total = sum(expected_counts.values())

    manifest_path = Path(ns.manifest).resolve()
    manifest_bytes = manifest_path.read_bytes()
    manifest = json.loads(manifest_bytes)
    if manifest.get("schema_version") != PROJECTION_SCHEMA or manifest.get("profile_ref") != PROFILE_REF:
        raise SystemExit("not a canonical GWB v0.1 projection manifest")
    loaded = preload_verified_documents(manifest, None)
    if len(loaded) != 10:
        raise SystemExit("relation certification requires the complete ten-document corpus")

    rust_bin = Path(ns.rust_bin).resolve()
    if not rust_bin.exists():
        raise SystemExit(f"missing Rust binary: {rust_bin}")

    load_start = time.perf_counter_ns()
    nlp = spacy.load(ns.model)
    model_load_ns = time.perf_counter_ns() - load_start

    with tempfile.NamedTemporaryFile(
        mode="w+", encoding="utf-8", prefix="gwb-relation-stderr-", delete=False
    ) as err_file:
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
        sink = CanonicalObservationHashingSink(proc.stdin)
        sentence_id = 0
        paragraph_id = 0
        parser_ns = 0
        run_start = time.perf_counter_ns()
        for revision_id, (_doc, text) in enumerate(loaded, 1):
            sentence_id, paragraph_id, doc_parse_ns, _sentences, _paragraphs = emit_document(
                nlp, sink, text, revision_id, sentence_id, paragraph_id
            )
            parser_ns += doc_parse_ns
        proc.stdin.close()
        rc = proc.wait()
        controller_ns = time.perf_counter_ns() - run_start
        err_file.flush()

    stderr = err_path.read_text(encoding="utf-8", errors="replace")
    try:
        err_path.unlink()
    except OSError:
        pass

    metric, counts = parse_relation(stderr)
    digest_matches = sink.hexdigest() == source_direct["canonical_parser_observation_sha256"]
    bytes_match = sink.bytes_hashed == source_direct["canonical_parser_observation_bytes"]
    counts_match_source = counts == expected_counts
    total_matches_source = sum(counts.values()) == expected_total
    parity_ok = (
        metric["parity_mode"] == 1
        and metric["parity_checked"] == sentence_id
        and metric["parity_failed"] == 0
    )
    no_authority = metric["semantic_authority"] == 0
    no_publication = metric["publication_effects"] == 0
    rust_ok = rc == 0
    gate = all(
        (
            digest_matches,
            bytes_match,
            counts_match_source,
            total_matches_source,
            parity_ok,
            no_authority,
            no_publication,
            rust_ok,
        )
    )

    receipt = {
        "schema_version": SCHEMA,
        "authority": "bounded_candidate_producer_certification_only",
        "source_receipt": str(source_path),
        "source_receipt_sha256": sha256_bytes(source_bytes),
        "source_observation_digest": source_direct["canonical_parser_observation_sha256"],
        "projection_manifest_sha256": sha256_bytes(manifest_bytes),
        "spacy_model": ns.model,
        "spacy_version": spacy.__version__,
        "spacy_model_cold_load_ns": model_load_ns,
        "parser_wall_occupancy_ns": parser_ns,
        "controller_wall_ns": controller_ns,
        "sentences": sentence_id,
        "paragraphs": paragraph_id,
        "relation_candidate_total": sum(counts.values()),
        "relation_candidate_counts": counts,
        "source_expected_counts": expected_counts,
        "producer_contract": {
            "candidate_only": True,
            "context_resolution_required": True,
            "chooses_legal_role": False,
            "changes_canonical_expanded_observation": False,
            "grants_admission_authority": False,
            "publication_effects": 0,
        },
        "invariants": {
            "source_v04_gate_passed": True,
            "same_canonical_parser_observation_as_source": digest_matches and bytes_match,
            "relation_direct_reference_parity": parity_ok,
            "relation_counts_equal_source_relative_fibre": counts_match_source,
            "relation_total_equals_source_relation_queue": total_matches_source,
            "semantic_authority_zero": no_authority,
            "publication_effects_zero": no_publication,
            "rust_process_success": rust_ok,
            "full_relation_candidate_gate_pass": gate,
        },
    }

    output = Path(ns.output).resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    for kind, count in counts.items():
        print(f"GWB_RELATION_CANDIDATE kind={kind} count={count}")
    print(f"GWB_RELATION_CANDIDATE_TOTAL count={sum(counts.values())} expected={expected_total}")
    print(
        "GWB_RELATION_CANDIDATE_PARITY "
        f"checked={metric['parity_checked']} failed={metric['parity_failed']}"
    )
    print(f"GWB_RELATION_CANDIDATE_CERTIFICATION {'PASS' if gate else 'FAIL'} receipt={output}")
    return 0 if gate else 1


if __name__ == "__main__":
    raise SystemExit(main())
