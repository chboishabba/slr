#!/usr/bin/env bash
# Integrated source/identity candidate campaign. No prototype development, CI,
# implicit world admission, or acceptance from a source-written proof alone.
set -euo pipefail
if [[ $# != 2 ]]; then
  echo "usage: $0 EXACT_S29_REVIEW_ITEM_REF ORIGINAL_PARSER_RUN_REF" >&2; exit 2
fi
review=$1
run=$2
: "${DASHI_LEAN_ROOT:?existing Lean #112 checkout required}"
: "${SLR_NATIVE_NUMERIC_SOURCE_ROOT:?original numeric source root required}"
: "${SENSIBLAW_DATABASE_URL:?original numeric DB required}"
command -v jq >/dev/null
command -v timeout >/dev/null
root=$(git rev-parse --show-toplevel)
cd "$root"
head=$(git rev-parse HEAD)
lean_head=$(git -C "$DASHI_LEAN_ROOT" rev-parse HEAD)
checkpoint=".native-v8-endpoint-checkpoint"
mkdir -p "$checkpoint"
manifest=$(jq -cn --arg review "$review" --arg run "$run" --arg slr "$head" --arg lean "$lean_head"   '{review:$review,run:$run,slr_head:$slr,lean_head:$lean}')
if [[ -f "$checkpoint/manifest.json" ]]; then
  jq -e --argjson m "$manifest" '.==$m' "$checkpoint/manifest.json" >/dev/null
else
  printf '%s\n' "$manifest" > "$checkpoint/manifest.json.tmp"
  mv "$checkpoint/manifest.json.tmp" "$checkpoint/manifest.json"
fi
export SLR_NATIVE_LEAN_WORKSET_EXECUTABLE
SLR_NATIVE_LEAN_WORKSET_EXECUTABLE=$(realpath "$DASHI_LEAN_ROOT/scripts/produce_native_pnf_workset.sh")
export SLR_NATIVE_BOUNDED_IDENTITY_PROBE=1
# Keep all individual work attempts bounded and resume from durable source
# and numeric publication owners, not an assumed successful partial JSON.
bounded() { timeout --kill-after=5s "${SLR_NATIVE_STEP_TIMEOUT:-90s}" "$@"; }
capture() {
  local dest=$1; shift
  bounded "$@" > "$dest.tmp"
  jq -e 'type=="object"' "$dest.tmp" >/dev/null
  mv "$dest.tmp" "$dest"
}
bounded "$SLR_NATIVE_LEAN_WORKSET_EXECUTABLE" --build >&2
bounded bash "$DASHI_LEAN_ROOT/scripts/check_native_typed_evidence_golden.sh" >&2
bounded cargo test -p sensiblaw-pg-source-store --lib numeric_pnf >&2
bounded cargo test -p sensiblaw-cli numeric_pnf_native >&2
bounded cargo build -p sensiblaw-cli >&2
bounded cargo build -p sensiblaw-pg-source-store --example scale1_long_document >&2
cli=(target/debug/sensiblaw numeric-pnf-native)
# Use the existing source parser worker only; this script does not extend its
# historical Python parser adapter. No unrelated parser run is drained.
worker=(target/debug/examples/scale1_long_document worker "$run" native-prefix-v8 32 "${SLR_PREFIX_PARSER_SCRIPT:-scripts/scale1_spacy_json_parser.py}")
ready=false
for pass in {1..8}; do
  capture "$checkpoint/prepare-$pass.json" "${cli[@]}" prefix-prepare "$review" "$run" --commit
  capture "$checkpoint/inspect-$pass.json" "${cli[@]}" prefix-prepare "$review" "$run" --inspect
  scope="$checkpoint/inspect-$pass.json"
  if jq -e '.parser_scope_ready and .source_region_prefix_covered and .original_parser_token_prefix_covered' "$scope" >/dev/null; then
    cp "$scope" "$checkpoint/ready.json"; ready=true; break
  fi
  if jq -e '.parser_scope_ready and ([.original_gap_paragraph_receipts[] | select(.original_heading_exclusion_matches and .nonempty_line_count>1)] | length>0)' "$scope" >/dev/null; then
    capture "$checkpoint/repair-$pass.json" "${cli[@]}" prefix-repair "$review" "$run" --commit
    jq -e '.candidate_only and (.typed_coverage_paid|not) and (.added_region_refs|length>0)' "$checkpoint/repair-$pass.json" >/dev/null
    continue
  fi
  if jq -e '.newly_enqueued_or_pending>0' "$scope" >/dev/null; then
    capture "$checkpoint/worker-$pass.json" "${worker[@]}"
  elif jq -e '(.missing_region_refs|length)>0' "$scope" >/dev/null; then
    continue
  else
    echo "Unresolved original source coverage; inspect $scope" >&2; exit 3
  fi
done
[[ "$ready" == true ]] || { echo "Prefix work still pending; bounded attempt stopped" >&2; exit 3; }
# Reopen full receipts rather than silently skipping the parity probes.
export SLR_NATIVE_IDENTITY_REPLAY_MODE=0
export SLR_NATIVE_ENDPOINT_REPLAY_MODE=0
capture "$checkpoint/cold.json" "${cli[@]}" endpoint "$review" "$run" --commit
# Actual represented H3 projection negative control; no fabricated fixture,
# no persisted evidence, no authority write. This is not owner revalidation.
command -v psql >/dev/null
demand=$(jq -er '.binding_passes[-1].demand_id | select(type=="number")' "$checkpoint/cold.json")
consumer=$(jq -er '.binding_passes[-1].consumer_horizon_work.consumer_ref' "$checkpoint/cold.json")
query=$(jq -er '.binding_passes[-1].consumer_horizon_work.query_ref' "$checkpoint/cold.json")
policy=$(jq -er '.binding_passes[-1].consumer_horizon_work.policy_ref' "$checkpoint/cold.json")
bounded psql "$SENSIBLAW_DATABASE_URL" -X -v "demand_id=$demand" \
  -v "consumer_ref=$consumer" -v "query_ref=$query" -v "policy_ref=$policy" \
  -f scripts/check_native_h6_producer_parity.sql > "$checkpoint/h6-producer-parity-rollback.log" 2>&1
bounded psql "$SENSIBLAW_DATABASE_URL" -X -v "demand_id=$demand" \
  -f scripts/check_native_horizon_withdrawal.sql > "$checkpoint/h3-withdrawal-rollback.log"
export SLR_NATIVE_IDENTITY_REPLAY_MODE=1
export SLR_NATIVE_ENDPOINT_REPLAY_MODE=1
# Install the explicit replay fence and obtain a quiescent full readback
# before attempting a no-observation hit. Missing/changed state cannot seed.
for attempt in {1..4}; do
  capture "$checkpoint/replay-$attempt.json" "${cli[@]}" endpoint "$review" "$run" --commit
  cp "$checkpoint/replay-$attempt.json" "$checkpoint/replay.json"
  if jq -e '.endpoint_execution_replay.hit==true' "$checkpoint/replay.json" >/dev/null; then
    break
  fi
done
jq -s '
  def binding: .binding_passes[-1];
  def candidate_gate:
    .prefix_scope.parser_scope_ready and .prefix_scope.source_region_prefix_covered and
    .prefix_scope.original_parser_token_prefix_covered and .observed_prefix_token_coverage and
    .projected_vs_original_entity_membership_parity and .exact_entity_inverse_complete and
    (.numeric_publication[-1].exact_parser_sentence_receipts==true) and
    (all(.numeric_publication[]; .lean_workset_checked and
      ((.newly_registered_sentence_refs-.missing_required_sentence_refs)|length)==0)) and
    (binding | .lean_typed_evidence != null and
      .consumer_horizon_work.h6_validity.original_consumer_authority_reconciled and
      (.consumer_horizon_work.nonmonotone_input_residual==false) and
      (.consumer_horizon_work.h6_validity.durable_nonmonotone_authority_pending==false) and
      .original_parser_identity_producer.support_index.delta_reference_exact and
      .original_parser_identity_producer.bounded_identity_parity.candidate_projection_exact and
      .original_parser_identity_producer.bounded_identity_parity.original_admission_projection_exact and
      (.original_parser_identity_producer.scoped_admission_parity as $p |
       any($p[]; .scenario=="empty-workset" and (.affected_factor_ids|length)==0) and
       any($p[]; .scenario=="agda-standalone-source-correction" and .golden_producer_exact) and
       any($p[]; .scenario=="direct-withdrawal" and .withdrawn_direct_candidates>0 and
          .changed_witness_keys>0 and (.changed_level_three_refs|length)>0) and
       all($p[]; .reference_admission_exact and .reference_level_three_exact and
          .unrelated_admission_revisions_preserved and .unrelated_derivation_ids_preserved and
          .immutable_level_one_preserved and .rollback_restored and (.persisted|not))));
  def identity_replay_gate: (binding | .identity_execution_replay.reference_baseline_present==true and
      .identity_execution_replay.input_events_pending==0 and .identity_execution_replay.source_support_scan_performed==false and
      .parser_identity_refresh_invocations==0 and .consumer_horizon_work.h6_producer_invocations==0);
  def warm_gate:
    (.endpoint_execution_replay |
      .hit==true and .source_observation_scan_performed==false and
      .external_canonical_source_bytes_verified==true and
      .numeric_binding_scan_performed==false and .parser_identity_refresh_invocations==0 and
      .semantic_recomputation_performed==false and .writes_semantic_state==false and
      .current_authority_reused==false);
  def semantic_gate: .governed_identity_paid and .level_three_paid and
      (binding | .complete_typed_coverage_paid and (.outside_model_possible|not)
        and .native_binding_bridge.structural_relation_parity_paid);
  {schema:"slr-native-antecedent-endpoint-v8",
   cold:.[0], replay:.[1],
   candidate_reference_parity_paid:(try (.[0]|candidate_gate) catch false),
   replay_candidate_reference_parity_observed:(try (.[1]|candidate_gate) catch false),
   exact_replayed_source_answer:(.[0].source_local_answer!=null and
      .[0].source_local_answer==.[1].source_local_answer),
   exact_replayed_persisted_fibre:(.[0].binding_passes[-1].original_horizon_observations!=null and
      .[0].binding_passes[-1].original_horizon_observations==.[1].binding_passes[-1].original_horizon_observations and
      .[0].binding_passes[-1].original_document_identity_observations==.[1].binding_passes[-1].original_document_identity_observations),
   identity_input_zero_refresh_paid:(try (.[1]|identity_replay_gate) catch false),
   warm_zero_semantic_work_paid:(try (.[1]|warm_gate) catch false),
   semantic_endpoint_paid:(try (.[1]|semantic_gate) catch false),
   external_qid_paid:false,historical_truth_paid:false}
  | . + {integrated_acceptance_paid:(.candidate_reference_parity_paid and
      .exact_replayed_source_answer and .exact_replayed_persisted_fibre and .warm_zero_semantic_work_paid and .semantic_endpoint_paid)}
' "$checkpoint/cold.json" "$checkpoint/replay.json" > "$checkpoint/acceptance.json.tmp"
mv "$checkpoint/acceptance.json.tmp" "$checkpoint/acceptance.json"
jq '{schema,candidate_reference_parity_paid,replay_candidate_reference_parity_observed,exact_replayed_source_answer,exact_replayed_persisted_fibre,identity_input_zero_refresh_paid,warm_zero_semantic_work_paid,semantic_endpoint_paid,integrated_acceptance_paid}' "$checkpoint/acceptance.json"
jq -e '.integrated_acceptance_paid==true' "$checkpoint/acceptance.json" >/dev/null
