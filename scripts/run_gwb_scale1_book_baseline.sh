#!/usr/bin/env bash
set -euo pipefail

: "${DATABASE_URL:?DATABASE_URL must point to the migrated SensibLaw PostgreSQL database}"
: "${GWB_BOOK:?GWB_BOOK must point to the 277-page GWB PDF/EPUB}"

MODEL_REF="${MODEL_REF:-en_core_web_sm}"
PARSER_SCRIPT="${PARSER_SCRIPT:-scripts/scale1_spacy_json_parser.py}"
SCALE1_BIN="${SCALE1_BIN:-target/release/examples/scale1_long_document}"
OUTPUT="${OUTPUT:-/tmp/gwb-scale1-book-baseline.json}"
BATCH_SIZE="${BATCH_SIZE:-32}"
PYTHON_BIN="${PYTHON_BIN:-python3}"
export PYTHON_BIN

cargo build --release -p sensiblaw-pg-source-store --example scale1_long_document
"$PYTHON_BIN" -m py_compile python/gwb_scale1_book_baseline.py
"$PYTHON_BIN" -m py_compile scripts/scale1_spacy_json_parser.py

export SENSIBLAW_RUNTIME_HEAD="${SENSIBLAW_RUNTIME_HEAD:-$(git rev-parse HEAD)}"

"$PYTHON_BIN" python/gwb_scale1_book_baseline.py   --book "$GWB_BOOK"   --scale1-bin "$SCALE1_BIN"   --model "$MODEL_REF"   --parser-script "$PARSER_SCRIPT"   --batch-size "$BATCH_SIZE"   --output "$OUTPUT"   > /tmp/gwb-scale1-book-baseline.stdout.json

jq -e '
  .invariants.canonical_projection_streamed_directly == true
  and .invariants.tsv_runtime_state_required == false
  and .invariants.projected_text_runtime_file_required == false
  and .invariants.canonical_projection_digest_matches_pg == true
  and .invariants.semantic_region_partition_complete == true
  and .invariants.zero_source_region_loss == true
  and .invariants.zero_unattempted_semantic_regions == true
  and .invariants.semantic_authority_created == false
  and .invariants.applicability_promoted == false
  and .invariants.claim_truth_promoted == false
  and .compiler_receipt.integrity.every_region_reloaded == true
  and .compiler_receipt.integrity.candidate_pnf_reopen_complete == true
' "$OUTPUT"

echo "GWB_SCALE1_BOOK_BASELINE_GREEN"
jq '{
  book: .book,
  source_revision_ref: .compiler_receipt.source.source_revision_ref,
  integrity: .compiler_receipt.integrity,
  parser: .compiler_receipt.parser,
  performance: .compiler_receipt.performance,
  candidate_cardinality: .compiler_receipt.candidate_cardinality
}' "$OUTPUT"
