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
checkpoint="${SLR_NATIVE_V8_CHECKPOINT:-.native-v8-endpoint-maxcut-checkpoint}"
mkdir -p "$checkpoint"
command -v sha256sum >/dev/null
reference_digest() {
  if [[ -n "${SLR_NATIVE_ORIGINAL_RELATION_REFERENCE:-}" ]]; then
    sha256sum -- "$SLR_NATIVE_ORIGINAL_RELATION_REFERENCE" | cut -d' ' -f1
  else
    printf '%s\n' absent
  fi
}
reference_sha=$(reference_digest)
manifest=$(jq -cn --arg review "$review" --arg run "$run" --arg slr "$head" --arg lean "$lean_head" --arg reference "$reference_sha" \
  '{review:$review,run:$run,slr_head:$slr,lean_head:$lean,original_relation_reference_sha256:$reference}')
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
# Keep the real first failing kernel diagnostic and exact checkout rather
# than replacing it with a generic failed endpoint or stale proof receipt.
# Each module has its own 90s bound inside --build; the outer bound must
# permit both independently bounded invocations.
if ! timeout --kill-after=5s "${SLR_NATIVE_FORMAL_BUILD_TIMEOUT:-200s}" "$SLR_NATIVE_LEAN_WORKSET_EXECUTABLE" --build 2>&1 | tee "$checkpoint/lean-build.log" >&2; then
  echo "Formal build blocked; inspect $checkpoint/lean-build.log" >&2
  exit 3
fi
bounded bash "$DASHI_LEAN_ROOT/scripts/check_native_typed_evidence_golden.sh" 2>&1 | tee "$checkpoint/lean-typed-evidence.log" >&2
bounded cargo test -p sensiblaw-pg-source-store --lib numeric_pnf >&2
bounded cargo test -p sensiblaw-cli numeric_pnf_native >&2
bounded cargo build -p sensiblaw-cli >&2
bounded cargo build -p sensiblaw-pg-source-store --example scale1_long_document >&2
cli=(target/debug/sensiblaw numeric-pnf-native)
# Bind actual executable/checked proof bytes as well as checkout heads.
execution_sha=$(sha256sum target/debug/sensiblaw \
  "$SLR_NATIVE_LEAN_WORKSET_EXECUTABLE" "$DASHI_LEAN_ROOT/lean-toolchain" \
  "$DASHI_LEAN_ROOT/tools/NativePnfWorkset.lean" \
  "$DASHI_LEAN_ROOT/DASHI/output-final_aristotle/RequestProject/Slr/NumericPnfRustPortContract.lean" \
  "$DASHI_LEAN_ROOT/DASHI/output-final_aristotle/RequestProject/Slr/GwbCoreferenceFormalRepair.lean" \
  "$DASHI_LEAN_ROOT/.lake/build/lib/lean/RequestProject/Slr/NumericPnfRustPortContract.olean" \
  "$DASHI_LEAN_ROOT/.lake/build/lib/lean/RequestProject/Slr/GwbCoreferenceFormalRepair.olean" \
  | cut -d' ' -f1 | sha256sum | cut -d' ' -f1)
scope_of() {
  jq -ce --argjson manifest "$manifest" --arg execution "$execution_sha" '
    select(.prefix_scope.parser_run_ref==$manifest.run and
        .source_local_answer.original_parser_run_ref==$manifest.run and
        .source_local_answer.original_review_item_ref==$manifest.review and
        .numeric_publication[-1].original_parser_run_ref==$manifest.run and
        .prefix_scope.source_revision_ref==.source_local_answer.source_revision_ref and
        .numeric_publication[-1].original_source_revision_ref==.source_local_answer.source_revision_ref and
        .prefix_scope.source_sha256==.source_local_answer.source_content_sha256 and
        .numeric_publication[-1].original_source_sha256==.source_local_answer.source_content_sha256 and
        .prefix_scope.focus_start_char==.source_local_answer.original_focus_span[0])
    | {manifest:$manifest,execution_sha256:$execution,
        source_revision_ref:.source_local_answer.source_revision_ref,
        source_sha256:.source_local_answer.source_content_sha256,
        source_document_ref:.source_local_answer.source_document_ref,
        focus_span:.source_local_answer.original_focus_span,
        numeric_run_ref:.numeric_publication[-1].numeric_run_ref}
    | select([.source_revision_ref,.source_sha256,.source_document_ref,.numeric_run_ref]
        | all(.[]; type=="string" and length>0))
    | select(.focus_span | type=="array" and length==2 and
        all(.[]; type=="number") and .[0]>=0 and .[1]>.[0])
  ' "$1"
}
check_reference() {
  [[ "$(reference_digest)" == "$reference_sha" ]] || {
    echo "Original relation comparator changed during campaign" >&2; exit 3;
  }
}
publish_closed() {
  local packet=$1 scope
  check_reference
  scope=$(scope_of "$packet")
  # Historical execution cannot pay current authority. Keep its receipt.
  local candidate="$checkpoint/candidate-acceptance.json"
  local candidate_match=false
  if [[ -f "$candidate" ]] && jq -e --argjson scope "$scope" '
      .campaign=="candidate_execution" and
      .candidate_execution_acceptance_paid==true and .exact_scope==$scope
    ' "$candidate" >/dev/null; then
    candidate_match=true
  fi
  jq --argjson scope "$scope" --argjson candidate "$candidate_match" '{
    schema:"slr-native-antecedent-endpoint-v8",
    campaign:"closed_original_authority", exact_scope:$scope,
    candidate_execution_acceptance_paid:false,
    historical_candidate_execution_scope_matched:$candidate,
    historical_candidate_execution_is_current_authority:false,
    current_owner_proof_authenticated:(.original_focus_certificate.independent_current_owner_authenticated==true),
    governed_focus_authority_paid:(.governed_identity_paid==true and .level_three_paid==true and
      .original_focus_certificate.independent_current_owner_authenticated==true and
      .original_focus_certificate.rollback_readback_exact==true and
      .original_focus_certificate.original.resolved_unique_target_from_original_sql==true),
    full_relation_parity_paid:(.full_relation_parity_paid==true),
    external_qid_paid:false,historical_truth_paid:false,
    original_focus_certificate
  } | . + {integrated_acceptance_paid:(.historical_candidate_execution_scope_matched and
       .governed_focus_authority_paid and .full_relation_parity_paid)}' "$packet" > "$checkpoint/closed-acceptance.json.tmp"
  mv "$checkpoint/closed-acceptance.json.tmp" "$checkpoint/closed-acceptance.json"
  cp "$checkpoint/closed-acceptance.json" "$checkpoint/acceptance.json.tmp"
  mv "$checkpoint/acceptance.json.tmp" "$checkpoint/acceptance.json"
  jq 'del(.original_focus_certificate)' "$checkpoint/acceptance.json"
  if jq -e '.integrated_acceptance_paid==true' "$checkpoint/acceptance.json" >/dev/null; then
    exit 0
  fi
  exit 3
}
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
# Closed original authority and candidate-only execution are different
# branches. Never index nonexistent binding passes or require a closed
# answer to hit a candidate-only cache. Publish the actual separate gate.
if jq -e '.original_focus_certificate != null' "$checkpoint/cold.json" >/dev/null; then
  # Use the existing explicit replay installation path once if the
  # closed owner readback has no database continuity fence yet.
  if jq -e '.original_focus_certificate.independent_current_owner_binding==null' "$checkpoint/cold.json" >/dev/null; then
    export SLR_NATIVE_IDENTITY_REPLAY_MODE=1
    export SLR_NATIVE_ENDPOINT_REPLAY_MODE=1
    capture "$checkpoint/closed.json" "${cli[@]}" endpoint "$review" "$run" --commit
    if jq -e '.original_focus_certificate!=null' "$checkpoint/closed.json" >/dev/null; then
      publish_closed "$checkpoint/closed.json"
    fi
    echo "Closed focus changed before current owner readback; no authority paid" >&2
    exit 3
  fi
  publish_closed "$checkpoint/cold.json"
fi
if ! jq -e '.outcome=="source_local_answer_with_governed_identity_unresolved" and
    (.binding_passes | type=="array" and length>0)' "$checkpoint/cold.json" >/dev/null; then
  echo "Candidate source/producer prerequisites unresolved; inspect cold.json" >&2
  exit 3
fi
# Actual represented H3 projection negative control; no fabricated fixture,
# no persisted evidence, no authority write. This is not owner revalidation.
command -v psql >/dev/null
demand=$(jq -er '.binding_passes[-1].demand_id | select(type=="number")' "$checkpoint/cold.json")
consumer=$(jq -er '.binding_passes[-1].consumer_horizon_work.consumer_ref' "$checkpoint/cold.json")
query=$(jq -er '.binding_passes[-1].consumer_horizon_work.query_ref' "$checkpoint/cold.json")
policy=$(jq -er '.binding_passes[-1].consumer_horizon_work.policy_ref' "$checkpoint/cold.json")
bounded psql "$SENSIBLAW_DATABASE_URL" -X -v ON_ERROR_STOP=1 -v "demand_id=$demand" \
  -v "consumer_ref=$consumer" -v "query_ref=$query" -v "policy_ref=$policy" \
  -f scripts/check_native_h6_producer_parity.sql > "$checkpoint/h6-producer-parity-rollback.log" 2>&1
bounded psql "$SENSIBLAW_DATABASE_URL" -X -v ON_ERROR_STOP=1 -v "demand_id=$demand" \
  -f scripts/check_native_horizon_withdrawal.sql > "$checkpoint/h3-withdrawal-rollback.log"
export SLR_NATIVE_IDENTITY_REPLAY_MODE=1
export SLR_NATIVE_ENDPOINT_REPLAY_MODE=1
# Install the explicit replay fence and obtain a quiescent full readback
# before attempting a no-observation hit. Missing/changed state cannot seed.
for attempt in {1..4}; do
  capture "$checkpoint/replay-$attempt.json" "${cli[@]}" endpoint "$review" "$run" --commit
  cp "$checkpoint/replay-$attempt.json" "$checkpoint/replay.json"
  if jq -e '.original_focus_certificate!=null' "$checkpoint/replay.json" >/dev/null; then
    publish_closed "$checkpoint/replay.json"
  fi
  if jq -e '.endpoint_execution_replay.hit==true' "$checkpoint/replay.json" >/dev/null; then
    break
  fi
done
check_reference
cold_scope=$(scope_of "$checkpoint/cold.json")
replay_scope=$(scope_of "$checkpoint/replay.json")
[[ "$cold_scope" == "$replay_scope" ]] || { echo "Cold/warm scope mismatch" >&2; exit 3; }
jq -s --argjson scope "$cold_scope" '
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
  def candidate_only: .outcome=="source_local_answer_with_governed_identity_unresolved" and
      .governed_identity_paid==false and .level_three_paid==false and
      (binding | .outside_model_possible==true and .complete_typed_coverage_paid==false);
  {schema:"slr-native-antecedent-endpoint-v8",
   campaign:"candidate_execution",exact_scope:$scope,
   both_candidate_only:(all(.[]; candidate_only)),
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
   semantic_endpoint_paid:false,
   supplied_prefix_relation_parity_paid:(.[0].binding_passes[-1].native_binding_bridge.original_relation_packet.complete_supplied_prefix_relation_parity==true),
   source_parser_leaf_scope_used:(.[1].endpoint_execution_replay.source_parser_leaf_scope_used==true),
   external_qid_paid:false,historical_truth_paid:false}
  | . + {candidate_execution_acceptance_paid:(.both_candidate_only and .candidate_reference_parity_paid and
      .exact_replayed_source_answer and .exact_replayed_persisted_fibre and .warm_zero_semantic_work_paid)}
  | . + {integrated_acceptance_paid:false}
' "$checkpoint/cold.json" "$checkpoint/replay.json" > "$checkpoint/acceptance.json.tmp"
mv "$checkpoint/acceptance.json.tmp" "$checkpoint/acceptance.json"
jq '{schema,candidate_reference_parity_paid,replay_candidate_reference_parity_observed,exact_replayed_source_answer,exact_replayed_persisted_fibre,identity_input_zero_refresh_paid,warm_zero_semantic_work_paid,candidate_execution_acceptance_paid,supplied_prefix_relation_parity_paid,source_parser_leaf_scope_used,semantic_endpoint_paid,integrated_acceptance_paid}' "$checkpoint/acceptance.json"
# Candidate-only success is independently durable; return incomplete
# for the integrated campaign, not a misleading fully accepted result.
if jq -e '.candidate_execution_acceptance_paid==true' "$checkpoint/acceptance.json" >/dev/null; then
  cp "$checkpoint/acceptance.json" "$checkpoint/candidate-acceptance.json.tmp"
  mv "$checkpoint/candidate-acceptance.json.tmp" "$checkpoint/candidate-acceptance.json"
  exit 3
fi
exit 1
