#!/usr/bin/env bash
set -euo pipefail

: "${COLD_DATABASE_URL:?COLD_DATABASE_URL must point to an isolated migrated SensibLaw database}"
: "${SOURCE_FAMILY:?SOURCE_FAMILY is required}"
: "${SOURCE_TEXT:?SOURCE_TEXT must point to canonical UTF-8 text}"
: "${SOURCE_REF:?SOURCE_REF is required}"
: "${PROVIDER_REF:?PROVIDER_REF is required}"
: "${ACQUISITION_RECEIPT_REF:?ACQUISITION_RECEIPT_REF is required}"

MODEL_REF="${MODEL_REF:-en_core_web_sm}"
PARSER_SCRIPT="${PARSER_SCRIPT:-scripts/scale1_spacy_json_parser.py}"
SCALE1_BIN="${SCALE1_BIN:-target/release/examples/scale1_source_compiler}"
BATCH_SIZE="${BATCH_SIZE:-64}"
CONFIG_JSON="${CONFIG_JSON:-{}}"
OUTPUT="${OUTPUT:-/tmp/scale1-cold-source-baseline.json}"

cargo build --release -p sensiblaw-pg-source-store --example scale1_source_compiler

# The cold benchmark is deliberately isolated from the established corpus.
# The caller owns database creation/migration; this gate owns only execution.
export DATABASE_URL="$COLD_DATABASE_URL"
export SENSIBLAW_RUNTIME_HEAD="${SENSIBLAW_RUNTIME_HEAD:-$(git rev-parse HEAD)}"

"$SCALE1_BIN" compile-source   "$SOURCE_FAMILY" "$SOURCE_TEXT" "$SOURCE_REF" "$PROVIDER_REF"   "$ACQUISITION_RECEIPT_REF" "$MODEL_REF" "$CONFIG_JSON"   "$PARSER_SCRIPT" "$BATCH_SIZE" > "$OUTPUT"

jq -e '
  .schema == "sensiblaw.scale1.source-compile-baseline.v0_1"
  and .parser.new_jobs == .integrity.semantic_eligible_regions
  and .parser.same_revision_reused_jobs == 0
  and .parser.cross_revision_reused_jobs == 0
  and .integrity.candidate_persistence_reused == false
  and .integrity.l2_reconciliation_reused == false
  and .integrity.auto_event_projection_reused == false
  and .integrity.unattempted_semantic_regions == 0
  and .integrity.source_region_loss_count == 0
  and .integrity.candidate_pnf_reopen_complete == true
  and .integrity.creates_semantic_authority == false
  and .integrity.applicability_promoted == false
  and .integrity.claim_truth_promoted == false
' "$OUTPUT"

echo "SCALE1_COLD_SOURCE_BASELINE_GREEN"
jq '{
  runtime_head,
  source_family,
  source_revision_ref: .source.source_revision_ref,
  wall_ns: .performance.total_ns,
  parser: {
    new_jobs: .parser.new_jobs,
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
    factor_rows_scanned: .candidate_cardinality.l2_factor_rows_scanned_this_run,
    proposition_occurrence_rows_inserted: .candidate_cardinality.l2_proposition_occurrence_rows_inserted_this_run,
    event_occurrence_rows_inserted: .candidate_cardinality.l2_event_occurrence_rows_inserted_this_run,
    pressure_rows_upserted: .candidate_cardinality.l2_pressure_rows_upserted_this_run
  }
}' "$OUTPUT"
