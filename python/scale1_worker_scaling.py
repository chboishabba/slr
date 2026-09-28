#!/usr/bin/env python3
"""Matched-workload SCALE-1 parser worker concurrency benchmark.

Each point compiles the same canonical UTF-8 text with the same parser/model
behaviour but a distinct non-semantic benchmark_nonce in parser config.  The
nonce deliberately changes parser-product identity so cross-revision cache reuse
cannot make later worker-count points cheaper.

Measured worker wall time covers only the distributed parser worker phase.
Finalize runs afterwards as an integrity check and is reported separately.
"""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import subprocess
import time


def percentile(values, pct):
    if not values:
        return 0
    xs = sorted(values)
    index = (len(xs) - 1) * pct / 100.0
    lo = int(index)
    hi = min(lo + 1, len(xs) - 1)
    if lo == hi:
        return xs[lo]
    frac = index - lo
    return int(round(xs[lo] + (xs[hi] - xs[lo]) * frac))


def concentration(values, top_n):
    if not values:
        return 0.0
    total = sum(values)
    if total == 0:
        return 0.0
    return sum(sorted(values, reverse=True)[:top_n]) / total


def run_json(cmd, *, stdin_text=None, env=None):
    proc = subprocess.run(
        cmd,
        input=stdin_text,
        text=True,
        capture_output=True,
        env=env,
        check=False,
    )
    if proc.returncode != 0:
        raise SystemExit(
            f"command failed ({proc.returncode}): {' '.join(cmd)}\n{proc.stderr}"
        )
    return json.loads(proc.stdout)


def merged_config(raw, nonce):
    value = json.loads(raw)
    if not isinstance(value, dict):
        raise SystemExit("config-json must decode to an object")
    value["benchmark_nonce"] = nonce
    return json.dumps(value, sort_keys=True, separators=(",", ":"))


def worker_point(args, worker_count, text, runtime_head):
    nonce = f"{args.benchmark_ref}:workers:{worker_count}"
    config_json = merged_config(args.config_json, nonce)
    source_ref = f"{args.source_ref_prefix}:workers:{worker_count}"
    acquisition_ref = f"{args.acquisition_ref_prefix}:workers:{worker_count}"

    env = os.environ.copy()
    if runtime_head:
        env["SENSIBLAW_RUNTIME_HEAD"] = runtime_head

    prepare = run_json(
        [
            args.scale1_bin,
            "prepare-stdin-family",
            args.source_family,
            source_ref,
            args.provider_ref,
            acquisition_ref,
            args.title,
            args.model,
            config_json,
            args.parser_script,
        ],
        stdin_text=text,
        env=env,
    )

    semantic_regions = int(prepare["semantic_regions"])
    new_jobs = int(prepare["new_jobs"])
    reused_jobs = int(prepare["reused_jobs"])
    if new_jobs != semantic_regions or reused_jobs != 0:
        raise SystemExit(
            f"worker point {worker_count} did not force fresh parser work: "
            f"new={new_jobs} reused={reused_jobs} semantic={semantic_regions}"
        )

    parser_run_ref = prepare["parser_run_ref"]
    commands = [
        [
            args.scale1_bin,
            "worker",
            parser_run_ref,
            f"worker:{args.benchmark_ref}:{worker_count}:{index}",
            str(args.batch_size),
            args.parser_script,
        ]
        for index in range(worker_count)
    ]

    started = time.perf_counter_ns()
    procs = [
        subprocess.Popen(
            cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            env=env,
        )
        for cmd in commands
    ]
    receipts = []
    for cmd, proc in zip(commands, procs, strict=True):
        stdout, stderr = proc.communicate()
        if proc.returncode != 0:
            raise SystemExit(
                f"worker failed ({proc.returncode}): {' '.join(cmd)}\n{stderr}"
            )
        receipts.append(json.loads(stdout))
    worker_wall_ns = time.perf_counter_ns() - started

    status = run_json([args.scale1_bin, "status", parser_run_ref], env=env)
    if (
        int(status["queued"]) != 0
        or int(status["leased"]) != 0
        or int(status["unattempted_semantic_regions"]) != 0
        or not status["complete"]
    ):
        raise SystemExit(f"worker point {worker_count} did not drain parser run")

    job_ns = []
    for receipt in receipts:
        job_ns.extend(int(value) for value in receipt.get("parser_job_ns", []))

    succeeded = sum(int(r["succeeded_this_worker"]) for r in receipts)
    residual = sum(int(r["residual_this_worker"]) for r in receipts)
    deferred = sum(int(r["deferred_retry_this_worker"]) for r in receipts)
    tokens = sum(int(r["token_count"]) for r in receipts)
    entities = sum(int(r["entity_count"]) for r in receipts)
    parser_process_ns = sum(int(r["parser_process_ns"]) for r in receipts)
    parser_persist_ns = sum(int(r["parser_persist_ns"]) for r in receipts)

    if succeeded + residual != semantic_regions:
        raise SystemExit(
            f"worker point {worker_count} attempted {succeeded + residual} "
            f"of {semantic_regions} semantic regions"
        )
    if residual != 0 or deferred != 0 or len(job_ns) != succeeded:
        raise SystemExit(
            f"worker point {worker_count} has residual/deferred/missing job timings"
        )

    finalize_started = time.perf_counter_ns()
    finalized = run_json([args.scale1_bin, "finalize", parser_run_ref], env=env)
    finalize_wall_ns = time.perf_counter_ns() - finalize_started
    if (
        int(finalized["unattempted_semantic_regions"]) != 0
        or int(finalized["source_region_loss_count"]) != 0
        or not finalized["candidate_pnf_reopen_complete"]
        or finalized["creates_semantic_authority"]
        or finalized["applicability_promoted"]
        or finalized["claim_truth_promoted"]
    ):
        raise SystemExit(f"worker point {worker_count} failed finalize integrity")

    return {
        "worker_count": worker_count,
        "source_ref": source_ref,
        "source_revision_ref": prepare["source_revision_ref"],
        "parser_run_ref": parser_run_ref,
        "semantic_regions": semantic_regions,
        "new_jobs": new_jobs,
        "reused_jobs": reused_jobs,
        "succeeded": succeeded,
        "residual": residual,
        "deferred_retry": deferred,
        "tokens": tokens,
        "entities": entities,
        "worker_wall_ns": worker_wall_ns,
        "aggregate_parser_process_ns": parser_process_ns,
        "aggregate_parser_persist_ns": parser_persist_ns,
        "aggregate_worker_busy_ns": parser_process_ns + parser_persist_ns,
        "tokens_per_second": (
            tokens * 1_000_000_000 / worker_wall_ns if worker_wall_ns else None
        ),
        "job_tail": {
            "count": len(job_ns),
            "p50_ns": percentile(job_ns, 50),
            "p95_ns": percentile(job_ns, 95),
            "p99_ns": percentile(job_ns, 99),
            "max_ns": max(job_ns, default=0),
            "c1": concentration(job_ns, 1),
            "c10": concentration(job_ns, 10),
        },
        "worker_balance": {
            "jobs_per_worker": [int(r["succeeded_this_worker"]) for r in receipts],
            "tokens_per_worker": [int(r["token_count"]) for r in receipts],
        },
        "finalize_wall_ns": finalize_wall_ns,
        "finalize_integrity": {
            "unattempted_semantic_regions": finalized["unattempted_semantic_regions"],
            "source_region_loss_count": finalized["source_region_loss_count"],
            "candidate_pnf_reopen_complete": finalized["candidate_pnf_reopen_complete"],
        },
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--scale1-bin", required=True)
    ap.add_argument("--source-family", required=True)
    ap.add_argument("--source-text", required=True)
    ap.add_argument("--source-ref-prefix", required=True)
    ap.add_argument("--provider-ref", required=True)
    ap.add_argument("--acquisition-ref-prefix", required=True)
    ap.add_argument("--benchmark-ref", required=True)
    ap.add_argument("--title", default="SCALE-1 worker scaling fixture")
    ap.add_argument("--model", default="en_core_web_sm")
    ap.add_argument("--config-json", default="{}")
    ap.add_argument("--parser-script", default="scripts/scale1_spacy_json_parser.py")
    ap.add_argument("--batch-size", type=int, default=64)
    ap.add_argument(
        "--workers",
        type=int,
        action="append",
        required=True,
        help="worker count; repeat for multiple points (must include 1)",
    )
    ap.add_argument("--runtime-head")
    ap.add_argument("--min-best-parallel-speedup", type=float, required=True)
    ap.add_argument("--min-max-worker-efficiency", type=float, required=True)
    ap.add_argument("--output", required=True)
    args = ap.parse_args()

    if args.min_best_parallel_speedup <= 1.0:
        raise SystemExit("minimum best parallel speedup must be greater than 1.0")
    if args.min_max_worker_efficiency <= 0.0:
        raise SystemExit("minimum max-worker efficiency must be positive")

    worker_counts = []
    for value in args.workers:
        if value <= 0:
            raise SystemExit("worker counts must be positive")
        if value not in worker_counts:
            worker_counts.append(value)
    worker_counts.sort()
    if 1 not in worker_counts or len(worker_counts) < 2:
        raise SystemExit("scaling series requires worker=1 plus at least one parallel point")

    text = Path(args.source_text).read_text()
    if not text:
        raise SystemExit("source text is empty")

    points = [
        worker_point(args, count, text, args.runtime_head) for count in worker_counts
    ]

    baseline = next(point for point in points if point["worker_count"] == 1)
    baseline_wall = baseline["worker_wall_ns"]
    baseline_tokens = baseline["tokens"]
    baseline_regions = baseline["semantic_regions"]
    for point in points:
        if point["tokens"] != baseline_tokens or point["semantic_regions"] != baseline_regions:
            raise SystemExit("worker points are not the same represented workload")
        speedup = baseline_wall / point["worker_wall_ns"]
        point["speedup_vs_one_worker"] = speedup
        point["parallel_efficiency"] = speedup / point["worker_count"]

    parallel_points = [p for p in points if p["worker_count"] > 1]
    best_parallel_speedup = max(p["speedup_vs_one_worker"] for p in parallel_points)
    max_worker_count = max(p["worker_count"] for p in points)
    max_worker_point = next(p for p in points if p["worker_count"] == max_worker_count)
    acceptance_green = (
        best_parallel_speedup >= args.min_best_parallel_speedup
        and max_worker_point["parallel_efficiency"]
        >= args.min_max_worker_efficiency
    )
    if not acceptance_green:
        raise SystemExit(
            "worker scaling acceptance failed: "
            f"best_parallel_speedup={best_parallel_speedup:.6f} "
            f"(required {args.min_best_parallel_speedup:.6f}), "
            f"max_worker_efficiency={max_worker_point['parallel_efficiency']:.6f} "
            f"(required {args.min_max_worker_efficiency:.6f})"
        )

    result = {
        "schema": "sensiblaw.scale1.worker-scaling-series.v0_2",
        "runtime_head": args.runtime_head,
        "benchmark_ref": args.benchmark_ref,
        "source_family": args.source_family,
        "represented_workload": {
            "content_bytes": len(text.encode()),
            "content_chars": len(text),
            "semantic_regions": baseline_regions,
            "tokens": baseline_tokens,
        },
        "comparison_rule": (
            "same canonical text and parser behaviour; benchmark_nonce changes "
            "parser-product identity only so every point performs fresh parser work"
        ),
        "acceptance": {
            "green": acceptance_green,
            "min_best_parallel_speedup": args.min_best_parallel_speedup,
            "observed_best_parallel_speedup": best_parallel_speedup,
            "min_max_worker_efficiency": args.min_max_worker_efficiency,
            "observed_max_worker_efficiency": max_worker_point["parallel_efficiency"],
            "max_worker_count": max_worker_count,
        },
        "points": points,
        "boundary": {
            "candidate_only": True,
            "creates_semantic_authority": False,
            "applicability_promoted": False,
            "claim_truth_promoted": False,
        },
    }
    Path(args.output).write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
