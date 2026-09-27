#!/usr/bin/env python3
"""Compose incomparable SCALE-1 performance measurements without averaging them.

Each input is a source-compiler receipt.  The caller assigns the benchmark
class explicitly; this script preserves the historical performance-constitution
rule that cold, upgrade/backfill, exact replay, small edit, and same-domain new
source are distinct experiments.
"""
import argparse
import json
from pathlib import Path

VALID_CLASSES = {
    "cold",
    "upgrade_backfill",
    "exact_replay",
    "small_edit",
    "same_domain_new_source",
}


def load(path):
    return json.loads(Path(path).read_text())


def measurement(kind, path):
    if kind not in VALID_CLASSES:
        raise SystemExit(f"unsupported benchmark class: {kind}")
    raw = load(path)
    compiler = raw.get("compiler_receipt", raw)
    parser = compiler.get("parser", {})
    perf = compiler.get("performance", {})
    integrity = compiler.get("integrity", {})
    source = compiler.get("source", {})
    return {
        "benchmark_class": kind,
        "receipt_path": str(path),
        "runtime_head": compiler.get("runtime_head"),
        "source_family": compiler.get("source_family"),
        "source_revision_ref": source.get("source_revision_ref"),
        "wall_ns": perf.get("total_ns"),
        "phase_ns": {
            "prepare": perf.get("prepare_ns"),
            "parser_process": perf.get("parser_process_ns"),
            "parser_persist": perf.get("parser_persist_ns"),
            "m12_compile": perf.get("m12_compile_ns"),
            "candidate_persist": perf.get("candidate_persist_ns"),
            "l2_reconciliation": perf.get("l2_reconciliation_ns"),
            "review_projection": perf.get("review_projection_ns"),
            "auto_event": perf.get("auto_event_ns"),
            "compilation_receipt_persist": perf.get("compilation_receipt_persist_ns"),
            "reload_verify": perf.get("reload_verify_ns"),
        },
        "parser_work": {
            "new_jobs": parser.get("new_jobs"),
            "reused_jobs": parser.get("reused_jobs"),
            "same_revision_reused_jobs": parser.get("same_revision_reused_jobs"),
            "cross_revision_reused_jobs": parser.get("cross_revision_reused_jobs"),
            "jobs_observed_this_run": parser.get("jobs_observed_this_run"),
            "tokens_this_run": parser.get("tokens_this_run"),
            "entities_this_run": parser.get("entities_this_run"),
        },
        "parser_tail": {
            "p50_ns": perf.get("parser_job_p50_ns"),
            "p95_ns": perf.get("parser_job_p95_ns"),
            "p99_ns": perf.get("parser_job_p99_ns"),
            "max_ns": perf.get("parser_job_max_ns"),
            "c1": perf.get("parser_job_c1"),
            "c10": perf.get("parser_job_c10"),
        },
        "candidate_work": {
            "stage_reused": integrity.get("candidate_persistence_reused"),
            "product_reuse_hits": integrity.get("candidate_product_reuse_hits_this_run"),
            "new_products": integrity.get("candidate_product_new_this_run"),
            "product_factor_rows_inserted": integrity.get(
                "candidate_product_factor_rows_inserted_this_run"
            ),
            "source_statement_rows_inserted": integrity.get(
                "source_statement_rows_inserted_this_run"
            ),
            "candidate_batch_rows_inserted": integrity.get(
                "candidate_batch_rows_inserted_this_run"
            ),
        },
        "downstream_reuse": {
            "l2": integrity.get("l2_reconciliation_reused"),
            "auto": integrity.get("auto_event_projection_reused"),
        },
        "integrity": {
            "semantic_eligible_regions": integrity.get("semantic_eligible_regions"),
            "unattempted_semantic_regions": integrity.get(
                "unattempted_semantic_regions"
            ),
            "source_region_loss_count": integrity.get("source_region_loss_count"),
            "candidate_pnf_reopen_complete": integrity.get(
                "candidate_pnf_reopen_complete"
            ),
            "creates_semantic_authority": integrity.get(
                "creates_semantic_authority"
            ),
            "applicability_promoted": integrity.get("applicability_promoted"),
            "claim_truth_promoted": integrity.get("claim_truth_promoted"),
        },
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument(
        "--measurement",
        action="append",
        nargs=2,
        metavar=("CLASS", "RECEIPT"),
        required=True,
    )
    ap.add_argument("--output", required=True)
    args = ap.parse_args()

    seen = set()
    rows = []
    for kind, path in args.measurement:
        if kind in seen:
            raise SystemExit(
                f"duplicate class {kind}; keep repeated trials as separate series files"
            )
        seen.add(kind)
        rows.append(measurement(kind, path))

    out = {
        "schema": "sensiblaw.scale1.performance-series.v0_1",
        "comparison_rule": "benchmark_classes_are_distinct_not_averaged",
        "classes_present": [row["benchmark_class"] for row in rows],
        "measurements": rows,
        "authority": "execution_measurement_only",
        "creates_semantic_authority": False,
        "claim_truth_promoted": False,
    }
    Path(args.output).write_text(json.dumps(out, indent=2) + "\n")
    print(json.dumps(out, indent=2))


if __name__ == "__main__":
    main()
