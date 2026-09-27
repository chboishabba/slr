#!/usr/bin/env bash
set -euo pipefail

: "${DATABASE_URL:?DATABASE_URL must point to the SensibLaw PostgreSQL database}"
: "${SOURCE_FAMILY:?SOURCE_FAMILY is required}"
: "${BEFORE_TEXT:?BEFORE_TEXT must point to the pre-edit canonical UTF-8 text}"
: "${AFTER_TEXT:?AFTER_TEXT must point to the post-edit canonical UTF-8 text}"
: "${SOURCE_REF:?SOURCE_REF is required}"
: "${PROVIDER_REF:?PROVIDER_REF is required}"
: "${ACQUISITION_RECEIPT_REF:?ACQUISITION_RECEIPT_REF is required}"

MODEL_REF="${MODEL_REF:-en_core_web_sm}"
PARSER_SCRIPT="${PARSER_SCRIPT:-scripts/scale1_spacy_json_parser.py}"
SCALE1_BIN="${SCALE1_BIN:-target/release/examples/scale1_source_compiler}"
BATCH_SIZE="${BATCH_SIZE:-64}"
CONFIG_JSON="${CONFIG_JSON:-{}}"
OUT_DIR="${OUT_DIR:-/tmp/scale1-small-edit}"
mkdir -p "$OUT_DIR"

BEFORE_SEG="$OUT_DIR/before-segmentation.json"
AFTER_SEG="$OUT_DIR/after-segmentation.json"
BEFORE_RECEIPT="$OUT_DIR/before-receipt.json"
AFTER_RECEIPT="$OUT_DIR/after-receipt.json"
AUDIT="$OUT_DIR/locality-audit.json"

cargo build --release -p sensiblaw-pg-source-store --example scale1_source_compiler
python3 -m py_compile python/scale1_small_edit_locality.py

"$SCALE1_BIN" segment-source "$SOURCE_FAMILY" "$BEFORE_TEXT" > "$BEFORE_SEG"
"$SCALE1_BIN" segment-source "$SOURCE_FAMILY" "$AFTER_TEXT" > "$AFTER_SEG"

# Re-running the pre-edit revision on the current code also backfills
# parser_product_key for historical succeeded jobs before the edited revision
# asks for cross-revision reuse.
"$SCALE1_BIN" compile-source   "$SOURCE_FAMILY" "$BEFORE_TEXT" "$SOURCE_REF" "$PROVIDER_REF"   "$ACQUISITION_RECEIPT_REF" "$MODEL_REF" "$CONFIG_JSON"   "$PARSER_SCRIPT" "$BATCH_SIZE" > "$BEFORE_RECEIPT"

"$SCALE1_BIN" compile-source   "$SOURCE_FAMILY" "$AFTER_TEXT" "$SOURCE_REF" "$PROVIDER_REF"   "$ACQUISITION_RECEIPT_REF" "$MODEL_REF" "$CONFIG_JSON"   "$PARSER_SCRIPT" "$BATCH_SIZE" > "$AFTER_RECEIPT"

python3 python/scale1_small_edit_locality.py   --before-text "$BEFORE_TEXT"   --after-text "$AFTER_TEXT"   --before-segmentation "$BEFORE_SEG"   --after-segmentation "$AFTER_SEG"   --after-receipt "$AFTER_RECEIPT"   --output "$AUDIT"

echo "SCALE1_SMALL_EDIT_PARSER_LOCALITY_GREEN"
