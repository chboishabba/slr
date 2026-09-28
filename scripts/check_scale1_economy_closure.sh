#!/usr/bin/env bash
set -euo pipefail

: "${SAME_RUN_RECEIPT:?SAME_RUN_RECEIPT is required}"
: "${SMALL_EDIT_RECEIPT:?SMALL_EDIT_RECEIPT is required}"
: "${SAME_DOMAIN_RECEIPT:?SAME_DOMAIN_RECEIPT is required}"

OUTPUT="${OUTPUT:-/tmp/scale1-economy-closure.json}"

python3 python/scale1_economy_closure.py   --same-run "$SAME_RUN_RECEIPT"   --small-edit "$SMALL_EDIT_RECEIPT"   --same-domain "$SAME_DOMAIN_RECEIPT"   --output "$OUTPUT"

jq -e '
  .schema == "sensiblaw.scale1.economy-closure.v0_1"
  and .same_run_parser_dominance.green == true
  and .small_edit_locality.green == true
  and .same_domain_new_source.green == true
  and .boundary.candidate_only == true
  and .boundary.creates_semantic_authority == false
  and .boundary.applicability_promoted == false
  and .boundary.claim_truth_promoted == false
  and .scale1_economy_closed == true
' "$OUTPUT"

echo "SCALE1_ECONOMY_CLOSURE_GREEN"
