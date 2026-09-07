#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

if [[ "${SENSIBLAW_LIVE_LEGAL_OPT_IN:-}" != "1" ]]; then
  echo "refusing live legal acquisition: export SENSIBLAW_LIVE_LEGAL_OPT_IN=1" >&2
  exit 2
fi

export SENSIBLAW_RUNTIME_HEAD="$(git rev-parse HEAD)"
export SENSIBLAW_LIVE_RECEIPT_DIR="${SENSIBLAW_LIVE_RECEIPT_DIR:-/tmp/sensiblaw-live-legal}"

cargo run \
  -p sensiblaw-governed-legal-provider \
  --features live-network \
  --example live_austlii_smoke

python3 scripts/verify_live_legal_receipt.py \
  "$SENSIBLAW_LIVE_RECEIPT_DIR/governed-legal-acquisition-v01.json"

echo "LIVE LEGAL SMOKE PASS head=$SENSIBLAW_RUNTIME_HEAD receipt=$SENSIBLAW_LIVE_RECEIPT_DIR/governed-legal-acquisition-v01.json"
