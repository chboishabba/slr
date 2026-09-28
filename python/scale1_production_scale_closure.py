#!/usr/bin/env python3
"""Compose final SCALE-1 production-scale closure from independent receipts."""

import argparse
import json
from pathlib import Path


def load(path):
    return json.loads(Path(path).read_text())


def require(cond, message):
    if not cond:
        raise SystemExit(message)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--economy", required=True)
    ap.add_argument("--worker-scaling", required=True)
    ap.add_argument("--archive-scale", required=True)
    ap.add_argument("--output", required=True)
    args = ap.parse_args()

    economy = load(args.economy)
    workers = load(args.worker_scaling)
    archive = load(args.archive_scale)

    require(
        economy.get("schema") == "sensiblaw.scale1.economy-closure.v0_1"
        and economy.get("scale1_economy_closed") is True,
        "SCALE-1 economy closure receipt is not green",
    )
    require(
        workers.get("schema") == "sensiblaw.scale1.worker-scaling-series.v0_2",
        "worker scaling receipt schema mismatch",
    )
    require(
        archive.get("schema") == "sensiblaw.scale1.archive-scale-series.v0_2",
        "archive scale receipt schema mismatch",
    )

    heads = {
        economy.get("runtime_head"),
        workers.get("runtime_head"),
        archive.get("runtime_head"),
    }
    require(None not in heads and len(heads) == 1,
            "production-scale receipts are not from one runtime head")

    # Closure must recompute acceptance from raw measurements. A green
    # revalidator flag or a claimed speedup is not independent evidence.
    worker_points = workers.get("points", [])
    require(len(worker_points) >= 2,
            "worker scaling series has fewer than two points")
    worker_counts = [int(p.get("worker_count", 0)) for p in worker_points]
    require(len(set(worker_counts)) == len(worker_counts)
            and all(count > 0 for count in worker_counts),
            "worker scaling worker-count series is invalid")
    baselines = [p for p in worker_points if int(p["worker_count"]) == 1]
    require(len(baselines) == 1, "worker scaling needs one baseline")
    base = baselines[0]
    baseline_ns = int(base.get("worker_wall_ns", 0))
    require(baseline_ns > 0, "worker baseline has invalid wall time")
    parallel_points = [p for p in worker_points if int(p["worker_count"]) > 1]
    require(parallel_points, "worker scaling lacks a parallel measurement")
    require(all(int(p.get("worker_wall_ns", 0)) > 0 for p in worker_points),
            "worker scaling contains a nonpositive measured duration")
    require(all(
        int(p.get("tokens", -1)) == int(base.get("tokens", -2))
        and int(p.get("semantic_regions", -1))
        == int(base.get("semantic_regions", -2))
        for p in worker_points
    ), "worker scaling does not represent one workload")
    measured_speedup = max(
        baseline_ns / int(p["worker_wall_ns"]) for p in parallel_points
    )
    max_workers = max(worker_counts)
    max_worker_point = next(
        p for p in worker_points if int(p["worker_count"]) == max_workers
    )
    measured_efficiency = (
        baseline_ns / int(max_worker_point["worker_wall_ns"]) / max_workers
    )

    worker_acceptance = workers.get("acceptance", {})
    require(worker_acceptance.get("green") is True,
            "worker scaling receipt is not accepted")
    require(
        float(worker_acceptance.get("min_best_parallel_speedup", 0.0)) > 1.0,
        "worker scaling declared speedup threshold is not super-baseline",
    )
    require(
        float(worker_acceptance.get("min_max_worker_efficiency", 0.0)) > 0.0,
        "worker scaling declared efficiency threshold is not positive",
    )
    require(
        measured_speedup
        >= float(worker_acceptance.get("min_best_parallel_speedup", float("inf"))),
        "worker scaling best parallel speedup is below declared threshold",
    )
    require(
        measured_efficiency
        >= float(worker_acceptance.get("min_max_worker_efficiency", float("inf"))),
        "worker scaling max-worker efficiency is below declared threshold",
    )

    points = worker_points
    require(len(points) >= 2, "worker scaling series has fewer than two points")
    require(any(int(p.get("worker_count", 0)) == 1 for p in points),
            "worker scaling has no one-worker baseline")
    require(any(int(p.get("worker_count", 0)) >= 2 for p in points),
            "worker scaling has no parallel point")
    require(
        all(
            int(p.get("residual", -1)) == 0
            and int(p.get("deferred_retry", -1)) == 0
            and int(p.get("new_jobs", -1)) == int(p.get("semantic_regions", -2))
            and int(p.get("reused_jobs", -1)) == 0
            for p in points
        ),
        "worker scaling series contains non-fresh or incomplete parser work",
    )

    archive_acceptance = archive.get("acceptance", {})
    require(archive_acceptance.get("green") is True,
            "archive scale receipt is not accepted")
    require(
        float(archive_acceptance.get("max_work_units_per_token", 0.0)) > 0.0,
        "archive scale declared work budget is not positive",
    )
    require(
        float(archive_acceptance.get("min_token_span_ratio", 0.0)) > 1.0,
        "archive scale declared span requirement is not nontrivial",
    )
    require(
        float(archive_acceptance.get("observed_slope_work_units_per_token", float("inf")))
        <= float(archive_acceptance.get("max_work_units_per_token", -1.0)),
        "archive scale observed slope exceeds declared budget",
    )
    require(
        float(archive_acceptance.get("observed_token_span_ratio", 0.0))
        >= float(archive_acceptance.get("min_token_span_ratio", float("inf"))),
        "archive scale token span is below declared minimum",
    )

    envelope = archive.get("observed_affine_envelope", {})
    archive_points = archive.get("points", [])
    require(len(archive_points) >= 2,
            "archive scale series has fewer than two observations")
    require(envelope.get("all_points_within") is True,
            "archive scale series violates its declared observed work envelope")
    require(archive.get("represented_carrier") == "parser_tokens",
            "archive scale carrier is not parser tokens")

    result = {
        "schema": "sensiblaw.scale1.production-scale-closure.v0_1",
        "runtime_head": next(iter(heads)),
        "authority": "empirical_execution_receipt_only",
        "economy": {
            "closed": True,
            "same_run_parser_dominance": economy[
                "same_run_parser_dominance"
            ]["green"],
            "small_edit_locality": economy["small_edit_locality"]["green"],
            "same_domain_new_source": economy["same_domain_new_source"]["green"],
        },
        "worker_scaling": {
            "green": True,
            "worker_counts": [p["worker_count"] for p in points],
            "min_best_parallel_speedup": worker_acceptance[
                "min_best_parallel_speedup"
            ],
            "observed_best_parallel_speedup": measured_speedup,
            "min_max_worker_efficiency": worker_acceptance[
                "min_max_worker_efficiency"
            ],
            "best_speedup_vs_one_worker": measured_speedup,
            "max_worker_parallel_efficiency": measured_efficiency,
        },
        "archive_scale": {
            "green": True,
            "represented_carrier": "parser_tokens",
            "source_family": archive["source_family"],
            "parser_model_ref": archive["parser_model_ref"],
            "parser_config_digest_ref": archive["parser_config_digest_ref"],
            "max_work_units_per_token": archive_acceptance[
                "max_work_units_per_token"
            ],
            "observed_slope_work_units_per_token": archive_acceptance[
                "observed_slope_work_units_per_token"
            ],
            "min_token_span_ratio": archive_acceptance[
                "min_token_span_ratio"
            ],
            "observed_token_span_ratio": archive_acceptance[
                "observed_token_span_ratio"
            ],
            "observation_count": len(archive_points),
            "min_tokens": min(int(p["represented_tokens"]) for p in archive_points),
            "max_tokens": max(int(p["represented_tokens"]) for p in archive_points),
            "observed_work_slope_per_token": envelope[
                "slope_work_units_per_token"
            ],
            "scope": envelope["scope"],
        },
        "boundary": {
            "creates_semantic_authority": False,
            "claim_truth_promoted": False,
        },
        "scale1_production_scale_closed": True,
    }

    Path(args.output).write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
