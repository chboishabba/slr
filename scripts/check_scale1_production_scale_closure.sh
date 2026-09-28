#!/usr/bin/env bash
set -euo pipefail

: "${ECONOMY_RECEIPT:?ECONOMY_RECEIPT is required}"
: "${WORKER_SCALING_RECEIPT:?WORKER_SCALING_RECEIPT is required}"
: "${ARCHIVE_SCALE_RECEIPT:?ARCHIVE_SCALE_RECEIPT is required}"

OUTPUT="${OUTPUT:-/tmp/scale1-production-scale-closure.json}"

python3 python/scale1_production_scale_closure.py   --economy "$ECONOMY_RECEIPT"   --worker-scaling "$WORKER_SCALING_RECEIPT"   --archive-scale "$ARCHIVE_SCALE_RECEIPT"   --output "$OUTPUT" >/dev/null

jq -e '
  .schema == "sensiblaw.scale1.production-scale-closure.v0_1"
  and .economy.closed == true
  and .economy.same_run_parser_dominance == true
  and .economy.small_edit_locality == true
  and .economy.same_domain_new_source == true
  and .worker_scaling.green == true
  and .worker_scaling.observed_best_parallel_speedup
      >= .worker_scaling.min_best_parallel_speedup
  and .worker_scaling.max_worker_parallel_efficiency
      >= .worker_scaling.min_max_worker_efficiency
  and .archive_scale.green == true
  and .archive_scale.observed_slope_work_units_per_token
      <= .archive_scale.max_work_units_per_token
  and .archive_scale.observed_token_span_ratio
      >= .archive_scale.min_token_span_ratio
  and .archive_scale.observation_count >= 2
  and .archive_scale.max_tokens > .archive_scale.min_tokens
  and .boundary.creates_semantic_authority == false
  and .boundary.claim_truth_promoted == false
  and .scale1_production_scale_closed == true
' "$OUTPUT"

echo "SCALE1_PRODUCTION_SCALE_CLOSURE_GREEN"
jq '{
  runtime_head,
  economy,
  worker_scaling,
  archive_scale,
  boundary,
  scale1_production_scale_closed
}' "$OUTPUT"
