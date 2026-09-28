#!/usr/bin/env bash
set -euo pipefail

: "${DATABASE_URL:?DATABASE_URL must point to the migrated SensibLaw PostgreSQL database}"
: "${SOURCE_FAMILY:?SOURCE_FAMILY is required}"
: "${SOURCE_TEXT:?SOURCE_TEXT must point to canonical UTF-8 text for the selected adapter}"
: "${SOURCE_REF:?SOURCE_REF is required}"
: "${PROVIDER_REF:?PROVIDER_REF is required}"
: "${ACQUISITION_RECEIPT_REF:?ACQUISITION_RECEIPT_REF is required}"

MODEL_REF="${MODEL_REF:-en_core_web_sm}"
PARSER_SCRIPT="${PARSER_SCRIPT:-scripts/scale1_spacy_json_parser.py}"
SCALE1_BIN="${SCALE1_BIN:-target/release/examples/scale1_source_compiler}"
BATCH_SIZE="${BATCH_SIZE:-64}"
CONFIG_JSON="${CONFIG_JSON:-{}}"
OUTPUT="${OUTPUT:-/tmp/scale1-same-run-parser-relative.json}"
TARGET_RATIO="${TARGET_RATIO:-0.1}"

cargo build --release -p sensiblaw-pg-source-store --example scale1_source_compiler
export SENSIBLAW_RUNTIME_HEAD="${SENSIBLAW_RUNTIME_HEAD:-$(git rev-parse HEAD)}"

"$SCALE1_BIN" compile-source   "$SOURCE_FAMILY"   "$SOURCE_TEXT"   "$SOURCE_REF"   "$PROVIDER_REF"   "$ACQUISITION_RECEIPT_REF"   "$MODEL_REF"   "$CONFIG_JSON"   "$PARSER_SCRIPT"   "$BATCH_SIZE"   > "$OUTPUT"

jq -e --argjson target "$TARGET_RATIO" '
  .schema == "sensiblaw.scale1.source-compile-baseline.v0_1"
  and .performance.same_run_parser_relative_metrics_available == true
  and .performance.parser_process_ns > 0
  and .parser.new_jobs == .integrity.semantic_eligible_regions
  and .parser.reused_jobs == 0
  and .performance.full_post_spacy_to_spacy_ratio != null
  and .performance.semantic_post_parser_to_spacy_ratio != null
  and .performance.post_spacy_target_ratio == $target
  and .performance.full_post_spacy_to_spacy_ratio <= $target
  and .performance.post_spacy_target_met == true
  and .candidate_cardinality.review_delta_fibre_input_used == true
  and .candidate_cardinality.review_delta_fibre_count
      == .candidate_cardinality.review_target_fibre_count
  and .candidate_cardinality.review_occurrence_lookup_count <= 1
  and .integrity.unattempted_semantic_regions == 0
  and .integrity.source_region_loss_count == 0
  and .integrity.candidate_pnf_reopen_complete == true
  and .integrity.creates_semantic_authority == false
  and .integrity.applicability_promoted == false
  and .integrity.claim_truth_promoted == false
' "$OUTPUT"

echo "SCALE1_SAME_RUN_PARSER_RELATIVE_GREEN"
jq '{
  runtime_head,
  source_family,
  source_revision_ref: .source.source_revision_ref,
  semantic_regions: .integrity.semantic_eligible_regions,
  parser: {
    process_ns: .performance.parser_process_ns,
    persist_ns: .performance.parser_persist_ns,
    new_jobs: .parser.new_jobs,
    reused_jobs: .parser.reused_jobs
  },
  post_parser: {
    semantic_ns: .performance.semantic_post_parser_ns,
    full_post_spacy_ns: .performance.full_post_spacy_ns,
    semantic_to_spacy_ratio: .performance.semantic_post_parser_to_spacy_ratio,
    full_to_spacy_ratio: .performance.full_post_spacy_to_spacy_ratio,
    target_ratio: .performance.post_spacy_target_ratio,
    target_met: .performance.post_spacy_target_met
  },
  review: {
    delta_fibre_input_used: .candidate_cardinality.review_delta_fibre_input_used,
    delta_fibre_count: .candidate_cardinality.review_delta_fibre_count,
    target_fibre_count: .candidate_cardinality.review_target_fibre_count,
    projection_ns: .performance.review_projection_ns,
    input_identity_ns: .performance.review_input_identity_ns,
    materialize_ns: .performance.review_materialize_ns
  }
}' "$OUTPUT"
