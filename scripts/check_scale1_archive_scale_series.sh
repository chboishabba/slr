#!/usr/bin/env bash
set -euo pipefail

: "${MAX_WORK_UNITS_PER_TOKEN:?MAX_WORK_UNITS_PER_TOKEN is required}"
: "${RECEIPTS:?RECEIPTS must be a colon-separated list of fresh compiler receipts}"

OUTPUT="${OUTPUT:-/tmp/scale1-archive-scale-series.json}"
RUNTIME_HEAD="${SENSIBLAW_RUNTIME_HEAD:-$(git rev-parse HEAD)}"

args=()
IFS=':' read -r -a receipt_paths <<< "$RECEIPTS"
for receipt in "${receipt_paths[@]}"; do
  args+=(--receipt "$receipt")
done

python3 python/scale1_archive_scale_series.py   "${args[@]}"   --output "$OUTPUT" >/dev/null

jq -e   --arg head "$RUNTIME_HEAD"   --argjson max_slope "$MAX_WORK_UNITS_PER_TOKEN" '
  .schema == "sensiblaw.scale1.archive-scale-series.v0_1"
  and .runtime_head == $head
  and .represented_carrier == "parser_tokens"
  and (.points | length) >= 2
  and .observed_affine_envelope.all_points_within == true
  and .observed_affine_envelope.slope_work_units_per_token <= $max_slope
  and all(.points[];
      .represented_tokens > 0
      and .semantic_regions > 0
      and .measured_elapsed_ns > 0
      and .measured_post_parser_elapsed_ns >= 0
      and .within_declared_work_envelope == true)
  and .boundary.creates_semantic_authority == false
  and .boundary.claim_truth_promoted == false
' "$OUTPUT"

echo "SCALE1_ARCHIVE_SCALE_SERIES_GREEN"
jq '{
  runtime_head,
  parser_model_ref,
  represented_carrier,
  work_unit_definition,
  observed_affine_envelope,
  points: [
    .points[] | {
      source_family,
      source_revision_ref,
      represented_tokens,
      semantic_regions,
      measured_post_parser_work_units,
      measured_elapsed_ns,
      measured_post_parser_elapsed_ns,
      wall_ns_per_token,
      post_parser_ns_per_token,
      parser_tail,
      review_work
    }
  ]
}' "$OUTPUT"
