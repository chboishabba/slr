#!/usr/bin/env python3
"""Compose a token-normalized SCALE-1 archive/corpus scaling series.

Inputs must be fresh, same-head source-compiler receipts.  The represented
carrier is parser tokens, not document count.  Post-parser work units are a
literal sum of measured durable/relational operations exposed by the compiler
receipt; elapsed time is retained separately.
"""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path


def load(path):
    raw = json.loads(Path(path).read_text())
    return raw.get("compiler_receipt", raw)


def require(cond, message):
    if not cond:
        raise SystemExit(message)


def intval(mapping, key):
    value = mapping.get(key, 0)
    return 0 if value is None else int(value)


def work_units(receipt):
    integrity = receipt.get("integrity", {})
    card = receipt.get("candidate_cardinality", {})
    # Count explicit row/scan attempts, not output cardinality alone.
    fields = [
        intval(integrity, "candidate_product_factor_rows_inserted_this_run"),
        intval(integrity, "source_statement_rows_inserted_this_run"),
        intval(integrity, "candidate_batch_rows_inserted_this_run"),
        intval(card, "l2_factor_rows_scanned_this_run"),
        intval(card, "l2_entity_mention_rows_inserted_this_run"),
        intval(card, "l2_named_entity_rows_inserted_this_run"),
        intval(card, "l2_temporal_rows_inserted_this_run"),
        intval(card, "l2_proposition_fingerprint_rows_inserted_this_run"),
        intval(card, "l2_proposition_occurrence_rows_inserted_this_run"),
        intval(card, "l2_event_fingerprint_rows_inserted_this_run"),
        intval(card, "l2_event_occurrence_rows_inserted_this_run"),
        intval(card, "l2_contestation_rows_inserted_this_run"),
        intval(card, "l2_pressure_rows_upserted_this_run"),
        intval(card, "review_pressure_rows_scanned"),
        intval(card, "review_contestation_rows_scanned"),
        intval(card, "review_occurrence_rows_scanned"),
        intval(card, "review_items_persist_attempted"),
    ]
    return sum(fields)


def point(path):
    receipt = load(path)
    parser = receipt.get("parser", {})
    perf = receipt.get("performance", {})
    integrity = receipt.get("integrity", {})
    card = receipt.get("candidate_cardinality", {})
    tokens = intval(parser, "tokens_this_run")
    semantic = intval(integrity, "semantic_eligible_regions")
    new_jobs = intval(parser, "new_jobs")
    reused = intval(parser, "reused_jobs")
    require(tokens > 0, f"{path}: token count unavailable/zero")
    require(new_jobs == semantic and reused == 0,
            f"{path}: archive point is not fresh parser work")
    require(intval(integrity, "unattempted_semantic_regions") == 0,
            f"{path}: unattempted semantic regions")
    require(intval(integrity, "source_region_loss_count") == 0,
            f"{path}: source-region loss")
    require(integrity.get("candidate_pnf_reopen_complete") is True,
            f"{path}: candidate reopen incomplete")
    require(integrity.get("creates_semantic_authority") is False,
            f"{path}: semantic authority boundary crossed")
    require(integrity.get("applicability_promoted") is False,
            f"{path}: applicability boundary crossed")
    require(integrity.get("claim_truth_promoted") is False,
            f"{path}: truth boundary crossed")

    work = work_units(receipt)
    return {
        "receipt_path": str(path),
        "runtime_head": receipt.get("runtime_head"),
        "source_family": receipt.get("source_family"),
        "source_revision_ref": receipt.get("source", {}).get("source_revision_ref"),
        "parser_model_ref": parser.get("model_ref"),
        "parser_config_digest_ref": parser.get("config_digest_ref"),
        "represented_tokens": tokens,
        "semantic_regions": semantic,
        "measured_post_parser_work_units": work,
        "measured_elapsed_ns": intval(perf, "total_ns"),
        "measured_post_parser_elapsed_ns": intval(perf, "semantic_post_parser_ns"),
        "wall_ns_per_token": intval(perf, "total_ns") / tokens,
        "post_parser_ns_per_token": intval(perf, "semantic_post_parser_ns") / tokens,
        "parser_tail": {
            "p50_ns": intval(perf, "parser_job_p50_ns"),
            "p95_ns": intval(perf, "parser_job_p95_ns"),
            "p99_ns": intval(perf, "parser_job_p99_ns"),
            "max_ns": intval(perf, "parser_job_max_ns"),
            "c1": perf.get("parser_job_c1"),
            "c10": perf.get("parser_job_c10"),
        },
        "review_work": {
            "target_fibres": intval(card, "review_target_fibre_count"),
            "pressure_rows_scanned": intval(card, "review_pressure_rows_scanned"),
            "contestation_rows_scanned": intval(card, "review_contestation_rows_scanned"),
            "occurrence_rows_scanned": intval(card, "review_occurrence_rows_scanned"),
        },
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--receipt", action="append", required=True)
    ap.add_argument("--output", required=True)
    args = ap.parse_args()

    points = [point(path) for path in args.receipt]
    require(len(points) >= 2, "archive scaling requires at least two observations")

    heads = {p["runtime_head"] for p in points}
    require(None not in heads and len(heads) == 1,
            "archive points must use one identified runtime head")
    models = {p["parser_model_ref"] for p in points}
    require(None not in models and len(models) == 1,
            "archive points must use one parser model")

    points.sort(key=lambda p: p["represented_tokens"])
    token_counts = [p["represented_tokens"] for p in points]
    require(len(set(token_counts)) == len(token_counts),
            "archive points must have distinct represented token counts")

    # Conservative observed envelope with zero intercept.  This is an empirical
    # envelope over the supplied points only; it is not an asymptotic theorem.
    slope = max(
        math.ceil(p["measured_post_parser_work_units"] / p["represented_tokens"])
        for p in points
    )
    intercept = 0
    for p in points:
        p["within_declared_work_envelope"] = (
            p["measured_post_parser_work_units"]
            <= slope * p["represented_tokens"] + intercept
        )

    result = {
        "schema": "sensiblaw.scale1.archive-scale-series.v0_1",
        "runtime_head": next(iter(heads)),
        "parser_model_ref": next(iter(models)),
        "represented_carrier": "parser_tokens",
        "work_unit_definition": (
            "candidate product writes + statement/batch writes + L2 factor scans/"
            "writes + review fibre scans/persist attempts"
        ),
        "observed_affine_envelope": {
            "slope_work_units_per_token": slope,
            "intercept_work_units": intercept,
            "all_points_within": all(
                p["within_declared_work_envelope"] for p in points
            ),
            "scope": "observed_points_only_not_asymptotic_claim",
        },
        "points": points,
        "boundary": {
            "authority": "execution_measurement_only",
            "creates_semantic_authority": False,
            "claim_truth_promoted": False,
        },
    }
    Path(args.output).write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
