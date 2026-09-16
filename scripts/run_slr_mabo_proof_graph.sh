#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo "usage: $0 admitted-legal-ir.slri [output-dir]" >&2
  exit 2
fi

SLRI_STREAM=$1
OUT_DIR=${2:-/tmp/slr-mabo-proof-graph}
ENV_FILE=${SLR_WORLD_ENV_FILE:-.env}
MATERIALIZER_BIN=${SLR_LEGAL_IR_MATERIALIZER_BIN:-sensiblaw-legal-ir-materializer}

SUBJECT_REF=${SLR_MABO_SUBJECT_REF:-mabo:radical-title-native-title}
SOURCE_REVISION_REF=${SLR_MABO_SOURCE_REVISION_REF:-source-revision:mabo:1992:hca:23:wikisource:page-39:rev-16058297:2026-06-29}
SPAN_REF=${SLR_MABO_SPAN_REF:-span:mabo:brennan:radical-title:no-automatic-beneficial-ownership}

if [[ ! -s "$SLRI_STREAM" ]]; then
  echo "mabo-legal-ir-stream-unavailable" >&2
  exit 3
fi

if ! command -v "$MATERIALIZER_BIN" >/dev/null 2>&1 && [[ ! -x "$MATERIALIZER_BIN" ]]; then
  echo "rust-legal-ir-materializer-unavailable" >&2
  exit 4
fi

mkdir -p "$OUT_DIR"
INGEST_RECEIPT="$OUT_DIR/legal-ir-materialisation-receipt.txt"
WELD_RECEIPT="$OUT_DIR/mabo-source-weld-receipt.txt"
STDERR_FILE="$OUT_DIR/mabo-legal-ir.stderr"

"$MATERIALIZER_BIN" ingest \
  --input "$SLRI_STREAM" \
  --env-file "$ENV_FILE" \
  >"$INGEST_RECEIPT" 2>"$STDERR_FILE"

"$MATERIALIZER_BIN" weld \
  --subject-ref "$SUBJECT_REF" \
  --source-revision-ref "$SOURCE_REVISION_REF" \
  --span-ref "$SPAN_REF" \
  --env-file "$ENV_FILE" \
  >"$WELD_RECEIPT" 2>>"$STDERR_FILE"

printf '%s\n' \
  "SLR_MABO_PROOF_GRAPH_RECEIPT binary_legal_ir=true exact_source_required=true proposition_truth_paid=false applicability_paid=false python_legal_semantics=false" \
  >"$OUT_DIR/mabo-proof-graph-receipt.txt"
