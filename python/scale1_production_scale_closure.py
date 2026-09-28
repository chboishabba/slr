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
        workers.get("schema") == "sensiblaw.scale1.worker-scaling-series.v0_1",
        "worker scaling receipt schema mismatch",
    )
    require(
        archive.get("schema") == "sensiblaw.scale1.archive-scale-series.v0_1",
        "archive scale receipt schema mismatch",
    )

    heads = {
        economy.get("runtime_head"),
        workers.get("runtime_head"),
        archive.get("runtime_head"),
    }
    require(None not in heads and len(heads) == 1,
            "production-scale receipts are not from one runtime head")

    points = workers.get("points", [])
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
            "best_speedup_vs_one_worker": max(
                float(p["speedup_vs_one_worker"]) for p in points
            ),
            "max_worker_parallel_efficiency": next(
                float(p["parallel_efficiency"])
                for p in points
                if int(p["worker_count"])
                == max(int(q["worker_count"]) for q in points)
            ),
        },
        "archive_scale": {
            "green": True,
            "represented_carrier": "parser_tokens",
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
