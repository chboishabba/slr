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
SCALE1_BIN="${SCALE1_BIN:-target/release/examples/scale1_long_document}"
BATCH_SIZE="${BATCH_SIZE:-64}"
CONFIG_JSON="${CONFIG_JSON:-{}}"
WARMUP_OUTPUT="${WARMUP_OUTPUT:-/tmp/scale1-source-replay-warmup.json}"
REPLAY_OUTPUT="${REPLAY_OUTPUT:-/tmp/scale1-source-replay-economy.json}"

cargo build --release -p sensiblaw-pg-source-store --example scale1_long_document
export SENSIBLAW_RUNTIME_HEAD="${SENSIBLAW_RUNTIME_HEAD:-$(git rev-parse HEAD)}"

run_compile() {
  local output="$1"
  "$SCALE1_BIN" compile-source     "$SOURCE_FAMILY"     "$SOURCE_TEXT"     "$SOURCE_REF"     "$PROVIDER_REF"     "$ACQUISITION_RECEIPT_REF"     "$MODEL_REF"     "$CONFIG_JSON"     "$PARSER_SCRIPT"     "$BATCH_SIZE"     > "$output"
}

# First pass may populate missing exact-stage receipts after a schema upgrade.
run_compile "$WARMUP_OUTPUT"

# The second pass is the acceptance measurement.
run_compile "$REPLAY_OUTPUT"

jq -e '
  .schema == "sensiblaw.scale1.source-compile-baseline.v0_1"
  and .parser.new_jobs == 0
  and .parser.reused_jobs == .integrity.semantic_eligible_regions
  and .integrity.candidate_persistence_reused == true
  and .integrity.l2_reconciliation_reused == true
  and .integrity.auto_event_projection_reused == true
  and .performance.downstream_candidate_persistence_reused == true
  and .performance.downstream_l2_reused == true
  and .performance.downstream_auto_reused == true
  and .integrity.unattempted_semantic_regions == 0
  and .integrity.source_region_loss_count == 0
  and .integrity.candidate_pnf_reopen_complete == true
  and .integrity.creates_semantic_authority == false
  and .integrity.applicability_promoted == false
  and .integrity.claim_truth_promoted == false
' "$REPLAY_OUTPUT"

echo "SCALE1_SOURCE_EXACT_REPLAY_ECONOMY_GREEN"
jq '{
  schema,
  source_family,
  runtime_head,
  source_revision_ref: .source.source_revision_ref,
  parser: {
    new_jobs: .parser.new_jobs,
    reused_jobs: .parser.reused_jobs
  },
  downstream_reuse: {
    candidate_persistence: .integrity.candidate_persistence_reused,
    l2_reconciliation: .integrity.l2_reconciliation_reused,
    auto_event_projection: .integrity.auto_event_projection_reused
  },
  performance: {
    total_ns: .performance.total_ns,
    candidate_persist_ns: .performance.candidate_persist_ns,
    l2_reconciliation_ns: .performance.l2_reconciliation_ns,
    auto_event_ns: .performance.auto_event_ns,
    parser_persist_ns: .performance.parser_persist_ns
  }
}' "$REPLAY_OUTPUT"
