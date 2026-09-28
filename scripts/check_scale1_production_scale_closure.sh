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
  and .archive_scale.green == true
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
