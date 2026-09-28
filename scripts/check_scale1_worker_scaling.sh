#!/usr/bin/env bash
set -euo pipefail

: "${DATABASE_URL:?DATABASE_URL must point to the SensibLaw PostgreSQL database}"
: "${SOURCE_FAMILY:?SOURCE_FAMILY is required}"
: "${SOURCE_TEXT:?SOURCE_TEXT must point to canonical UTF-8 text}"
: "${SOURCE_REF_PREFIX:?SOURCE_REF_PREFIX is required}"
: "${PROVIDER_REF:?PROVIDER_REF is required}"
: "${ACQUISITION_REF_PREFIX:?ACQUISITION_REF_PREFIX is required}"

SCALE1_BIN="${SCALE1_BIN:-target/release/examples/scale1_source_compiler}"
MODEL_REF="${MODEL_REF:-en_core_web_sm}"
PARSER_SCRIPT="${PARSER_SCRIPT:-scripts/scale1_spacy_json_parser.py}"
BATCH_SIZE="${BATCH_SIZE:-64}"
CONFIG_JSON="${CONFIG_JSON:-{}}"
BENCHMARK_REF="${BENCHMARK_REF:-scale1-worker-scaling}"
WORKERS="${WORKERS:-1,2,4}"
MIN_BEST_PARALLEL_SPEEDUP="${MIN_BEST_PARALLEL_SPEEDUP:-1.05}"
MIN_MAX_WORKER_EFFICIENCY="${MIN_MAX_WORKER_EFFICIENCY:-0.20}"
OUTPUT="${OUTPUT:-/tmp/scale1-worker-scaling.json}"

cargo build --release -p sensiblaw-pg-source-store --example scale1_source_compiler
RUNTIME_HEAD="${SENSIBLAW_RUNTIME_HEAD:-$(git rev-parse HEAD)}"

worker_args=()
IFS=',' read -r -a worker_counts <<< "$WORKERS"
for count in "${worker_counts[@]}"; do
  worker_args+=(--workers "$count")
done

python3 python/scale1_worker_scaling.py   --scale1-bin "$SCALE1_BIN"   --source-family "$SOURCE_FAMILY"   --source-text "$SOURCE_TEXT"   --source-ref-prefix "$SOURCE_REF_PREFIX"   --provider-ref "$PROVIDER_REF"   --acquisition-ref-prefix "$ACQUISITION_REF_PREFIX"   --benchmark-ref "$BENCHMARK_REF"   --model "$MODEL_REF"   --config-json "$CONFIG_JSON"   --parser-script "$PARSER_SCRIPT"   --batch-size "$BATCH_SIZE"   --runtime-head "$RUNTIME_HEAD"   --min-best-parallel-speedup "$MIN_BEST_PARALLEL_SPEEDUP"   --min-max-worker-efficiency "$MIN_MAX_WORKER_EFFICIENCY"   "${worker_args[@]}"   --output "$OUTPUT" >/dev/null

jq -e   --arg head "$RUNTIME_HEAD" '
  .schema == "sensiblaw.scale1.worker-scaling-series.v0_2"
  and .runtime_head == $head
  and .acceptance.green == true
  and .acceptance.observed_best_parallel_speedup >= .acceptance.min_best_parallel_speedup
  and .acceptance.observed_max_worker_efficiency >= .acceptance.min_max_worker_efficiency
  and (.points | length) >= 2
  and any(.points[]; .worker_count == 1)
  and any(.points[]; .worker_count > 1)
  and all(.points[];
      .new_jobs == .semantic_regions
      and .reused_jobs == 0
      and .succeeded == .semantic_regions
      and .residual == 0
      and .deferred_retry == 0
      and .job_tail.count == .succeeded
      and .finalize_integrity.unattempted_semantic_regions == 0
      and .finalize_integrity.source_region_loss_count == 0
      and .finalize_integrity.candidate_pnf_reopen_complete == true)
  and .boundary.candidate_only == true
  and .boundary.creates_semantic_authority == false
  and .boundary.applicability_promoted == false
  and .boundary.claim_truth_promoted == false
' "$OUTPUT"

echo "SCALE1_WORKER_SCALING_GREEN"
jq '{
  runtime_head,
  benchmark_ref,
  represented_workload,
  acceptance,
  points: [
    .points[] | {
      worker_count,
      worker_wall_ns,
      tokens_per_second,
      speedup_vs_one_worker,
      parallel_efficiency,
      job_tail,
      worker_balance
    }
  ]
}' "$OUTPUT"
