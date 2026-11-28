#!/usr/bin/env bash
# Original Q207 indexed JMD receipt -> existing book packet -> selected
# native source inverses -> existing family/ITIR investigation. No reparse,
# no invented source IDs, no external acquisition, no proof promotion.
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT="$PWD"
BIN="${SLR_NATIVE_EXECUTABLE:-$ROOT/target/debug/sensiblaw}"
OUT="${GWB_INVESTIGATION_OUT:-$ROOT/artifacts/gwb/jmd-investigation}"
LIMIT="${GWB_TIMEOUT_SECONDS:-90}"
: "${GWB_INDEXED_RECEIPT:?Replay Lean #125 IndexedWorldFibreRunner on original Q207 first}"
[[ -x "$BIN" && -s "$GWB_INDEXED_RECEIPT" ]]
command -v jq >/dev/null
command -v timeout >/dev/null
mkdir -p "$OUT"
# These are immutable original book-packet coordinates, not the old rollback
# semantic state. An unavailable packet is a hold, not permission to substitute.
book="$ROOT/artifacts/gwb/v8-receipt-inspection.json"
run="$(jq -er .numeric_run_ref "$book")"
snapshot="$(jq -er .snapshot_ref "$book")"
packet="$(jq -er .packet_sha256 "$book")"
revision="$(jq -er .source_revision_ref "$book")"
book_sha="$(jq -er .source_sha256 "$book")"
doc="document:sha256:$book_sha"
consumer="${GWB_INVESTIGATION_CONSUMER:-consumer:gwb-jmd-book-elections-v1}"
jq -e '.schema=="slr.jmd-indexed-world-fibre-receipt.v1" and
    .qid=="Q207" and .inventory.article_revision=="enwiki-revid:1385850169" and
    .source_sha256=="a88ef63b9c109f0f09c2fc2a266373a9ecaae855104cb0e11c4e6c53d90d3233" and
    .source_observation_checked and (.world_truth_paid==false)' "$GWB_INDEXED_RECEIPT" >/dev/null
invoke() {
  local dest="$1"; shift
  rm -f "$OUT/$dest.tmp" "$OUT/$dest"
  timeout "$LIMIT" "$BIN" relational-family "$@" >"$OUT/$dest.tmp" 2>"$OUT/$dest.stderr"
  jq -e 'type=="object"' "$OUT/$dest.tmp" >/dev/null
  mv "$OUT/$dest.tmp" "$OUT/$dest"
}
invoke indexed-inventory.json inspect-indexed-inventory "$GWB_INDEXED_RECEIPT" "$consumer" '["article_lanes:event_candidates","wikidata_lanes:P39"]'
invoke overlap.json inspect-heterogeneous-overlap "$GWB_INDEXED_RECEIPT" "$consumer" "$run" "$doc" "$snapshot" "$packet"
jq -e '.candidate_only and (.same_manifestation_paid==false) and
  (.left_source.sha256!=.right_source.sha256) and (.licensed_crosswalk_count==0)' "$OUT/overlap.json" >/dev/null
# Original event occurrence from Lean GwbArticleSystemSoundness.electionRowMatches.
# Select by exact original occurrence, not title/word-frequency/first row.
event="source-event-candidate:abfee93630a3426fc1149fd816a5237e292f813f110bd36649554dee9e59dd1b"
jq -e --arg event "$event" '
  . as $o |
  [.left_members|to_entries[]|select(.value.original.event_candidate_ref==$event)] as $left |
  if ($left|length)!=1 then error("original election occurrence unavailable or ambiguous") else
  $left[0] as $l |
  [ .feature_groups[]|select(.left_members|index($l.key))|
     select(.axis=="predicate_dependency_surface" or .axis=="predicate_dependency_lemma") |
     .right_members[] ] | unique |
  {schema:"slr.gwb-election-retrieval.v1",indexed_member_ref:$l.value.member_ref,
    original_event_ref:$event,left_original:$l.value.original,
    candidate_right_members:[.[]|$o.right_members[.]],candidate_only:true,
    semantic_alignment_paid:false,world_truth_paid:false}
  end' "$OUT/overlap.json" >"$OUT/election-candidates.json.tmp"
mv "$OUT/election-candidates.json.tmp" "$OUT/election-candidates.json"
count="$(jq -r '.candidate_right_members|length' "$OUT/election-candidates.json")"
if [[ "$count" == 0 ]]; then
  echo "HOLD: no dependent-feature candidate in recorded book scope; not a denial or refutation."
  exit 3
fi
other="${GWB_OTHER_RELATION_REF:-}"
if [[ -z "$other" ]]; then
  if [[ "$count" != 1 ]]; then
    echo "HOLD: $count original book candidates; select a relation after reviewing their source inverses."
    exit 3
  fi
  other="$(jq -er '.candidate_right_members[0].member_ref' "$OUT/election-candidates.json")"
fi
jq -e --arg other "$other" 'any(.candidate_right_members[];.member_ref==$other)' "$OUT/election-candidates.json" >/dev/null
# Reverse-discover actual checked products from the nominated original
# occurrences before requesting any unavailable article numeric chart.
# Both source digests are pinned; any ambiguity is explicit.
article_sha="$(jq -er .source_sha256 "$GWB_INDEXED_RECEIPT")"
article_revision="$(jq -er .inventory.article_revision "$GWB_INDEXED_RECEIPT")"
article_start="$(jq -er '.left_original.predicate.start_char' "$OUT/election-candidates.json")"
article_end="$(jq -er '.left_original.predicate.end_char' "$OUT/election-candidates.json")"
article_literal="$(jq -er '.left_original.predicate.surface' "$OUT/election-candidates.json")"
book_parser="$(jq -er .parser_run_ref "$book")"
jq -e --arg other "$other" '[.candidate_right_members[]|select(.member_ref==$other)|
  .original.role_inverse[]|select(.original_role.role=="head")|.original_atom] |
  if length==1 then .[0] else error("book predicate head inverse not unique") end' \
  "$OUT/election-candidates.json" >"$OUT/book-predicate-head.json"
book_start="$(jq -er '.span[0]' "$OUT/book-predicate-head.json")"
book_end="$(jq -er '.span[1]' "$OUT/book-predicate-head.json")"
book_literal="$(jq -er '.text' "$OUT/book-predicate-head.json")"
invoke book-inverse.json discover-checked-predicate "$revision" "$book_sha" "$book_start" "$book_end" "$book_literal" "$book_parser"
if ! invoke article-inverse.json discover-checked-predicate "$article_revision" "$article_sha" "$article_start" "$article_end" "$article_literal" "${GWB_ARTICLE_PARSER_RUN_REF:---any-run}"; then
  echo "HOLD: original article native source unavailable or inconsistent; see article-inverse stderr."
  exit 3
fi
if [[ -z "${GWB_BOOK_BATCHES_JSON:-}" ]]; then
  if ! jq -e '.status=="unique_checked_candidate"' "$OUT/book-inverse.json" >/dev/null; then
    echo "HOLD: book checked predicate inverse missing/ambiguous; see book-inverse.json."
    exit 3
  fi
  GWB_BOOK_BATCHES_JSON="$OUT/book-batches.json"
  jq '[.candidates[].batch_ref]' "$OUT/book-inverse.json" >"$GWB_BOOK_BATCHES_JSON"
fi
if [[ -z "${GWB_ARTICLE_CHART_JSON:-}" ]]; then
  echo "HOLD: original article numeric packet coordinates still required; checked source alternatives in article-inverse.json."
  exit 3
fi
jq -e '.revision=="enwiki-revid:1385850169" and
  .document=="document:sha256:a88ef63b9c109f0f09c2fc2a266373a9ecaae855104cb0e11c4e6c53d90d3233" and
  (.batches|type=="array" and length>0)' "$GWB_ARTICLE_CHART_JSON" >/dev/null
indexed="$(jq -er .indexed_member_ref "$OUT/election-candidates.json")"
jq -n --arg receipt "$GWB_INDEXED_RECEIPT" --arg consumer "$consumer" --arg indexed "$indexed" \
  --arg other "$other" --arg revision "$revision" --arg run "$run" --arg document "$doc" \
  --arg snapshot "$snapshot" --arg packet "$packet" --slurpfile article "$GWB_ARTICLE_CHART_JSON" \
  --slurpfile batches "$GWB_BOOK_BATCHES_JSON" \
  '{indexed_receipt_path:$receipt,consumer_ref:$consumer,indexed_member_ref:$indexed,other_relation_ref:$other,
    article:$article[0],other:{revision:$revision,run:$run,document:$document,snapshot:$snapshot,
    packet_digest:$packet,batches:$batches[0]},context_axes:["scope","time","modality","attribution"]}' >"$OUT/selection-request.json"
invoke selection.json inspect-heterogeneous-selection "$OUT/selection-request.json"
if ! jq -e '.candidate_selector_ready and (.holds==[]) and (.world_truth_paid==false)' "$OUT/selection.json" >/dev/null; then
  echo "HOLD: source inverse not unique; see selection.json."
  exit 3
fi
if [[ "${1:-}" != "--commit" ]]; then
  echo "Candidate selection ready; explicit --commit and GWB_ACCESS_REF required for native family production."
  exit 3
fi
: "${GWB_ACCESS_REF:?Explicit existing access constraint required}"
selection="$(jq -er .selection_ref "$OUT/selection.json")"
invoke journey.json run-heterogeneous-family "$OUT/selection-request.json" "$GWB_ACCESS_REF" "$selection" --commit
family="$(jq -er .family.family_ref "$OUT/journey.json")"
jq -e '.candidate_only and (.formal_consumer_equivalence_paid==false) and
  (.family.projection.directed_comparisons|length==2) and
  (.investigation.gap_rows|length>0)' "$OUT/journey.json" >/dev/null
invoke frontier.json inspect-research-frontier "$family"
invoke local-research.json inspect-local-research "$family"
if [[ -n "${ITIR_ROOT:-}" ]]; then
  ITIR_SLR_NATIVE_EXECUTABLE="$BIN" PYTHONPATH="$ITIR_ROOT/itir-mcp/src${PYTHONPATH:+:$PYTHONPATH}" \
    GWB_FAMILY_REF="$family" timeout "$LIMIT" python3 - <<'PY' >"$OUT/itir.json.tmp"
import json, os
from itir_mcp.native_relational_tools import inspect_relational_family
out = {a: inspect_relational_family({"action": a, "family_ref": os.environ["GWB_FAMILY_REF"]})
       for a in ("family", "investigation", "research_frontier", "local_research")}
assert all(v["candidate_only"] and not v["mutates"] for v in out.values())
print(json.dumps(out, indent=2))
PY
  mv "$OUT/itir.json.tmp" "$OUT/itir.json"
fi
echo "Bounded GWB heterogeneous candidate journey emitted. HOLD: actual semantic licence, independent referee, acquisition/re-entry and complete user-story acceptance remain."
exit 3
