#!/usr/bin/env python3
"""Offline revalidation of retained SCALE-1 worker/archive measurement receipts.

This tool never manufactures benchmark data.  It only upgrades an existing raw
v0.1 series to the v0.2 acceptance shape when the retained measurements satisfy
the stronger non-vacuous policy.
"""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path


def load(path: str) -> dict:
    return json.loads(Path(path).read_text())


def require(cond: bool, message: str) -> None:
    if not cond:
        raise SystemExit(message)


def write(path: str, value: dict) -> None:
    Path(path).write_text(json.dumps(value, indent=2) + "\n")
    print(json.dumps(value, indent=2))


def upgrade_worker(raw: dict, min_speedup: float, min_efficiency: float) -> dict:
    require(
        raw.get("schema") in {
            "sensiblaw.scale1.worker-scaling-series.v0_1",
            "sensiblaw.scale1.worker-scaling-series.v0_2",
        },
        "worker receipt schema mismatch",
    )
    require(min_speedup > 1.0, "minimum parallel speedup must be greater than 1.0")
    require(min_efficiency > 0.0, "minimum max-worker efficiency must be positive")

    points = raw.get("points", [])
    require(len(points) >= 2, "worker series needs at least two points")
    require(any(int(p.get("worker_count", 0)) == 1 for p in points),
            "worker series has no one-worker baseline")
    parallel = [p for p in points if int(p.get("worker_count", 0)) > 1]
    require(parallel, "worker series has no parallel observation")
    require(
        all(
            int(p.get("new_jobs", -1)) == int(p.get("semantic_regions", -2))
            and int(p.get("reused_jobs", -1)) == 0
            and int(p.get("succeeded", -1)) == int(p.get("semantic_regions", -2))
            and int(p.get("residual", -1)) == 0
            and int(p.get("deferred_retry", -1)) == 0
            for p in points
        ),
        "worker series contains reused/incomplete parser work",
    )

    # Every concurrency point must be an independently compiled workload,
    # not the same cached parser run relabelled with another worker count.
    source_revisions = [p.get("source_revision_ref") for p in points]
    parser_runs = [p.get("parser_run_ref") for p in points]
    require(
        all(source_revisions) and len(set(source_revisions)) == len(points)
        and all(parser_runs) and len(set(parser_runs)) == len(points),
        "worker series repeats or omits source/parser product identity",
    )
    counts = [int(p["worker_count"]) for p in points]
    require(len(set(counts)) == len(counts),
            "worker series repeats worker-count points")
    baseline = next(p for p in points if int(p["worker_count"]) == 1)
    baseline_ns = int(baseline.get("worker_wall_ns", 0))
    require(baseline_ns > 0, "worker baseline has invalid measured wall time")
    baseline_tokens = int(baseline.get("tokens", 0))
    baseline_regions = int(baseline.get("semantic_regions", 0))
    require(baseline_tokens > 0 and baseline_regions > 0,
            "worker baseline has no measured work")
    require(
        all(
            int(p.get("worker_wall_ns", 0)) > 0
            and int(p.get("tokens", -1)) == baseline_tokens
            and int(p.get("semantic_regions", -1)) == baseline_regions
            for p in points
        ),
        "worker series has invalid wall time or unmatched workload",
    )
    for p in points:
        require(
            int(p.get("worker_count", 0)) > 0
            and int(p.get("succeeded", -1)) > 0
            and int(p.get("tokens", -1)) > 0
            and int(p.get("aggregate_parser_process_ns", -1)) >= 0
            and int(p.get("aggregate_parser_persist_ns", -1)) >= 0
            and int(p.get("aggregate_worker_busy_ns", -1))
                == int(p.get("aggregate_parser_process_ns", -2))
                   + int(p.get("aggregate_parser_persist_ns", -3)),
            "worker series has missing or contradictory busy-time counters",
        )
        require(
            int(p.get("job_tail", {}).get("count", -1))
            == int(p["succeeded"]),
            "worker point is missing job-duration observations",
        )
    best_parallel = max(
        baseline_ns / int(p["worker_wall_ns"]) for p in parallel
    )
    max_workers = max(counts)
    max_point = next(p for p in points if int(p["worker_count"]) == max_workers)
    max_eff = baseline_ns / int(max_point["worker_wall_ns"]) / max_workers
    require(best_parallel >= min_speedup,
            "retained worker series misses parallel speedup threshold")
    require(max_eff >= min_efficiency,
            "retained worker series misses max-worker efficiency threshold")

    out = dict(raw)
    out["schema"] = "sensiblaw.scale1.worker-scaling-series.v0_2"
    out["acceptance"] = {
        "green": True,
        "min_best_parallel_speedup": min_speedup,
        "observed_best_parallel_speedup": best_parallel,
        "min_max_worker_efficiency": min_efficiency,
        "observed_max_worker_efficiency": max_eff,
        "max_worker_count": max_workers,
        "revalidated_from_retained_measurement": True,
    }
    return out


def upgrade_archive(raw: dict, max_work: float, min_span: float) -> dict:
    require(
        raw.get("schema") in {
            "sensiblaw.scale1.archive-scale-series.v0_1",
            "sensiblaw.scale1.archive-scale-series.v0_2",
        },
        "archive receipt schema mismatch",
    )
    require(max_work > 0.0, "archive work-unit budget must be positive")
    require(min_span > 1.0, "archive minimum token span must be greater than 1.0")

    points = raw.get("points", [])
    require(len(points) >= 2, "archive series needs at least two points")
    heads = {p.get("runtime_head") for p in points}
    require(None not in heads and len(heads) == 1,
            "archive points are not from one runtime head")
    models = {p.get("parser_model_ref") for p in points}
    require(None not in models and len(models) == 1,
            "archive points do not share parser model")
    configs = {p.get("parser_config_digest_ref") for p in points}
    require(None not in configs and len(configs) == 1,
            "archive points do not share parser config")
    families = {p.get("source_family") for p in points}
    require(None not in families and len(families) == 1,
            "archive points do not share source family")

    tokens = [int(p.get("represented_tokens", 0)) for p in points]
    require(all(value > 0 for value in tokens), "archive point has zero tokens")
    require(len(set(tokens)) == len(tokens), "archive token sizes are not distinct")
    min_tokens = min(tokens)
    max_tokens = max(tokens)
    span = max_tokens / min_tokens

    # Source revisions must be distinct, and the promoted series cannot
    # replace a missing measurement with an arbitrary zero-work point.
    source_revisions = [p.get("source_revision_ref") for p in points]
    require(
        all(source_revisions) and len(set(source_revisions)) == len(points),
        "archive series repeats or omits source revisions",
    )
    require(
        all(
            int(p.get("semantic_regions", 0)) > 0
            and int(p.get("measured_post_parser_elapsed_ns", 0)) > 0
            and int(p.get("measured_elapsed_ns", 0))
                >= int(p.get("measured_post_parser_elapsed_ns", -1))
            for p in points
        ),
        "archive series lacks valid elapsed measurements",
    )
    work = [int(p.get("measured_post_parser_work_units", -1)) for p in points]
    require(all(w > 0 for w in work),
            "archive point lacks positive measured post-parser work")
    slope = max(math.ceil(w / t) for w, t in zip(work, tokens))
    require(
        all(w <= slope * t for w, t in zip(work, tokens)),
        "archive recomputed work envelope failed",
    )
    require(slope <= max_work, "retained archive series misses work/token budget")
    require(span >= min_span, "retained archive series misses token-span requirement")

    out = dict(raw)
    out["observed_affine_envelope"] = {
        "slope_work_units_per_token": slope,
        "intercept_work_units": 0,
        "all_points_within": True,
        "scope": "observed_points_only_not_asymptotic_claim",
    }
    out["schema"] = "sensiblaw.scale1.archive-scale-series.v0_2"
    out["runtime_head"] = next(iter(heads))
    out["parser_model_ref"] = next(iter(models))
    out["parser_config_digest_ref"] = next(iter(configs))
    out["source_family"] = next(iter(families))
    out["acceptance"] = {
        "green": True,
        "max_work_units_per_token": max_work,
        "observed_slope_work_units_per_token": slope,
        "min_token_span_ratio": min_span,
        "observed_token_span_ratio": span,
        "min_tokens": min_tokens,
        "max_tokens": max_tokens,
        "revalidated_from_retained_measurement": True,
    }
    return out


def main() -> None:
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="kind", required=True)

    worker = sub.add_parser("worker")
    worker.add_argument("--input", required=True)
    worker.add_argument("--min-best-parallel-speedup", type=float, required=True)
    worker.add_argument("--min-max-worker-efficiency", type=float, required=True)
    worker.add_argument("--output", required=True)

    archive = sub.add_parser("archive")
    archive.add_argument("--input", required=True)
    archive.add_argument("--max-work-units-per-token", type=float, required=True)
    archive.add_argument("--min-token-span-ratio", type=float, required=True)
    archive.add_argument("--output", required=True)

    args = ap.parse_args()
    raw = load(args.input)
    if args.kind == "worker":
        upgraded = upgrade_worker(
            raw,
            args.min_best_parallel_speedup,
            args.min_max_worker_efficiency,
        )
    else:
        upgraded = upgrade_archive(
            raw,
            args.max_work_units_per_token,
            args.min_token_span_ratio,
        )
    write(args.output, upgraded)


if __name__ == "__main__":
    main()
