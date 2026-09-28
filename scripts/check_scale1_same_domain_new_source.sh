#!/usr/bin/env bash
set -euo pipefail

: "${DATABASE_URL:?DATABASE_URL must point to the established SensibLaw corpus}"
: "${SOURCE_FAMILY:?SOURCE_FAMILY is required}"
: "${SOURCE_TEXT:?SOURCE_TEXT must point to the new canonical UTF-8 source}"
: "${SOURCE_REF:?SOURCE_REF must identify a genuinely new source}"
: "${PROVIDER_REF:?PROVIDER_REF is required}"
: "${ACQUISITION_RECEIPT_REF:?ACQUISITION_RECEIPT_REF is required}"

MODEL_REF="${MODEL_REF:-en_core_web_sm}"
PARSER_SCRIPT="${PARSER_SCRIPT:-scripts/scale1_spacy_json_parser.py}"
SCALE1_BIN="${SCALE1_BIN:-target/release/examples/scale1_source_compiler}"
BATCH_SIZE="${BATCH_SIZE:-64}"
CONFIG_JSON="${CONFIG_JSON:-{}}"
OUTPUT="${OUTPUT:-/tmp/scale1-same-domain-new-source.json}"
MIN_REUSE_HITS="${MIN_REUSE_HITS:-1}"

cargo build --release -p sensiblaw-pg-source-store --example scale1_source_compiler
export SENSIBLAW_RUNTIME_HEAD="${SENSIBLAW_RUNTIME_HEAD:-$(git rev-parse HEAD)}"

"$SCALE1_BIN" compile-source   "$SOURCE_FAMILY" "$SOURCE_TEXT" "$SOURCE_REF" "$PROVIDER_REF"   "$ACQUISITION_RECEIPT_REF" "$MODEL_REF" "$CONFIG_JSON"   "$PARSER_SCRIPT" "$BATCH_SIZE" > "$OUTPUT"

# This benchmark is neither cold nor replay.  Same-revision reuse would mean
# the supplied source was not actually new. Cross-revision/content-product
# reuse is permitted and is one of the measurements we want.
jq -e --argjson min_reuse "$MIN_REUSE_HITS" '
  .schema == "sensiblaw.scale1.source-compile-baseline.v0_1"
  and .parser.same_revision_reused_jobs == 0
  and .integrity.candidate_persistence_reused == false
  and .integrity.l2_reconciliation_reused == false
  and .integrity.auto_event_projection_reused == false
  and .performance.candidate_product_reuse_hits_this_run >= $min_reuse
  and .candidate_cardinality.l2_product_summary_reuse_hits_this_run
      >= .performance.candidate_product_reuse_hits_this_run
  and .candidate_cardinality.l2_product_summaries_created_this_run
      <= .performance.candidate_product_new_this_run
  and .candidate_cardinality.l2_factor_rows_scanned_this_run
      <= .performance.candidate_product_factor_rows_inserted_this_run
  and .candidate_cardinality.review_occurrence_lookup_count <= 1
  and .candidate_cardinality.review_pressure_rows_scanned
      <= .candidate_cardinality.review_target_fibre_count
  and .candidate_cardinality.review_contestation_rows_scanned
      <= (2 * (.candidate_cardinality.review_target_fibre_count | if . < 1 then 1 else . end))
  and .integrity.unattempted_semantic_regions == 0
  and .integrity.source_region_loss_count == 0
  and .integrity.candidate_pnf_reopen_complete == true
  and .integrity.creates_semantic_authority == false
  and .integrity.applicability_promoted == false
  and .integrity.claim_truth_promoted == false
' "$OUTPUT"

echo "SCALE1_SAME_DOMAIN_NEW_SOURCE_GREEN"
jq '{
  runtime_head,
  source_family,
  source_revision_ref: .source.source_revision_ref,
  wall_ns: .performance.total_ns,
  parser_work: {
    new_jobs: .parser.new_jobs,
    cross_revision_reused_jobs: .parser.cross_revision_reused_jobs,
    p50_ns: .performance.parser_job_p50_ns,
    p95_ns: .performance.parser_job_p95_ns,
    p99_ns: .performance.parser_job_p99_ns,
    max_ns: .performance.parser_job_max_ns,
    c1: .performance.parser_job_c1,
    c10: .performance.parser_job_c10
  },
  candidate_work: {
    product_reuse_hits: .integrity.candidate_product_reuse_hits_this_run,
    new_products: .integrity.candidate_product_new_this_run,
    product_factor_rows_inserted: .integrity.candidate_product_factor_rows_inserted_this_run,
    source_statement_rows_inserted: .integrity.source_statement_rows_inserted_this_run,
    candidate_batch_rows_inserted: .integrity.candidate_batch_rows_inserted_this_run
  },
  l2_work: {
    product_summary_reuse_hits: .candidate_cardinality.l2_product_summary_reuse_hits_this_run,
    product_summaries_created: .candidate_cardinality.l2_product_summaries_created_this_run,
    factor_rows_scanned: .candidate_cardinality.l2_factor_rows_scanned_this_run,
    entity_mention_rows_inserted: .candidate_cardinality.l2_entity_mention_rows_inserted_this_run,
    proposition_occurrence_rows_inserted: .candidate_cardinality.l2_proposition_occurrence_rows_inserted_this_run,
    event_occurrence_rows_inserted: .candidate_cardinality.l2_event_occurrence_rows_inserted_this_run,
    pressure_rows_upserted: .candidate_cardinality.l2_pressure_rows_upserted_this_run
  },
  review_work: {
    target_fibre_count: .candidate_cardinality.review_target_fibre_count,
    pressure_rows_scanned: .candidate_cardinality.review_pressure_rows_scanned,
    contestation_rows_scanned: .candidate_cardinality.review_contestation_rows_scanned,
    occurrence_rows_scanned: .candidate_cardinality.review_occurrence_rows_scanned,
    occurrence_lookup_count: .candidate_cardinality.review_occurrence_lookup_count,
    items_persist_attempted: .candidate_cardinality.review_items_persist_attempted
  }
}' "$OUTPUT"
