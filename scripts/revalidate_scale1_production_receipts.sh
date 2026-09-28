#!/usr/bin/env bash
set -euo pipefail

: "${WORKER_RECEIPT:?WORKER_RECEIPT must point to a retained worker scaling receipt}"
: "${ARCHIVE_RECEIPT:?ARCHIVE_RECEIPT must point to a retained archive scale receipt}"
: "${MAX_WORK_UNITS_PER_TOKEN:?MAX_WORK_UNITS_PER_TOKEN is required}"

MIN_BEST_PARALLEL_SPEEDUP="${MIN_BEST_PARALLEL_SPEEDUP:-1.05}"
MIN_MAX_WORKER_EFFICIENCY="${MIN_MAX_WORKER_EFFICIENCY:-0.20}"
MIN_TOKEN_SPAN_RATIO="${MIN_TOKEN_SPAN_RATIO:-2.0}"
WORKER_OUTPUT="${WORKER_OUTPUT:-/tmp/scale1-worker-scaling-v02.json}"
ARCHIVE_OUTPUT="${ARCHIVE_OUTPUT:-/tmp/scale1-archive-scale-v02.json}"

python3 python/scale1_revalidate_scale_receipt.py worker   --input "$WORKER_RECEIPT"   --min-best-parallel-speedup "$MIN_BEST_PARALLEL_SPEEDUP"   --min-max-worker-efficiency "$MIN_MAX_WORKER_EFFICIENCY"   --output "$WORKER_OUTPUT" >/dev/null

python3 python/scale1_revalidate_scale_receipt.py archive   --input "$ARCHIVE_RECEIPT"   --max-work-units-per-token "$MAX_WORK_UNITS_PER_TOKEN"   --min-token-span-ratio "$MIN_TOKEN_SPAN_RATIO"   --output "$ARCHIVE_OUTPUT" >/dev/null

jq -e '
  .schema == "sensiblaw.scale1.worker-scaling-series.v0_2"
  and .acceptance.green == true
  and .acceptance.revalidated_from_retained_measurement == true
' "$WORKER_OUTPUT" >/dev/null

jq -e '
  .schema == "sensiblaw.scale1.archive-scale-series.v0_2"
  and .acceptance.green == true
  and .acceptance.revalidated_from_retained_measurement == true
' "$ARCHIVE_OUTPUT" >/dev/null

echo "SCALE1_RETAINED_SCALE_RECEIPTS_REVALIDATED"
echo "worker=$WORKER_OUTPUT"
echo "archive=$ARCHIVE_OUTPUT"
