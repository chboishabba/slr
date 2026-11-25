#!/usr/bin/env bash
# Bounded direct candidate lane, not governed/whole-relation acceptance.
# Reuse existing native SLR parser products and candidate persistence.
set -euo pipefail
if [[ $# != 4 ]]; then
  echo "usage: $0 SOURCE_REV PARSER_RUN FIRST_SENTENCE MAX_SENTENCES" >&2
  exit 2
fi
source_rev=$1; run=$2; first=$3; count=$4
command -v jq >/dev/null
command -v timeout >/dev/null
command -v sha256sum >/dev/null
cd "$(git rev-parse --show-toplevel)"
checkpoint="${SLR_NATIVE_DIRECT_CHECKPOINT:-.native-direct-candidate-checkpoint}"
mkdir -p "$checkpoint"
bounded() { timeout --kill-after=5s "${SLR_NATIVE_STEP_TIMEOUT:-90s}" "$@"; }
capture() {
  local dest=$1; shift
  bounded "$@" > "$dest.tmp"
  jq -e 'type=="object"' "$dest.tmp" >/dev/null
  mv "$dest.tmp" "$dest"
}
bounded cargo test -p sensiblaw-pg-source-store --lib numeric_pnf >&2
bounded cargo test -p sensiblaw-cli numeric_pnf_native >&2
bounded cargo build -p sensiblaw-cli >&2
cli=(target/debug/sensiblaw numeric-pnf-native)
head=$(git rev-parse HEAD)
binary=$(sha256sum target/debug/sensiblaw | cut -d' ' -f1)
manifest=$(jq -cn --arg source "$source_rev" --arg run "$run" --arg first "$first" --arg count "$count" --arg head "$head" --arg binary "$binary"   '{source_revision_ref:$source,parser_run_ref:$run,first:$first,count:$count,slr_head:$head,binary_sha256:$binary}')
if [[ -f "$checkpoint/manifest.json" ]]; then
  jq -e --argjson m "$manifest" '.==$m' "$checkpoint/manifest.json" >/dev/null
else
  printf '%s\n' "$manifest" > "$checkpoint/manifest.json.tmp"
  mv "$checkpoint/manifest.json.tmp" "$checkpoint/manifest.json"
fi
capture "$checkpoint/inspect.json" "${cli[@]}" direct-inspect "$source_rev" "$run" "$first" "$count"
capture "$checkpoint/first-stage.json" "${cli[@]}" direct-stage "$source_rev" "$run" "$first" "$count" --commit
capture "$checkpoint/warm-stage.json" "${cli[@]}" direct-stage "$source_rev" "$run" "$first" "$count" --commit
[[ "$(sha256sum target/debug/sensiblaw | cut -d' ' -f1)" == "$binary" ]]
jq -s --argjson manifest "$manifest" '
  def candidate_gate:
    .schema=="slr-direct-numeric-candidate-slice-v1" and
    .source_revision_ref==$manifest.source_revision_ref and
    .original_parser_run_ref==$manifest.parser_run_ref and
    .first_sentence==($manifest.first|tonumber) and
    (.source_inventory_sha256|type=="string" and length==64) and
    (.sentences|length)>0 and
    .next_sentence==(.first_sentence+(.sentences|length)) and
    (.sentences|length)<=($manifest.count|tonumber) and
    .next_sentence<=.represented_sentence_count and
    .sentence_local_database_crossings==0 and
    .production_parser_token_writes==0 and
    .candidate_only==true and .governed_identity_paid==false and
    .level_three_paid==false and .direct_production_activation_paid==false;
  def staged:
    .schema=="slr-direct-numeric-staging-v1" and
    .compiler_executable_sha256==$manifest.binary_sha256 and
    .committed==true and .creates_semantic_authority==false and
    .authority_publication_paid==false and
    .physical_candidate_row_mutations==.inserted_candidate_products and
    .locally_recomputed_sentences==.inserted_candidate_products and
    (.inserted_candidate_products+.unchanged_candidate_products_skipped)==(.candidate.sentences|length);
  {schema:"slr-direct-candidate-stage-acceptance-v1",manifest:$manifest,
   exact_candidate_readback:(.[0]==.[1].candidate and .[0]==.[2].candidate),
   nonvacuous_candidate_scope:(.[0]|candidate_gate),
   first_stage_committed:(.[1]|staged),
   warm_stage_committed:(.[2]|staged),
   warm_zero_rule_solves_and_mutations:(.[2].locally_recomputed_sentences==0 and
       .[2].physical_candidate_row_mutations==0 and
       .[2].unchanged_candidate_products_skipped==(.[0].sentences|length)),
   source_observation_tokens_revalidated:.[2].candidate.source_observation_tokens_loaded,
   whole_relation_parity_paid:false,governed_identity_paid:false,
   authority_publication_paid:false,direct_production_activation_paid:false}
  | . + {direct_candidate_staging_paid:(.exact_candidate_readback and .nonvacuous_candidate_scope and
     .first_stage_committed and .warm_stage_committed and .warm_zero_rule_solves_and_mutations)}
' "$checkpoint/inspect.json" "$checkpoint/first-stage.json" "$checkpoint/warm-stage.json" > "$checkpoint/acceptance.json.tmp"
mv "$checkpoint/acceptance.json.tmp" "$checkpoint/acceptance.json"
cat "$checkpoint/acceptance.json"
jq -e '.direct_candidate_staging_paid==true' "$checkpoint/acceptance.json" >/dev/null
