#!/usr/bin/env bash
# Continue the existing Lean GWB multiedition replay using its exact original
# receipts and the selected native source/packet coordinates. No new parser,
# no synthetic equivalence and no automatic formal evidence payment.
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT="$PWD"
BIN="${SLR_NATIVE_EXECUTABLE:-$ROOT/target/debug/sensiblaw}"
OUT="${GWB_MULTIVIEW_OUT:-$ROOT/artifacts/gwb/multiview-native}"
LIMIT="${GWB_TIMEOUT_SECONDS:-90}"
: "${GWB_NATIVE_VIEW_INPUTS_JSON:?Supply exact indexed receipt/member and native chart coordinates}"
: "${GWB_VIEW_QUERY_REF:?Explicit typed consumer query coordinate required}"
: "${GWB_VIEW_CONSUMER_REF:?Explicit existing consumer scope or fresh consumer coordinate required}"
[[ -x "$BIN" ]]
command -v jq >/dev/null
command -v timeout >/dev/null
mkdir -p "$OUT"
rm -f "$OUT/selected.json" "$OUT/journey.json" "$OUT/proof-cohort.json" "$OUT/selected.jsonl"
jq -e 'type=="array" and length>=3 and length<=8 and
  all(.[]; ([.receipt_path,.indexed_member_ref,.revision,.run,.document,.snapshot,.packet_digest]
    |all(.[];type=="string" and length>0)) and
    (.batches|type=="array" and length>0 and length<=4096))' "$GWB_NATIVE_VIEW_INPUTS_JSON" >/dev/null
invoke() {
  local dest="$1"; shift
  rm -f "$OUT/$dest" "$OUT/$dest.tmp"
  timeout --kill-after=5s "$LIMIT" "$BIN" relational-family "$@" >"$OUT/$dest.tmp" 2>"$OUT/$dest.stderr"
  jq -e 'type=="object"' "$OUT/$dest.tmp" >/dev/null
  mv "$OUT/$dest.tmp" "$OUT/$dest"
}
i=0
while IFS= read -r row; do
  receipt="$(jq -er .receipt_path <<<"$row")"
  member="$(jq -er .indexed_member_ref <<<"$row")"
  revision="$(jq -er .revision <<<"$row")"
  run="$(jq -er .run <<<"$row")"
  doc="$(jq -er .document <<<"$row")"
  snapshot="$(jq -er .snapshot <<<"$row")"
  digest="$(jq -er .packet_digest <<<"$row")"
  batches="$(jq -c .batches <<<"$row")"
  edition="$(jq -er .edition <<<"$row")"
  if [[ "$revision" == source-revision:sha256:* ]]; then
    bound="$OUT/view-$i-bound-receipt.json"
    python3 tools/bind_gwb_native_receipt.py --edition "$edition" --receipt "$receipt" --output "$bound"
    receipt="$bound"
  fi
  if [[ "$digest" == native-numeric-chart:missing || "$snapshot" == native-numeric-chart:missing ]]; then
    echo "HOLD: exact native numeric chart missing for $edition" >&2
    exit 3
  fi
  jq -e --arg revision "$revision" --arg edition "$edition" '
    .qid=="Q207" and .inventory.article_revision==$revision and .source_observation_checked and
    (.world_truth_paid==false) and
    (if .schema=="slr.jmd-indexed-world-fibre-receipt.v3" then .edition==$edition
      elif .schema=="slr.jmd-indexed-world-fibre-receipt.v1" then $edition=="en"
      else .edition==$edition end)' "$receipt" >/dev/null
  invoke "view-$i-meet.json" inspect-indexed-native-meet "$receipt" "$GWB_VIEW_CONSUMER_REF" \
    "$revision" "$run" "$doc" "$snapshot" "$digest" "$batches"
  # Select only a uniquely reopened checked inverse of the explicitly
  # nominated indexed occurrence. A numeric-only row cannot seed this.
  if ! jq -e --arg member "$member" --arg edition "$edition" '
    . as $m |
    [.coordinates[] | select(any(.indexed[];.request_ref==$member or .original.event_candidate_ref==$member))] |
    if length!=1 then error("held: indexed occurrence unavailable/ambiguous") else .[0] end |
    if .status!="unique_coordinate_match" or (.native|length)!=1 or
       .native[0].kind!="checked_predicate" then error("held: checked inverse unavailable/ambiguous")
    else {edition:$edition,indexed_member_ref:$member,source_revision_ref:$m.source_revision_ref,
        content_digest_ref:$m.source_content_digest_ref,batch_ref:.native[0].original.batch_ref,
        predicate_candidate_ref:.native[0].original.checked_observation.predicate_candidate_ref}
    end' "$OUT/view-$i-meet.json" >"$OUT/view-$i-selected.json.tmp"; then
    echo "HOLD: original checked predicate inverse missing or ambiguous for $edition" >&2
    exit 3
  fi
  mv "$OUT/view-$i-selected.json.tmp" "$OUT/view-$i-selected.json"
  cat "$OUT/view-$i-selected.json" | jq -c . >>"$OUT/selected.jsonl"
  i=$((i+1))
done < <(jq -c '.[]' "$GWB_NATIVE_VIEW_INPUTS_JSON")
jq -se 'length>=3 and (map(.source_revision_ref)|unique|length)==length and
  any(.[];.edition=="en") and any(.[];.edition=="simple") and
  any(.[];.edition!="en" and .edition!="simple" and .edition!="abstract")' "$OUT/selected.jsonl" >/dev/null
jq -s 'map(del(.edition,.indexed_member_ref))' "$OUT/selected.jsonl" >"$OUT/selected.json"
if [[ "${1:-}" != "--commit" ]]; then
  echo "Candidate checked inverses ready; explicit --commit and GWB_VIEW_ACCESS_REF needed."
  exit 3
fi
: "${GWB_VIEW_ACCESS_REF:?Explicit access constraint required}"
invoke journey.json run-pnf-cohort "$GWB_VIEW_CONSUMER_REF" "$GWB_VIEW_ACCESS_REF" "$OUT/selected.json" \
  '["scope","time","modality","quantifier","attribution","ontology","language"]' --commit
family="$(jq -er .family.family_ref "$OUT/journey.json")"
evidence='[]'
if [[ -n "${GWB_VIEW_PAIR_EVIDENCE_JSON:-}" ]]; then
  evidence="$(jq -ce 'type as $t | if $t=="array" then . else error("expected exact evidence locators") end' "$GWB_VIEW_PAIR_EVIDENCE_JSON")"
fi
invoke proof-cohort.json inspect-licensed-view-cohort "$family" "$GWB_VIEW_QUERY_REF" "$evidence"
jq -e '.schema=="slr.authenticated-view-cohort.v1" and
  (.world_truth_paid==false) and (.derives_transitive_licences==false)' "$OUT/proof-cohort.json" >/dev/null
# Context capsule acceptance applies even if proof pairs are held.
invoke context-capsule.json inspect-query-context-capsule "$family" "$GWB_VIEW_QUERY_REF" "$evidence"
invoke context-capsule-repeat.json inspect-query-context-capsule "$family" "$GWB_VIEW_QUERY_REF" "$evidence"
cmp "$OUT/context-capsule.json" "$OUT/context-capsule-repeat.json"
cap_digest="$(jq -er .capsule_digest "$OUT/context-capsule.json")"
invoke context-expanded.json expand-query-context-capsule "$family" "$GWB_VIEW_QUERY_REF" "$evidence" "$cap_digest"
jq -e --arg digest "$cap_digest" '.exact_current_replay and .capsule_digest==$digest and
  (.world_truth_paid // false | not) and (.writes_database==false)' "$OUT/context-expanded.json" >/dev/null
# Compare full readback and exact residual inverse, not just counts.
jq -e --slurpfile proof "$OUT/proof-cohort.json" --slurpfile journey "$OUT/journey.json" '
  .cohort==$proof[0] and
  (.family.family_ref==$journey[0].family.family_ref) and
  ((.capsule_envelope.capsule.residuals|map(.residual_ref)|sort)==
   ($journey[0].investigation.gap_rows|map(.residual_ref)|sort))' "$OUT/context-expanded.json" >/dev/null
wrong="sha256:$(printf '%064d' 0)"
rm -f "$OUT/context-stale.json"
if timeout --kill-after=5s "$LIMIT" "$BIN" relational-family expand-query-context-capsule \
  "$family" "$GWB_VIEW_QUERY_REF" "$evidence" "$wrong" >"$OUT/context-stale.tmp" 2>"$OUT/context-stale.stderr"; then
  echo "UNSOUND: accepted wrong context digest" >&2; exit 1
fi
[[ ! -s "$OUT/context-stale.tmp" ]] || { echo "Rejected expansion emitted result" >&2; exit 1; }
jq '{full_readback_json_bytes,capsule_body_json_bytes,body_smaller_than_full_readback,measurement}' "$OUT/context-capsule.json"
if ! jq -e '.all_pairs_bounded_signed_support_equal' "$OUT/proof-cohort.json" >/dev/null; then
  echo "GWB bounded cohort reopened; missing/different/unavailable authenticated answers retained."
  exit 3
fi
echo "GWB bounded signed support parity for this explicit query only; source independence, all-live coverage and whole programme remain separate."
