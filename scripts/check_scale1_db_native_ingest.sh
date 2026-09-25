#!/usr/bin/env bash
set -euo pipefail

: "${DATABASE_URL:?DATABASE_URL must point to a disposable PostgreSQL database}"

PYTHON_BIN="${PYTHON_BIN:-python3}"
MODEL_REF="${MODEL_REF:-en_core_web_sm}"
SCALE1_BIN="${SCALE1_BIN:-target/debug/examples/scale1_long_document}"
PARSER_SCRIPT="${PARSER_SCRIPT:-scripts/scale1_spacy_json_parser.py}"

"$PYTHON_BIN" -m py_compile "$PARSER_SCRIPT"
"$PYTHON_BIN" -m py_compile python/gwb_scale1_ingest.py

cargo build -p sensiblaw-pg-source-store --example scale1_long_document
cargo test -p sensiblaw-pg-source-store

cat > /tmp/scale1-source.txt <<'EOF'
Alice called Bob on January 1, 1997. Alice did not call Bob on January 1, 1997. Alice called Bob on January 1, 1997.
EOF

"$SCALE1_BIN" prepare-stdin   source:scale1:ci   provider:scale1:ci   acquisition:scale1:ci   scale1-ci-document   "$MODEL_REF"   '{}'   "$PARSER_SCRIPT"   < /tmp/scale1-source.txt   > /tmp/scale1-prepare.json

jq -e '.input_transport == "stdin"' /tmp/scale1-prepare.json
jq -e '.creates_semantic_authority == false and .claim_truth_promoted == false'   /tmp/scale1-prepare.json

run_ref="$(jq -r '.parser_run_ref' /tmp/scale1-prepare.json)"
"$SCALE1_BIN" worker   "$run_ref" worker:scale1:ci 16 "$PARSER_SCRIPT"   > /tmp/scale1-worker.json

jq -e '.unattempted_semantic_regions == 0 and .queued == 0 and .leased == 0'   /tmp/scale1-worker.json

"$SCALE1_BIN" finalize "$run_ref" > /tmp/scale1-final.json

jq -e '
  .unattempted_semantic_regions == 0
  and .source_region_loss_count == 0
  and .canonical_bytes_reload_identically == true
  and .every_region_reloaded == true
  and .candidate_pnf_reopen_complete == true
  and .compiled_statement_count > 0
  and .candidate_pnf_count > 0
  and .named_entity_mention_candidates > 0
  and .temporal_mention_candidates > 0
  and .proposition_occurrence_candidates > 0
  and .event_occurrence_candidates > 0
  and .reconciliation_review_items > 0
  and .reconciliation_creates_entity_identity == false
  and .reconciliation_creates_proposition_identity == false
  and .reconciliation_creates_event_identity == false
  and .reconciliation_review_creates_event_identity == false
  and .reconciliation_review_creates_semantic_authority == false
  and .creates_semantic_authority == false
  and .applicability_promoted == false
  and .claim_truth_promoted == false
' /tmp/scale1-final.json

echo "SCALE1_DB_NATIVE_GREEN"
cat /tmp/scale1-final.json


review_item_ref="$(jq -r '.reconciliation_review_item_refs[] | select(contains("proposition-fingerprint:"))' /tmp/scale1-final.json | head -n 1)"
if [[ -z "$review_item_ref" ]]; then
  echo "no proposition reconciliation review item available for L3 smoke" >&2
  exit 1
fi

"$SCALE1_BIN" review   "$review_item_ref"   accept   reviewer:scale1:ci   > /tmp/scale1-review.json

jq -e '
  .current_status == "accepted"
  and .creates_semantic_authority == false
  and .applicability_promoted == false
  and .claim_truth_promoted == false
' /tmp/scale1-review.json

accepted_command_ref="$(jq -r '.command_ref' /tmp/scale1-review.json)"
"$SCALE1_BIN" materialize-proposition   "$review_item_ref"   "$accepted_command_ref"   > /tmp/scale1-materialized-proposition.json

jq -e '
  .reviewed_grouping_identity == true
  and .claim_review_state_unreviewed == true
  and .grouping_review_is_claim_review == false
  and (.claim_refs | length) > 0
  and .creates_semantic_authority == false
  and .applicability_promoted == false
  and .claim_truth_promoted == false
' /tmp/scale1-materialized-proposition.json

echo "SCALE1_L3_REVIEWED_GROUPING_GREEN"
cat /tmp/scale1-materialized-proposition.json
