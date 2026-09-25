#!/usr/bin/env bash
set -euo pipefail

: "${DATABASE_URL:?DATABASE_URL must point to the migrated SensibLaw PostgreSQL database}"
: "${GWB_BOOK:?GWB_BOOK must point to the retained GWB PDF/EPUB specimen}"

MODEL_REF="${MODEL_REF:-en_core_web_sm}"
PARSER_SCRIPT="${PARSER_SCRIPT:-scripts/scale1_spacy_json_parser.py}"
SCALE1_BIN="${SCALE1_BIN:-target/release/examples/scale1_long_document}"
BATCH_SIZE="${BATCH_SIZE:-64}"
WARMUP_OUTPUT="${WARMUP_OUTPUT:-/tmp/gwb-scale1-replay-warmup.json}"
REPLAY_OUTPUT="${REPLAY_OUTPUT:-/tmp/gwb-scale1-replay-economy.json}"

cargo build --release -p sensiblaw-pg-source-store --example scale1_long_document
python3 -m py_compile python/gwb_scale1_book_baseline.py

export SENSIBLAW_RUNTIME_HEAD="${SENSIBLAW_RUNTIME_HEAD:-$(git rev-parse HEAD)}"

run_baseline() {
  local output="$1"
  python3 python/gwb_scale1_book_baseline.py     --book "$GWB_BOOK"     --scale1-bin "$SCALE1_BIN"     --model "$MODEL_REF"     --parser-script "$PARSER_SCRIPT"     --batch-size "$BATCH_SIZE"     --output "$output"     >/dev/null
}

# First pass upgrades an existing pre-reuse database by materialising the new
# exact-stage receipts. On an already upgraded DB this pass should itself reuse.
run_baseline "$WARMUP_OUTPUT"

# Second pass is the actual exact-replay economy acceptance measurement.
run_baseline "$REPLAY_OUTPUT"

jq -e '
  .compiler_receipt.parser.new_jobs == 0
  and .compiler_receipt.parser.reused_jobs
      == .compiler_receipt.integrity.semantic_eligible_regions
  and .compiler_receipt.integrity.candidate_persistence_reused == true
  and .compiler_receipt.integrity.l2_reconciliation_reused == true
  and .compiler_receipt.integrity.auto_event_projection_reused == true
  and .compiler_receipt.performance.downstream_candidate_persistence_reused == true
  and .compiler_receipt.performance.downstream_l2_reused == true
  and .compiler_receipt.performance.downstream_auto_reused == true
  and .compiler_receipt.integrity.unattempted_semantic_regions == 0
  and .compiler_receipt.integrity.source_region_loss_count == 0
  and .compiler_receipt.integrity.candidate_pnf_reopen_complete == true
  and .compiler_receipt.integrity.creates_semantic_authority == false
  and .compiler_receipt.integrity.applicability_promoted == false
  and .compiler_receipt.integrity.claim_truth_promoted == false
' "$REPLAY_OUTPUT"

echo "SCALE1_EXACT_REPLAY_ECONOMY_GREEN"
jq '{
  runtime_head: .compiler_receipt.runtime_head,
  source_revision_ref: .compiler_receipt.source.source_revision_ref,
  parser: {
    new_jobs: .compiler_receipt.parser.new_jobs,
    reused_jobs: .compiler_receipt.parser.reused_jobs
  },
  downstream_reuse: {
    candidate_persistence: .compiler_receipt.integrity.candidate_persistence_reused,
    l2_reconciliation: .compiler_receipt.integrity.l2_reconciliation_reused,
    auto_event_projection: .compiler_receipt.integrity.auto_event_projection_reused
  },
  performance: {
    total_ns: .compiler_receipt.performance.total_ns,
    candidate_persist_ns: .compiler_receipt.performance.candidate_persist_ns,
    l2_reconciliation_ns: .compiler_receipt.performance.l2_reconciliation_ns,
    auto_event_ns: .compiler_receipt.performance.auto_event_ns,
    review_projection_ns: .compiler_receipt.performance.review_projection_ns,
    reload_verify_ns: .compiler_receipt.performance.reload_verify_ns
  }
}' "$REPLAY_OUTPUT"
