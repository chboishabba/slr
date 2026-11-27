#!/usr/bin/env bash
# Exact original parser-run prefix only; never run the full SensibLaw compiler.
set -euo pipefail
if [[ $# -ne 2 ]]; then
  echo "usage: $0 EXACT_S29_REVIEW_ITEM_REF ORIGINAL_PARSER_RUN_REF" >&2
  exit 2
fi
review=$1
run=$2
out="native-numeric-binding-v7-endpoint.json"
checkpoint=".native-v7-endpoint-checkpoint"
mkdir -p "$checkpoint"
command -v jq >/dev/null
# Formal/golden before implementation execution, per repository constitution.
: "${DASHI_LEAN_ROOT:?Set DASHI_LEAN_ROOT to the existing Lean PR #112 checkout}"
export SLR_NATIVE_LEAN_WORKSET_EXECUTABLE="$(realpath "$DASHI_LEAN_ROOT/scripts/produce_native_pnf_workset.sh")"
"$SLR_NATIVE_LEAN_WORKSET_EXECUTABLE" --build >&2
printf '%s\n' '{"schema":"slr-native-prefix-workset-input-v1","source_sha256":"0000000000000000000000000000000000000000000000000000000000000000","numeric_run_ref":"golden","required_sentence_refs":["s1","s2","s3"],"already_published_required_refs":["s1","s3"]}' |
  "$SLR_NATIVE_LEAN_WORKSET_EXECUTABLE" |
  jq -e '.missing_required_sentence_refs == ["s2"] and (.semantic_admission_paid|not) and (.identity_paid|not)' >/dev/null
cargo test -p sensiblaw-pg-source-store --lib numeric_pnf >&2
cargo test -p sensiblaw-cli numeric_pnf_native >&2
cargo build -p sensiblaw-cli >&2
cargo build -p sensiblaw-pg-source-store --example scale1_long_document >&2
cargo run -q -p sensiblaw-cli -- numeric-pnf-native prefix-prepare \
  "$review" "$run" --commit > "$checkpoint/prepare.json.tmp"
mv "$checkpoint/prepare.json.tmp" "$checkpoint/prepare.json"
if jq -e '.newly_enqueued_or_pending > 0' "$checkpoint/prepare.json" >/dev/null; then
  timeout --kill-after=5s "${SLR_PREFIX_WORKER_TIMEOUT:-90s}" \
    cargo run -q -p sensiblaw-pg-source-store --example scale1_long_document -- \
      worker "$run" native-prefix-v7 32 "${SLR_PREFIX_PARSER_SCRIPT:-scripts/scale1_spacy_json_parser.py}" \
    > "$checkpoint/worker.json.tmp"
  mv "$checkpoint/worker.json.tmp" "$checkpoint/worker.json"
else
  printf '%s\n' '{"parser_worker_required":false}' > "$checkpoint/worker.json"
fi
cargo run -q -p sensiblaw-cli -- numeric-pnf-native endpoint \
  "$review" "$run" --commit > "$checkpoint/endpoint.json.tmp"
mv "$checkpoint/endpoint.json.tmp" "$checkpoint/endpoint.json"
# This is a physical workset receipt, not a semantic identity licence.
jq -e 'if .numeric_publication then all(.numeric_publication[]; .lean_workset_checked and ((.newly_registered_sentence_refs - .missing_required_sentence_refs)|length)==0 and (.consumer_prefix_start_bound as $bound | all(.closed_sentence_region_starts[]; . <= $bound))) else true end' "$checkpoint/endpoint.json" >/dev/null
jq -s '{schema:"slr-native-antecedent-endpoint-campaign-v7",prefix_prepare:.[0],parser_worker:.[1],endpoint:.[2]}' \
  "$checkpoint/prepare.json" "$checkpoint/worker.json" "$checkpoint/endpoint.json" > "$out.tmp"
mv "$out.tmp" "$out"
jq '{schema,outcome:.endpoint.outcome,governed_identity_paid:.endpoint.governed_identity_paid,
     level_three_paid:.endpoint.level_three_paid,observed_prefix_token_coverage:.endpoint.observed_prefix_token_coverage,
     projected_vs_original_entity_membership_parity:.endpoint.projected_vs_original_entity_membership_parity,
     exact_entity_inverse_complete:.endpoint.exact_entity_inverse_complete}' "$out"
