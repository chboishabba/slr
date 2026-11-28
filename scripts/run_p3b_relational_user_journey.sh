#!/usr/bin/env bash
# Bounded acceptance on the operator's existing database. No source fabrication,
# no CI, no network acquisition. Old family and its immutable evidence stay intact.
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT="$PWD"
BIN="${SLR_NATIVE_EXECUTABLE:-$ROOT/target/debug/sensiblaw}"
OUT="${P3B_OUT:-$ROOT/artifacts/p3b-user-journey}"
LIMIT="${P3B_TIMEOUT_SECONDS:-90}"
ITIR_ROOT="${ITIR_ROOT:-}"
mkdir -p "$OUT"
[[ -x "$BIN" ]] || { echo "Build the current branch locally first: cargo build -p sensiblaw-cli --bin sensiblaw" >&2; exit 2; }
command -v jq >/dev/null
command -v timeout >/dev/null
seed="$ROOT/artifacts/p3b-live/selection-seed-run-parity.json"
old="$ROOT/artifacts/p3b-live/family-run-parity-fixed.json"
[[ -s "$seed" && -s "$old" ]]
left_batch="$(jq -er .left_batch_ref "$seed")"
right_batch="$(jq -er .right_batch_ref "$seed")"
left_selector="$(jq -er .left_selector_ref "$seed")"
right_selector="$(jq -er .right_selector_ref "$seed")"
left_pred="$(jq -er --arg s "$left_selector" '.projection.sources[] | select(.selector_ref==$s) | .observation.predicate_candidate_ref' "$old")"
right_pred="$(jq -er --arg s "$right_selector" '.projection.sources[] | select(.selector_ref==$s) | .observation.predicate_candidate_ref' "$old")"
old_family="$(jq -er .family_ref "$old")"
consumer="${P3B_CONSUMER_REF:-consumer:native-pnf-p3b-recorded-syntax-v2}"
access="${P3B_ACCESS_REF:-access:native-pnf-p3b}"
run() {
    local dest="$1"; shift
    # Never leave a failed/partial command under the successful receipt filename.
    rm -f "$OUT/$dest.tmp" "$OUT/$dest"
    timeout "$LIMIT" "$BIN" relational-family "$@" >"$OUT/$dest.tmp" 2>"$OUT/$dest.stderr"
    jq -e 'type=="object"' "$OUT/$dest.tmp" >/dev/null
    mv "$OUT/$dest.tmp" "$OUT/$dest"
}
# Validate the historical live reference before producing a new scoped family.
run previous-frontier.json inspect-research-frontier "$old_family"
jq -e '.candidate_only and (.writes_database==false)' "$OUT/previous-frontier.json" >/dev/null
# Recorded coordinate demand does not require seven unrelated context axes.
# This is an explicit new consumer, not a rewrite of the old semantic consumer.
run scoped-seed.json seed-pnf-pair-scoped "$consumer" "$left_batch" "$left_pred" "$right_batch" "$right_pred" '[]'
jq -e '(.requested_context_axes==[]) and (.source_metadata|length==2) and
       all(.source_metadata[]; .native_source_family=="document" and .source_family=="other")' "$OUT/scoped-seed.json" >/dev/null
left="$(jq -er .left_selector_ref "$OUT/scoped-seed.json")"
right="$(jq -er .right_selector_ref "$OUT/scoped-seed.json")"
run scoped-family.json "$consumer" "$access" "$left" "$right"
family="$(jq -er .family_ref "$OUT/scoped-family.json")"
run scoped-frontier.json inspect-research-frontier "$family"
run recorded-parity.json inspect-recorded-parity "$family"
jq -e '.all_recorded_answers_equal and (.pairs|length==1) and
       (.pairs[0].left_count==4) and (.pairs[0].right_count==4) and
       (.pairs[0].left_only==[]) and (.pairs[0].right_only==[]) and
       (.recorded_consumer_adequacy|length==2) and
       (.reviewed_representation_parity_paid==false) and (.world_truth_paid==false)' "$OUT/recorded-parity.json" >/dev/null
run local-research.json inspect-local-research "$family"
jq -e '.candidate_only and (.network_requests==0) and (.planned_moves==[]) and
       (.source_manifest|length==2) and
       ([.retrieval[].passages[].source_revision_ref]|unique|length==2)' "$OUT/local-research.json" >/dev/null

if [[ -n "${P3B_CALIBRATION_JSON:-}" ]]; then
    run calibrated-local-research.json plan-local-research "$family" "$P3B_CALIBRATION_JSON"
fi

# Optional actual heterogeneous overlap against original independent JMD receipt.
# All coordinates must be supplied from the preexisting original numeric owner.
heterogeneous=not_run_missing_operator_coordinates
if [[ -n "${P3B_JMD_RECEIPT:-}" ]]; then
    : "${P3B_BOOK_RUN:?}" "${P3B_BOOK_DOCUMENT:?}" "${P3B_BOOK_SNAPSHOT:?}" "${P3B_BOOK_PACKET_DIGEST:?}"
    run heterogeneous-overlap.json inspect-heterogeneous-overlap "$P3B_JMD_RECEIPT" "$consumer" "$P3B_BOOK_RUN" "$P3B_BOOK_DOCUMENT" "$P3B_BOOK_SNAPSHOT" "$P3B_BOOK_PACKET_DIGEST"
    jq -e '.candidate_only and (.same_manifestation_paid==false) and
        (.left_source.sha256!=.right_source.sha256) and
        (.left_members|length>0) and (.right_members|length>0)' "$OUT/heterogeneous-overlap.json" >/dev/null
    heterogeneous=bounded_candidate_overlap_replayed
fi

itir=not_run_missing_itir_root
if [[ -n "$ITIR_ROOT" ]]; then
    export ITIR_SLR_NATIVE_EXECUTABLE="$BIN"
    export PYTHONPATH="$ITIR_ROOT/itir-mcp/src${PYTHONPATH:+:$PYTHONPATH}"
    export P3B_FAMILY_REF="$family"
    timeout "$LIMIT" python3 - <<'PY' >"$OUT/itir-live.json.tmp"
import json, os
from itir_mcp.native_relational_tools import inspect_relational_family
results = {a: inspect_relational_family({"action": a, "family_ref": os.environ["P3B_FAMILY_REF"]})
           for a in ("research_frontier", "recorded_parity", "local_research")}
assert all(v["candidate_only"] and not v["mutates"] for v in results.values())
print(json.dumps(results, indent=2))
PY
    jq -e '(.recorded_parity.native_receipt.all_recorded_answers_equal==true) and
       (.local_research.native_receipt.network_requests==0)' "$OUT/itir-live.json.tmp" >/dev/null
    mv "$OUT/itir-live.json.tmp" "$OUT/itir-live.json"
    itir=passed
fi
jq -n --arg family "$family" --arg itir "$itir" --arg heterogeneous "$heterogeneous" \
    --arg commit "$(git rev-parse HEAD)" \
    '{schema:"slr.p3b-user-journey-replay.v1",commit:$commit,family_ref:$family,
      bounded_recorded_parity:"passed",local_candidate_retrieval:"passed",
      itir_live:$itir,heterogeneous_overlap:$heterogeneous,
      calibrated_planning_required_for_full_journey:true,governed_acquisition_reentry_paid:false,
      independent_document_consumer_equivalence_paid:false,full_user_story_acceptance:false}' >"$OUT/acceptance.json"
cat "$OUT/acceptance.json"
# HOLD is deliberate: this is a bounded candidate journey, not all J01-J11.
exit 3
